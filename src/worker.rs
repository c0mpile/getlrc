use crate::{
    api::LrcLibClient,
    cache::{signature::TrackSignature, NegativeCache},
    messages::{UiMessage, WorkerMessage},
    scanner::{self, metadata},
    session::{PersistentSession, StatusType},
};
use anyhow::Result;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;

const RATE_LIMIT_MS: u64 = 100; // 10 requests/second max

pub async fn run(
    target_dir: PathBuf,
    cache: NegativeCache,
    tx: mpsc::UnboundedSender<WorkerMessage>,
    mut ui_rx: mpsc::UnboundedReceiver<UiMessage>,
    session_path: PathBuf,
    mut session: Option<PersistentSession>,
) -> Result<()> {
    tracing::info!("Worker started for directory: {}", target_dir.display());

    // Track if we're resuming from a session
    let is_resuming = session.is_some();

    // Determine if we're resuming or starting fresh
    let (mut files_to_process, mut downloaded, mut cached, existing, mut failed) =
        if let Some(ref sess) = session {
            tracing::info!(
                "Resuming from saved session with {} pending files",
                sess.pending_files.len()
            );

            // Send restoring message to TUI
            tx.send(WorkerMessage::SessionRestoring)?;

            // Restore counts
            let downloaded = sess.downloaded_count;
            let cached = sess.cached_count;
            let existing = sess.existing_count;
            let failed = sess.failed_count;

            // Restore log history
            for entry in &sess.log_history {
                tx.send(WorkerMessage::LogRestore {
                    filename: entry.filename.clone(),
                    status: entry.status.clone(),
                })?;
            }

            (
                sess.pending_files.clone(),
                downloaded,
                cached,
                existing,
                failed,
            )
        } else {
            // Fresh scan
            tracing::info!("Starting fresh scan");
            let all_audio_files: Vec<PathBuf> = scanner::walk_directory(&target_dir).collect();

            let mut files_to_process = Vec::new();
            let mut files_with_lrc = Vec::new();

            for path in all_audio_files {
                if scanner::has_lrc_sidecar(&path) {
                    files_with_lrc.push(path);
                } else {
                    files_to_process.push(path);
                }
            }

            // Send messages for files that already have .lrc
            let existing_count = files_with_lrc.len();
            for path in files_with_lrc {
                tx.send(WorkerMessage::AlreadyHasLrc {
                    path: path.display().to_string(),
                })?;
            }

            // Create new session
            session = Some(PersistentSession::new(
                target_dir.clone(),
                files_to_process.clone(),
            ));

            (files_to_process, 0, 0, existing_count, 0)
        };

    let total_files = files_to_process.len() + existing;
    tx.send(WorkerMessage::ScanStarted { total_files })?;

    let client = LrcLibClient::new();

    // Start in paused state if resuming from a session
    let mut paused = is_resuming;

    if is_resuming {
        tracing::info!("Worker initialized in paused state (resuming session)");
    }

    while let Some(path) = files_to_process.first().cloned() {
        // Check for pause/resume/quit messages
        while let Ok(msg) = ui_rx.try_recv() {
            match msg {
                UiMessage::Pause => {
                    paused = true;
                    tracing::info!("Worker paused");

                    // Save session state
                    if let Some(ref mut sess) = session {
                        sess.pending_files = files_to_process.clone();
                        sess.downloaded_count = downloaded;
                        sess.cached_count = cached;
                        sess.existing_count = existing;
                        sess.failed_count = failed;

                        if let Err(e) = sess.save(&session_path) {
                            tracing::error!("Failed to save session: {}", e);
                        }
                    }
                }
                UiMessage::Resume => {
                    paused = false;
                    tracing::info!("Worker resumed");
                }
                UiMessage::Quit => {
                    tracing::info!("Worker received quit signal");

                    // Save session if paused
                    if paused {
                        if let Some(ref mut sess) = session {
                            sess.pending_files = files_to_process.clone();
                            sess.downloaded_count = downloaded;
                            sess.cached_count = cached;
                            sess.existing_count = existing;
                            sess.failed_count = failed;

                            if let Err(e) = sess.save(&session_path) {
                                tracing::error!("Failed to save session on quit: {}", e);
                            }
                        }
                    }
                    return Ok(());
                }
            }
        }

        // Wait while paused
        while paused {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            if let Ok(msg) = ui_rx.try_recv() {
                match msg {
                    UiMessage::Resume => {
                        paused = false;
                        tracing::info!("Worker resumed");
                    }
                    UiMessage::Quit => {
                        tracing::info!("Worker received quit signal while paused");
                        return Ok(());
                    }
                    _ => {}
                }
            }
        }

        // Remove the file from the queue (we're processing it now)
        files_to_process.remove(0);

        // Extract metadata
        let track = match metadata::extract(&path) {
            Ok(t) => t,
            Err(e) => {
                let filename = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                tx.send(WorkerMessage::Error {
                    path: path.display().to_string(),
                    error: e.to_string(),
                })?;

                failed += 1;
                if let Some(ref mut sess) = session {
                    sess.add_log(filename, StatusType::Error);
                }
                continue;
            }
        };

        tx.send(WorkerMessage::TrackProcessing {
            track: track.clone(),
        })?;

        // Generate signature
        let signature = TrackSignature {
            artist: track.artist.clone(),
            title: track.title.clone(),
            album: Some(track.album.clone()),
            duration_sec: track.duration_secs as u32,
        };
        let sig_hash = signature.generate_hash();

        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Phase 2: Check negative cache
        if cache.is_cached(&sig_hash)? {
            tx.send(WorkerMessage::CacheHit {
                path: path.display().to_string(),
            })?;
            cached += 1;
            if let Some(ref mut sess) = session {
                sess.add_log(filename, StatusType::Cached);
            }
            continue;
        }

        // Phase 3: Fetch lyrics with rate limiting
        tokio::time::sleep(tokio::time::Duration::from_millis(RATE_LIMIT_MS)).await;

        match client.get_lyrics(&track).await {
            Ok(Some(lyrics)) => {
                // Write .lrc file
                if let Some(synced) = lyrics.synced_lyrics {
                    if let Err(e) = write_lrc_file(&path, &synced) {
                        tx.send(WorkerMessage::Error {
                            path: path.display().to_string(),
                            error: e.to_string(),
                        })?;
                        failed += 1;
                        if let Some(ref mut sess) = session {
                            sess.add_log(filename, StatusType::Error);
                        }
                    } else {
                        tx.send(WorkerMessage::LyricsFound {
                            path: path.display().to_string(),
                        })?;
                        downloaded += 1;
                        if let Some(ref mut sess) = session {
                            sess.add_log(filename, StatusType::Downloaded);
                        }
                    }
                } else {
                    // No synced lyrics, add to negative cache
                    cache.add(&sig_hash)?;
                    tx.send(WorkerMessage::LyricsNotFound {
                        path: path.display().to_string(),
                    })?;
                    failed += 1;
                    if let Some(ref mut sess) = session {
                        sess.add_log(filename, StatusType::NotFound);
                    }
                }
            }
            Ok(None) => {
                // 404 - Add to negative cache
                cache.add(&sig_hash)?;
                tx.send(WorkerMessage::LyricsNotFound {
                    path: path.display().to_string(),
                })?;
                failed += 1;
                if let Some(ref mut sess) = session {
                    sess.add_log(filename, StatusType::NotFound);
                }
            }
            Err(e) => {
                tx.send(WorkerMessage::Error {
                    path: path.display().to_string(),
                    error: e.to_string(),
                })?;
                failed += 1;
                if let Some(ref mut sess) = session {
                    sess.add_log(filename, StatusType::Error);
                }
            }
        }
    }

    let processed = downloaded + cached + failed;
    tx.send(WorkerMessage::ScanComplete {
        processed,
        found: downloaded,
    })?;
    tracing::info!("Worker complete: {}/{} lyrics found", downloaded, processed);

    // Delete session file on successful completion
    if let Err(e) = PersistentSession::delete(&session_path) {
        tracing::warn!("Failed to delete session file: {}", e);
    }

    Ok(())
}

fn write_lrc_file(audio_path: &Path, lyrics: &str) -> Result<()> {
    let mut lrc_path = audio_path.to_path_buf();
    lrc_path.set_extension("lrc");
    std::fs::write(lrc_path, lyrics)?;
    Ok(())
}

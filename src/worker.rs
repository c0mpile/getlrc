use crate::{
    api::LrcLibClient,
    cache::{signature::TrackSignature, NegativeCache},
    messages::{UiMessage, WorkerMessage},
    scanner::{self, metadata, parallel},
    session::{PersistentSession, StatusType},
};
use anyhow::Result;
use governor::{Quota, RateLimiter};
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, Semaphore};

const RATE_LIMIT_PER_SEC: u32 = 10; // 10 requests/second max
const MAX_CONCURRENT_WORKERS: usize = 5; // Number of concurrent API workers

/// Shared state for worker pool
struct WorkerPoolState {
    session: Mutex<PersistentSession>,
    cache: Mutex<NegativeCache>,
    downloaded: Mutex<usize>,
    cached: Mutex<usize>,
    failed: Mutex<usize>,
}

pub async fn run(
    target_dir: PathBuf,
    cache: NegativeCache,
    tx: mpsc::UnboundedSender<WorkerMessage>,
    mut ui_rx: mpsc::UnboundedReceiver<UiMessage>,
    session_path: PathBuf,
    mut session: Option<PersistentSession>,
) -> Result<()> {
    tracing::info!(
        "Worker pool started for directory: {}",
        target_dir.display()
    );

    // Track if we're resuming from a session
    let is_resuming = session.is_some();

    // Determine if we're resuming or starting fresh
    let (files_to_process, downloaded, cached, existing, failed) = if let Some(ref sess) = session {
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

        // Send counts to TUI for progress bar
        tx.send(WorkerMessage::CountsRestored {
            downloaded,
            cached,
            skipped: existing,
            processed: downloaded + cached + failed,
        })?;

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
        // Fresh scan - use parallel directory walker
        tracing::info!("Starting fresh parallel scan");

        // Spawn scanning task to avoid blocking
        let target_dir_clone = target_dir.clone();
        let tx_clone = tx.clone();
        let scan_handle = tokio::task::spawn_blocking(move || {
            let all_audio_files = parallel::walk_directory_parallel(&target_dir_clone);

            // Send progress update
            let _ = tx_clone.send(WorkerMessage::ScanProgress {
                files_found: all_audio_files.len(),
            });

            all_audio_files
        });

        let all_audio_files = scan_handle.await?;

        let mut files_to_process = Vec::new();
        let mut files_with_lrc = Vec::new();

        // Filter files that already have .lrc sidecars
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

    // Calculate total files: already processed + existing + pending
    let total_files = downloaded + cached + failed + existing + files_to_process.len();
    tx.send(WorkerMessage::ScanStarted { total_files })?;

    // Create shared state
    let shared_state = Arc::new(WorkerPoolState {
        session: Mutex::new(session.unwrap()),
        cache: Mutex::new(cache),
        downloaded: Mutex::new(downloaded),
        cached: Mutex::new(cached),
        failed: Mutex::new(failed),
    });

    // Create rate limiter (10 requests per second)
    let rate_limiter = Arc::new(RateLimiter::direct(Quota::per_second(
        NonZeroU32::new(RATE_LIMIT_PER_SEC).unwrap(),
    )));

    // Create semaphore for concurrent worker limit
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_WORKERS));

    // Create work queue using Arc<Mutex<VecDeque>> for work-stealing
    let work_queue = Arc::new(Mutex::new(
        files_to_process
            .into_iter()
            .collect::<std::collections::VecDeque<_>>(),
    ));

    // Control flags
    let paused = Arc::new(Mutex::new(is_resuming));
    let should_quit = Arc::new(Mutex::new(false));

    // Spawn control message handler
    let paused_clone = paused.clone();
    let should_quit_clone = should_quit.clone();
    let shared_state_clone = shared_state.clone();
    let session_path_clone = session_path.clone();
    let work_queue_clone = work_queue.clone();

    tokio::spawn(async move {
        while let Some(msg) = ui_rx.recv().await {
            match msg {
                UiMessage::Pause => {
                    *paused_clone.lock().await = true;
                    tracing::info!("Worker pool paused");

                    // Save session state with remaining work queue
                    let mut sess = shared_state_clone.session.lock().await;
                    let remaining_files: Vec<PathBuf> =
                        work_queue_clone.lock().await.iter().cloned().collect();
                    sess.pending_files = remaining_files;

                    if let Err(e) = sess.save(&session_path_clone) {
                        tracing::error!("Failed to save session: {}", e);
                    }
                }
                UiMessage::Resume => {
                    *paused_clone.lock().await = false;
                    tracing::info!("Worker pool resumed");
                }
                UiMessage::Quit => {
                    tracing::info!("Worker pool received quit signal");
                    *should_quit_clone.lock().await = true;

                    // Save session if paused
                    if *paused_clone.lock().await {
                        let mut sess = shared_state_clone.session.lock().await;
                        let remaining_files: Vec<PathBuf> =
                            work_queue_clone.lock().await.iter().cloned().collect();
                        sess.pending_files = remaining_files;

                        if let Err(e) = sess.save(&session_path_clone) {
                            tracing::error!("Failed to save session on quit: {}", e);
                        }
                    }
                    break;
                }
            }
        }
    });

    // Spawn worker tasks
    let mut worker_handles = Vec::new();

    for worker_id in 0..MAX_CONCURRENT_WORKERS {
        let work_queue_clone = work_queue.clone();
        let tx_clone = tx.clone();
        let shared_state_clone = shared_state.clone();
        let rate_limiter_clone = rate_limiter.clone();
        let semaphore_clone = semaphore.clone();
        let paused_clone = paused.clone();
        let should_quit_clone = should_quit.clone();

        let handle = tokio::spawn(async move {
            let client = LrcLibClient::new();

            loop {
                // Check for quit signal
                if *should_quit_clone.lock().await {
                    tracing::debug!("Worker {} received quit signal", worker_id);
                    break;
                }

                // Wait while paused
                while *paused_clone.lock().await {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    if *should_quit_clone.lock().await {
                        return;
                    }
                }

                // Try to get work from queue
                let path = {
                    let mut queue = work_queue_clone.lock().await;
                    queue.pop_front()
                };

                let Some(path) = path else {
                    // No more work
                    tracing::debug!("Worker {} finished (no more work)", worker_id);
                    break;
                };

                // Acquire semaphore permit
                let _permit = semaphore_clone.acquire().await.unwrap();

                // Process the file
                if let Err(e) = process_file(
                    &path,
                    &client,
                    &shared_state_clone,
                    &rate_limiter_clone,
                    &tx_clone,
                )
                .await
                {
                    tracing::error!(
                        "Worker {} error processing {}: {}",
                        worker_id,
                        path.display(),
                        e
                    );
                }
            }

            tracing::debug!("Worker {} shutting down", worker_id);
        });

        worker_handles.push(handle);
    }

    // Wait for all workers to complete
    for handle in worker_handles {
        let _ = handle.await;
    }

    // Check if we should quit early
    if *should_quit.lock().await {
        tracing::info!("Worker pool terminated early by user");
        return Ok(());
    }

    // Final counts
    let final_downloaded = *shared_state.downloaded.lock().await;
    let final_cached = *shared_state.cached.lock().await;
    let final_failed = *shared_state.failed.lock().await;
    let processed = final_downloaded + final_cached + final_failed;
    let total_files_processed = processed + existing;

    tx.send(WorkerMessage::ScanComplete {
        processed: total_files_processed,
        found: final_downloaded,
    })?;

    tracing::info!(
        "Worker pool complete: {} lyrics downloaded, {} total files ({} downloaded, {} cached, {} existing, {} failed)",
        final_downloaded,
        total_files_processed,
        final_downloaded,
        final_cached,
        existing,
        final_failed
    );

    // Delete session file on successful completion
    if let Err(e) = PersistentSession::delete(&session_path) {
        tracing::warn!("Failed to delete session file: {}", e);
    }

    Ok(())
}

/// Process a single file
async fn process_file(
    path: &Path,
    client: &LrcLibClient,
    shared_state: &WorkerPoolState,
    rate_limiter: &RateLimiter<
        governor::state::direct::NotKeyed,
        governor::state::InMemoryState,
        governor::clock::DefaultClock,
    >,
    tx: &mpsc::UnboundedSender<WorkerMessage>,
) -> Result<()> {
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    // Extract metadata
    let track = match metadata::extract(path) {
        Ok(t) => t,
        Err(e) => {
            tx.send(WorkerMessage::Error {
                path: path.display().to_string(),
                error: e.to_string(),
            })?;

            *shared_state.failed.lock().await += 1;
            shared_state
                .session
                .lock()
                .await
                .add_log(filename, StatusType::Error);
            return Ok(());
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

    // Check negative cache
    if shared_state.cache.lock().await.is_cached(&sig_hash)? {
        tx.send(WorkerMessage::CacheHit {
            path: path.display().to_string(),
        })?;
        *shared_state.cached.lock().await += 1;
        shared_state
            .session
            .lock()
            .await
            .add_log(filename, StatusType::Cached);
        return Ok(());
    }

    // Wait for rate limiter
    rate_limiter.until_ready().await;

    // Fetch lyrics
    match client.get_lyrics(&track).await {
        Ok(Some(lyrics)) => {
            // Write .lrc file
            if let Some(synced) = lyrics.synced_lyrics {
                if let Err(e) = write_lrc_file(path, &synced) {
                    tx.send(WorkerMessage::Error {
                        path: path.display().to_string(),
                        error: e.to_string(),
                    })?;
                    *shared_state.failed.lock().await += 1;
                    shared_state
                        .session
                        .lock()
                        .await
                        .add_log(filename, StatusType::Error);
                } else {
                    tx.send(WorkerMessage::LyricsFound {
                        path: path.display().to_string(),
                    })?;
                    *shared_state.downloaded.lock().await += 1;
                    shared_state
                        .session
                        .lock()
                        .await
                        .add_log(filename, StatusType::Downloaded);
                }
            } else {
                // No synced lyrics, add to negative cache
                shared_state.cache.lock().await.add(&sig_hash)?;
                tx.send(WorkerMessage::LyricsNotFound {
                    path: path.display().to_string(),
                })?;
                *shared_state.failed.lock().await += 1;
                shared_state
                    .session
                    .lock()
                    .await
                    .add_log(filename, StatusType::NotFound);
            }
        }
        Ok(None) => {
            // 404 - Add to negative cache
            shared_state.cache.lock().await.add(&sig_hash)?;
            tx.send(WorkerMessage::LyricsNotFound {
                path: path.display().to_string(),
            })?;
            *shared_state.failed.lock().await += 1;
            shared_state
                .session
                .lock()
                .await
                .add_log(filename, StatusType::NotFound);
        }
        Err(e) => {
            tx.send(WorkerMessage::Error {
                path: path.display().to_string(),
                error: e.to_string(),
            })?;
            *shared_state.failed.lock().await += 1;
            shared_state
                .session
                .lock()
                .await
                .add_log(filename, StatusType::Error);
        }
    }

    Ok(())
}

fn write_lrc_file(audio_path: &Path, lyrics: &str) -> Result<()> {
    let mut lrc_path = audio_path.to_path_buf();
    lrc_path.set_extension("lrc");
    std::fs::write(lrc_path, lyrics)?;
    Ok(())
}

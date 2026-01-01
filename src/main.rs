use anyhow::Result;
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn print_usage(program: &str) {
    eprintln!("getlrc - Fetch and store lyrics from lrclib.net\n");
    eprintln!("USAGE:");
    eprintln!(
        "    {} <music_directory>    Scan directory for audio files",
        program
    );
    eprintln!(
        "    {} install              Install to ~/.local/bin",
        program
    );
    eprintln!(
        "    {} uninstall            Remove from ~/.local/bin",
        program
    );
    eprintln!(
        "    {} --help               Show this help message",
        program
    );
    eprintln!("\nEXAMPLES:");
    eprintln!("    {} ~/Music", program);
    eprintln!("    {} /mnt/media/music", program);
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI arguments first to check for subcommands
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage(&args[0]);
        std::process::exit(1);
    }

    // Handle subcommands that don't need logging
    match args[1].as_str() {
        "install" => {
            return getlrc::install::install();
        }
        "uninstall" => {
            return getlrc::install::uninstall();
        }
        "--help" | "-h" => {
            print_usage(&args[0]);
            return Ok(());
        }
        _ => {
            // Continue to scanner mode with file-based logging
        }
    }

    // Initialize file-based logging for TUI mode
    let log_dir = getlrc::paths::get_log_dir()?;
    let file_appender = tracing_appender::rolling::never(&log_dir, "getlrc.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "getlrc=debug,reqwest=warn".into()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false),
        )
        .init();

    // Treat argument as directory path
    let target_dir = PathBuf::from(&args[1]);
    if !target_dir.is_dir() {
        anyhow::bail!("Path is not a directory: {}", target_dir.display());
    }

    run_scanner(target_dir).await
}

async fn run_scanner(target_dir: PathBuf) -> Result<()> {
    tracing::info!("Starting getlrc for directory: {}", target_dir.display());

    // Initialize negative cache using XDG-compliant paths
    let cache_path = getlrc::paths::get_cache_db_path()?;
    tracing::info!("Cache database: {}", cache_path.display());
    let cache = getlrc::cache::NegativeCache::open(&cache_path)?;

    // Check for existing session
    let session_path = getlrc::paths::get_session_path()?;
    let session = if getlrc::session::PersistentSession::exists(&session_path) {
        match getlrc::session::PersistentSession::load(&session_path) {
            Ok(sess) => {
                // Perform integrity check
                if sess.check_integrity() {
                    tracing::info!("Session integrity check passed, resuming from previous state");
                    println!("📂 Resuming previous session...");
                    Some(sess)
                } else {
                    tracing::warn!(
                        "Session integrity check failed (stale session), starting fresh scan"
                    );
                    println!("⚠️  Stale session detected, starting fresh scan...");
                    // Delete the stale session file
                    let _ = getlrc::session::PersistentSession::delete(&session_path);
                    None
                }
            }
            Err(e) => {
                tracing::warn!("Failed to load session file: {}, starting fresh", e);
                println!("⚠️  Could not load session, starting fresh scan...");
                None
            }
        }
    } else {
        None
    };

    // Create mpsc channels
    let (worker_tx, worker_rx) = tokio::sync::mpsc::unbounded_channel();
    let (ui_tx, ui_rx) = tokio::sync::mpsc::unbounded_channel();

    // Spawn worker task
    let target_dir_clone = target_dir.clone();
    let session_path_clone = session_path.clone();
    let worker_handle = tokio::spawn(async move {
        if let Err(e) = getlrc::worker::run(
            target_dir_clone,
            cache,
            worker_tx,
            ui_rx,
            session_path_clone,
            session,
        )
        .await
        {
            tracing::error!("Worker error: {}", e);
        }
    });

    // Run TUI
    let mut app = getlrc::tui::App::new(worker_rx, ui_tx);
    app.run().await?;

    // Wait for worker to complete
    worker_handle.await?;

    Ok(())
}

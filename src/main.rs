use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(name = "getlrc")]
#[command(about = "Fetch and store lyrics from lrclib.net", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Music directory to scan for audio files
    #[arg(value_name = "DIRECTORY")]
    directory: Option<PathBuf>,

    /// Force retry: ignore negative cache and retry all files
    #[arg(short = 'f', long = "force-retry")]
    force_retry: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Install getlrc to ~/.local/bin
    Install,
    /// Uninstall getlrc from ~/.local/bin
    Uninstall,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Handle subcommands that don't need logging
    match cli.command {
        Some(Commands::Install) => {
            return getlrc::install::install();
        }
        Some(Commands::Uninstall) => {
            return getlrc::install::uninstall();
        }
        None => {
            // Continue to scanner mode
        }
    }

    // Require directory argument for scanner mode
    let target_dir = cli.directory.ok_or_else(|| {
        anyhow::anyhow!(
            "Missing required argument: DIRECTORY\n\nFor more information, try '--help'."
        )
    })?;

    if !target_dir.is_dir() {
        anyhow::bail!("Path is not a directory: {}", target_dir.display());
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

    run_scanner(target_dir, cli.force_retry).await
}

async fn run_scanner(target_dir: PathBuf, force_retry: bool) -> Result<()> {
    tracing::info!(
        "Starting getlrc for directory: {} (force_retry: {})",
        target_dir.display(),
        force_retry
    );

    if force_retry {
        tracing::info!("Force retry enabled: negative cache will be bypassed");
        println!("🔄 Force retry mode: bypassing negative cache");
    }

    // Verify environment before initializing TUI
    getlrc::env::verify_environment()
        .context("Failed to verify environment. Check logs for details.")?;

    // Initialize negative cache using XDG-compliant paths
    let cache_path = getlrc::paths::get_cache_db_path()?;
    tracing::info!("Cache database: {}", cache_path.display());
    let cache = getlrc::cache::NegativeCache::open(&cache_path)?;

    // Check for existing session
    let session_path = getlrc::paths::get_session_path()?;
    let session = if getlrc::session::PersistentSession::exists(&session_path) {
        match getlrc::session::PersistentSession::load(&session_path) {
            Ok(mut sess) => {
                // Perform integrity check
                if sess.check_integrity() {
                    tracing::info!("Session integrity check passed, resuming from previous state");

                    // Update force_retry flag if it changed
                    if sess.force_retry != force_retry {
                        tracing::info!(
                            "Force retry flag changed from {} to {}",
                            sess.force_retry,
                            force_retry
                        );
                        sess.force_retry = force_retry;
                    }

                    println!("📂 Resuming previous session...");
                    if force_retry {
                        println!("🔄 Force retry enabled for resumed session");
                    }
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
            force_retry,
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

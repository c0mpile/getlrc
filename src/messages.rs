use crate::scanner::metadata::Track;
use crate::session::StatusType;

/// Messages sent from Worker to TUI
#[derive(Debug, Clone)]
pub enum WorkerMessage {
    SessionRestoring,
    CountsRestored {
        downloaded: usize,
        cached: usize,
        skipped: usize,
        processed: usize,
    },
    ScanProgress {
        files_found: usize,
    },
    ScanStarted {
        total_files: usize,
    },
    TrackProcessing {
        track: Track,
    },
    LyricsFound {
        path: String,
    },
    LyricsNotFound {
        path: String,
    },
    CacheHit {
        path: String,
    },
    AlreadyHasLrc {
        path: String,
    },
    Error {
        path: String,
        error: String,
    },
    LogRestore {
        filename: String,
        status: StatusType,
    },
    ScanComplete {
        processed: usize,
        found: usize,
    },
}

/// Messages sent from TUI to Worker
#[derive(Debug, Clone)]
pub enum UiMessage {
    Quit,
    Pause,
    Resume,
}

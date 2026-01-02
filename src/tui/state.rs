use crate::messages::WorkerMessage;
use std::collections::VecDeque;

const MAX_LOG_LINES: usize = 100;

#[derive(Debug, Clone)]
pub struct AppState {
    pub total_files: usize,
    pub processed: usize,
    pub found: usize,
    pub downloaded: usize,
    pub cached: usize,
    pub skipped: usize,
    pub current_track: Option<String>,
    pub logs: VecDeque<String>,
    pub status: Status,
    pub should_quit: bool,
    pub paused: bool,
    pub scroll_offset: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    Idle,
    Restoring,
    Scanning,
    Processing,
    Complete,
    Error(String),
}

impl AppState {
    pub fn new() -> Self {
        Self {
            total_files: 0,
            processed: 0,
            found: 0,
            downloaded: 0,
            cached: 0,
            skipped: 0,
            current_track: None,
            logs: VecDeque::new(),
            status: Status::Idle,
            should_quit: false,
            paused: false,
            scroll_offset: 0,
        }
    }

    /// Update state based on worker messages (Elm Architecture - Update)
    pub fn update(&mut self, msg: WorkerMessage) {
        match msg {
            WorkerMessage::SessionRestoring => {
                self.status = Status::Restoring;
                self.paused = true; // Initialize in paused state when restoring
            }
            WorkerMessage::CountsRestored {
                downloaded,
                cached,
                skipped,
                processed,
            } => {
                // Restore counts from session
                self.downloaded = downloaded;
                self.cached = cached;
                self.skipped = skipped;
                self.processed = processed;
                self.found = downloaded; // found = downloaded
            }
            WorkerMessage::ScanProgress { files_found } => {
                self.status = Status::Scanning;
                self.add_log(format!("Scanning... {} files found", files_found));
            }
            WorkerMessage::ScanStarted { total_files } => {
                self.total_files = total_files;
                self.status = Status::Processing;
                self.add_log(format!("Scan complete: {} files to process", total_files));
            }
            WorkerMessage::TrackProcessing { track } => {
                self.current_track = Some(format!("{} - {}", track.artist, track.title));
                self.status = Status::Processing;
            }
            WorkerMessage::LyricsFound { path } => {
                self.found += 1;
                self.processed += 1;
                self.downloaded += 1;
                let filename = std::path::Path::new(&path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&path);
                self.add_log(format!("[✓] {}", filename));
            }
            WorkerMessage::LyricsNotFound { path } => {
                self.processed += 1;
                let filename = std::path::Path::new(&path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&path);
                self.add_log(format!("[✗] {}", filename));
            }
            WorkerMessage::CacheHit { path } => {
                self.processed += 1;
                self.cached += 1;
                let filename = std::path::Path::new(&path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&path);
                self.add_log(format!("[~] {}", filename));
            }
            WorkerMessage::AlreadyHasLrc { path } => {
                self.skipped += 1;
                let filename = std::path::Path::new(&path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&path);
                self.add_log(format!("[○] {}", filename));
            }
            WorkerMessage::Error { path, error } => {
                self.processed += 1;
                let filename = std::path::Path::new(&path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&path);
                self.add_log(format!("[!] {}: {}", filename, error));
            }
            WorkerMessage::LogRestore { filename, status } => {
                // Restore log entry from session without updating counts
                use crate::session::StatusType;
                let log_msg = match status {
                    StatusType::Downloaded => format!("[✓] {}", filename),
                    StatusType::Cached => format!("[~] {}", filename),
                    StatusType::Existing => format!("[○] {}", filename),
                    StatusType::NotFound => format!("[✗] {}", filename),
                    StatusType::Error => format!("[!] {}", filename),
                };
                self.add_log(log_msg);
            }

            WorkerMessage::ScanComplete { processed, found } => {
                self.processed = processed;
                self.found = found;
                self.status = Status::Complete;
                self.current_track = None;
                self.add_log(format!(
                    "Scan complete: {} lyrics downloaded, {} files total",
                    found, processed
                ));
            }
        }
    }

    fn add_log(&mut self, msg: String) {
        if self.logs.len() >= MAX_LOG_LINES {
            self.logs.pop_front();
        }
        self.logs.push_back(msg);
        // Auto-scroll to bottom when new log is added
        self.scroll_offset = 0;
    }

    pub fn progress(&self) -> f64 {
        if self.total_files == 0 {
            0.0
        } else {
            self.processed as f64 / self.total_files as f64
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

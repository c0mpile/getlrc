use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const MAX_LOG_HISTORY: usize = 500;
const INTEGRITY_CHECK_SAMPLE_SIZE: usize = 10;
const INTEGRITY_CHECK_THRESHOLD: usize = 5;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PersistentSession {
    /// The original root directory being scanned
    pub root_path: PathBuf,

    /// List of file paths that still need processing
    pub pending_files: Vec<PathBuf>,

    /// Current counts for the UI Progress widget
    pub downloaded_count: usize,
    pub cached_count: usize,
    pub existing_count: usize,
    pub failed_count: usize,

    /// Buffer of recent TUI log entries to restore visual history
    pub log_history: Vec<LogEntry>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LogEntry {
    pub filename: String,
    pub status: StatusType,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum StatusType {
    Downloaded, // [✓]
    Cached,     // [~]
    Existing,   // [○]
    NotFound,   // [✗]
    Error,      // [!]
}

impl PersistentSession {
    /// Create a new session from initial scan
    pub fn new(root_path: PathBuf, pending_files: Vec<PathBuf>) -> Self {
        Self {
            root_path,
            pending_files,
            downloaded_count: 0,
            cached_count: 0,
            existing_count: 0,
            failed_count: 0,
            log_history: Vec::new(),
        }
    }

    /// Save session to disk using atomic write pattern
    pub fn save(&self, path: &Path) -> Result<()> {
        // Create a copy with capped log history
        let mut session_to_save = self.clone();
        session_to_save.cap_log_history();

        // Write to temporary file first
        let temp_path = path.with_extension("json.tmp");
        let json = serde_json::to_string_pretty(&session_to_save)
            .context("Failed to serialize session")?;

        std::fs::write(&temp_path, json).with_context(|| {
            format!(
                "Failed to write temporary session file: {}",
                temp_path.display()
            )
        })?;

        // Atomic rename to final destination
        std::fs::rename(&temp_path, path).with_context(|| {
            format!(
                "Failed to rename {} to {}",
                temp_path.display(),
                path.display()
            )
        })?;

        tracing::info!(
            "Session saved atomically to {} ({} pending files, {} log entries)",
            path.display(),
            session_to_save.pending_files.len(),
            session_to_save.log_history.len()
        );
        Ok(())
    }

    /// Load session from disk with integrity check
    pub fn load(path: &Path) -> Result<Self> {
        let json = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read session file: {}", path.display()))?;

        let session: PersistentSession =
            serde_json::from_str(&json).context("Failed to deserialize session")?;

        tracing::info!(
            "Session loaded from {} ({} pending files, {} log entries)",
            path.display(),
            session.pending_files.len(),
            session.log_history.len()
        );

        Ok(session)
    }

    /// Check if a session file exists
    pub fn exists(path: &Path) -> bool {
        path.exists()
    }

    /// Delete the session file
    pub fn delete(path: &Path) -> Result<()> {
        if path.exists() {
            std::fs::remove_file(path)
                .with_context(|| format!("Failed to delete session file: {}", path.display()))?;
            tracing::info!("Session file deleted: {}", path.display());
        }

        // Also clean up any leftover temp files
        let temp_path = path.with_extension("json.tmp");
        if temp_path.exists() {
            let _ = std::fs::remove_file(&temp_path);
            tracing::debug!("Cleaned up temporary session file: {}", temp_path.display());
        }

        Ok(())
    }

    /// Perform shallow integrity check on pending files
    /// Returns true if the session appears valid, false if stale
    pub fn check_integrity(&self) -> bool {
        if self.pending_files.is_empty() {
            tracing::warn!("Session has no pending files");
            return false;
        }

        let sample_size = INTEGRITY_CHECK_SAMPLE_SIZE.min(self.pending_files.len());
        let mut missing_count = 0;

        tracing::debug!(
            "Checking integrity of first {} files in session",
            sample_size
        );

        for path in self.pending_files.iter().take(sample_size) {
            if !path.exists() {
                missing_count += 1;
                tracing::debug!("Missing file in session: {}", path.display());
            }
        }

        let is_valid = missing_count < INTEGRITY_CHECK_THRESHOLD;

        if !is_valid {
            tracing::warn!(
                "Session integrity check failed: {}/{} sampled files are missing (threshold: {})",
                missing_count,
                sample_size,
                INTEGRITY_CHECK_THRESHOLD
            );
        } else {
            tracing::info!(
                "Session integrity check passed: {}/{} sampled files exist",
                sample_size - missing_count,
                sample_size
            );
        }

        is_valid
    }

    /// Add a log entry to the history (maintains max size)
    pub fn add_log(&mut self, filename: String, status: StatusType) {
        self.log_history.push(LogEntry { filename, status });

        // Trim to max size if needed
        if self.log_history.len() > MAX_LOG_HISTORY {
            self.log_history
                .drain(0..self.log_history.len() - MAX_LOG_HISTORY);
        }
    }

    /// Cap log history to MAX_LOG_HISTORY entries (most recent)
    fn cap_log_history(&mut self) {
        if self.log_history.len() > MAX_LOG_HISTORY {
            self.log_history
                .drain(0..self.log_history.len() - MAX_LOG_HISTORY);
            tracing::debug!("Capped log history to {} entries", MAX_LOG_HISTORY);
        }
    }

    /// Update counts based on status
    pub fn update_counts(&mut self, status: &StatusType) {
        match status {
            StatusType::Downloaded => self.downloaded_count += 1,
            StatusType::Cached => self.cached_count += 1,
            StatusType::Existing => self.existing_count += 1,
            StatusType::NotFound | StatusType::Error => self.failed_count += 1,
        }
    }

    /// Get total files processed
    pub fn total_processed(&self) -> usize {
        self.downloaded_count + self.cached_count + self.failed_count
    }

    /// Get total files in session
    pub fn total_files(&self) -> usize {
        self.pending_files.len() + self.total_processed()
    }
}

impl StatusType {
    /// Convert to display symbol
    pub fn to_symbol(&self) -> &'static str {
        match self {
            StatusType::Downloaded => "[✓]",
            StatusType::Cached => "[~]",
            StatusType::Existing => "[○]",
            StatusType::NotFound => "[✗]",
            StatusType::Error => "[!]",
        }
    }

    /// Convert to display string with filename
    pub fn format_log(&self, filename: &str) -> String {
        format!("{} {}", self.to_symbol(), filename)
    }
}

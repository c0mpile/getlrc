use anyhow::{Context, Result};
use std::path::PathBuf;

/// Get the application's data directory following XDG standards
/// On Linux: ~/.local/share/getlrc
pub fn get_data_dir() -> Result<PathBuf> {
    let data_dir = dirs::data_local_dir()
        .context("Failed to determine user data directory")?
        .join("getlrc");

    // Ensure directory exists
    std::fs::create_dir_all(&data_dir)
        .with_context(|| format!("Failed to create data directory: {}", data_dir.display()))?;

    tracing::debug!("Data directory: {}", data_dir.display());
    Ok(data_dir)
}

/// Get the path to the negative cache database
pub fn get_cache_db_path() -> Result<PathBuf> {
    Ok(get_data_dir()?.join("negative_cache.db"))
}

/// Get the path to the session state file
pub fn get_session_path() -> Result<PathBuf> {
    Ok(get_data_dir()?.join("session.json"))
}

/// Get the application's log directory
pub fn get_log_dir() -> Result<PathBuf> {
    let log_dir = get_data_dir()?.join("logs");
    std::fs::create_dir_all(&log_dir)
        .with_context(|| format!("Failed to create log directory: {}", log_dir.display()))?;
    Ok(log_dir)
}

/// Get the user's local bin directory for installation
/// On Linux: ~/.local/bin
pub fn get_local_bin_dir() -> Result<PathBuf> {
    let bin_dir = dirs::home_dir()
        .context("Failed to determine home directory")?
        .join(".local")
        .join("bin");

    Ok(bin_dir)
}

/// Check if ~/.local/bin is in the user's PATH
pub fn is_local_bin_in_path() -> bool {
    if let Ok(path_var) = std::env::var("PATH") {
        if let Ok(local_bin) = get_local_bin_dir() {
            let local_bin_str = local_bin.to_string_lossy();
            return path_var.split(':').any(|p| p == local_bin_str);
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_data_dir() {
        let data_dir = get_data_dir().unwrap();
        assert!(data_dir.ends_with("getlrc"));
    }

    #[test]
    fn test_get_cache_db_path() {
        let cache_path = get_cache_db_path().unwrap();
        assert!(cache_path.ends_with("negative_cache.db"));
    }
}

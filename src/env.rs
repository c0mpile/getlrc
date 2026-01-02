use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::path::PathBuf;

/// Verify and setup the application environment
pub fn verify_environment() -> Result<()> {
    // Check PATH for ~/.local/bin
    check_local_bin_in_path();

    // Ensure XDG directories exist
    ensure_xdg_directories()?;

    // Verify write permissions
    verify_write_permissions()?;

    Ok(())
}

/// Check if ~/.local/bin is in PATH and log warning if missing
fn check_local_bin_in_path() {
    if let Some(home) = env::var_os("HOME") {
        let local_bin = PathBuf::from(home).join(".local/bin");

        if let Some(path_var) = env::var_os("PATH") {
            let paths: Vec<PathBuf> = env::split_paths(&path_var).collect();

            if !paths.contains(&local_bin) {
                tracing::warn!(
                    "~/.local/bin is not in PATH. The installed binary may not be accessible."
                );
                tracing::warn!(
                    "Add 'export PATH=\"$HOME/.local/bin:$PATH\"' to your shell configuration"
                );
            } else {
                tracing::debug!("~/.local/bin is in PATH");
            }
        }
    }
}

/// Ensure all required XDG directories exist with proper permissions
fn ensure_xdg_directories() -> Result<()> {
    let data_dir = crate::paths::get_data_dir()?;
    let log_dir = data_dir.join("logs");

    // Create data directory
    if !data_dir.exists() {
        fs::create_dir_all(&data_dir)
            .with_context(|| format!("Failed to create data directory: {}", data_dir.display()))?;
        tracing::info!("Created data directory: {}", data_dir.display());
    }

    // Create logs directory
    if !log_dir.exists() {
        fs::create_dir_all(&log_dir)
            .with_context(|| format!("Failed to create logs directory: {}", log_dir.display()))?;
        tracing::info!("Created logs directory: {}", log_dir.display());
    }

    // Verify permissions (Unix-specific)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let data_perms = fs::metadata(&data_dir)?.permissions();
        let mode = data_perms.mode();

        // Check if owner has write permission (bit 7)
        if mode & 0o200 == 0 {
            tracing::error!(
                "Data directory is not writable: {} (mode: {:o})",
                data_dir.display(),
                mode
            );
            anyhow::bail!("Data directory is not writable: {}", data_dir.display());
        }

        tracing::debug!(
            "Data directory permissions verified: {} (mode: {:o})",
            data_dir.display(),
            mode
        );
    }

    Ok(())
}

/// Verify write permissions to data directory
fn verify_write_permissions() -> Result<()> {
    let data_dir = crate::paths::get_data_dir()?;

    // Try to create a test file
    let test_file = data_dir.join(".write_test");

    match fs::write(&test_file, b"test") {
        Ok(_) => {
            // Clean up test file
            let _ = fs::remove_file(&test_file);
            tracing::debug!("Write permissions verified for: {}", data_dir.display());
            Ok(())
        }
        Err(e) => {
            tracing::error!(
                "Cannot write to data directory: {} - {}",
                data_dir.display(),
                e
            );
            anyhow::bail!(
                "Cannot write to data directory: {} - {}",
                data_dir.display(),
                e
            )
        }
    }
}

/// Get the installation path for the binary
pub fn get_install_path() -> Result<PathBuf> {
    let home = env::var("HOME").context("HOME environment variable not set")?;

    Ok(PathBuf::from(home).join(".local/bin/getlrc"))
}

/// Check if the binary is installed
pub fn is_installed() -> bool {
    if let Ok(install_path) = get_install_path() {
        install_path.exists()
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_install_path() {
        if let Ok(path) = get_install_path() {
            assert!(path.to_string_lossy().contains(".local/bin/getlrc"));
        }
    }

    #[test]
    fn test_verify_environment() {
        // This should not panic even if directories don't exist
        let result = verify_environment();
        // We can't assert success because it depends on the environment
        // but we can ensure it doesn't panic
        let _ = result;
    }
}

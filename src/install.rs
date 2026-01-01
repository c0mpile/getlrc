use anyhow::{Context, Result};
use std::fs;

/// Install the getlrc binary to ~/.local/bin
pub fn install() -> Result<()> {
    println!("=== getlrc Installation ===\n");

    // Get current executable path
    let current_exe = std::env::current_exe().context("Failed to get current executable path")?;

    println!("Current binary: {}", current_exe.display());

    // Get installation directory
    let install_dir = crate::paths::get_local_bin_dir()?;

    // Create install directory if it doesn't exist
    fs::create_dir_all(&install_dir)
        .with_context(|| format!("Failed to create directory: {}", install_dir.display()))?;

    let install_path = install_dir.join("getlrc");

    // Copy binary
    println!("Installing to: {}", install_path.display());
    fs::copy(&current_exe, &install_path)
        .with_context(|| format!("Failed to copy binary to {}", install_path.display()))?;

    // Set executable permissions (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&install_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&install_path, perms)?;
    }

    println!("\n✓ Installation complete!\n");

    // Check PATH
    if !crate::paths::is_local_bin_in_path() {
        println!("⚠ WARNING: {} is not in your PATH\n", install_dir.display());
        println!("Add the following line to your shell configuration file:");
        println!("  (~/.bashrc, ~/.zshrc, or ~/.config/fish/config.fish)\n");
        println!("  export PATH=\"$HOME/.local/bin:$PATH\"\n");
        println!("Then reload your shell configuration:");
        println!("  source ~/.bashrc  # or ~/.zshrc\n");
    } else {
        println!("✓ {} is in your PATH\n", install_dir.display());
    }

    println!("You can now run: getlrc <music_directory>");
    println!("\nData will be stored in: ~/.local/share/getlrc/");

    Ok(())
}

/// Uninstall the getlrc binary from ~/.local/bin
pub fn uninstall() -> Result<()> {
    println!("=== getlrc Uninstallation ===\n");

    let install_dir = crate::paths::get_local_bin_dir()?;
    let install_path = install_dir.join("getlrc");

    if install_path.exists() {
        fs::remove_file(&install_path)
            .with_context(|| format!("Failed to remove {}", install_path.display()))?;
        println!("✓ Removed: {}", install_path.display());
    } else {
        println!("Binary not found at: {}", install_path.display());
    }

    // Ask about data directory
    let data_dir = crate::paths::get_data_dir()?;
    println!("\nData directory: {}", data_dir.display());
    println!("To remove cached data, run:");
    println!("  rm -rf {}", data_dir.display());

    Ok(())
}

use jwalk::WalkDir;
use std::path::{Path, PathBuf};

const AUDIO_EXTENSIONS: &[&str] = &["flac", "mp3", "m4a", "aac", "opus", "ogg", "ape", "wav"];

/// Parallel directory walker using jwalk
/// Returns a vector of all audio files found in the directory tree
pub fn walk_directory_parallel(path: &Path) -> Vec<PathBuf> {
    tracing::info!("Starting parallel directory scan: {}", path.display());

    let audio_files: Vec<PathBuf> = WalkDir::new(path)
        .skip_hidden(false)
        .into_iter()
        .filter_map(|entry| match entry {
            Ok(e) => {
                if !e.file_type().is_file() {
                    return None;
                }

                let path = e.path();
                if let Some(ext) = path.extension() {
                    let ext_str = ext.to_str().unwrap_or("").to_lowercase();
                    if AUDIO_EXTENSIONS.contains(&ext_str.as_str()) {
                        return Some(path);
                    }
                }
                None
            }
            Err(e) => {
                tracing::warn!("Error walking directory: {}", e);
                None
            }
        })
        .collect();

    tracing::info!(
        "Parallel scan complete: {} audio files found",
        audio_files.len()
    );
    audio_files
}

/// Check if a .lrc sidecar file exists for the given audio file
pub fn has_lrc_sidecar(path: &Path) -> bool {
    let mut lrc_path = path.to_path_buf();
    lrc_path.set_extension("lrc");
    lrc_path.exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_lrc_sidecar() {
        let path = Path::new("/tmp/test.flac");
        assert!(!has_lrc_sidecar(path));
    }
}

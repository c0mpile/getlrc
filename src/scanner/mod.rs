pub mod metadata;

use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const AUDIO_EXTENSIONS: &[&str] = &["flac", "mp3", "m4a", "aac", "opus", "ogg", "ape", "wav"];

/// Walk a directory and yield all audio file paths
pub fn walk_directory(path: &Path) -> impl Iterator<Item = PathBuf> {
    WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| {
            let path = e.path();
            if let Some(ext) = path.extension() {
                if AUDIO_EXTENSIONS.contains(&ext.to_str().unwrap_or("").to_lowercase().as_str()) {
                    return Some(path.to_path_buf());
                }
            }
            None
        })
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

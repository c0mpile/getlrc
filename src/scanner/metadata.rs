use anyhow::{Context, Result};
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::Accessor;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Track {
    pub path: PathBuf,
    pub artist: String,
    pub title: String,
    pub album: String,
    pub duration_secs: u64,
}

/// Extract metadata from an audio file using lofty
pub fn extract(path: &Path) -> Result<Track> {
    let tagged_file = Probe::open(path)
        .context("Failed to open audio file")?
        .read()
        .context("Failed to read audio file")?;

    let tag = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag())
        .context("No tags found in audio file")?;

    let artist = tag.artist().map(|s| s.to_string()).unwrap_or_default();
    let title = tag.title().map(|s| s.to_string()).unwrap_or_default();
    let album = tag.album().map(|s| s.to_string()).unwrap_or_default();

    let duration_secs = tagged_file.properties().duration().as_secs();

    Ok(Track {
        path: path.to_path_buf(),
        artist,
        title,
        album,
        duration_secs,
    })
}

pub mod types;

use crate::scanner::metadata::Track;
use anyhow::Result;
use reqwest::StatusCode;
use types::LyricsResponse;

const LRCLIB_BASE_URL: &str = "https://lrclib.net/api";

pub struct LrcLibClient {
    client: reqwest::Client,
}

impl LrcLibClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Fetch lyrics from LRCLIB API
    /// Returns None if lyrics are not found (404)
    pub async fn get_lyrics(&self, track: &Track) -> Result<Option<LyricsResponse>> {
        let url = format!(
            "{}/get?artist_name={}&track_name={}&album_name={}&duration={}",
            LRCLIB_BASE_URL,
            urlencoding::encode(&track.artist),
            urlencoding::encode(&track.title),
            urlencoding::encode(&track.album),
            track.duration_secs
        );

        tracing::debug!("Fetching lyrics: {}", url);

        let response = self.client.get(&url).send().await?;

        match response.status() {
            StatusCode::OK => {
                let lyrics = response.json::<LyricsResponse>().await?;
                Ok(Some(lyrics))
            }
            StatusCode::NOT_FOUND => {
                tracing::debug!("Lyrics not found for: {} - {}", track.artist, track.title);
                Ok(None)
            }
            status => {
                anyhow::bail!("Unexpected status code from LRCLIB: {}", status);
            }
        }
    }
}

impl Default for LrcLibClient {
    fn default() -> Self {
        Self::new()
    }
}

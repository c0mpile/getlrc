use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct LyricsResponse {
    #[serde(rename = "syncedLyrics")]
    pub synced_lyrics: Option<String>,
    #[serde(rename = "plainLyrics")]
    pub plain_lyrics: Option<String>,
}

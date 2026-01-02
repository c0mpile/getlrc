use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct LyricsResponse {
    #[serde(rename = "artistName")]
    pub artist_name: String,
    #[serde(rename = "trackName")]
    pub track_name: String,
    #[serde(rename = "syncedLyrics")]
    pub synced_lyrics: Option<String>,
    #[serde(rename = "plainLyrics")]
    pub plain_lyrics: Option<String>,
}

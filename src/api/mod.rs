pub mod types;

use crate::scanner::clean::{self, NormalizedMetadata};
use crate::scanner::metadata::Track;
use anyhow::Result;
use reqwest::StatusCode;
use types::LyricsResponse;

const LRCLIB_BASE_URL: &str = "https://lrclib.net/api";
const SIMILARITY_THRESHOLD_AUTO: f64 = 0.85;
const SIMILARITY_THRESHOLD_POTENTIAL: f64 = 0.6;

pub struct LrcLibClient {
    client: reqwest::Client,
}

/// Result of a lyrics search with fuzzy matching
#[derive(Debug)]
pub enum SearchResult {
    /// Exact match found
    Found(LyricsResponse),
    /// Potential match found (similarity between 0.6 and 0.85)
    PotentialMatch {
        lyrics: LyricsResponse,
        similarity: f64,
    },
    /// No match found
    NotFound,
}

impl LrcLibClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Fetch lyrics with smart normalization and fuzzy matching
    /// Attempts multiple search strategies:
    /// 1. Normalized metadata with parentheticals
    /// 2. Stripped metadata (no parentheticals or featuring)
    pub async fn get_lyrics_smart(&self, track: &Track) -> Result<SearchResult> {
        // Normalize metadata
        let normalized = clean::normalize_metadata(&track.artist, &track.title, &track.album);

        tracing::debug!(
            "Searching for: {} - {} (normalized from: {} - {})",
            normalized.artist,
            normalized.title,
            track.artist,
            track.title
        );

        // Attempt 1: Search with normalized metadata (keeps parentheticals)
        match self
            .search_with_fuzzy(&normalized, track.duration_secs)
            .await?
        {
            SearchResult::Found(lyrics) => {
                tracing::info!("Found exact match for: {} - {}", track.artist, track.title);
                return Ok(SearchResult::Found(lyrics));
            }
            SearchResult::PotentialMatch { lyrics, similarity } => {
                tracing::info!(
                    "Found potential match for: {} - {} (similarity: {:.2})",
                    track.artist,
                    track.title,
                    similarity
                );
                return Ok(SearchResult::PotentialMatch { lyrics, similarity });
            }
            SearchResult::NotFound => {
                tracing::debug!("No match with normalized metadata, trying stripped version");
            }
        }

        // Attempt 2: Search with stripped metadata (removes parentheticals and featuring)
        let stripped_title = clean::get_stripped_title(&normalized);
        if stripped_title != normalized.title {
            tracing::debug!(
                "Attempting fallback search with stripped title: {}",
                stripped_title
            );

            let stripped_normalized = NormalizedMetadata {
                artist: normalized.artist.clone(),
                title: stripped_title,
                album: normalized.album.clone(),
                original_artist: normalized.original_artist.clone(),
                original_title: normalized.original_title.clone(),
            };

            match self
                .search_with_fuzzy(&stripped_normalized, track.duration_secs)
                .await?
            {
                SearchResult::Found(lyrics) => {
                    tracing::info!(
                        "Found match with stripped title for: {} - {}",
                        track.artist,
                        track.title
                    );
                    return Ok(SearchResult::Found(lyrics));
                }
                SearchResult::PotentialMatch { lyrics, similarity } => {
                    tracing::info!(
                        "Found potential match with stripped title for: {} - {} (similarity: {:.2})",
                        track.artist,
                        track.title,
                        similarity
                    );
                    return Ok(SearchResult::PotentialMatch { lyrics, similarity });
                }
                SearchResult::NotFound => {
                    tracing::debug!("No match with stripped metadata either");
                }
            }
        }

        Ok(SearchResult::NotFound)
    }

    /// Search with fuzzy matching on the result
    async fn search_with_fuzzy(
        &self,
        normalized: &NormalizedMetadata,
        duration_secs: u64,
    ) -> Result<SearchResult> {
        let url = format!(
            "{}/get?artist_name={}&track_name={}&album_name={}&duration={}",
            LRCLIB_BASE_URL,
            urlencoding::encode(&normalized.artist),
            urlencoding::encode(&normalized.title),
            urlencoding::encode(&normalized.album),
            duration_secs
        );

        tracing::debug!("API request: {}", url);

        let response = self.client.get(&url).send().await?;

        match response.status() {
            StatusCode::OK => {
                let lyrics = response.json::<LyricsResponse>().await?;

                // Calculate similarity scores
                let artist_similarity =
                    clean::similarity_score(&normalized.artist, &lyrics.artist_name.to_lowercase());
                let title_similarity =
                    clean::similarity_score(&normalized.title, &lyrics.track_name.to_lowercase());

                let avg_similarity = (artist_similarity + title_similarity) / 2.0;

                tracing::debug!(
                    "Similarity scores - Artist: {:.2}, Title: {:.2}, Average: {:.2}",
                    artist_similarity,
                    title_similarity,
                    avg_similarity
                );

                // Determine match quality
                if avg_similarity >= SIMILARITY_THRESHOLD_AUTO {
                    Ok(SearchResult::Found(lyrics))
                } else if avg_similarity >= SIMILARITY_THRESHOLD_POTENTIAL {
                    Ok(SearchResult::PotentialMatch {
                        lyrics,
                        similarity: avg_similarity,
                    })
                } else {
                    tracing::debug!(
                        "Similarity too low ({:.2}), treating as not found",
                        avg_similarity
                    );
                    Ok(SearchResult::NotFound)
                }
            }
            StatusCode::NOT_FOUND => {
                tracing::debug!(
                    "API returned 404 for: {} - {}",
                    normalized.artist,
                    normalized.title
                );
                Ok(SearchResult::NotFound)
            }
            status => {
                anyhow::bail!("Unexpected status code from LRCLIB: {}", status);
            }
        }
    }

    /// Legacy method for backward compatibility
    /// Use get_lyrics_smart() for new code
    pub async fn get_lyrics(&self, track: &Track) -> Result<Option<LyricsResponse>> {
        match self.get_lyrics_smart(track).await? {
            SearchResult::Found(lyrics) | SearchResult::PotentialMatch { lyrics, .. } => {
                Ok(Some(lyrics))
            }
            SearchResult::NotFound => Ok(None),
        }
    }
}

impl Default for LrcLibClient {
    fn default() -> Self {
        Self::new()
    }
}

use regex::Regex;
use std::sync::OnceLock;

/// Lazy-initialized regex patterns for metadata cleaning
static TRACK_NUMBER_REGEX: OnceLock<Regex> = OnceLock::new();
static FEAT_REGEX: OnceLock<Regex> = OnceLock::new();
static WHITESPACE_REGEX: OnceLock<Regex> = OnceLock::new();

/// Initialize regex patterns (called once)
fn get_track_number_regex() -> &'static Regex {
    TRACK_NUMBER_REGEX.get_or_init(|| Regex::new(r"^\d+[\.\-\s]+").unwrap())
}

fn get_feat_regex() -> &'static Regex {
    FEAT_REGEX.get_or_init(|| {
        Regex::new(r"(?i)\s*[\(\[]?\s*(feat\.?|ft\.?|featuring|with|w/)\s+[^\)\]]*[\)\]]?").unwrap()
    })
}

fn get_whitespace_regex() -> &'static Regex {
    WHITESPACE_REGEX.get_or_init(|| Regex::new(r"\s+").unwrap())
}

/// Normalized metadata for searching
#[derive(Debug, Clone)]
pub struct NormalizedMetadata {
    pub artist: String,
    pub title: String,
    pub album: String,
    pub original_artist: String,
    pub original_title: String,
    pub original_album: String,
    pub cleaned_original_title: String, // Original title with track number removed
}

/// Clean a string by applying normalization rules (for artist/album names)
/// Does NOT remove track numbers (use clean_string for titles)
fn clean_artist_or_album(input: &str) -> String {
    let mut result = input.to_string();

    // Replace underscores, hyphens, and ampersands with spaces (word separators)
    result = result.replace(['_', '-', '&'], " ");

    // Remove dots (for acronyms like P.I.M.P. -> PIMP)
    result = result.replace('.', "");

    // Remove all types of apostrophes and quotes (for matching "Robbin'" vs "Robbin")
    // Includes: ' " ‘ ’ “ ”
    result = result.replace(['\'', '"', '‘', '’', '“', '”'], "");

    // Normalize whitespace
    result = get_whitespace_regex().replace_all(&result, " ").to_string();

    // Trim and convert to lowercase
    result.trim().to_lowercase()
}

/// Clean a string by applying all normalization rules (for titles)
/// Removes track numbers from the beginning
pub fn clean_string(input: &str) -> String {
    let mut result = input.to_string();

    // Remove track numbers from the beginning
    result = get_track_number_regex().replace(&result, "").to_string();

    // Replace underscores, hyphens, and ampersands with spaces (word separators)
    result = result.replace(['_', '-', '&'], " ");

    // Remove dots (for acronyms like P.I.M.P. -> PIMP)
    result = result.replace('.', "");

    // Remove all types of apostrophes and quotes (for matching "Robbin'" vs "Robbin")
    // Includes: ' " ‘ ’ “ ”
    result = result.replace(['\'', '"', '‘', '’', '“', '”'], "");

    // Normalize whitespace
    result = get_whitespace_regex().replace_all(&result, " ").to_string();

    // Trim and convert to lowercase
    result.trim().to_lowercase()
}

/// Clean a title by removing featuring artists AND parenthetical content
/// This is for the fallback search - removes everything extra
pub fn clean_title(title: &str) -> String {
    let mut result = title.to_string();

    // Remove featuring artists
    result = get_feat_regex().replace_all(&result, "").to_string();

    // Remove parenthetical and bracketed content ENTIRELY (including the content inside)
    // This regex removes everything from ( to ) or [ to ]
    result = regex::Regex::new(r"\([^)]*\)|\[[^\]]*\]")
        .unwrap()
        .replace_all(&result, "")
        .to_string();

    clean_string(&result)
}

/// Clean a title but keep parenthetical content (for first attempt)
/// Removes the parentheses themselves but keeps the text inside
pub fn clean_title_keep_parens(title: &str) -> String {
    let mut result = title.to_string();

    // Remove featuring artists
    result = get_feat_regex().replace_all(&result, "").to_string();

    // Remove parentheses but keep content
    result = result.replace(['(', ')'], " ");
    result = result.replace(['[', ']'], " ");

    clean_string(&result)
}

/// Normalize metadata for searching with multiple fallback levels
pub fn normalize_metadata(artist: &str, title: &str, album: &str) -> NormalizedMetadata {
    NormalizedMetadata {
        artist: clean_artist_or_album(artist),
        title: clean_title_keep_parens(title),
        album: clean_artist_or_album(album),
        original_artist: artist.to_string(),
        original_title: title.to_string(),
        original_album: album.to_string(),
        cleaned_original_title: get_cleaned_original_title(title),
    }
}

/// Get a stripped version of the title (removes all extras)
pub fn get_stripped_title(normalized: &NormalizedMetadata) -> String {
    clean_title(&normalized.original_title)
}

/// Get a cleaned version of the original title for API queries
/// Removes track numbers and apostrophes but keeps everything else (including remix info, featuring, etc.)
pub fn get_cleaned_original_title(original_title: &str) -> String {
    let mut result = original_title.to_string();
    
    // Remove track numbers from the beginning
    result = get_track_number_regex().replace(&result, "").to_string();
    
    // Remove all types of apostrophes and quotes (for matching "Robbin'" vs "Robbin")
    result = result.replace(['\'', '"', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}'], "");
    
    result.trim().to_string()
}

/// Get a stripped original title for fallback API queries
/// Removes track numbers, featuring artists, and parenthetical content
pub fn get_stripped_original_title(original_title: &str) -> String {
    let mut result = original_title.to_string();

    // Remove track numbers
    result = get_track_number_regex().replace(&result, "").to_string();

    // Remove featuring artists
    result = get_feat_regex().replace_all(&result, "").to_string();

    // Remove all parenthetical and bracketed content
    result = result.replace(|c| c == '(' || c == ')' || c == '[' || c == ']', "");

    result.trim().to_string()
}

/// Calculate similarity score between two strings (0.0 to 1.0)
pub fn similarity_score(a: &str, b: &str) -> f64 {
    strsim::jaro_winkler(a, b)
}

/// Check if two strings are similar enough to be considered a match
pub fn is_similar(a: &str, b: &str, threshold: f64) -> bool {
    similarity_score(a, b) >= threshold
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_string() {
        // clean_string is for titles - removes track numbers
        assert_eq!(clean_string("20. Song Name"), "song name");
        assert_eq!(clean_string("P.I.M.P."), "pimp"); // Acronyms: dots removed
        assert_eq!(clean_string("Song_Name-Here"), "song name here");
        assert_eq!(clean_string("  Extra   Spaces  "), "extra spaces");
        // "50 Cent" would have "50 " removed as a track number
        assert_eq!(clean_string("50 Cent"), "cent");
    }

    #[test]
    fn test_clean_title() {
        // Removes featuring artists AND parenthetical content entirely
        assert_eq!(clean_title("P.I.M.P. (feat. Snoop Dogg)"), "pimp");
        assert_eq!(clean_title("Song Name (Remix)"), "song name"); // Remix removed!
        assert_eq!(clean_title("Track ft. Artist"), "track");
        assert_eq!(clean_title("Song (Live)"), "song"); // Live removed!
    }

    #[test]
    fn test_clean_title_keep_parens() {
        // Removes featuring artists AND parentheses, keeps content
        assert_eq!(clean_title_keep_parens("P.I.M.P. (Remix)"), "pimp remix");
        assert_eq!(clean_title_keep_parens("Song (feat. Artist)"), "song");
        assert_eq!(
            clean_title_keep_parens("Track (Acoustic)"),
            "track acoustic"
        );
    }

    #[test]
    fn test_similarity_score() {
        assert!(similarity_score("hello", "hello") > 0.99);
        assert!(similarity_score("hello", "helo") > 0.8);
        assert!(similarity_score("hello", "world") < 0.5);
    }

    #[test]
    fn test_is_similar() {
        assert!(is_similar("P.I.M.P.", "PIMP", 0.7));
        assert!(is_similar("50 Cent", "50cent", 0.8));
        assert!(!is_similar("completely", "different", 0.8));
    }

    #[test]
    fn test_pimp_regression_fix() {
        // Test the specific examples from the bug report

        // Example 1: "11. 50 Cent - P.I.M.P..flac"
        let normalized = normalize_metadata("50 Cent", "11. P.I.M.P.", "Get Rich or Die Tryin'");
        assert_eq!(normalized.artist, "50 cent"); // Artist name preserved
        assert_eq!(normalized.title, "pimp"); // Track number removed, dots removed

        // Verify fuzzy matching would work with API response
        let api_artist = "50 Cent".to_lowercase();
        let api_title = "PIMP".to_lowercase();

        let artist_sim = similarity_score(&normalized.artist, &api_artist);
        let title_sim = similarity_score(&normalized.title, &api_title);
        let avg_sim = (artist_sim + title_sim) / 2.0;

        println!("Artist similarity: {:.4}", artist_sim);
        println!("Title similarity: {:.4}", title_sim);
        println!("Average similarity: {:.4}", avg_sim);

        assert!(artist_sim >= 0.85, "Artist similarity should be high");
        assert!(title_sim >= 0.85, "Title similarity should be high");
        assert!(avg_sim >= 0.85, "Should match with ≥85% similarity");

        // Example 2: Remix version (should still normalize correctly)
        let normalized_remix = normalize_metadata(
            "50 Cent",
            "20. P.I.M.P. (Snoop Dogg remix) (feat. Snoop Dogg and Lloyd Banks & Young Buck of G Unit)",
            "Get Rich or Die Tryin'"
        );
        assert_eq!(normalized_remix.artist, "50 cent");
        // First attempt keeps parenthetical content (minus featuring)
        assert_eq!(normalized_remix.title, "pimp snoop dogg remix");

        // Stripped version removes ALL parenthetical content
        let stripped = get_stripped_title(&normalized_remix);
        assert_eq!(stripped, "pimp"); // Remix info removed!
    }
}

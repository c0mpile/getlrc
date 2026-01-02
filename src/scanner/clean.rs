use regex::Regex;
use std::sync::OnceLock;

/// Lazy-initialized regex patterns for metadata cleaning
static TRACK_NUMBER_REGEX: OnceLock<Regex> = OnceLock::new();
static FEAT_REGEX: OnceLock<Regex> = OnceLock::new();
static WHITESPACE_REGEX: OnceLock<Regex> = OnceLock::new();
static PUNCTUATION_REGEX: OnceLock<Regex> = OnceLock::new();

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

fn get_punctuation_regex() -> &'static Regex {
    PUNCTUATION_REGEX.get_or_init(|| Regex::new(r"[_\-&\.]").unwrap())
}

/// Normalized metadata for searching
#[derive(Debug, Clone)]
pub struct NormalizedMetadata {
    pub artist: String,
    pub title: String,
    pub album: String,
    pub original_artist: String,
    pub original_title: String,
}

/// Clean a string by applying all normalization rules
pub fn clean_string(input: &str) -> String {
    let mut result = input.to_string();

    // Remove track numbers from the beginning
    result = get_track_number_regex().replace(&result, "").to_string();

    // Replace punctuation with spaces
    result = get_punctuation_regex()
        .replace_all(&result, " ")
        .to_string();

    // Normalize whitespace
    result = get_whitespace_regex().replace_all(&result, " ").to_string();

    // Trim and convert to lowercase
    result.trim().to_lowercase()
}

/// Clean a title by removing featuring artists but preserving version info
/// Keeps: Remix, Live, Acoustic, Unplugged, Radio Edit, etc.
/// Removes: Featuring artists only
pub fn clean_title(title: &str) -> String {
    let mut result = title.to_string();

    // Remove featuring artists (but not other parentheticals)
    result = get_feat_regex().replace_all(&result, "").to_string();

    // Remove parentheses but keep the content inside
    // This preserves version info like "Remix", "Live", etc.
    result = result.replace(['(', ')'], " ");
    result = result.replace(['[', ']'], " ");

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
        artist: clean_string(artist),
        title: clean_title_keep_parens(title),
        album: clean_string(album),
        original_artist: artist.to_string(),
        original_title: title.to_string(),
    }
}

/// Get a stripped version of the title (removes all extras)
pub fn get_stripped_title(normalized: &NormalizedMetadata) -> String {
    clean_title(&normalized.original_title)
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
        assert_eq!(clean_string("20. Song Name"), "song name");
        assert_eq!(clean_string("Artist_Name-Here"), "artist name here");
        assert_eq!(clean_string("  Extra   Spaces  "), "extra spaces");
    }

    #[test]
    fn test_clean_title() {
        // Removes featuring artists but preserves version info
        assert_eq!(clean_title("P.I.M.P. (feat. Snoop Dogg)"), "p i m p");
        assert_eq!(clean_title("Song Name (Remix)"), "song name remix"); // Remix preserved!
        assert_eq!(clean_title("Track ft. Artist"), "track");
        assert_eq!(clean_title("Song (Live)"), "song live"); // Live preserved!
    }

    #[test]
    fn test_clean_title_keep_parens() {
        // Same behavior - both preserve version info
        assert_eq!(clean_title_keep_parens("P.I.M.P. (Remix)"), "p i m p remix");
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
}

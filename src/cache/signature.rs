use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Serialize, Debug)]
pub struct TrackSignature {
    pub artist: String,
    pub title: String,
    pub album: Option<String>,
    pub duration_sec: u32,
}

impl TrackSignature {
    /// Generates a hex-encoded SHA-256 hash of the track metadata
    pub fn generate_hash(&self) -> String {
        let mut hasher = Sha256::new();
        // Serialize the struct to JSON to ensure consistent input for the hasher
        if let Ok(serialized) = serde_json::to_vec(self) {
            hasher.update(&serialized);
        }
        let result = hasher.finalize();
        format!("{:x}", result) // Convert bytes to a hex string
    }
}

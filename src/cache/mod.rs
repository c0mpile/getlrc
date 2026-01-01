pub mod signature;

use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;

pub struct NegativeCache {
    conn: Connection,
}

impl NegativeCache {
    /// Open or create the SQLite database
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;

        // Create table if it doesn't exist
        conn.execute(
            "CREATE TABLE IF NOT EXISTS negative_cache (
                signature TEXT PRIMARY KEY,
                timestamp INTEGER NOT NULL
            )",
            [],
        )?;

        Ok(Self { conn })
    }

    /// Check if a track signature is in the negative cache
    pub fn is_cached(&self, signature: &str) -> Result<bool> {
        let mut stmt = self
            .conn
            .prepare("SELECT 1 FROM negative_cache WHERE signature = ?1")?;

        let exists = stmt.exists([signature])?;
        Ok(exists)
    }

    /// Add a track signature to the negative cache
    pub fn add(&self, signature: &str) -> Result<()> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        self.conn.execute(
            "INSERT OR REPLACE INTO negative_cache (signature, timestamp) VALUES (?1, ?2)",
            [signature, &timestamp.to_string()],
        )?;

        Ok(())
    }
}

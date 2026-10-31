use rusqlite::params;
use rusqlite::Connection;
use rusqlite::OptionalExtension;

use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use std::collections::HashMap;

use super::types::Result;

const HASH_CACHE_TTL_SEC: i64 = 604_800; // 7 days

pub struct Vault {
    connection: Connection,
}

impl Vault {
    pub fn open(path: &str) -> Result<Self> {
        let connection = Connection::open(path)?;

        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "NORMAL")?;
        connection.pragma_update(None, "foreign_keys", "ON")?;

        connection.execute_batch(include_str!("schema.sql"))?;

        Ok(Self { connection })
    }

    // --- Hash Management ---

    /// returns a list of all file metadata with matching filepaths in the cache
    pub fn get_hashes(
        &self,
        paths: &[String],
    ) -> Result<HashMap<String, (u64, u64, [u8; 32])>> {
        let mut result = HashMap::new();

        const BATCH_SIZE: usize = 900;

        for chunk in paths.chunks(BATCH_SIZE) {
            let placeholders = std::iter::repeat("?")
                .take(chunk.len())
                .collect::<Vec<_>>()
                .join(", ");

            let sql = format!(
                "SELECT file_path, file_size, mtime, file_hash \
                 FROM hash_cache WHERE file_path IN ({})",
                placeholders
            );

            let mut stmt = self.connection.prepare(&sql)?;

            let rows = stmt.query_map(
                rusqlite::params_from_iter(chunk.iter()),
                |row| {
                    let path: String = row.get(0)?;
                    let size: i64 = row.get(1)?;
                    let mtime: i64 = row.get(2)?;
                    let hash: Vec<u8> = row.get(3)?;
                    Ok((path, size as u64, mtime as u64, hash))
                },
            )?;

            for row in rows {
                let (path, size, mtime, hash) = row?;
                if let Ok(arr) = <[u8; 32]>::try_from(hash) {
                    result.insert(path, (size, mtime, arr));
                }
            }
        }

        Ok(result)
    }
    
    /// add a new file entry to the hash cache
    pub fn insert_hash(
        &self,
        filepath: String,
        size: u64,
        mtime: u64,
        hash: [u8; 32],
    ) -> Result<()> {
        let now = now_unix();

        self.connection.execute(
            "INSERT INTO hash_cache (file_path, file_size, mtime, file_hash, ctime) \
             VALUES (?1, ?2, ?3, ?4, ?5) \
             ON CONFLICT(file_path) DO UPDATE SET \
                file_size = excluded.file_size, \
                mtime     = excluded.mtime, \
                file_hash = excluded.file_hash",
            params![&filepath, size as i64, mtime as i64, &hash[..], &now],
        )?;

        Ok(())
    }
    
    // run at Beacon boot to purge all stale caches
    pub fn purge_hashes(&self) -> Result<()> {
        let now = now_unix();
        
        self.connection.execute(
            "DELETE FROM hash_cache WHERE ctime < ?1", 
            params![now - HASH_CACHE_TTL_SEC]
        )?;

        Ok(())
    }
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
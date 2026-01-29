use crate::error::Result;

/// Manages lazy-loaded photo caching using DuckDB
pub struct PhotoCache<'a> {
    conn: &'a duckdb::Connection,
}

impl<'a> PhotoCache<'a> {
    /// Creates a new PhotoCache with a connection reference
    pub fn new(conn: &'a duckdb::Connection) -> Self {
        Self { conn }
    }

    /// Creates the photo cache table if it doesn't exist
    pub fn create_table(&self) -> Result<()> {
        let sql = "
            CREATE TABLE IF NOT EXISTS photo_cache (
                photo_path VARCHAR PRIMARY KEY,
                cached_file_path VARCHAR NOT NULL,
                file_size BIGINT NOT NULL,
                last_accessed TIMESTAMP NOT NULL,
                extracted_at TIMESTAMP NOT NULL
            )
        ";

        self.conn.execute(sql, []).map_err(|e| {
            log::error!("Failed to create photo_cache table: {e}");
            e
        })?;

        // Create index on last_accessed for LRU eviction queries
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_last_accessed ON photo_cache(last_accessed)",
            [],
        )?;

        log::info!("Photo cache table created successfully");
        Ok(())
    }

    /// Adds or updates a photo in the cache
    pub fn add_photo(
        &self,
        photo_path: &str,
        cached_file_path: &str,
        file_size: i64,
    ) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();

        self.conn.execute(
            "INSERT OR REPLACE INTO photo_cache
             (photo_path, cached_file_path, file_size, last_accessed, extracted_at)
             VALUES (?, ?, ?, ?, ?)",
            duckdb::params![photo_path, cached_file_path, file_size, &now, &now],
        )?;

        Ok(())
    }

    /// Updates the last accessed time for a photo
    pub fn update_access_time(&self, photo_path: &str) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();

        self.conn.execute(
            "UPDATE photo_cache SET last_accessed = ? WHERE photo_path = ?",
            duckdb::params![&now, photo_path],
        )?;

        Ok(())
    }

    /// Gets a cached photo path if it exists
    pub fn get_cached_photo(&self, photo_path: &str) -> Result<Option<String>> {
        let result = self.conn.query_row(
            "SELECT cached_file_path FROM photo_cache WHERE photo_path = ?",
            [photo_path],
            |row| row.get(0),
        );

        match result {
            Ok(path) => Ok(Some(path)),
            Err(duckdb::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Gets the total size of cached photos in bytes
    pub fn get_cache_size(&self) -> Result<i64> {
        let size: Option<i64> = self.conn.query_row(
            "SELECT COALESCE(SUM(file_size), 0) FROM photo_cache",
            [],
            |row| row.get(0),
        )?;

        Ok(size.unwrap_or(0))
    }

    /// Evicts least recently used photos until cache is under the size limit
    /// Returns the number of photos evicted
    pub fn evict_lru(&self, max_cache_size: i64) -> Result<usize> {
        let current_size = self.get_cache_size()?;

        if current_size <= max_cache_size {
            return Ok(0);
        }

        // Get photos ordered by least recently used
        let mut stmt = self.conn.prepare(
            "SELECT photo_path, cached_file_path, file_size
             FROM photo_cache
             ORDER BY last_accessed ASC"
        )?;

        let photos: Vec<(String, String, i64)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let mut size_to_free = current_size - max_cache_size;
        let mut evicted = 0;

        for (photo_path, cached_file_path, file_size) in photos {
            if size_to_free <= 0 {
                break;
            }

            // Delete the cached file
            if let Err(e) = std::fs::remove_file(&cached_file_path) {
                log::warn!("Failed to delete cached photo {cached_file_path}: {e}");
            }

            // Remove from database
            self.conn.execute(
                "DELETE FROM photo_cache WHERE photo_path = ?",
                [&photo_path],
            )?;

            size_to_free -= file_size;
            evicted += 1;
        }

        log::info!("Evicted {evicted} photos from cache");
        Ok(evicted)
    }
}

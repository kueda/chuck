use crate::error::Result;
use std::path::{Path, PathBuf};

/// Manages lazy-loaded photo caching using the filesystem
/// Uses file modification times for LRU eviction
pub struct PhotoCache {
    cache_dir: PathBuf,
}

impl PhotoCache {
    /// Creates a new PhotoCache with a cache directory path
    pub fn new(cache_dir: &Path) -> Self {
        Self {
            cache_dir: cache_dir.to_path_buf(),
        }
    }

    /// Gets a cached photo path if it exists
    pub fn get_cached_photo(&self, photo_path: &str) -> Result<Option<PathBuf>> {
        let safe_filename = photo_path.replace(['/', '\\'], "_");
        let cached_file_path = self.cache_dir.join(&safe_filename);

        if cached_file_path.exists() {
            Ok(Some(cached_file_path))
        } else {
            Ok(None)
        }
    }

    /// Updates the file modification time to mark it as recently accessed (for LRU)
    pub fn touch_file(&self, cached_path: &Path) -> Result<()> {
        let now = filetime::FileTime::now();
        filetime::set_file_mtime(cached_path, now).map_err(|e| {
            log::warn!("Failed to update mtime for {}: {}", cached_path.display(), e);
            crate::error::ChuckError::FileRead {
                path: cached_path.to_path_buf(),
                source: e,
            }
        })?;
        Ok(())
    }

    /// Returns the path where a photo should be cached
    /// The caller is responsible for actually writing the file
    pub fn get_cache_path(&self, photo_path: &str) -> PathBuf {
        let safe_filename = photo_path.replace(['/', '\\'], "_");
        self.cache_dir.join(&safe_filename)
    }

    /// Gets the total size of cached photos in bytes by scanning the cache directory
    pub fn get_cache_size(&self) -> Result<u64> {
        if !self.cache_dir.exists() {
            return Ok(0);
        }

        let mut total_size = 0u64;
        for entry in std::fs::read_dir(&self.cache_dir)
            .map_err(|e| crate::error::ChuckError::FileRead {
                path: self.cache_dir.clone(),
                source: e,
            })?
            .flatten()
        {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    total_size += metadata.len();
                }
            }
        }

        Ok(total_size)
    }

    /// Evicts least recently used photos until cache is under the size limit
    /// Returns the number of photos evicted
    pub fn evict_lru(&self, max_cache_size: u64) -> Result<usize> {
        if !self.cache_dir.exists() {
            return Ok(0);
        }

        let current_size = self.get_cache_size()?;
        if current_size <= max_cache_size {
            return Ok(0);
        }

        // Collect files with their modification times and sizes
        let mut files: Vec<(PathBuf, std::time::SystemTime, u64)> = Vec::new();

        for entry in std::fs::read_dir(&self.cache_dir)
            .map_err(|e| crate::error::ChuckError::FileRead {
                path: self.cache_dir.clone(),
                source: e,
            })?
            .flatten()
        {
            let path = entry.path();
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    let mtime = metadata.modified().unwrap_or(std::time::UNIX_EPOCH);
                    files.push((path, mtime, metadata.len()));
                }
            }
        }

        // Sort by modification time (oldest first)
        files.sort_by_key(|(_, mtime, _)| *mtime);

        let mut size_to_free = current_size.saturating_sub(max_cache_size);
        let mut evicted = 0;

        for (path, _, file_size) in files {
            if size_to_free == 0 {
                break;
            }

            if let Err(e) = std::fs::remove_file(&path) {
                log::warn!("Failed to delete cached photo {}: {}", path.display(), e);
            } else {
                size_to_free = size_to_free.saturating_sub(file_size);
                evicted += 1;
            }
        }

        log::info!("Evicted {evicted} photos from cache");
        Ok(evicted)
    }
}

use std::path::{Path, PathBuf};
use sha2::{Digest, Sha256};
use crate::error::{ChuckError, Result};

/// Creates a storage directory for an archive file using a hash of its contents
/// Returns the path to the created directory
pub fn create_storage_dir(archive_path: &Path, base_dir: &Path) -> Result<PathBuf> {
    let mut file = std::fs::File::open(archive_path).map_err(|e| ChuckError::FileOpen {
        path: archive_path.to_path_buf(),
        source: e,
    })?;

    let fname = archive_path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| ChuckError::InvalidFileName(archive_path.to_path_buf()))?;

    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher).map_err(|e| ChuckError::FileRead {
        path: archive_path.to_path_buf(),
        source: e,
    })?;

    let file_hash = hasher.finalize();
    let file_hash_string = format!("{}-{:x}", fname, file_hash);
    let target_dir = base_dir.join(file_hash_string);

    std::fs::create_dir_all(&target_dir).map_err(|e| ChuckError::DirectoryCreate {
        path: target_dir.clone(),
        source: e,
    })?;

    Ok(target_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_create_storage_dir() {
        let temp_dir = std::env::temp_dir();
        let test_archive = temp_dir.join("test_archive.zip");
        let base_dir = temp_dir.join("chuck_test_storage");

        // Create a test file
        let mut file = std::fs::File::create(&test_archive).unwrap();
        file.write_all(b"test content").unwrap();

        let result = create_storage_dir(&test_archive, &base_dir);
        assert!(result.is_ok());

        let storage_dir = result.unwrap();
        assert!(storage_dir.exists());
        assert!(storage_dir.starts_with(&base_dir));

        // Cleanup
        std::fs::remove_dir_all(&base_dir).ok();
        std::fs::remove_file(&test_archive).ok();
    }
}

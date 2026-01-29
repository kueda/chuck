use crate::auth::AuthError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageBackendConfig {
    pub backend_type: StorageBackendType,
    pub custom_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StorageBackendType {
    Keyring,
    File,
}

impl StorageBackendConfig {
    pub fn load() -> Result<Option<Self>, AuthError> {
        let path = Self::get_config_path()?;

        if !path.exists() {
            return Ok(None);
        }

        let contents = std::fs::read_to_string(path)
            .map_err(AuthError::IoError)?;

        let config: Self = serde_json::from_str(&contents)
            .map_err(AuthError::JsonError)?;

        Ok(Some(config))
    }

    pub fn save(&self) -> Result<(), AuthError> {
        let path = Self::get_config_path()?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(AuthError::IoError)?;
        }

        let contents = serde_json::to_string_pretty(self)
            .map_err(AuthError::JsonError)?;

        std::fs::write(path, contents)
            .map_err(AuthError::IoError)?;

        Ok(())
    }

    fn get_config_path() -> Result<PathBuf, AuthError> {
        let config_dir = dirs::config_dir().ok_or_else(|| {
            AuthError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not find config directory",
            ))
        })?;

        Ok(config_dir.join("chuck").join("config.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_save_and_load() {
        // Note: This test uses actual config directory
        // Save a test config
        let config = StorageBackendConfig {
            backend_type: StorageBackendType::File,
            custom_path: Some(PathBuf::from("/tmp/test_path.json")),
        };

        config.save().unwrap();

        // Load it back
        let loaded = StorageBackendConfig::load().unwrap();
        assert!(loaded.is_some());

        let loaded_config = loaded.unwrap();
        assert!(matches!(loaded_config.backend_type, StorageBackendType::File));
        assert_eq!(loaded_config.custom_path, Some(PathBuf::from("/tmp/test_path.json")));

        // Clean up
        let config_path = dirs::config_dir().unwrap().join("chuck").join("config.json");
        let _ = std::fs::remove_file(config_path);
    }

    #[test]
    fn test_config_load_nonexistent() {
        // Make sure config doesn't exist
        let config_path = dirs::config_dir().unwrap().join("chuck").join("config.json");
        let _ = std::fs::remove_file(&config_path);

        let loaded = StorageBackendConfig::load().unwrap();
        assert!(loaded.is_none());
    }
}

use crate::auth::{AuthError, AuthToken, TokenStorage};
use std::path::PathBuf;
use serde_json;

pub struct CustomFileStorage {
    path: PathBuf,
}

impl CustomFileStorage {
    pub fn new(path: PathBuf) -> Result<Self, AuthError> {
        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(AuthError::IoError)?;
        }
        Ok(Self { path })
    }
}

impl TokenStorage for CustomFileStorage {
    fn save_token(&self, token: &AuthToken) -> Result<(), AuthError> {
        let contents = serde_json::to_string_pretty(token)
            .map_err(AuthError::JsonError)?;

        std::fs::write(&self.path, contents)
            .map_err(AuthError::IoError)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&self.path)
                .map_err(AuthError::IoError)?
                .permissions();
            perms.set_mode(0o600);
            std::fs::set_permissions(&self.path, perms)
                .map_err(AuthError::IoError)?;
        }

        Ok(())
    }

    fn load_token(&self) -> Result<Option<AuthToken>, AuthError> {
        if !self.path.exists() {
            return Ok(None);
        }

        let contents = std::fs::read_to_string(&self.path)
            .map_err(AuthError::IoError)?;

        let token: AuthToken = serde_json::from_str(&contents)
            .map_err(AuthError::JsonError)?;

        if token.is_expired() {
            return Err(AuthError::TokenExpired);
        }

        Ok(Some(token))
    }

    fn clear_token(&self) -> Result<(), AuthError> {
        if self.path.exists() {
            std::fs::remove_file(&self.path)
                .map_err(AuthError::IoError)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use chrono::{Utc, Duration};

    #[test]
    fn test_save_and_load_token() {
        let temp_dir = TempDir::new().unwrap();
        let token_path = temp_dir.path().join("test_token.json");

        let storage = CustomFileStorage::new(token_path.clone()).unwrap();

        let token = AuthToken {
            access_token: "test_access".to_string(),
            refresh_token: Some("test_refresh".to_string()),
            expires_at: Some(Utc::now() + Duration::hours(1)),
            token_type: "Bearer".to_string(),
        };

        // Save token
        storage.save_token(&token).unwrap();

        // Verify file exists
        assert!(token_path.exists());

        // Load token
        let loaded = storage.load_token().unwrap();
        assert!(loaded.is_some());

        let loaded_token = loaded.unwrap();
        assert_eq!(loaded_token.access_token, "test_access");
        assert_eq!(loaded_token.refresh_token, Some("test_refresh".to_string()));
    }

    #[test]
    fn test_load_nonexistent_token() {
        let temp_dir = TempDir::new().unwrap();
        let token_path = temp_dir.path().join("nonexistent.json");

        let storage = CustomFileStorage::new(token_path).unwrap();

        let loaded = storage.load_token().unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn test_clear_token() {
        let temp_dir = TempDir::new().unwrap();
        let token_path = temp_dir.path().join("test_token.json");

        let storage = CustomFileStorage::new(token_path.clone()).unwrap();

        let token = AuthToken {
            access_token: "test_access".to_string(),
            refresh_token: None,
            expires_at: Some(Utc::now() + Duration::hours(1)),
            token_type: "Bearer".to_string(),
        };

        storage.save_token(&token).unwrap();
        assert!(token_path.exists());

        storage.clear_token().unwrap();
        assert!(!token_path.exists());
    }

    #[test]
    fn test_parent_directory_creation() {
        let temp_dir = TempDir::new().unwrap();
        let token_path = temp_dir.path().join("nested/dir/token.json");

        // Parent directory doesn't exist yet
        assert!(!token_path.parent().unwrap().exists());

        let storage = CustomFileStorage::new(token_path.clone()).unwrap();

        let token = AuthToken {
            access_token: "test".to_string(),
            refresh_token: None,
            expires_at: Some(Utc::now() + Duration::hours(1)),
            token_type: "Bearer".to_string(),
        };

        storage.save_token(&token).unwrap();

        // Parent directory should be created
        assert!(token_path.parent().unwrap().exists());
        assert!(token_path.exists());
    }

    #[cfg(unix)]
    #[test]
    fn test_file_permissions_unix() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new().unwrap();
        let token_path = temp_dir.path().join("test_token.json");

        let storage = CustomFileStorage::new(token_path.clone()).unwrap();

        let token = AuthToken {
            access_token: "test".to_string(),
            refresh_token: None,
            expires_at: Some(Utc::now() + Duration::hours(1)),
            token_type: "Bearer".to_string(),
        };

        storage.save_token(&token).unwrap();

        let metadata = std::fs::metadata(&token_path).unwrap();
        let permissions = metadata.permissions();

        // Should be 0o600 (read/write owner only)
        assert_eq!(permissions.mode() & 0o777, 0o600);
    }
}

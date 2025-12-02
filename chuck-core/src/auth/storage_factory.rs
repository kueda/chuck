use crate::auth::{AuthError, AuthToken, TokenStorage, CustomFileStorage, StorageBackendConfig, StorageBackendType};
#[cfg(feature = "keyring-storage")]
use crate::auth::KeyringStorage;
use std::path::PathBuf;
use std::io::{self, Write};

pub enum StorageInstance {
    #[cfg(feature = "keyring-storage")]
    Keyring(KeyringStorage),
    File(CustomFileStorage),
}

impl TokenStorage for StorageInstance {
    fn save_token(&self, token: &AuthToken) -> Result<(), AuthError> {
        match self {
            #[cfg(feature = "keyring-storage")]
            StorageInstance::Keyring(s) => s.save_token(token),
            StorageInstance::File(s) => s.save_token(token),
        }
    }

    fn load_token(&self) -> Result<Option<AuthToken>, AuthError> {
        match self {
            #[cfg(feature = "keyring-storage")]
            StorageInstance::Keyring(s) => s.load_token(),
            StorageInstance::File(s) => s.load_token(),
        }
    }

    fn clear_token(&self) -> Result<(), AuthError> {
        match self {
            #[cfg(feature = "keyring-storage")]
            StorageInstance::Keyring(s) => s.clear_token(),
            StorageInstance::File(s) => s.clear_token(),
        }
    }
}

pub struct StorageFactory;

impl StorageFactory {
    /// Auto-detect storage without user interaction (for non-interactive contexts)
    pub fn create() -> Result<StorageInstance, AuthError> {
        // Try loading saved config first
        if let Ok(Some(config)) = StorageBackendConfig::load() {
            return Self::create_from_config(&config);
        }

        // No saved config - try keyring auto-detect
        Self::create_auto_detect()
    }

    /// Interactive creation for CLI (prompts user if needed)
    pub fn create_interactive() -> Result<StorageInstance, AuthError> {
        // Try loading saved config first
        if let Ok(Some(config)) = StorageBackendConfig::load() {
            return Self::create_from_config(&config);
        }

        // No saved config - try keyring
        #[cfg(feature = "keyring-storage")]
        {
            if KeyringStorage::is_available() {
                let storage = KeyringStorage::new()?;
                Self::save_config(StorageBackendType::Keyring, None)?;
                println!("Using OS keyring for secure token storage.");
                return Ok(StorageInstance::Keyring(storage));
            }
        }

        // Keyring unavailable - prompt for custom path
        println!("\nOS keyring is not available in this environment.");
        println!("This is normal for SSH sessions and headless servers.\n");
        println!("Please specify a custom path for storing authentication tokens.");
        println!("This file will contain sensitive information and should be kept secure.\n");

        let custom_path = Self::prompt_for_storage_path()?;
        let storage = CustomFileStorage::new(custom_path.clone())?;
        Self::save_config(StorageBackendType::File, Some(custom_path))?;

        println!("\nToken storage configured successfully!");
        Ok(StorageInstance::File(storage))
    }

    fn create_from_config(config: &StorageBackendConfig) -> Result<StorageInstance, AuthError> {
        match config.backend_type {
            #[cfg(feature = "keyring-storage")]
            StorageBackendType::Keyring => {
                KeyringStorage::new()
                    .map(StorageInstance::Keyring)
                    .map_err(|_| AuthError::OAuthFailed(
                        "Configured to use keyring but it's unavailable. Delete ~/.config/chuck/config.json and run 'chuck auth' to reconfigure.".to_string()
                    ))
            }
            StorageBackendType::File => {
                let path = config.custom_path.as_ref()
                    .ok_or_else(|| AuthError::OAuthFailed(
                        "File storage configured but no path specified".to_string()
                    ))?;
                CustomFileStorage::new(path.clone())
                    .map(StorageInstance::File)
            }
            #[cfg(not(feature = "keyring-storage"))]
            StorageBackendType::Keyring => {
                Err(AuthError::OAuthFailed(
                    "Keyring storage not available (feature not enabled)".to_string()
                ))
            }
        }
    }

    fn create_auto_detect() -> Result<StorageInstance, AuthError> {
        #[cfg(feature = "keyring-storage")]
        {
            if let Ok(storage) = KeyringStorage::new() {
                Self::save_config(StorageBackendType::Keyring, None)?;
                return Ok(StorageInstance::Keyring(storage));
            }
        }

        Err(AuthError::OAuthFailed(
            "OS keyring unavailable. Run 'chuck auth' to configure file-based storage.".to_string()
        ))
    }

    fn prompt_for_storage_path() -> Result<PathBuf, AuthError> {
        let default_path = dirs::config_dir()
            .ok_or_else(|| AuthError::IoError(io::Error::new(
                io::ErrorKind::NotFound,
                "Could not find config directory"
            )))?
            .join("crinat")
            .join("auth.json");

        println!("Default path: {}", default_path.display());
        print!("Enter custom path (or press Enter for default): ");
        io::stdout().flush()
            .map_err(|e| AuthError::IoError(e))?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)
            .map_err(|e| AuthError::IoError(e))?;

        let path = if input.trim().is_empty() {
            default_path
        } else {
            PathBuf::from(input.trim())
        };

        Ok(path)
    }

    fn save_config(backend_type: StorageBackendType, custom_path: Option<PathBuf>) -> Result<(), AuthError> {
        let config = StorageBackendConfig {
            backend_type,
            custom_path,
        };
        config.save()
    }
}

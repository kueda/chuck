use chuck_core::auth::{AuthToken, TokenStorage, AuthError};
use tauri::AppHandle;
use keyring::Entry;

/// Secure storage implementation for Tauri apps using OS-level credential storage.
///
/// This uses the operating system's secure credential store:
/// - macOS: Keychain
/// - Windows: Credential Manager
/// - Linux: Secret Service API (libsecret)
///
/// Tokens are stored under:
/// - Service: "Chuck"
/// - Account: "iNaturalist access token"
///
/// The OAuth token is encrypted at rest by the OS and can only be accessed
/// by this application.
pub struct KeyringStorage {
    service_name: String,
    account_name: String,
}

impl KeyringStorage {
    pub fn new(_app: AppHandle) -> Result<Self, AuthError> {
        Ok(Self {
            service_name: "Chuck".to_string(),
            account_name: "iNaturalist access token".to_string(),
        })
    }

    fn get_entry(&self) -> Result<Entry, AuthError> {
        Entry::new(&self.service_name, &self.account_name)
            .map_err(|e| AuthError::OAuthFailed(format!("Failed to access secure storage: {}", e)))
    }
}

impl TokenStorage for KeyringStorage {
    fn save_token(&self, token: &AuthToken) -> Result<(), AuthError> {
        let entry = self.get_entry()?;

        // Serialize token to JSON
        let json = serde_json::to_string(token)
            .map_err(|e| AuthError::OAuthFailed(format!("Failed to serialize token: {}", e)))?;

        // Store in OS keyring/keychain (encrypted)
        entry.set_password(&json)
            .map_err(|e| AuthError::OAuthFailed(format!("Failed to store token: {}", e)))?;

        Ok(())
    }

    fn load_token(&self) -> Result<Option<AuthToken>, AuthError> {
        let entry = self.get_entry()?;

        // Try to read from OS keyring
        match entry.get_password() {
            Ok(json) => {
                // Deserialize token from JSON
                let token: AuthToken = serde_json::from_str(&json)
                    .map_err(|e| AuthError::OAuthFailed(format!("Failed to deserialize token: {}", e)))?;
                Ok(Some(token))
            }
            Err(keyring::Error::NoEntry) => {
                // Token doesn't exist
                Ok(None)
            }
            Err(e) => {
                Err(AuthError::OAuthFailed(format!("Failed to load token: {}", e)))
            }
        }
    }

    fn clear_token(&self) -> Result<(), AuthError> {
        let entry = self.get_entry()?;
        match entry.delete_credential() {
            Ok(_) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(AuthError::OAuthFailed(format!("Failed to clear token: {}", e))),
        }
    }
}

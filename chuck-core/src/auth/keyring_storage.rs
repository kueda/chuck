use crate::auth::{AuthError, AuthToken, TokenStorage};
use keyring::Entry;
use serde_json;

pub struct KeyringStorage {
    service_name: &'static str,
    account_name: &'static str,
}

impl KeyringStorage {
    pub fn new() -> Result<Self, AuthError> {
        Ok(Self {
            service_name: "Chuck",
            account_name: "iNaturalist access token",
        })
    }

    pub fn is_available() -> bool {
        // Try to create an entry to test availability
        Entry::new("Chuck", "iNaturalist access token").is_ok()
    }

    /// Initialize keyring storage for use as application state
    /// Returns None if keyring is unavailable (e.g., on Linux without secret service)
    pub fn init() -> Option<Self> {
        Self::new().ok()
    }

    fn get_entry(&self) -> Result<Entry, AuthError> {
        Entry::new(self.service_name, self.account_name)
            .map_err(|e| AuthError::OAuthFailed(format!("Keyring unavailable: {}", e)))
    }
}

impl TokenStorage for KeyringStorage {
    fn save_token(&self, token: &AuthToken) -> Result<(), AuthError> {
        let token_json = serde_json::to_string(token)
            .map_err(|e| AuthError::JsonError(e))?;

        self.get_entry()?.set_password(&token_json)
            .map_err(|e| AuthError::OAuthFailed(format!("Failed to save to keyring: {}", e)))
    }

    fn load_token(&self) -> Result<Option<AuthToken>, AuthError> {
        log::info!("[KeyringStorage] Loading token from keychain");
        let entry = self.get_entry()?;

        log::info!("[KeyringStorage] Calling get_password() on Entry (accessing keychain)");
        match entry.get_password() {
            Ok(token_json) => {
                log::info!("[KeyringStorage] Successfully retrieved password from keychain");
                let token: AuthToken = serde_json::from_str(&token_json)
                    .map_err(|e| AuthError::JsonError(e))?;

                if token.is_expired() {
                    return Err(AuthError::TokenExpired);
                }

                Ok(Some(token))
            }
            Err(_) => {
                log::info!("[KeyringStorage] No token found in keychain");
                Ok(None)
            },
        }
    }

    fn clear_token(&self) -> Result<(), AuthError> {
        let entry = self.get_entry()?;
        match entry.delete_credential() {
            Ok(_) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(AuthError::OAuthFailed(format!("Failed to clear keyring: {}", e))),
        }
    }
}

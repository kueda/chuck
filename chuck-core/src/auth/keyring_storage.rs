use crate::auth::{AuthError, AuthToken, TokenStorage};
use keyring::Entry;
use serde_json;

pub struct KeyringStorage {
    service_name: String,
    account_name: String,
}

impl KeyringStorage {
    pub fn new() -> Result<Self, AuthError> {
        let storage = Self {
            service_name: "Chuck".to_string(),
            account_name: "iNaturalist access token".to_string(),
        };

        // Test availability by attempting to get entry
        storage.get_entry()?;
        Ok(storage)
    }

    pub fn is_available() -> bool {
        Self::new().is_ok()
    }

    fn get_entry(&self) -> Result<Entry, AuthError> {
        Entry::new(&self.service_name, &self.account_name)
            .map_err(|e| AuthError::OAuthFailed(format!("Keyring unavailable: {}", e)))
    }
}

impl TokenStorage for KeyringStorage {
    fn save_token(&self, token: &AuthToken) -> Result<(), AuthError> {
        let entry = self.get_entry()?;
        let token_json = serde_json::to_string(token)
            .map_err(|e| AuthError::JsonError(e))?;

        entry.set_password(&token_json)
            .map_err(|e| AuthError::OAuthFailed(format!("Failed to save to keyring: {}", e)))
    }

    fn load_token(&self) -> Result<Option<AuthToken>, AuthError> {
        let entry = self.get_entry()?;

        match entry.get_password() {
            Ok(token_json) => {
                let token: AuthToken = serde_json::from_str(&token_json)
                    .map_err(|e| AuthError::JsonError(e))?;

                if token.is_expired() {
                    return Err(AuthError::TokenExpired);
                }

                Ok(Some(token))
            }
            Err(_) => Ok(None),
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

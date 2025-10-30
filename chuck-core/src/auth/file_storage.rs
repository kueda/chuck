use super::{AuthToken, TokenStorage, AuthError, save_auth_token, load_auth_token, clear_auth_token};

/// File-based token storage for CLI.
/// Stores OAuth tokens in ~/.config/crinat/auth.json
pub struct FileStorage;

impl FileStorage {
    pub fn new() -> Result<Self, AuthError> {
        Ok(Self)
    }
}

impl TokenStorage for FileStorage {
    fn save_token(&self, token: &AuthToken) -> Result<(), AuthError> {
        save_auth_token(token)
    }

    fn load_token(&self) -> Result<Option<AuthToken>, AuthError> {
        match load_auth_token() {
            Ok(token) => Ok(Some(token)),
            Err(AuthError::TokenNotFound) => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn clear_token(&self) -> Result<(), AuthError> {
        clear_auth_token()
    }
}

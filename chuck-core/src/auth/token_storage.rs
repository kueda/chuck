use super::{AuthToken, AuthError};

/// Trait for pluggable OAuth token storage.
/// JWT is NOT stored - it's fetched on-demand via fetch_jwt().
pub trait TokenStorage: Send + Sync {
    fn save_token(&self, token: &AuthToken) -> Result<(), AuthError>;
    fn load_token(&self) -> Result<Option<AuthToken>, AuthError>;
    fn clear_token(&self) -> Result<(), AuthError>;
}

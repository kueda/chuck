//! Saved token management
//!
//! Methods for saving and loading an OAuth access token locally.

use chrono::{DateTime, Utc};
use dirs::config_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::auth::AuthError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub token_type: String,
}

impl AuthToken {
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() >= expires_at
        } else {
            false
        }
    }
}

fn get_auth_config_path() -> Result<PathBuf, AuthError> {
    let config_dir = config_dir().ok_or_else(|| {
        AuthError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not find config directory",
        ))
    })?;

    let chuck_dir = config_dir.join("chuck");
    if !chuck_dir.exists() {
        fs::create_dir_all(&chuck_dir)?;
    }

    Ok(chuck_dir.join("auth.json"))
}

pub fn load_auth_token() -> Result<AuthToken, AuthError> {
    let auth_path = get_auth_config_path()?;

    if !auth_path.exists() {
        return Err(AuthError::TokenNotFound);
    }

    let contents = fs::read_to_string(auth_path)?;
    let token: AuthToken = serde_json::from_str(&contents)?;

    if token.is_expired() {
        return Err(AuthError::TokenExpired);
    }

    Ok(token)
}

pub fn save_auth_token(token: &AuthToken) -> Result<(), AuthError> {
    let auth_path = get_auth_config_path()?;
    let contents = serde_json::to_string_pretty(token)?;

    fs::write(auth_path, contents)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let auth_path = get_auth_config_path()?;
        let mut perms = fs::metadata(&auth_path)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(auth_path, perms)?;
    }

    Ok(())
}

pub fn clear_auth_token() -> Result<(), AuthError> {
    let auth_path = get_auth_config_path()?;
    if auth_path.exists() {
        fs::remove_file(auth_path)?;
    }
    Ok(())
}

use std::fmt;

#[derive(Debug)]
pub enum AuthError {
    TokenNotFound,
    TokenExpired,
    OAuthFailed(String),
    IoError(std::io::Error),
    JsonError(serde_json::Error),
    HttpError(reqwest::Error),
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthError::TokenNotFound => write!(f, "No authentication token found"),
            AuthError::TokenExpired => write!(f, "Authentication token has expired"),
            AuthError::OAuthFailed(msg) => write!(f, "OAuth authentication failed: {}", msg),
            AuthError::IoError(e) => write!(f, "I/O error: {}", e),
            AuthError::JsonError(e) => write!(f, "JSON error: {}", e),
            AuthError::HttpError(e) => write!(f, "HTTP error: {}", e),
        }
    }
}

impl std::error::Error for AuthError {}

impl From<std::io::Error> for AuthError {
    fn from(error: std::io::Error) -> Self {
        AuthError::IoError(error)
    }
}

impl From<serde_json::Error> for AuthError {
    fn from(error: serde_json::Error) -> Self {
        AuthError::JsonError(error)
    }
}

impl From<reqwest::Error> for AuthError {
    fn from(error: reqwest::Error) -> Self {
        AuthError::HttpError(error)
    }
}

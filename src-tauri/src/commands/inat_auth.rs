use tauri::{command, State};
use serde::{Serialize, Deserialize};

use chuck_core::auth::{authenticate_user, fetch_jwt, AuthCache};

#[derive(Serialize, Deserialize, Clone)]
pub struct AuthStatus {
    pub authenticated: bool,
    pub username: Option<String>,
}

#[derive(serde::Deserialize)]
struct UserResponse {
    results: Vec<UserResult>,
}

#[derive(serde::Deserialize)]
struct UserResult {
    login: String,
}

/// Internal helper to get auth status from cache
async fn get_auth_status_from_cache(cache: &AuthCache) -> Result<AuthStatus, String> {
    match cache.load_token() {
        Ok(Some(token)) => {
            // Fetch JWT to get username
            let jwt = fetch_jwt(&token).await
                .map_err(|e| format!("Failed to fetch JWT: {e}"))?;

            let username = fetch_username_from_api(&jwt)
                .await
                .map_err(|e| format!("Failed to fetch username: {e}"))?;

            Ok(AuthStatus {
                authenticated: true,
                username: Some(username),
            })
        }
        Ok(None) => Ok(AuthStatus {
            authenticated: false,
            username: None,
        }),
        Err(e) => Err(format!("Failed to load token: {e}")),
    }
}

/// Initiates OAuth authentication flow and stores the token
#[command]
pub async fn inat_authenticate(
    cache: State<'_, AuthCache>
) -> Result<AuthStatus, String> {
    // Authenticate and save token
    authenticate_user(cache.inner()).await
        .map_err(|e| format!("Authentication failed: {e}"))?;

    // Return auth status
    get_auth_status_from_cache(cache.inner()).await
}

/// Returns the current authentication status
#[command]
pub async fn inat_get_auth_status(
    cache: State<'_, AuthCache>
) -> Result<AuthStatus, String> {
    get_auth_status_from_cache(cache.inner()).await
}

/// Signs out by clearing the stored token
#[command]
pub async fn inat_sign_out(
    cache: State<'_, AuthCache>
) -> Result<(), String> {
    cache.clear_token()
        .map_err(|e| format!("Failed to clear token: {e}"))
}

/// Fetches a JWT for authenticated API requests
/// Returns None if not authenticated
#[command]
pub async fn inat_get_jwt(
    cache: State<'_, AuthCache>
) -> Result<Option<String>, String> {
    match cache.load_token() {
        Ok(Some(oauth_token)) => {
            let jwt = fetch_jwt(&oauth_token).await
                .map_err(|e| format!("Failed to fetch JWT: {e}"))?;
            Ok(Some(jwt))
        }
        Ok(None) => Ok(None),
        Err(e) => Err(format!("Failed to load token: {e}")),
    }
}

/// Helper function to fetch username from iNaturalist API using user_id from JWT
/// TODO: Use the inaturalist crate's users_id_get when it correctly returns the response body
async fn fetch_username_from_api(jwt: &str) -> Result<String, String> {
    // Decode JWT to get user_id
    let parts: Vec<&str> = jwt.split('.').collect();
    if parts.len() != 3 {
        return Err("Invalid JWT format".into());
    }

    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    let payload_bytes = URL_SAFE_NO_PAD
        .decode(parts[1])
        .map_err(|e| format!("Base64 decode error: {e}"))?;

    #[derive(serde::Deserialize)]
    struct JwtClaims {
        user_id: i32,
    }

    let claims: JwtClaims = serde_json::from_slice(&payload_bytes)
        .map_err(|e| format!("JSON parse error: {e}"))?;

    // Fetch user info from public API
    let url = format!("https://api.inaturalist.org/v1/users/{}", claims.user_id);
    let client = reqwest::Client::new();
    let response = client.get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch user info: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("API returned status: {}", response.status()));
    }

    let user_response: UserResponse = response.json()
        .await
        .map_err(|e| format!("Failed to parse user response: {e}"))?;

    user_response.results
        .first()
        .map(|u| u.login.clone())
        .ok_or_else(|| "No user found in response".to_string())
}

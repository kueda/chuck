use super::{AuthError, AuthToken};

pub async fn fetch_jwt(oauth_token: &AuthToken) -> Result<String, AuthError> {
    let client = reqwest::Client::new();

    let response = client
        .get("https://www.inaturalist.org/users/api_token")
        .bearer_auth(&oauth_token.access_token)
        .send()
        .await
        .map_err(|e| AuthError::OAuthFailed(format!("Failed to fetch JWT: {e}")))?;

    if !response.status().is_success() {
        return Err(AuthError::OAuthFailed(format!(
            "JWT fetch failed with status: {}",
            response.status()
        )));
    }

    let jwt_response: serde_json::Value = response
        .json()
        .await
        .map_err(|e| AuthError::OAuthFailed(format!("Failed to parse JWT response: {e}")))?;

    let jwt_string = jwt_response
        .get("api_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AuthError::OAuthFailed("No api_token in response".to_string()))?;

    Ok(jwt_string.to_string())
}

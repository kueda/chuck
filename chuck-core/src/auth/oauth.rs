use chrono::{Duration, Utc};
use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, CsrfToken, PkceCodeChallenge,
    RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use std::collections::HashMap;
use std::io::prelude::*;
use std::net::TcpListener;
use url::Url;

use crate::auth::{save_auth_token, AuthError, AuthToken};

const INATURALIST_AUTH_URL: &str = "https://www.inaturalist.org/oauth/authorize";
const INATURALIST_TOKEN_URL: &str = "https://www.inaturalist.org/oauth/token";
const REDIRECT_URI: &str = "http://localhost:8080/callback";

pub async fn authenticate_user() -> Result<AuthToken, AuthError> {
    println!("Starting iNaturalist authentication...");

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let client_id = get_client_id()?;
    let client = BasicClient::new(
        ClientId::new(client_id),
        None,
        AuthUrl::new(INATURALIST_AUTH_URL.to_string())
            .map_err(|e| AuthError::OAuthFailed(format!("Invalid auth URL: {}", e)))?,
        Some(
            TokenUrl::new(INATURALIST_TOKEN_URL.to_string())
                .map_err(|e| AuthError::OAuthFailed(format!("Invalid token URL: {}", e)))?,
        ),
    )
    .set_redirect_uri(
        RedirectUrl::new(REDIRECT_URI.to_string())
            .map_err(|e| AuthError::OAuthFailed(format!("Invalid redirect URL: {}", e)))?,
    );

    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("write".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    println!("Opening browser for authentication...");
    println!("If the browser doesn't open automatically, please visit: {}", auth_url);

    if let Err(e) = open::that(auth_url.to_string()) {
        eprintln!("Failed to open browser: {}. Please visit the URL manually.", e);
    }

    let authorization_code = wait_for_callback(csrf_token).await?;

    let token_result = client
        .exchange_code(authorization_code)
        .set_pkce_verifier(pkce_verifier)
        .request_async(oauth2::reqwest::async_http_client)
        .await
        .map_err(|e| AuthError::OAuthFailed(format!("Token exchange failed: {}", e)))?;

    let expires_at = token_result
        .expires_in()
        .map(|duration| Utc::now() + Duration::seconds(duration.as_secs() as i64));

    let auth_token = AuthToken {
        access_token: token_result.access_token().secret().clone(),
        refresh_token: token_result
            .refresh_token()
            .map(|token| token.secret().clone()),
        expires_at,
        token_type: "Bearer".to_string(),
    };

    save_auth_token(&auth_token)?;
    println!("Authentication successful! Token saved.");

    Ok(auth_token)
}

fn get_client_id() -> Result<String, AuthError> {
    const CLIENT_ID: &str = "iF85tDHdCdGXR-mfk2ILjLezSr6OaO-ZZINK_dG-RcQ";
    Ok(CLIENT_ID.to_string())
}

async fn wait_for_callback(expected_csrf: CsrfToken) -> Result<AuthorizationCode, AuthError> {
    let listener = TcpListener::bind("127.0.0.1:8080").map_err(|e| {
        AuthError::OAuthFailed(format!("Failed to bind to localhost:8080: {}", e))
    })?;

    println!("Waiting for authentication callback...");

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let mut buffer = [0; 1024];
                if let Ok(size) = stream.read(&mut buffer) {
                    let request = String::from_utf8_lossy(&buffer[..size]);

                    if let Some(line) = request.lines().next() {
                        if line.starts_with("GET /callback") {
                            let response = handle_callback_request(line, expected_csrf.clone())?;

                            let _ = stream.write_all(
                                b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n"
                            );
                            let _ = stream.write_all(
                                b"<html><body><h1>Authentication successful!</h1><p>You can close this window and return to the terminal.</p></body></html>"
                            );
                            let _ = stream.flush();

                            return Ok(response);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Connection failed: {}", e);
            }
        }
    }

    Err(AuthError::OAuthFailed("No valid callback received".to_string()))
}

fn handle_callback_request(
    request_line: &str,
    expected_csrf: CsrfToken,
) -> Result<AuthorizationCode, AuthError> {
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(AuthError::OAuthFailed("Invalid request format".to_string()));
    }

    let url_str = format!("http://localhost:8080{}", parts[1]);
    let url = Url::parse(&url_str)
        .map_err(|e| AuthError::OAuthFailed(format!("Failed to parse callback URL: {}", e)))?;

    let query_params: HashMap<String, String> = url.query_pairs().into_owned().collect();

    if let Some(error) = query_params.get("error") {
        return Err(AuthError::OAuthFailed(format!("OAuth error: {}", error)));
    }

    let state = query_params
        .get("state")
        .ok_or_else(|| AuthError::OAuthFailed("Missing state parameter".to_string()))?;

    if state != expected_csrf.secret() {
        return Err(AuthError::OAuthFailed("CSRF token mismatch".to_string()));
    }

    let code = query_params
        .get("code")
        .ok_or_else(|| AuthError::OAuthFailed("Missing authorization code".to_string()))?;

    Ok(AuthorizationCode::new(code.clone()))
}

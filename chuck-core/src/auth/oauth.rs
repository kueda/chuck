use chrono::{Duration, Utc};
use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, CsrfToken, PkceCodeChallenge,
    RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use std::collections::HashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use url::Url;

use crate::auth::{TokenStorage, AuthError, AuthToken};

const INATURALIST_AUTH_URL: &str = "https://www.inaturalist.org/oauth/authorize";
const INATURALIST_TOKEN_URL: &str = "https://www.inaturalist.org/oauth/token";
const REDIRECT_URI: &str = "http://localhost:8080/callback";

pub async fn authenticate_user<S: TokenStorage>(storage: &S) -> Result<AuthToken, AuthError> {
    log::info!("Starting iNaturalist authentication...");

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let client_id = get_client_id()?;
    let client = BasicClient::new(
        ClientId::new(client_id),
        None,
        AuthUrl::new(INATURALIST_AUTH_URL.to_string())
            .map_err(|e| AuthError::OAuthFailed(format!("Invalid auth URL: {e}")))?,
        Some(
            TokenUrl::new(INATURALIST_TOKEN_URL.to_string())
                .map_err(|e| AuthError::OAuthFailed(format!("Invalid token URL: {e}")))?,
        ),
    )
    .set_redirect_uri(
        RedirectUrl::new(REDIRECT_URI.to_string())
            .map_err(|e| AuthError::OAuthFailed(format!("Invalid redirect URL: {e}")))?,
    );

    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("write".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    log::info!("Opening browser for authentication...");
    log::info!("If the browser doesn't open automatically, please visit: {auth_url}");

    if let Err(e) = open::that(auth_url.to_string()) {
        log::warn!("Failed to open browser: {e}. Please visit the URL manually.");
    }

    let authorization_code = wait_for_callback(csrf_token).await?;
    log::info!("CSRF token validated, exchanging authorization code for token...");

    let token_result = client
        .exchange_code(authorization_code)
        .set_pkce_verifier(pkce_verifier)
        .request_async(|req| async move {
            let resp = oauth2::reqwest::async_http_client(req).await;
            match &resp {
                Ok(r) => log::info!(
                    "Token exchange response: status={}, body={}",
                    r.status_code,
                    String::from_utf8_lossy(&r.body)
                ),
                Err(e) => log::error!("Token exchange HTTP error: {e}"),
            }
            resp
        })
        .await
        .map_err(|e| AuthError::OAuthFailed(format!("Token exchange failed: {e}")))?;

    log::info!("Token exchange successful");

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

    storage.save_token(&auth_token)?;
    log::info!("Authentication successful! Token saved.");

    Ok(auth_token)
}

fn get_client_id() -> Result<String, AuthError> {
    const CLIENT_ID: &str = "iF85tDHdCdGXR-mfk2ILjLezSr6OaO-ZZINK_dG-RcQ";
    Ok(CLIENT_ID.to_string())
}

async fn wait_for_callback(expected_csrf: CsrfToken) -> Result<AuthorizationCode, AuthError> {
    let listener = TcpListener::bind("127.0.0.1:8080")
        .await
        .map_err(|e| AuthError::OAuthFailed(format!("Failed to bind to localhost:8080: {e}")))?;

    log::info!("Waiting for authentication callback...");

    tokio::time::timeout(
        std::time::Duration::from_secs(300),
        wait_for_callback_inner(&listener, expected_csrf),
    )
    .await
    .map_err(|_| AuthError::OAuthFailed("Authentication timed out after 5 minutes".to_string()))?
}

async fn wait_for_callback_inner(
    listener: &TcpListener,
    expected_csrf: CsrfToken,
) -> Result<AuthorizationCode, AuthError> {
    let mut auth_code: Option<AuthorizationCode> = None;

    loop {
        // Once we have the code, wait up to 1 second for additional asset requests
        // (e.g. the success page icon), then return.
        let accept_result = if auth_code.is_some() {
            match tokio::time::timeout(
                std::time::Duration::from_secs(1),
                listener.accept(),
            )
            .await
            {
                Ok(r) => r,
                Err(_) => {
                    return auth_code.ok_or_else(|| {
                        AuthError::OAuthFailed("No valid callback received".to_string())
                    });
                }
            }
        } else {
            listener.accept().await
        };

        match accept_result {
            Ok((mut stream, _)) => {
                let mut buffer = [0u8; 1024];
                match stream.read(&mut buffer).await {
                    Ok(size) if size > 0 => {
                        let request = String::from_utf8_lossy(&buffer[..size]);
                        if let Some(line) = request.lines().next() {
                            if line.starts_with("GET /chuck_icon.png") {
                                const ICON: &[u8] =
                                    include_bytes!("../../../src-tauri/icons/icon.png");
                                let _ = stream
                                    .write_all(
                                        b"HTTP/1.1 200 OK\r\nContent-Type: image/png\r\n\r\n",
                                    )
                                    .await;
                                let _ = stream.write_all(ICON).await;
                            } else if line.starts_with("GET /callback") && auth_code.is_none() {
                                auth_code = Some(handle_callback_request(
                                    line,
                                    expected_csrf.clone(),
                                )?);
                                log::debug!("Callback received, serving success page");
                                const SUCCESS_PAGE: &str = include_str!("oauth_success.html");
                                let _ = stream
                                    .write_all(
                                        b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n",
                                    )
                                    .await;
                                let _ = stream.write_all(SUCCESS_PAGE.as_bytes()).await;
                            }
                        }
                    }
                    Ok(_) => {}
                    Err(e) => {
                        log::warn!("Connection read failed: {e}");
                    }
                }
            }
            Err(e) => {
                log::warn!("Connection accept failed: {e}");
            }
        }
    }
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
        .map_err(|e| AuthError::OAuthFailed(format!("Failed to parse callback URL: {e}")))?;

    let query_params: HashMap<String, String> = url.query_pairs().into_owned().collect();

    if let Some(error) = query_params.get("error") {
        return Err(AuthError::OAuthFailed(format!("OAuth error: {error}")));
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

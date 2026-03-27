use inaturalist::apis::{configuration::{Configuration, ApiKey}, observations_api, Error};
use inaturalist::models::ObservationsResponse;
use tokio::sync::{OnceCell, RwLock};

use crate::auth::{fetch_jwt, TokenStorage};

// OnceCell ensures the config is initialized exactly once across the entire application,
// avoiding redundant API calls and JWT fetching. RwLock provides interior mutability
// so the shared config can be updated (e.g., JWT token refresh) while maintaining
// thread safety for concurrent read/write access.
static CONFIG: OnceCell<RwLock<Configuration>> = OnceCell::const_new();

/// Get or create the shared API configuration
/// Returns a RwLock-wrapped Configuration that can be safely shared and mutated
pub async fn get_config() -> &'static RwLock<Configuration> {
    CONFIG.get_or_init(|| async { RwLock::new(create_config().await) }).await
}

/// Create API configuration for iNaturalist without authentication
///
/// NOTE: This is used by both CLI and GUI. For GUI apps using Tauri,
/// authentication should be handled separately via AuthCache to avoid
/// duplicate keychain access prompts. The CLI will use StorageFactory
/// for auth during command execution.
async fn create_config() -> Configuration {
    Configuration {
        base_path: "https://api.inaturalist.org/v1".to_string(),
        ..Configuration::default()
    }
}

/// Create API configuration with optional JWT
/// Used by Tauri to pass JWT from StrongholdStorage
pub fn create_config_with_jwt(jwt: Option<String>) -> Configuration {
    let mut config = Configuration {
        base_path: "https://api.inaturalist.org/v1".to_string(),
        ..Configuration::default()
    };

    if let Some(jwt_token) = jwt {
        config.api_key = Some(ApiKey {
            prefix: None,
            key: jwt_token,
        });
    }

    config
}

/// Create API configuration with custom base URL and optional JWT
/// Used for testing with mock servers
pub fn create_config_with_base_url_and_jwt(
    base_url: String,
    jwt: Option<String>,
) -> Configuration {
    let mut config = Configuration {
        base_path: base_url,
        ..Configuration::default()
    };

    if let Some(jwt_token) = jwt {
        config.api_key = Some(ApiKey {
            prefix: None,
            key: jwt_token,
        });
    }

    config
}

/// Refresh JWT token in the provided configuration
pub async fn refresh_jwt_in_config(config: &RwLock<Configuration>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let storage = crate::auth::StorageFactory::create()?;
    let oauth_token = storage.load_token()?
        .ok_or_else(|| crate::auth::AuthError::TokenNotFound)?;
    let jwt = fetch_jwt(&oauth_token).await?;

    let mut config_guard = config.write().await;
    config_guard.api_key = Some(ApiKey {
        prefix: None,
        key: jwt,
    });

    println!("JWT token refreshed");
    Ok(())
}

/// Fetch observations with automatic retry on network errors and 401 auth refresh.
///
/// Retries up to 3 times with exponential backoff on connection-level errors
/// (e.g., connection reset after sleep/resume or a transient network blip).
/// On 401, refreshes the JWT token and retries once.
pub async fn fetch_observations_with_retry(
    config: &RwLock<Configuration>,
    params: observations_api::ObservationsGetParams,
) -> Result<ObservationsResponse, Error<observations_api::ObservationsGetError>> {
    const MAX_RETRIES: u32 = 3;
    const RETRY_BASE_DELAY: std::time::Duration = std::time::Duration::from_secs(2);

    let mut attempt = 0;
    loop {
        attempt += 1;

        let config_read = config.read().await;
        let result = observations_api::observations_get(&config_read, params.clone()).await;
        drop(config_read);

        match result {
            Ok(response) => return Ok(response),
            Err(Error::ResponseError(ref response)) if response.status.as_u16() == 401 => {
                eprintln!("Got 401 Unauthorized - attempting to refresh JWT token");
                match refresh_jwt_in_config(config).await {
                    Ok(_) => {
                        eprintln!("Retrying request with refreshed token");
                        let config_read = config.read().await;
                        return observations_api::observations_get(&config_read, params).await;
                    }
                    Err(e) => {
                        eprintln!("Failed to refresh JWT token: {e}");
                        eprintln!("Run `chuck auth` to re-authenticate");
                        return Err(Error::ResponseError(response.clone()));
                    }
                }
            }
            Err(Error::Reqwest(ref e)) if attempt < MAX_RETRIES => {
                let delay = RETRY_BASE_DELAY * 2_u32.pow(attempt - 1);
                log::warn!(
                    "Observation fetch attempt {attempt}/{MAX_RETRIES} failed ({}), \
                    retrying in {delay:?}...",
                    describe_reqwest_error(e)
                );
                tokio::time::sleep(delay).await;
            }
            Err(ref e) => {
                log_observation_fetch_error(e);
                return Err(result.unwrap_err());
            }
        }
    }
}

fn describe_reqwest_error(e: &reqwest::Error) -> String {
    if e.is_timeout() {
        format!("request timed out: {e}")
    } else if e.is_connect() {
        format!("connection error: {e}")
    } else {
        format!("{e}")
    }
}

fn log_observation_fetch_error(e: &Error<observations_api::ObservationsGetError>) {
    match e {
        Error::ResponseError(r) => {
            let body = r.content.trim();
            if body.is_empty() {
                log::error!("Observation fetch failed: HTTP {}", r.status);
            } else {
                log::error!("Observation fetch failed: HTTP {} — {body}", r.status);
            }
        }
        Error::Reqwest(re) => {
            log::error!(
                "Observation fetch failed after all retries: {}",
                describe_reqwest_error(re)
            );
        }
        _ => {
            log::error!("Observation fetch failed: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::io::AsyncWriteExt;
    use tokio::net::TcpListener;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn test_fetch_observations_retries_on_connection_error() {
        let call_count = Arc::new(AtomicUsize::new(0));
        let call_count_clone = call_count.clone();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        tokio::spawn(async move {
            loop {
                let (mut stream, _) = listener.accept().await.unwrap();
                let n = call_count_clone.fetch_add(1, Ordering::SeqCst) + 1;
                if n == 1 {
                    // First connection: drop immediately (connection reset)
                    drop(stream);
                } else {
                    // Second connection: return valid empty response
                    let body = r#"{"total_results":0,"results":[]}"#;
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
                        Content-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = stream.write_all(response.as_bytes()).await;
                    break;
                }
            }
        });

        let config = create_config_with_base_url_and_jwt(
            format!("http://127.0.0.1:{port}"),
            None,
        );
        let config_lock = RwLock::new(config);
        let params = inaturalist::apis::observations_api::ObservationsGetParams {
            ..crate::api::params::DEFAULT_GET_PARAMS.clone()
        };

        let result = fetch_observations_with_retry(&config_lock, params).await;
        assert!(result.is_ok(), "should succeed after retry, got: {result:?}");
        assert_eq!(call_count.load(Ordering::SeqCst), 2, "should have made 2 calls");
    }
}

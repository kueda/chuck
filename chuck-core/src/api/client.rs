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

/// Create API configuration for iNaturalist with authentication if available
async fn create_config() -> Configuration {
    let mut config = Configuration {
        base_path: "https://api.inaturalist.org/v1".to_string(),
        ..Configuration::default()
    };

    match crate::auth::StorageFactory::create() {
        Ok(storage) => {
            match storage.load_token() {
                Ok(Some(oauth_token)) => {
                    match fetch_jwt(&oauth_token).await {
                        Ok(jwt) => {
                            config.api_key = Some(ApiKey {
                                prefix: None,
                                key: jwt,
                            });
                            eprintln!("Authenticated");
                        }
                        Err(_) => {
                            eprintln!("Not authenticated (failed to fetch JWT)");
                        }
                    }
                }
                Ok(None) => {
                    eprintln!("Not authenticated (run `chuck auth`)");
                }
                Err(e) => {
                    eprintln!("Token load error: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("Storage error: {}", e);
            eprintln!("Run `chuck auth` to configure authentication");
        }
    }

    config
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

/// Fetch observations with automatic 401 retry
pub async fn fetch_observations_with_retry(
    config: &RwLock<Configuration>,
    params: observations_api::ObservationsGetParams,
) -> Result<ObservationsResponse, Error<observations_api::ObservationsGetError>> {
    // First attempt with read lock
    let config_read = config.read().await;
    let first_result = observations_api::observations_get(&*config_read, params.clone()).await;
    drop(config_read); // Release read lock early

    match first_result {
        Ok(response) => Ok(response),
        Err(Error::ResponseError(ref response)) if response.status.as_u16() == 401 => {
            eprintln!("Got 401 Unauthorized - attempting to refresh JWT token");

            match refresh_jwt_in_config(config).await {
                Ok(_) => {
                    eprintln!("Retrying request with refreshed token");
                    let config_read = config.read().await;
                    observations_api::observations_get(&*config_read, params).await
                }
                Err(e) => {
                    eprintln!("Failed to refresh JWT token: {}", e);
                    eprintln!("Run `chuck auth` to re-authenticate");
                    Err(Error::ResponseError(response.clone()))
                }
            }
        }
        Err(e) => Err(e),
    }
}

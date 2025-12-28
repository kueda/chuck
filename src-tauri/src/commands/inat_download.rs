use chuck_core::api::{client, params};
use chuck_core::auth::{fetch_jwt, AuthCache};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};

#[derive(Debug, Deserialize)]
pub struct CountParams {
    taxon_id: Option<i32>,
    place_id: Option<i32>,
    user: Option<String>,
    d1: Option<String>,
    d2: Option<String>,
    created_d1: Option<String>,
    created_d2: Option<String>,
}

#[tauri::command]
pub async fn get_observation_count(params: CountParams) -> Result<i32, String> {
    // Build API params
    let api_params = params::build_params(
        params.taxon_id.map(|id| id.to_string()),
        params.place_id,
        params.user,
        params.d1,
        params.d2,
        params.created_d1,
        params.created_d2,
    );

    // Get API config
    let config = client::get_config().await;
    let config_guard = config.read().await;

    // Fetch with per_page=0 to just get count
    let mut count_params = api_params;
    count_params.per_page = Some("0".to_string());

    // Call iNaturalist API
    match inaturalist::apis::observations_api::observations_get(&*config_guard, count_params).await {
        Ok(response) => Ok(response.total_results.unwrap_or(0)),
        Err(e) => {
            log::error!("Failed to get observation count: {:?}", e);
            Err(format!("Failed to get observation count: {}", e))
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "stage", rename_all = "camelCase")]
pub enum InatProgress {
    Fetching { current: usize, total: usize },
    DownloadingPhotos { current: usize, total: usize },
    Building { message: String },
    Complete,
    Error { message: String },
}

#[derive(Debug, Deserialize)]
pub struct GenerateParams {
    output_path: String,
    taxon_id: Option<i32>,
    place_id: Option<i32>,
    user: Option<String>,
    d1: Option<String>,
    d2: Option<String>,
    created_d1: Option<String>,
    created_d2: Option<String>,
    fetch_photos: bool,
    extensions: Vec<String>,
}

// Global cancellation flag
static CANCEL_FLAG: LazyLock<Arc<AtomicBool>> = LazyLock::new(|| Arc::new(AtomicBool::new(false)));

#[tauri::command]
pub async fn generate_inat_archive(
    app: AppHandle,
    params: GenerateParams,
    cache: State<'_, AuthCache>,
) -> Result<(), String> {
    use chuck_core::downloader::Downloader;

    // Reset cancellation flag
    CANCEL_FLAG.as_ref().store(false, Ordering::Relaxed);

    // Parse extensions
    let mut extensions = Vec::new();
    for ext in &params.extensions {
        match ext.as_str() {
            "SimpleMultimedia" => extensions.push(chuck_core::DwcaExtension::SimpleMultimedia),
            "Audiovisual" => extensions.push(chuck_core::DwcaExtension::Audiovisual),
            "Identifications" => extensions.push(chuck_core::DwcaExtension::Identifications),
            _ => {
                log::warn!("Unknown extension: {}", ext);
            }
        }
    }

    // Build API params
    let api_params = params::build_params(
        params.taxon_id.map(|id| id.to_string()),
        params.place_id,
        params.user.clone(),
        params.d1.clone(),
        params.d2.clone(),
        params.created_d1.clone(),
        params.created_d2.clone(),
    );

    // Emit building message
    app.emit("inat-progress", InatProgress::Building {
        message: "Initializing archive...".to_string()
    }).map_err(|e| e.to_string())?;

    // Fetch JWT if authenticated
    let jwt = match cache.load_token() {
        Ok(Some(oauth_token)) => fetch_jwt(&oauth_token).await.ok(),
        _ => None
    };

    // Create downloader with JWT for authenticated requests
    let downloader = Downloader::new(api_params, extensions, params.fetch_photos, jwt);

    // Create progress callback
    let app_clone = app.clone();
    let progress_callback = move |progress: chuck_core::downloader::DownloadProgress| {
        use chuck_core::downloader::DownloadStage;

        let event = match progress.stage {
            DownloadStage::Fetching => InatProgress::Fetching {
                current: progress.observations_current,
                total: progress.observations_total,
            },
            DownloadStage::DownloadingPhotos => InatProgress::DownloadingPhotos {
                current: progress.photos_current,
                total: progress.photos_total,
            },
            DownloadStage::Building => InatProgress::Building {
                message: "Finalizing archive...".to_string()
            },
        };

        let _ = app_clone.emit("inat-progress", event);
    };

    // Execute download
    let cancel_token = Arc::clone(&CANCEL_FLAG);
    downloader
        .execute(&params.output_path, progress_callback, Some(cancel_token))
        .await
        .map_err(|e| e.to_string())?;

    // Emit completion
    app.emit("inat-progress", InatProgress::Complete)
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn cancel_inat_archive() -> Result<(), String> {
    CANCEL_FLAG.as_ref().store(true, Ordering::Relaxed);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    #[test]
    fn test_convert_observations_to_multimedia_when_simple_multimedia_enabled() {
        use chuck_core::downloader::convert_to_multimedia;

        // Create observations with photos
        let observations = vec![
            inaturalist::models::Observation {
                id: Some(123),
                photos: Some(vec![
                    inaturalist::models::Photo {
                        id: Some(456),
                        url: Some("https://example.com/photo.jpg".to_string()),
                        ..Default::default()
                    }
                ]),
                user: Some(Box::new(inaturalist::models::User {
                    login: Some("testuser".to_string()),
                    ..Default::default()
                })),
                ..Default::default()
            }
        ];

        let photo_mapping = HashMap::new();

        // This function should exist and convert observations to multimedia records
        let multimedia = convert_to_multimedia(&observations, &photo_mapping);

        // Should have created multimedia records
        assert_eq!(multimedia.len(), 1);
        assert_eq!(multimedia[0].occurrence_id, "https://www.inaturalist.org/observations/123");
    }

    #[test]
    fn test_convert_observations_to_multimedia_without_photos() {
        use chuck_core::downloader::convert_to_multimedia;

        let observations = vec![
            inaturalist::models::Observation {
                id: Some(123),
                photos: None,
                ..Default::default()
            }
        ];

        let photo_mapping = HashMap::new();

        let multimedia = convert_to_multimedia(&observations, &photo_mapping);

        // Should be empty when there are no photos
        assert_eq!(multimedia.len(), 0);
    }
}

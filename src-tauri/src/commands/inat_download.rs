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
    url_params: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ParsedInatUrl {
    effective_params: String,
}

/// Extract query string from a URL string (everything after '?'),
/// or use the input as-is if it contains no '?'.
fn extract_query(url: &str) -> &str {
    url.find('?').map(|i| &url[i + 1..]).unwrap_or(url)
}

/// Build ObservationsGetParams from either url_params or individual fields.
fn build_api_params_from_count(
    p: &CountParams,
) -> inaturalist::apis::observations_api::ObservationsGetParams {
    if let Some(ref url_params) = p.url_params {
        params::parse_url_params(extract_query(url_params))
    } else {
        params::build_params(
            p.taxon_id.map(|id| id.to_string()),
            p.place_id,
            p.user.clone(),
            p.d1.clone(),
            p.d2.clone(),
            p.created_d1.clone(),
            p.created_d2.clone(),
        )
    }
}

/// Build ObservationsGetParams from either url_params or individual fields.
fn build_api_params_from_generate(
    p: &GenerateParams,
) -> inaturalist::apis::observations_api::ObservationsGetParams {
    if let Some(ref url_params) = p.url_params {
        params::parse_url_params(extract_query(url_params))
    } else {
        params::build_params(
            p.taxon_id.map(|id| id.to_string()),
            p.place_id,
            p.user.clone(),
            p.d1.clone(),
            p.d2.clone(),
            p.created_d1.clone(),
            p.created_d2.clone(),
        )
    }
}

#[tauri::command]
pub async fn get_observation_count(params: CountParams) -> Result<i32, String> {
    // Build API params
    let api_params = build_api_params_from_count(&params);

    // Get API config
    let config = client::get_config().await;
    let config_guard = config.read().await;

    // Fetch with per_page=0 to just get count
    let mut count_params = api_params;
    count_params.per_page = Some("0".to_string());

    // Call iNaturalist API
    match inaturalist::apis::observations_api::observations_get(&config_guard, count_params).await {
        Ok(response) => Ok(response.total_results.unwrap_or(0)),
        Err(e) => {
            log::error!("Failed to get observation count: {e:?}");
            Err(format!("Failed to get observation count: {e}"))
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MediaEstimate {
    photo_count: usize,
    sound_count: usize,
    sample_size: usize,
}

#[tauri::command]
pub async fn estimate_media_count(params: CountParams) -> Result<MediaEstimate, String> {
    // Build API params
    let api_params = build_api_params_from_count(&params);

    // Get API config
    let config = client::get_config().await;
    let config_guard = config.read().await;

    // Fetch one page of random observations to sample media density
    let mut sample_params = api_params;
    sample_params.per_page = Some("200".to_string());
    sample_params.order_by = Some("random".to_string());

    // Call iNaturalist API
    match inaturalist::apis::observations_api::observations_get(&config_guard, sample_params).await {
        Ok(response) => {
            let sample_size = response.results.len();
            let photo_count = response
                .results
                .iter()
                .filter_map(|o| o.photos.as_ref())
                .flatten()
                .count();
            let sound_count = response
                .results
                .iter()
                .filter_map(|o| o.sounds.as_ref())
                .flatten()
                .filter(|s| s.file_url.is_some() && !s.hidden.unwrap_or(false))
                .count();
            log::info!(
                "Media estimate sample: {photo_count} photos + {sound_count} sounds \
                / {sample_size} obs"
            );
            Ok(MediaEstimate {
                photo_count,
                sound_count,
                sample_size,
            })
        }
        Err(e) => {
            log::error!("Failed to estimate media count: {e:?}");
            Err(format!("Failed to estimate media count: {e}"))
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "stage", rename_all = "camelCase")]
pub enum InatProgress {
    Fetching { current: usize, total: usize },
    DownloadingMedia { current: usize, total: usize },
    Building { message: String },
    Merging { current: usize, total: usize },
    Complete,
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
    fetch_media: bool,
    extensions: Vec<String>,
    url_params: Option<String>,
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
            "Comments" => extensions.push(chuck_core::DwcaExtension::Comments),
            _ => {
                log::warn!("Unknown extension: {ext}");
            }
        }
    }

    // Build API params
    let api_params = build_api_params_from_generate(&params);

    log::info!(
        "generate_inat_archive: output={}, fetch_media={}, extensions={:?}",
        params.output_path, params.fetch_media, params.extensions
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
    let downloader = Downloader::new(api_params, extensions, params.fetch_media, jwt);

    // Create progress callback
    let app_clone = app.clone();
    let progress_callback = move |progress: chuck_core::downloader::DownloadProgress| {
        use chuck_core::downloader::DownloadStage;

        let event = match progress.stage {
            DownloadStage::Fetching => InatProgress::Fetching {
                current: progress.observations_current,
                total: progress.observations_total,
            },
            DownloadStage::DownloadingMedia => InatProgress::DownloadingMedia {
                current: progress.media_current,
                total: progress.media_total,
            },
            DownloadStage::Building => InatProgress::Building {
                message: "Finalizing archive...".to_string()
            },
            // Merging only occurs during updates, not initial creation
            DownloadStage::Merging { .. } => return,
        };

        let _ = app_clone.emit("inat-progress", event);
    };

    // Hold a system sleep inhibitor for the duration of the download.
    // Non-fatal: if the OS rejects it (e.g. non-systemd Linux), we just log and continue.
    let _awake = keepawake::Builder::default()
        .reason("Downloading iNaturalist archive".to_string())
        .app_name("Chuck".to_string())
        .app_reverse_domain("org.inaturalist.chuck".to_string())
        .idle(true)
        .sleep(true)
        .create()
        .inspect_err(|e| log::warn!("Could not acquire sleep inhibitor: {e}"))
        .ok();

    // Execute download
    let cancel_token = Arc::clone(&CANCEL_FLAG);
    let result = downloader
        .execute(&params.output_path, progress_callback, Some(cancel_token))
        .await;

    match &result {
        Ok(()) => log::info!("generate_inat_archive: complete"),
        Err(e) => log::error!("generate_inat_archive: failed: {e}"),
    }
    result.map_err(|e| e.to_string())?;

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

#[tauri::command]
pub async fn parse_inat_url(url: String) -> Result<ParsedInatUrl, String> {
    let api_params = params::parse_url_params(extract_query(&url));
    let effective_params = params::serialize_params(&api_params);
    Ok(ParsedInatUrl { effective_params })
}

#[derive(Debug, Serialize)]
pub struct ChuckArchiveInfo {
    inat_query: Option<String>,
    extensions: Vec<String>,
    has_media: bool,
    file_size_bytes: u64,
    pub_date: Option<String>,
}

#[tauri::command]
pub async fn read_chuck_archive_info(path: String) -> Result<ChuckArchiveInfo, String> {
    use chuck_core::archive_updater::read_archive_preview;

    let preview = read_archive_preview(&path).map_err(|e| e.to_string())?;
    let extensions: Vec<String> = preview.extensions.iter().map(|e| format!("{e:?}")).collect();
    let file_size_bytes = std::fs::metadata(&path).map_err(|e| e.to_string())?.len();
    Ok(ChuckArchiveInfo {
        inat_query: preview.inat_query,
        extensions,
        has_media: preview.has_media,
        file_size_bytes,
        pub_date: preview.pub_date,
    })
}

#[tauri::command]
pub async fn get_update_observation_count(path: String) -> Result<i32, String> {
    use chuck_core::api::{client, params};
    use chuck_core::archive_updater::updated_since_from_pub_date;
    use chuck_core::chuck_metadata::{read_chuck_metadata, read_pub_date};

    let chuck_meta = read_chuck_metadata(&path)
        .map_err(|e| e.to_string())?
        .ok_or("Not a Chuck archive")?;
    let inat_query = chuck_meta.inat_query
        .ok_or("No inat_query stored in archive")?;
    let pub_date = read_pub_date(&path)
        .map_err(|e| e.to_string())?
        .ok_or("No pubDate in archive")?;
    let updated_since = updated_since_from_pub_date(&pub_date).map_err(|e| e.to_string())?;

    let mut api_params = params::parse_url_params(&inat_query);
    api_params.updated_since = Some(updated_since);
    api_params.per_page = Some("0".to_string());

    let config = client::get_config().await;
    let config_guard = config.read().await;

    match inaturalist::apis::observations_api::observations_get(&config_guard, api_params).await {
        Ok(response) => Ok(response.total_results.unwrap_or(0)),
        Err(e) => {
            log::error!("Failed to get update observation count: {e:?}");
            Err(format!("Failed to get update observation count: {e}"))
        }
    }
}

#[tauri::command]
pub async fn update_inat_archive(
    app: AppHandle,
    path: String,
    cache: State<'_, AuthCache>,
) -> Result<(), String> {
    use chuck_core::archive_updater::update_archive;

    CANCEL_FLAG.as_ref().store(false, Ordering::Relaxed);

    app.emit("inat-progress", InatProgress::Building {
        message: "Initializing update...".to_string()
    }).map_err(|e| e.to_string())?;

    let jwt = match cache.load_token() {
        Ok(Some(oauth_token)) => fetch_jwt(&oauth_token).await.ok(),
        _ => None,
    };

    let app_clone = app.clone();
    let progress_callback = move |progress: chuck_core::downloader::DownloadProgress| {
        use chuck_core::downloader::DownloadStage;

        let event = match progress.stage {
            DownloadStage::Fetching => InatProgress::Fetching {
                current: progress.observations_current,
                total: progress.observations_total,
            },
            DownloadStage::DownloadingMedia => InatProgress::DownloadingMedia {
                current: progress.media_current,
                total: progress.media_total,
            },
            DownloadStage::Building => InatProgress::Building {
                message: "Merging records...".to_string(),
            },
            DownloadStage::Merging { current, total } => InatProgress::Merging { current, total },
        };
        let _ = app_clone.emit("inat-progress", event);
    };

    let _awake = keepawake::Builder::default()
        .reason("Updating iNaturalist archive".to_string())
        .app_name("Chuck".to_string())
        .app_reverse_domain("org.inaturalist.chuck".to_string())
        .idle(true)
        .sleep(true)
        .create()
        .inspect_err(|e| log::warn!("Could not acquire sleep inhibitor: {e}"))
        .ok();

    let cancel_token = Arc::clone(&CANCEL_FLAG);
    update_archive(&path, progress_callback, jwt, Some(cancel_token))
        .await
        .map_err(|e| e.to_string())?;

    app.emit("inat-progress", InatProgress::Complete)
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    #[test]
    fn test_extract_query_strips_url() {
        assert_eq!(
            super::extract_query("https://www.inaturalist.org/observations?taxon_id=47790"),
            "taxon_id=47790"
        );
    }

    #[test]
    fn test_extract_query_passthrough_when_no_question_mark() {
        assert_eq!(super::extract_query("taxon_id=47790"), "taxon_id=47790");
    }

    #[test]
    fn test_convert_observations_to_multimedia_when_simple_multimedia_enabled() {
        use chuck_core::downloader::convert_to_photo_multimedia;

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
        let multimedia = convert_to_photo_multimedia(&observations, &photo_mapping);

        // Should have created multimedia records
        assert_eq!(multimedia.len(), 1);
        assert_eq!(multimedia[0].occurrence_id, "https://www.inaturalist.org/observations/123");
    }

    #[test]
    fn test_convert_observations_to_multimedia_without_photos() {
        use chuck_core::downloader::convert_to_photo_multimedia;

        let observations = vec![
            inaturalist::models::Observation {
                id: Some(123),
                photos: None,
                ..Default::default()
            }
        ];

        let photo_mapping = HashMap::new();

        let multimedia = convert_to_photo_multimedia(&observations, &photo_mapping);

        // Should be empty when there are no photos
        assert_eq!(multimedia.len(), 0);
    }
}

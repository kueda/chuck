use chuck_core::api::{client, params};
use chuck_core::auth::{fetch_jwt, TokenStorage};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use std::sync::atomic::{AtomicBool, Ordering};
use crate::inat_auth::KeyringStorage;

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
static CANCEL_FLAG: AtomicBool = AtomicBool::new(false);

/// Convert observations to multimedia records based on enabled extensions
fn convert_observations_to_multimedia(
    observations: &[inaturalist::models::Observation],
    extensions: &[chuck_core::DwcaExtension],
    photo_mapping: &std::collections::HashMap<i32, String>,
) -> Vec<chuck_core::darwin_core::Multimedia> {
    use chuck_core::darwin_core::Multimedia;

    if !extensions.contains(&chuck_core::DwcaExtension::SimpleMultimedia) {
        return Vec::new();
    }

    observations
        .iter()
        .filter_map(|obs| {
            let occurrence_id = obs.id.map(|id| format!("{}", id))?;
            Some(obs.photos.as_ref()?.iter().map(|photo| {
                Multimedia::from((photo, occurrence_id.as_str(), obs.user.as_deref(), photo_mapping))
            }).collect::<Vec<_>>())
        })
        .flatten()
        .collect()
}

/// Convert observations to audiovisual records based on enabled extensions
fn convert_observations_to_audiovisual(
    observations: &[inaturalist::models::Observation],
    extensions: &[chuck_core::DwcaExtension],
    photo_mapping: &std::collections::HashMap<i32, String>,
) -> Vec<chuck_core::darwin_core::Audiovisual> {
    use chuck_core::darwin_core::Audiovisual;

    if !extensions.contains(&chuck_core::DwcaExtension::Audiovisual) {
        return Vec::new();
    }

    observations
        .iter()
        .filter_map(|obs| {
            let occurrence_id = obs.id.map(|id| format!("{}", id))?;
            Some(obs.photos.as_ref()?.iter().map(|photo| {
                Audiovisual::from((photo, occurrence_id.as_str(), obs, photo_mapping))
            }).collect::<Vec<_>>())
        })
        .flatten()
        .collect()
}

/// Convert observations to identification records based on enabled extensions
fn convert_observations_to_identifications(
    observations: &[inaturalist::models::Observation],
    extensions: &[chuck_core::DwcaExtension],
    taxa_hash: &std::collections::HashMap<i32, inaturalist::models::ShowTaxon>,
) -> Vec<chuck_core::darwin_core::Identification> {
    use chuck_core::darwin_core::Identification;

    if !extensions.contains(&chuck_core::DwcaExtension::Identifications) {
        return Vec::new();
    }

    observations
        .iter()
        .filter_map(|obs| {
            let occurrence_id = obs.id.map(|id| format!("{}", id))?;
            Some(obs.identifications.as_ref()?.iter().map(|identification| {
                Identification::from((identification, occurrence_id.as_str(), taxa_hash))
            }).collect::<Vec<_>>())
        })
        .flatten()
        .collect()
}

#[tauri::command]
pub async fn generate_inat_archive(
    app: AppHandle,
    params: GenerateParams,
) -> Result<(), String> {
    use chuck_core::darwin_core::{
        ArchiveBuilder,
        Occurrence,
        Metadata,
        collect_taxon_ids,
        fetch_taxa_for_observations,
    };
    use inaturalist::apis::observations_api;
    use std::collections::HashMap;

    // Reset cancellation flag
    CANCEL_FLAG.store(false, Ordering::Relaxed);

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

    // Create metadata
    let mut abstract_lines = Vec::new();
    if let Some(taxon_id) = params.taxon_id {
        abstract_lines.push(format!("Taxon ID: {}", taxon_id));
    }
    if let Some(place_id) = params.place_id {
        abstract_lines.push(format!("Place ID: {}", place_id));
    }
    if let Some(d1) = &params.d1 {
        abstract_lines.push(format!("Observed after: {}", d1));
    }
    if let Some(d2) = &params.d2 {
        abstract_lines.push(format!("Observed before: {}", d2));
    }
    let metadata = Metadata { abstract_lines };

    // Create archive builder
    app.emit("inat-progress", InatProgress::Building {
        message: "Initializing archive...".to_string()
    }).map_err(|e| e.to_string())?;

    let mut archive = ArchiveBuilder::new(extensions.clone(), metadata)
        .map_err(|e| format!("Failed to create archive builder: {}", e))?;

    // Fetch JWT if authenticated
    let jwt = match KeyringStorage::new(app.clone()) {
        Ok(storage) => {
            match storage.load_token() {
                Ok(Some(oauth_token)) => {
                    fetch_jwt(&oauth_token).await.ok()
                }
                _ => None
            }
        }
        Err(_) => None
    };

    // Create API config with JWT if available
    let config_instance = client::create_config_with_jwt(jwt);
    let config = tokio::sync::RwLock::new(config_instance);

    // Fetch observations in batches
    let mut total_fetched = 0;
    let mut id_below: Option<i32> = None;
    let mut all_taxa_ids: std::collections::HashSet<i32> = std::collections::HashSet::new();
    let mut total_photos_downloaded = 0;
    // Capture total from first response to keep progress stable
    let mut total_observations: Option<usize> = None;
    let mut estimated_total_photos: Option<usize> = None;

    loop {
        // Check cancellation
        if CANCEL_FLAG.load(Ordering::Relaxed) {
            return Err("Generation cancelled".to_string());
        }

        let mut fetch_params = api_params.clone();
        if let Some(id) = id_below {
            fetch_params.id_below = Some(id.to_string());
        }

        // Fetch observations
        let config_guard = config.read().await;
        let result = observations_api::observations_get(&*config_guard, fetch_params).await;
        drop(config_guard);

        match result {
            Ok(response) => {
                let results = response.results;

                if results.is_empty() {
                    break;
                }

                // Capture total from first response only
                if total_observations.is_none() {
                    total_observations = Some(response.total_results.unwrap_or(0) as usize);
                }

                total_fetched += results.len();

                // Update progress with stable total
                let _ = app.emit("inat-progress", InatProgress::Fetching {
                    current: total_fetched,
                    total: total_observations.unwrap_or(0),
                });

                // Collect taxon IDs for taxonomic hierarchy. This was part of a previous
                // effort to fetch taxa and may be vestigial.
                for obs in &results {
                    if let Some(taxon) = &obs.taxon {
                        if let Some(ancestor_ids) = &taxon.ancestor_ids {
                            all_taxa_ids.extend(ancestor_ids.iter());
                        }
                    }
                }

                // Fetch taxa for this batch to populate taxonomic hierarchy
                let taxon_ids = collect_taxon_ids(&results);

                // let app_for_taxa = app.clone();
                let taxa_hash = fetch_taxa_for_observations(
                    &taxon_ids,
                    // IMO updating the progress with this is just
                    // distracting. The user is just concerned with getting
                    // the observations
                    None::<fn(usize, usize)>
                ).await.map_err(|e| format!("Failed to fetch taxa: {}", e))?;

                let occurrences: Vec<Occurrence> = results
                    .iter()
                    .map(|obs| Occurrence::from((obs, &taxa_hash)))
                    .collect();

                // Add to archive
                archive.add_occurrences(&occurrences).await
                    .map_err(|e| format!("Failed to add occurrences: {}", e))?;

                // Download photos if fetch_photos is enabled
                let photo_mapping = if params.fetch_photos {
                    use chuck_core::darwin_core::PhotoDownloader;
                    use std::sync::Arc;
                    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

                    // Count total photos in this batch
                    let photos_in_batch = results.iter()
                        .filter_map(|o| o.photos.as_ref())
                        .flatten()
                        .count();

                    if photos_in_batch > 0 {
                        // Estimate total photos from first batch
                        if estimated_total_photos.is_none() {
                            let avg_photos_per_obs = photos_in_batch as f64 / results.len() as f64;
                            estimated_total_photos = Some(
                                (avg_photos_per_obs * total_observations.unwrap_or(0) as f64).round() as usize
                            );
                        }

                        // Create atomic counter for progress tracking
                        let photos_downloaded = Arc::new(AtomicUsize::new(0));
                        let app_handle = app.clone();
                        let photos_downloaded_clone = photos_downloaded.clone();
                        let est_total = estimated_total_photos.unwrap_or(photos_in_batch);

                        // Create callback to emit progress events
                        let progress_callback = move |count: usize| {
                            let current = photos_downloaded_clone.fetch_add(count, AtomicOrdering::Relaxed) + count;
                            let _ = app_handle.emit("inat-progress", InatProgress::DownloadingPhotos {
                                current: total_photos_downloaded + current,
                                total: est_total,
                            });
                        };

                        // Download photos for this batch
                        let mapping = PhotoDownloader::fetch_photos_to_dir(&results, archive.media_dir(), progress_callback)
                            .await
                            .map_err(|e| format!("Failed to download photos: {}", e))?;

                        total_photos_downloaded += photos_in_batch;
                        mapping
                    } else {
                        HashMap::new()
                    }
                } else {
                    HashMap::new()
                };

                let multimedia = convert_observations_to_multimedia(&results, &extensions, &photo_mapping);
                if !multimedia.is_empty() {
                    archive.add_multimedia(&multimedia).await
                        .map_err(|e| format!("Failed to add multimedia: {}", e))?;
                }

                let audiovisual = convert_observations_to_audiovisual(&results, &extensions, &photo_mapping);
                if !audiovisual.is_empty() {
                    archive.add_audiovisual(&audiovisual).await
                        .map_err(|e| format!("Failed to add audiovisual: {}", e))?;
                }

                let identifications = convert_observations_to_identifications(&results, &extensions, &taxa_hash);
                if !identifications.is_empty() {
                    archive.add_identifications(&identifications).await
                        .map_err(|e| format!("Failed to add identifications: {}", e))?;
                }

                // Get last ID for pagination
                if let Some(last_obs) = results.last() {
                    id_below = last_obs.id;
                }

                // Rate limiting
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            }
            Err(e) => {
                log::error!("Failed to fetch observations: {:?}", e);
                let _ = app.emit("inat-progress", InatProgress::Error {
                    message: format!("Failed to fetch observations: {}", e)
                });
                return Err(format!("Failed to fetch observations: {}", e));
            }
        }
    }

    // Check if cancelled before building
    if CANCEL_FLAG.load(Ordering::Relaxed) {
        return Err("Generation cancelled".to_string());
    }

    // Build the archive
    let _ = app.emit("inat-progress", InatProgress::Building {
        message: "Finalizing archive...".to_string()
    });

    archive.build(&params.output_path).await
        .map_err(|e| format!("Failed to build archive: {}", e))?;

    // Emit completion
    app.emit("inat-progress", InatProgress::Complete)
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn cancel_inat_archive() -> Result<(), String> {
    CANCEL_FLAG.store(true, Ordering::Relaxed);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_convert_observations_to_multimedia_when_simple_multimedia_enabled() {
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

        let extensions = vec![chuck_core::DwcaExtension::SimpleMultimedia];
        let photo_mapping = HashMap::new();

        // This function should exist and convert observations to multimedia records
        let multimedia = convert_observations_to_multimedia(&observations, &extensions, &photo_mapping);

        // Should have created multimedia records because SimpleMultimedia is enabled
        assert_eq!(multimedia.len(), 1);
        assert_eq!(multimedia[0].occurrence_id, "123");
    }

    #[test]
    fn test_convert_observations_to_multimedia_when_extension_not_enabled() {
        let observations = vec![
            inaturalist::models::Observation {
                id: Some(123),
                photos: Some(vec![
                    inaturalist::models::Photo {
                        id: Some(456),
                        ..Default::default()
                    }
                ]),
                ..Default::default()
            }
        ];

        let extensions = vec![]; // No extensions enabled
        let photo_mapping = HashMap::new();

        let multimedia = convert_observations_to_multimedia(&observations, &extensions, &photo_mapping);

        // Should be empty because SimpleMultimedia is not enabled
        assert_eq!(multimedia.len(), 0);
    }
}

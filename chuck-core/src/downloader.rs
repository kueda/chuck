use inaturalist::apis::observations_api;
use crate::DwcaExtension;
use crate::darwin_core::Metadata;

/// Progress information for download operations
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub stage: DownloadStage,
    pub observations_current: usize,
    pub observations_total: usize,
    pub photos_current: usize,
    pub photos_total: usize,
}

impl Default for DownloadProgress {
    fn default() -> Self {
        Self {
            stage: DownloadStage::Fetching,
            observations_current: 0,
            observations_total: 0,
            photos_current: 0,
            photos_total: 0,
        }
    }
}

/// Stages of the download process
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadStage {
    Fetching,
    DownloadingPhotos,
    Building,
}

/// Centralized downloader for iNaturalist observations to DarwinCore Archive
pub struct Downloader {
    params: observations_api::ObservationsGetParams,
    extensions: Vec<DwcaExtension>,
    fetch_photos: bool,
    metadata: Metadata,
    config: Option<inaturalist::apis::configuration::Configuration>,
    jwt: Option<String>,
}

impl Downloader {
    pub fn new(
        params: observations_api::ObservationsGetParams,
        extensions: Vec<DwcaExtension>,
        fetch_photos: bool,
        jwt: Option<String>,
    ) -> Self {
        Self::from_parts(params, extensions, fetch_photos, None, jwt)
    }

    /// Create downloader with custom configuration for testing
    pub fn with_config(
        params: observations_api::ObservationsGetParams,
        extensions: Vec<DwcaExtension>,
        fetch_photos: bool,
        config: inaturalist::apis::configuration::Configuration,
    ) -> Self {
        Self::from_parts(params, extensions, fetch_photos, Some(config), None)
    }

    /// Internal constructor that builds metadata and creates the Downloader
    fn from_parts(
        params: observations_api::ObservationsGetParams,
        extensions: Vec<DwcaExtension>,
        fetch_photos: bool,
        config: Option<inaturalist::apis::configuration::Configuration>,
        jwt: Option<String>,
    ) -> Self {
        // Build metadata
        let mut abstract_lines = vec![
            "Observations exported from iNaturalist using the following criteria:".to_string()
        ];
        abstract_lines.extend(
            crate::api::params::extract_criteria(&params)
                .into_iter()
                .map(|c| format!("* {}", c))
        );
        if fetch_photos {
            abstract_lines.push("* Photos downloaded and included in archive".to_string());
        }
        let metadata = Metadata { abstract_lines };

        Self {
            params,
            extensions,
            fetch_photos,
            metadata,
            config,
            jwt,
        }
    }

    /// Execute the download and build the archive
    pub async fn execute<F>(
        &self,
        output_path: &str,
        progress_callback: F,
        cancellation_token: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        F: Fn(DownloadProgress) + Send + Sync + Clone + 'static,
    {
        use std::sync::atomic::Ordering;
        use crate::darwin_core::ArchiveBuilder;

        // Create archive builder
        let mut archive = ArchiveBuilder::new(
            self.extensions.clone(),
            self.metadata.clone(),
        )?;

        let mut progress = DownloadProgress::default();

        // Pagination loop
        let mut id_below: Option<i32> = None;
        loop {
            // Check cancellation
            if let Some(token) = &cancellation_token {
                if token.load(Ordering::Relaxed) {
                    return Err("Download cancelled".into());
                }
            }

            // Fetch batch
            let batch = self.fetch_batch(id_below).await?;
            if batch.results.is_empty() {
                break;
            }

            // Capture total from first batch
            if progress.observations_total == 0 {
                progress.observations_total = batch.total_results.unwrap_or(0) as usize;
            }

            // Process batch
            self.process_batch(&batch, &mut archive, &mut progress, &progress_callback).await?;

            // Update pagination
            id_below = batch.results.last().and_then(|o| o.id);

            // Rate limit
            crate::api::rate_limiter::get_rate_limiter()
                .await
                .wait_for_next_request()
                .await;
        }

        // Build final archive
        progress.stage = DownloadStage::Building;
        progress_callback(progress.clone());
        archive.build(output_path).await?;

        Ok(())
    }

    async fn fetch_batch(
        &self,
        id_below: Option<i32>,
    ) -> Result<inaturalist::models::ObservationsResponse, Box<dyn std::error::Error>> {
        use crate::api::client;

        let mut fetch_params = self.params.clone();
        if let Some(id) = id_below {
            fetch_params.id_below = Some(id.to_string());
        }

        // Use custom config if provided, otherwise use global config with JWT
        if let Some(ref config) = self.config {
            let config_lock = tokio::sync::RwLock::new(config.clone());
            let response = client::fetch_observations_with_retry(&config_lock, fetch_params).await?;
            Ok(response)
        } else if let Some(ref jwt) = self.jwt {
            let config_instance = client::create_config_with_jwt(Some(jwt.clone()));
            let config = tokio::sync::RwLock::new(config_instance);
            let response = client::fetch_observations_with_retry(&config, fetch_params).await?;
            Ok(response)
        } else {
            let config = client::get_config().await;
            let response = client::fetch_observations_with_retry(config, fetch_params).await?;
            Ok(response)
        }
    }

    async fn process_batch<F>(
        &self,
        batch: &inaturalist::models::ObservationsResponse,
        archive: &mut crate::darwin_core::ArchiveBuilder,
        progress: &mut DownloadProgress,
        callback: &F,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        F: Fn(DownloadProgress) + Send + Sync + Clone + 'static,
    {
        use crate::darwin_core::{Occurrence, collect_taxon_ids, fetch_taxa_for_observations, PhotoDownloader};
        use std::sync::Arc;

        // Fetch taxa for this batch
        let taxon_ids = collect_taxon_ids(&batch.results);
        let taxa_hash = fetch_taxa_for_observations(
            &taxon_ids,
            None::<fn(usize, usize)>,
            self.config.as_ref(),
        ).await?;

        // Convert to occurrences
        let occurrences: Vec<Occurrence> = batch.results
            .iter()
            .map(|obs| Occurrence::from((obs, &taxa_hash)))
            .collect();

        // Add to archive
        archive.add_occurrences(&occurrences).await?;

        // Update progress
        progress.observations_current += batch.results.len();
        progress.stage = DownloadStage::Fetching;
        callback(progress.clone());

        // Download photos if enabled
        let photo_mapping = if self.fetch_photos {
            let photos_count = batch.results
                .iter()
                .filter_map(|o| o.photos.as_ref())
                .flatten()
                .count();

            if photos_count > 0 {
                // Estimate total photos from first batch
                if progress.photos_total == 0 {
                    let avg_photos_per_obs = photos_count as f64 / batch.results.len() as f64;
                    progress.photos_total = (avg_photos_per_obs * progress.observations_total as f64).round() as usize;
                }

                progress.stage = DownloadStage::DownloadingPhotos;

                // Create progress callback for photo downloads
                let photos_downloaded = Arc::new(std::sync::atomic::AtomicUsize::new(0));
                let photos_downloaded_clone = photos_downloaded.clone();
                let progress_for_callback = progress.clone();
                let callback_clone = callback.clone();

                let photo_callback = move |count: usize| {
                    let current = photos_downloaded_clone.fetch_add(count, std::sync::atomic::Ordering::Relaxed) + count;
                    let mut updated_progress = progress_for_callback.clone();
                    updated_progress.photos_current = current;
                    callback_clone(updated_progress);
                };

                let mapping = PhotoDownloader::fetch_photos_to_dir(
                    &batch.results,
                    archive.media_dir(),
                    photo_callback,
                ).await?;

                progress.photos_current = photos_downloaded.load(std::sync::atomic::Ordering::Relaxed);
                mapping
            } else {
                HashMap::new()
            }
        } else {
            HashMap::new()
        };

        // Process extensions
        self.process_extensions(&batch.results, archive, &photo_mapping, &taxa_hash).await?;

        Ok(())
    }

    async fn process_extensions(
        &self,
        observations: &[Observation],
        archive: &mut crate::darwin_core::ArchiveBuilder,
        photo_mapping: &HashMap<i32, String>,
        taxa_hash: &HashMap<i32, ShowTaxon>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Multimedia extension
        if self.extensions.contains(&DwcaExtension::SimpleMultimedia) {
            let records = convert_to_multimedia(observations, photo_mapping);
            if !records.is_empty() {
                archive.add_multimedia(&records).await?;
            }
        }

        // Audiovisual extension
        if self.extensions.contains(&DwcaExtension::Audiovisual) {
            let records = convert_to_audiovisual(observations, photo_mapping);
            if !records.is_empty() {
                archive.add_audiovisual(&records).await?;
            }
        }

        // Identifications extension
        if self.extensions.contains(&DwcaExtension::Identifications) {
            let records = convert_to_identifications(observations, taxa_hash);
            if !records.is_empty() {
                archive.add_identifications(&records).await?;
            }
        }

        Ok(())
    }
}

use std::collections::HashMap;
use inaturalist::models::{Observation, ShowTaxon};
use crate::darwin_core::{Multimedia, Audiovisual, Identification};

/// Convert observations to multimedia records
pub fn convert_to_multimedia(
    observations: &[Observation],
    photo_mapping: &HashMap<i32, String>,
) -> Vec<Multimedia> {
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

/// Convert observations to audiovisual records
pub fn convert_to_audiovisual(
    observations: &[Observation],
    photo_mapping: &HashMap<i32, String>,
) -> Vec<Audiovisual> {
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

/// Convert observations to identification records
pub fn convert_to_identifications(
    observations: &[Observation],
    taxa_hash: &HashMap<i32, ShowTaxon>,
) -> Vec<Identification> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DwcaExtension;
    use inaturalist::apis::observations_api;

    #[test]
    fn test_downloader_new() {
        let params = observations_api::ObservationsGetParams {
            taxon_id: Some(vec!["47126".to_string()]),
            per_page: Some("200".to_string()),
            ..crate::api::params::DEFAULT_GET_PARAMS.clone()
        };
        let extensions = vec![DwcaExtension::SimpleMultimedia];

        let downloader = Downloader::new(params, extensions, true, None);

        assert!(downloader.params.taxon_id == Some(vec!["47126".to_string()]));
        assert_eq!(downloader.extensions.len(), 1);
        assert_eq!(downloader.fetch_photos, true);
    }

    #[test]
    fn test_download_progress_default() {
        let progress = DownloadProgress::default();

        assert_eq!(progress.observations_current, 0);
        assert_eq!(progress.observations_total, 0);
        assert_eq!(progress.photos_current, 0);
        assert_eq!(progress.photos_total, 0);
        assert!(matches!(progress.stage, DownloadStage::Fetching));
    }

    #[test]
    fn test_convert_to_multimedia_with_photos() {
        use std::collections::HashMap;
        use inaturalist::models::{Observation, Photo, User};

        let observations = vec![
            Observation {
                id: Some(123),
                photos: Some(vec![
                    Photo {
                        id: Some(456),
                        url: Some("https://example.com/photo.jpg".to_string()),
                        ..Default::default()
                    }
                ]),
                user: Some(Box::new(User {
                    login: Some("testuser".to_string()),
                    ..Default::default()
                })),
                ..Default::default()
            }
        ];

        let photo_mapping = HashMap::new();
        let multimedia = convert_to_multimedia(&observations, &photo_mapping);

        assert_eq!(multimedia.len(), 1);
        assert_eq!(multimedia[0].occurrence_id, "123");
    }

    #[test]
    fn test_convert_to_multimedia_without_photos() {
        use std::collections::HashMap;
        use inaturalist::models::Observation;

        let observations = vec![
            Observation {
                id: Some(123),
                photos: None,
                ..Default::default()
            }
        ];

        let photo_mapping = HashMap::new();
        let multimedia = convert_to_multimedia(&observations, &photo_mapping);

        assert_eq!(multimedia.len(), 0);
    }

    #[test]
    fn test_convert_to_audiovisual_with_photos() {
        use std::collections::HashMap;
        use inaturalist::models::{Observation, Photo};

        let observations = vec![
            Observation {
                id: Some(123),
                photos: Some(vec![
                    Photo {
                        id: Some(456),
                        ..Default::default()
                    }
                ]),
                ..Default::default()
            }
        ];

        let photo_mapping = HashMap::new();
        let audiovisual = convert_to_audiovisual(&observations, &photo_mapping);

        assert_eq!(audiovisual.len(), 1);
        assert_eq!(audiovisual[0].occurrence_id, "123");
    }

    #[test]
    fn test_convert_to_identifications() {
        use std::collections::HashMap;
        use inaturalist::models::Observation;

        let observations = vec![
            Observation {
                id: Some(123),
                identifications: Some(vec![
                    Default::default()
                ]),
                ..Default::default()
            }
        ];

        let taxa_hash = HashMap::new();
        let identifications = convert_to_identifications(&observations, &taxa_hash);

        assert_eq!(identifications.len(), 1);
        assert_eq!(identifications[0].occurrence_id, "123");
    }

}

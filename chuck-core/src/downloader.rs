use inaturalist::apis::observations_api;
use crate::DwcaExtension;
use crate::darwin_core::Metadata;

/// Progress information for download operations
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub stage: DownloadStage,
    pub observations_current: usize,
    pub observations_total: usize,
    pub media_current: usize,
    pub media_total: usize,
}

impl Default for DownloadProgress {
    fn default() -> Self {
        Self {
            stage: DownloadStage::Fetching,
            observations_current: 0,
            observations_total: 0,
            media_current: 0,
            media_total: 0,
        }
    }
}

/// Stages of the download process
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadStage {
    Fetching,
    DownloadingMedia,
    Building,
}

/// Centralized downloader for iNaturalist observations to DarwinCore Archive
pub struct Downloader {
    params: observations_api::ObservationsGetParams,
    extensions: Vec<DwcaExtension>,
    fetch_media: bool,
    metadata: Metadata,
    config: Option<inaturalist::apis::configuration::Configuration>,
    jwt: Option<String>,
}

impl Downloader {
    pub fn new(
        params: observations_api::ObservationsGetParams,
        extensions: Vec<DwcaExtension>,
        fetch_media: bool,
        jwt: Option<String>,
    ) -> Self {
        Self::from_parts(params, extensions, fetch_media, None, jwt)
    }

    /// Create downloader with custom configuration for testing
    pub fn with_config(
        params: observations_api::ObservationsGetParams,
        extensions: Vec<DwcaExtension>,
        fetch_media: bool,
        config: inaturalist::apis::configuration::Configuration,
    ) -> Self {
        Self::from_parts(params, extensions, fetch_media, Some(config), None)
    }

    /// Internal constructor that builds metadata and creates the Downloader
    fn from_parts(
        params: observations_api::ObservationsGetParams,
        extensions: Vec<DwcaExtension>,
        fetch_media: bool,
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
                .map(|c| format!("* {c}"))
        );
        if fetch_media {
            abstract_lines.push(
                "* Photos and sounds downloaded and included in archive".to_string()
            );
        }
        let inat_query = Some(crate::api::params::serialize_params(&params));
        let metadata = Metadata { abstract_lines, inat_query };

        Self {
            params,
            extensions,
            fetch_media,
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

        // Pass the output path directly so the builder opens the ZIP immediately and
        // places temp files on the same filesystem (avoids Linux tmpfs exhaustion).
        let mut archive = ArchiveBuilder::new(
            self.extensions.clone(),
            self.metadata.clone(),
            std::path::Path::new(output_path),
        )?;

        log::info!(
            "Download starting: output={output_path}, fetch_media={}, extensions={:?}",
            self.fetch_media, self.extensions
        );

        let mut progress = DownloadProgress::default();
        let mut cumulative_media_seen: usize = 0;

        // Track pending media download from previous batch
        #[allow(clippy::type_complexity)]
        let mut pending_media: Option<(
            tokio::task::JoinHandle<Result<(HashMap<i32, String>, HashMap<i32, String>, usize), String>>,
            Vec<inaturalist::models::Observation>,
            HashMap<i32, inaturalist::models::ShowTaxon>,
        )> = None;

        // Pagination loop with true pipeline
        let mut id_below: Option<i32> = None;
        loop {
            // Check cancellation
            if let Some(token) = &cancellation_token {
                if token.load(Ordering::Relaxed) {
                    // Abort any pending media download task
                    if let Some((media_handle, _, _)) = pending_media.take() {
                        media_handle.abort();
                    }
                    return Err("Download cancelled".into());
                }
            }

            // Fetch next batch. Abort any in-flight media task before propagating errors:
            // dropping a JoinHandle does not abort the task in Tokio, so if we return
            // early the spawned task would keep running and sending stale progress events.
            let batch = match self.fetch_batch_with_rate_limit(id_below).await {
                Ok(b) => b,
                Err(e) => {
                    if let Some((handle, _, _)) = pending_media.take() {
                        handle.abort();
                    }
                    return Err(e);
                }
            };
            if batch.results.is_empty() {
                // Before breaking, finish any pending media downloads
                if let Some((media_handle, observations, taxa_hash)) = pending_media.take() {
                    // Check cancellation before waiting for media
                    if let Some(token) = &cancellation_token {
                        if token.load(Ordering::Relaxed) {
                            media_handle.abort();
                            return Err("Download cancelled".into());
                        }
                    }
                    log::debug!("Waiting for final media batch task");
                    let (photo_mapping, sound_mapping, media_count) = media_handle.await
                        .map_err(|e| format!("Media download task failed: {e}"))??;
                    log::debug!(
                        "Final media batch done: {media_count} downloaded \
                        ({} photos, {} sounds)",
                        photo_mapping.len(), sound_mapping.len()
                    );
                    progress.media_current += media_count;
                    self.process_extensions(
                        &observations, &mut archive, &photo_mapping, &sound_mapping, &taxa_hash
                    ).await?;
                    for rel_path in photo_mapping.values().chain(sound_mapping.values()) {
                        archive.add_media_from_temp(rel_path)?;
                    }
                }
                break;
            }

            // Capture total from first batch
            if progress.observations_total == 0 {
                progress.observations_total = batch.total_results.unwrap_or(0) as usize;
            }

            // Prepare batch: fetch taxa, convert to occurrences, write to CSV
            let (taxa_hash, media_count) = match self.prepare_batch(
                &batch, &mut archive, &mut progress, &progress_callback
            ).await {
                Ok(r) => r,
                Err(e) => {
                    if let Some((handle, _, _)) = pending_media.take() {
                        handle.abort();
                    }
                    return Err(e);
                }
            };

            // Update media estimate using running average (never decreasing)
            if self.fetch_media {
                cumulative_media_seen += media_count;
                progress.media_total = update_photo_estimate(
                    progress.media_total,
                    cumulative_media_seen,
                    progress.observations_current,
                    progress.observations_total,
                );
            }

            // If there's a pending media download from the previous batch, wait and process
            if let Some((media_handle, observations, prev_taxa_hash)) = pending_media.take() {
                // Check cancellation before waiting for media
                if let Some(token) = &cancellation_token {
                    if token.load(Ordering::Relaxed) {
                        media_handle.abort();
                        return Err("Download cancelled".into());
                    }
                }
                log::debug!("Waiting for previous media batch task");
                let (photo_mapping, sound_mapping, media_count) = media_handle.await
                    .map_err(|e| format!("Media download task failed: {e}"))??;
                log::debug!(
                    "Media batch done: {media_count} downloaded \
                    ({} photos, {} sounds)",
                    photo_mapping.len(), sound_mapping.len()
                );
                progress.media_current += media_count;
                self.process_extensions(
                    &observations, &mut archive, &photo_mapping, &sound_mapping, &prev_taxa_hash
                ).await?;
                for rel_path in photo_mapping.values().chain(sound_mapping.values()) {
                    archive.add_media_from_temp(rel_path)?;
                }
            }

            // Start media downloads for current batch in background
            let media_handle = self.start_media_downloads(
                &batch,
                archive.media_dir(),
                &mut progress,
                &progress_callback,
                cancellation_token.clone(),
            );

            // Store handle and data for next iteration (or process immediately if no media)
            if let Some(handle) = media_handle {
                pending_media = Some((handle, batch.results.clone(), taxa_hash));
            } else {
                // No media to download, process extensions immediately
                self.process_extensions(
                    &batch.results, &mut archive, &HashMap::new(), &HashMap::new(), &taxa_hash
                ).await?;
            }

            // Update pagination for next iteration
            id_below = batch.results.last().and_then(|o| o.id);
        }

        // Build final archive
        log::info!(
            "Building archive: {} obs, {}/{} media",
            progress.observations_current,
            progress.media_current,
            progress.media_total
        );
        progress.stage = DownloadStage::Building;
        progress_callback(progress.clone());
        archive.build().await?;

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

    /// Fetch batch with rate limiting applied
    async fn fetch_batch_with_rate_limit(
        &self,
        id_below: Option<i32>,
    ) -> Result<inaturalist::models::ObservationsResponse, Box<dyn std::error::Error>> {
        // Rate limit (skip for custom configs, e.g. tests with mock servers)
        if self.config.is_none() {
            crate::api::rate_limiter::get_rate_limiter()
                .await
                .wait_for_next_request()
                .await;
        }
        self.fetch_batch(id_below).await
    }

    /// Start media (photo + sound) downloads as a background task.
    /// Returns a handle that resolves to (photo_mapping, sound_mapping, media_downloaded_count).
    #[allow(clippy::type_complexity)]
    fn start_media_downloads<F>(
        &self,
        batch: &inaturalist::models::ObservationsResponse,
        media_dir: std::path::PathBuf,
        progress: &mut DownloadProgress,
        callback: &F,
        cancellation_token: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    ) -> Option<tokio::task::JoinHandle<Result<(HashMap<i32, String>, HashMap<i32, String>, usize), String>>>
    where
        F: Fn(DownloadProgress) + Send + Sync + Clone + 'static,
    {
        use crate::darwin_core::{PhotoDownloader, SoundDownloader};
        use std::sync::Arc;

        let photos_count = batch.results
            .iter()
            .filter_map(|o| o.photos.as_ref())
            .flatten()
            .count();
        let sounds_count = batch.results
            .iter()
            .filter_map(|o| o.sounds.as_ref())
            .flatten()
            .filter(|s| s.file_url.is_some() && !s.hidden.unwrap_or(false))
            .count();

        if !self.fetch_media || (photos_count == 0 && sounds_count == 0) {
            return None;
        }

        log::info!(
            "Spawning media batch task: {photos_count} photos + {sounds_count} sounds"
        );
        progress.stage = DownloadStage::DownloadingMedia;

        let media_downloaded = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let media_downloaded_clone = media_downloaded.clone();
        let base_media_current = progress.media_current;
        let progress_for_callback = progress.clone();
        let callback_clone = callback.clone();

        let media_callback = move |count: usize| {
            let batch_media_count = media_downloaded_clone.fetch_add(
                count, std::sync::atomic::Ordering::Relaxed
            ) + count;
            let mut updated_progress = progress_for_callback.clone();
            updated_progress.media_current = base_media_current + batch_media_count;
            callback_clone(updated_progress);
        };

        let observations = batch.results.clone();

        let handle = tokio::spawn(async move {
            let photo_callback = media_callback.clone();
            let sound_callback = media_callback.clone();

            let photo_mapping = PhotoDownloader::fetch_photos_to_dir(
                &observations,
                &media_dir,
                photo_callback,
                cancellation_token.clone(),
            )
            .await
            .map_err(|e| e.to_string())?;

            let sound_mapping = SoundDownloader::fetch_sounds_to_dir(
                &observations,
                &media_dir,
                sound_callback,
                cancellation_token,
            )
            .await
            .map_err(|e| e.to_string())?;

            let count = media_downloaded.load(std::sync::atomic::Ordering::Relaxed);
            Ok((photo_mapping, sound_mapping, count))
        });

        Some(handle)
    }

    /// Prepare a batch by fetching taxa, converting to occurrences, and writing to CSV.
    /// Returns taxa_hash and media_count (photos + sounds) for use in media downloading.
    async fn prepare_batch<F>(
        &self,
        batch: &inaturalist::models::ObservationsResponse,
        archive: &mut crate::darwin_core::ArchiveBuilder,
        progress: &mut DownloadProgress,
        callback: &F,
    ) -> Result<(HashMap<i32, ShowTaxon>, usize), Box<dyn std::error::Error>>
    where
        F: Fn(DownloadProgress) + Send + Sync + Clone + 'static,
    {
        use crate::darwin_core::{Occurrence, collect_taxon_ids, fetch_taxa_for_observations};

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

        // Count photos + sounds for media download estimate
        let photos_count = batch.results
            .iter()
            .filter_map(|o| o.photos.as_ref())
            .flatten()
            .count();
        let sounds_count = batch.results
            .iter()
            .filter_map(|o| o.sounds.as_ref())
            .flatten()
            .filter(|s| s.file_url.is_some() && !s.hidden.unwrap_or(false))
            .count();

        Ok((taxa_hash, photos_count + sounds_count))
    }

    async fn process_extensions(
        &self,
        observations: &[Observation],
        archive: &mut crate::darwin_core::ArchiveBuilder,
        photo_mapping: &HashMap<i32, String>,
        sound_mapping: &HashMap<i32, String>,
        taxa_hash: &HashMap<i32, ShowTaxon>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Multimedia extension
        if self.extensions.contains(&DwcaExtension::SimpleMultimedia) {
            let mut records = convert_to_photo_multimedia(observations, photo_mapping);
            records.extend(convert_to_sound_multimedia(observations, sound_mapping));
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

        // Comments extension
        if self.extensions.contains(&DwcaExtension::Comments) {
            let records = convert_to_comments(observations);
            if !records.is_empty() {
                archive.add_comments(&records).await?;
            }
        }

        Ok(())
    }
}

/// Compute an updated photo count estimate using a running average across observed batches.
/// Returns the new estimate, but never less than `current_estimate` (never decreases).
pub fn update_photo_estimate(
    current_estimate: usize,
    cumulative_photos: usize,
    cumulative_obs: usize,
    total_obs: usize,
) -> usize {
    if cumulative_obs == 0 || total_obs == 0 {
        return current_estimate;
    }
    let avg = cumulative_photos as f64 / cumulative_obs as f64;
    let new_estimate = (avg * total_obs as f64).round() as usize;
    current_estimate.max(new_estimate)
}

use std::collections::HashMap;
use inaturalist::models::{Observation, ShowTaxon};
use crate::darwin_core::{Multimedia, Audiovisual, Identification, Comment};

/// Convert observations to photo multimedia records
pub fn convert_to_photo_multimedia(
    observations: &[Observation],
    photo_mapping: &HashMap<i32, String>,
) -> Vec<Multimedia> {
    observations
        .iter()
        .filter_map(|obs| {
            let occurrence_id = obs.id.map(|id| format!("{id}"))?;
            Some(obs.photos.as_ref()?.iter().map(|photo| {
                Multimedia::from((photo, occurrence_id.as_str(), obs.user.as_deref(), photo_mapping))
            }).collect::<Vec<_>>())
        })
        .flatten()
        .collect()
}

/// Convert observations to sound multimedia records
pub fn convert_to_sound_multimedia(
    observations: &[Observation],
    sound_mapping: &HashMap<i32, String>,
) -> Vec<Multimedia> {
    observations
        .iter()
        .filter_map(|obs| {
            let occurrence_id = obs.id.map(|id| id.to_string())?;
            Some(obs.sounds.as_ref()?.iter()
                .filter(|s| !s.hidden.unwrap_or(false))
                .filter(|s| s.file_url.is_some() || sound_mapping.contains_key(&s.id.unwrap_or_default()))
                .map(|sound| Multimedia::from((sound, occurrence_id.as_str(), obs.user.as_deref(), sound_mapping)))
                .collect::<Vec<_>>())
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
            let occurrence_id = obs.id.map(|id| format!("{id}"))?;
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
            let occurrence_id = obs.id.map(|id| format!("{id}"))?;
            Some(obs.identifications.as_ref()?.iter().map(|identification| {
                Identification::from((identification, occurrence_id.as_str(), taxa_hash))
            }).collect::<Vec<_>>())
        })
        .flatten()
        .collect()
}

/// Convert observations to comment records, filtering out hidden comments
pub fn convert_to_comments(observations: &[Observation]) -> Vec<Comment> {
    observations
        .iter()
        .filter_map(|obs| {
            let occurrence_id = obs.id.map(|id| format!("{id}"))?;
            Some(
                obs.comments
                    .as_ref()?
                    .iter()
                    .filter(|c| !c.hidden.unwrap_or(false))
                    .map(|comment| {
                        Comment::from((comment, occurrence_id.as_str()))
                    })
                    .collect::<Vec<_>>(),
            )
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
        assert!(downloader.fetch_media);
    }

    #[test]
    fn test_download_progress_default() {
        let progress = DownloadProgress::default();

        assert_eq!(progress.observations_current, 0);
        assert_eq!(progress.observations_total, 0);
        assert_eq!(progress.media_current, 0);
        assert_eq!(progress.media_total, 0);
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
        let multimedia = convert_to_photo_multimedia(&observations, &photo_mapping);

        assert_eq!(multimedia.len(), 1);
        assert_eq!(
            multimedia[0].occurrence_id,
            "https://www.inaturalist.org/observations/123"
        );
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
        let multimedia = convert_to_photo_multimedia(&observations, &photo_mapping);

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
        assert_eq!(
            audiovisual[0].occurrence_id,
            "https://www.inaturalist.org/observations/123"
        );
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
        assert_eq!(
            identifications[0].occurrence_id,
            "https://www.inaturalist.org/observations/123"
        );
    }

    #[test]
    fn test_update_photo_estimate_uses_running_average() {
        // First batch: 100 obs, 100 photos → avg 1.0/obs, total 1000 → estimate 1000
        let est = update_photo_estimate(0, 100, 100, 1000);
        assert_eq!(est, 1000);

        // Second batch cumulative: 200 obs, 400 photos → avg 2.0/obs → estimate 2000
        let est = update_photo_estimate(est, 400, 200, 1000);
        assert_eq!(est, 2000);

        // Third batch with lower density: 300 obs, 420 photos → avg 1.4/obs → estimate 1400
        // But it should never decrease, so estimate stays at 2000
        let est = update_photo_estimate(est, 420, 300, 1000);
        assert_eq!(est, 2000);
    }

    #[test]
    fn test_update_photo_estimate_handles_zero_obs() {
        // Should return current estimate unchanged
        assert_eq!(update_photo_estimate(500, 0, 0, 1000), 500);
        assert_eq!(update_photo_estimate(0, 0, 0, 1000), 0);
    }

    #[test]
    fn test_convert_sounds_to_multimedia() {
        use inaturalist::models::{Observation, Sound, User};

        let observations = vec![
            Observation {
                id: Some(123),
                sounds: Some(vec![
                    Sound {
                        id: Some(456),
                        file_url: Some("https://static.inaturalist.org/sounds/456.mp3".to_string()),
                        file_content_type: Some("audio/mpeg".to_string()),
                        license_code: Some("cc-by".to_string()),
                        attribution: Some("(c) testuser".to_string()),
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

        let sound_mapping = HashMap::new();
        let multimedia = convert_to_sound_multimedia(&observations, &sound_mapping);

        assert_eq!(multimedia.len(), 1);
        assert_eq!(multimedia[0].r#type, Some("Sound".to_string()));
        assert_eq!(multimedia[0].format, Some("audio/mpeg".to_string()));
        assert_eq!(
            multimedia[0].identifier,
            Some("https://static.inaturalist.org/sounds/456.mp3".to_string())
        );
        assert_eq!(multimedia[0].license, Some("cc-by".to_string()));
        assert_eq!(multimedia[0].source, None);
        assert_eq!(
            multimedia[0].occurrence_id,
            "https://www.inaturalist.org/observations/123"
        );
    }

    #[test]
    fn test_convert_sounds_to_multimedia_uses_local_path_when_downloaded() {
        use inaturalist::models::{Observation, Sound};

        let observations = vec![
            Observation {
                id: Some(123),
                sounds: Some(vec![
                    Sound {
                        id: Some(456),
                        file_url: Some("https://static.inaturalist.org/sounds/456.mp3".to_string()),
                        file_content_type: Some("audio/mpeg".to_string()),
                        ..Default::default()
                    }
                ]),
                ..Default::default()
            }
        ];

        let mut sound_mapping = HashMap::new();
        sound_mapping.insert(456i32, "media/2024/01/01/456.mp3".to_string());

        let multimedia = convert_to_sound_multimedia(&observations, &sound_mapping);

        assert_eq!(multimedia.len(), 1);
        assert_eq!(
            multimedia[0].identifier,
            Some("media/2024/01/01/456.mp3".to_string())
        );
    }

    #[test]
    fn test_convert_sounds_to_multimedia_skips_hidden() {
        use inaturalist::models::{Observation, Sound};

        let observations = vec![
            Observation {
                id: Some(123),
                sounds: Some(vec![
                    Sound {
                        id: Some(456),
                        file_url: Some("https://static.inaturalist.org/sounds/456.mp3".to_string()),
                        hidden: Some(true),
                        ..Default::default()
                    },
                    Sound {
                        id: Some(789),
                        file_url: Some("https://static.inaturalist.org/sounds/789.mp3".to_string()),
                        hidden: Some(false),
                        ..Default::default()
                    }
                ]),
                ..Default::default()
            }
        ];

        let sound_mapping = HashMap::new();
        let multimedia = convert_to_sound_multimedia(&observations, &sound_mapping);

        assert_eq!(multimedia.len(), 1);
        assert_eq!(multimedia[0].identifier, Some("https://static.inaturalist.org/sounds/789.mp3".to_string()));
    }

    #[test]
    fn test_convert_to_comments_filters_hidden() {
        use inaturalist::models::{
            Comment as InatComment, Observation,
        };

        let observations = vec![Observation {
            id: Some(123),
            comments: Some(vec![
                InatComment {
                    id: Some(1),
                    body: Some("visible".to_string()),
                    hidden: Some(false),
                    ..Default::default()
                },
                InatComment {
                    id: Some(2),
                    body: Some("hidden".to_string()),
                    hidden: Some(true),
                    ..Default::default()
                },
                InatComment {
                    id: Some(3),
                    body: Some("no hidden field".to_string()),
                    hidden: None,
                    ..Default::default()
                },
            ]),
            ..Default::default()
        }];

        let comments = convert_to_comments(&observations);

        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].text, Some("visible".to_string()));
        assert_eq!(comments[1].text, Some("no hidden field".to_string()));
    }

}

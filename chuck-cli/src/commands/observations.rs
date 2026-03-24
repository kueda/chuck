use tokio::sync::mpsc;
use inaturalist::models::ObservationsResponse;
use inaturalist::apis::observations_api::ObservationsGetParams;
use crate::output::{CsvOutput, ObservationWriter};
use chuck_core::api::{client, params::{build_params, parse_url_params}, rate_limiter::get_rate_limiter};
use chuck_core::archive_updater::update_archive;
use chuck_core::downloader::Downloader;
use crate::progress::ProgressManager;

#[derive(Default)]
pub struct FetchObservationsOptions {
    pub url: Option<String>,
    pub taxon: Option<String>,
    pub place_id: Option<i32>,
    pub user: Option<String>,
    pub d1: Option<String>,
    pub d2: Option<String>,
    pub created_d1: Option<String>,
    pub created_d2: Option<String>,
    pub file: Option<String>,
    pub fetch_media: bool,
    pub format: crate::OutputFormat,
    pub dwc_extensions: Vec<crate::DwcExtension>,
    pub update: bool,
}

fn setup_progress_bar(
    response: &ObservationsResponse,
    progress_manager: &ProgressManager,
) {
    let total_results = response.total_results.unwrap_or(0) as u64;
    if total_results > progress_manager.observations_bar.length().unwrap_or(0) {
        progress_manager.set_observations_total(total_results);
    }
    if let Some(photos_bar) = progress_manager.photos_bar.as_ref() {
        // Estimate number of total photos from this page of results
        let num_photos: u64 = response.results
            .iter()
            .map(|o| match o.photos.as_ref() { Some(photos) => photos.len() as u64, None => 0 })
            .sum();
        let est_total_photos = (
            (num_photos as f64 / (response.results.len() as f64))
            * total_results as f64
            // Slight fudge factor assuming ordering by date posted and newer
            // obs are more likely to have more photos
            * 0.9
        ).round() as u64;
        if est_total_photos > photos_bar.length().unwrap_or(0) {
            photos_bar.set_length(est_total_photos);
        }
    }
}

fn spawn_observation_write_task<W: ObservationWriter + Send + 'static>(
    mut writer: W,
    mut rx: mpsc::Receiver<ObservationsResponse>,
    progress_manager: ProgressManager,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        while let Some(response) = rx.recv().await {
            setup_progress_bar(&response, &progress_manager);
            writer.write_observations(&response.results, &progress_manager).await.unwrap();
        }
        writer.finalize().await.unwrap();
    })
}

/// Build API params from either a URL/query string or individual filter fields.
pub fn build_fetch_params(opts: &FetchObservationsOptions) -> ObservationsGetParams {
    if let Some(ref url) = opts.url {
        let query = url.find('?').map(|i| &url[i + 1..]).unwrap_or(url);
        parse_url_params(query)
    } else {
        build_params(
            opts.taxon.clone(),
            opts.place_id,
            opts.user.clone(),
            opts.d1.clone(),
            opts.d2.clone(),
            opts.created_d1.clone(),
            opts.created_d2.clone(),
        )
    }
}

fn has_filter_args(opts: &FetchObservationsOptions) -> bool {
    opts.url.is_some()
        || opts.taxon.is_some()
        || opts.place_id.is_some()
        || opts.user.is_some()
        || opts.d1.is_some()
        || opts.d2.is_some()
        || opts.created_d1.is_some()
        || opts.created_d2.is_some()
}

pub async fn fetch_observations(
    mut opts: FetchObservationsOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    // Infer DwC format from .zip extension when --update is set and --format
    // was not explicitly provided (default is Csv)
    if opts.update
        && opts.format == crate::OutputFormat::Csv
        && opts.file.as_deref().map(|f| f.ends_with(".zip")).unwrap_or(false)
    {
        opts.format = crate::OutputFormat::Dwc;
    }

    // --- Validate --update constraints ---
    if opts.update {
        if opts.file.is_none() {
            return Err("--update requires --file".into());
        }
        if opts.format == crate::OutputFormat::Dwc && has_filter_args(&opts) {
            return Err(
                "--update with --format dwc does not accept filter args; \
                 filters are read from the archive".into()
            );
        }
    }

    // --- DwC update path ---
    if opts.update && opts.format == crate::OutputFormat::Dwc {
        let zip_path = opts.file.as_deref().unwrap();
        let show_progress = true;
        let progress_manager = ProgressManager::new(show_progress, true);
        let progress_callback = move |progress: chuck_core::downloader::DownloadProgress| {
            match progress.stage {
                chuck_core::downloader::DownloadStage::Fetching => {
                    if progress.observations_total as u64
                        > progress_manager.observations_bar.length().unwrap_or(0)
                    {
                        progress_manager
                            .set_observations_total(progress.observations_total as u64);
                    }
                    progress_manager
                        .observations_bar
                        .set_position(progress.observations_current as u64);
                }
                chuck_core::downloader::DownloadStage::DownloadingMedia => {
                    if let Some(ref bar) = progress_manager.photos_bar {
                        if progress.media_total as u64 > bar.length().unwrap_or(0) {
                            bar.set_length(progress.media_total as u64);
                        }
                        bar.set_position(progress.media_current as u64);
                    }
                }
                chuck_core::downloader::DownloadStage::Building => {}
            }
        };
        return update_archive(zip_path, progress_callback, None, None).await;
    }

    // --- CSV update path ---
    if opts.update && opts.format == crate::OutputFormat::Csv {
        let csv_path = opts.file.as_deref().unwrap();
        return update_csv(csv_path, &opts).await;
    }

    let config = client::get_config().await;
    let params = build_fetch_params(&opts);

    let show_progress = opts.file.is_some();
    let progress_manager = ProgressManager::new(show_progress, opts.fetch_media);

    // Create channel for sending observations from fetcher to writer
    let (tx, rx) = mpsc::channel::<ObservationsResponse>(10);

    // Clone progress manager for the writer task
    let progress_manager_clone = ProgressManager {
        multi: progress_manager.multi.clone(),
        observations_bar: progress_manager.observations_bar.clone(),
        photos_bar: progress_manager.photos_bar.clone(),
    };

    // Spawn writer task based on format
    match opts.format {
        crate::OutputFormat::Csv => {
            let writer = CsvOutput::new(opts.file).unwrap();
            let writer_handle = spawn_observation_write_task(writer, rx, progress_manager_clone);

            // Spawn API fetcher task
            let fetcher_handle = tokio::spawn(async move {
                let mut last_id = 0;
                let rate_limiter = get_rate_limiter().await;

                loop {
                    let mut page_params = params.clone();
                    if last_id != 0 {
                        page_params.id_below = Some(last_id.to_string());
                    }

                    let obs_response = match client::fetch_observations_with_retry(config, page_params).await {
                        Ok(response) => response,
                        Err(e) => {
                            eprintln!("API request failed: {e}");
                            break;
                        }
                    };

                    if obs_response.results.is_empty() {
                        break;
                    }

                    if let Some(id) = obs_response.results.last().and_then(|obs| obs.id) {
                        last_id = id;
                    } else {
                        break;
                    }

                    // Send observations to writer (non-blocking)
                    if tx.send(obs_response).await.is_err() {
                        break; // Writer task has been dropped
                    }

                    // Wait for next allowed request slot to stay under API rate limits
                    rate_limiter.wait_for_next_request().await;
                }

                // Close the channel to signal completion
                drop(tx);
            });

            // Wait for both tasks to complete
            let (writer_result, fetcher_result) = tokio::join!(writer_handle, fetcher_handle);
            writer_result.unwrap();
            fetcher_result.unwrap();
        }
        crate::OutputFormat::Dwc => {
            let output_path = opts.file.unwrap_or_else(|| "observations.zip".to_string());

            let core_extensions: Vec<chuck_core::DwcaExtension> = opts.dwc_extensions
                .iter()
                .map(|e| e.clone().into())
                .collect();

            // Create downloader (CLI uses file-based auth, so no JWT needed)
            let downloader = Downloader::new(params, core_extensions, opts.fetch_media, None);

            // Create progress callback
            let progress_callback = move |progress: chuck_core::downloader::DownloadProgress| {
                match progress.stage {
                    chuck_core::downloader::DownloadStage::Fetching => {
                        if progress.observations_total as u64 > progress_manager.observations_bar.length().unwrap_or(0) {
                            progress_manager.set_observations_total(progress.observations_total as u64);
                        }
                        progress_manager.observations_bar.set_position(progress.observations_current as u64);
                    }
                    chuck_core::downloader::DownloadStage::DownloadingMedia => {
                        if let Some(ref bar) = progress_manager.photos_bar {
                            if progress.media_total as u64 > bar.length().unwrap_or(0) {
                                bar.set_length(progress.media_total as u64);
                            }
                            bar.set_position(progress.media_current as u64);
                        }
                    }
                    chuck_core::downloader::DownloadStage::Building => {
                        // Building message already shown by progress bar completion
                    }
                }
            };

            // Execute download
            downloader.execute(&output_path, progress_callback, None).await?;
        }
    }

    Ok(())
}

/// Update an existing CSV file with observations changed since the last download.
/// `updated_since` is derived from `max(updated_at) - 1 day` in the existing CSV.
async fn update_csv(
    csv_path: &str,
    opts: &FetchObservationsOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::collections::HashMap;
    use chuck_core::merge::merge_csv;

    // Find max(updated_at) from existing CSV
    let mut max_updated_at: Option<String> = None;
    {
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_path(csv_path)?;
        let headers = rdr.headers()?.clone();
        let updated_at_idx = headers
            .iter()
            .position(|h| h == "updated_at")
            .ok_or("CSV has no 'updated_at' column")?;
        for result in rdr.records() {
            let record = result?;
            if let Some(val) = record.get(updated_at_idx) {
                if !val.is_empty() {
                    match &max_updated_at {
                        None => max_updated_at = Some(val.to_string()),
                        Some(current) if val > current.as_str() => {
                            max_updated_at = Some(val.to_string());
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    let max_val = max_updated_at.ok_or("No updated_at values found in CSV")?;
    // Parse date portion (updated_at may be a full datetime like "2026-03-24T12:00:00Z")
    let date_str = max_val.get(..10).unwrap_or(&max_val);
    let date = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")?;
    let updated_since = (date - chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();

    // Fetch updated observations
    let mut params = build_fetch_params(opts);
    params.updated_since = Some(updated_since);

    let config = client::get_config().await;
    let mut updates: HashMap<String, Vec<String>> = HashMap::new();

    // Determine id column index from existing CSV headers
    let id_col_idx = {
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_path(csv_path)?;
        rdr.headers()?
            .iter()
            .position(|h| h == "id")
            .ok_or("CSV has no 'id' column")?
    };

    // Collect updated observations from API
    let mut last_id: Option<i32> = None;
    let rate_limiter = get_rate_limiter().await;
    loop {
        let mut page_params = params.clone();
        if let Some(id) = last_id {
            page_params.id_below = Some(id.to_string());
        }
        let response = client::fetch_observations_with_retry(config, page_params).await?;
        if response.results.is_empty() {
            break;
        }
        let Some(next_last_id) = response.results.last().and_then(|o| o.id) else {
            break;
        };
        last_id = Some(next_last_id);
        for obs in &response.results {
            if let Some(id) = obs.id {
                let row = vec![
                    id.to_string(),
                    obs.user.as_ref().and_then(|u| u.login.clone()).unwrap_or_default(),
                    obs.taxon.as_ref().and_then(|t| t.name.clone()).unwrap_or_default(),
                    obs.taxon.as_ref().and_then(|t| t.id.map(|i| i.to_string())).unwrap_or_default(),
                    obs.location.as_ref().and_then(|l| {
                        let parts: Vec<&str> = l.splitn(2, ',').collect();
                        parts.first().map(|s| s.to_string())
                    }).unwrap_or_default(),
                    obs.location.as_ref().and_then(|l| {
                        let parts: Vec<&str> = l.splitn(2, ',').collect();
                        parts.get(1).map(|s| s.to_string())
                    }).unwrap_or_default(),
                    // private_latitude, private_longitude (not in public API)
                    String::new(),
                    String::new(),
                    obs.positional_accuracy.map(|a| a.to_string()).unwrap_or_default(),
                    obs.public_positional_accuracy.map(|a| a.to_string()).unwrap_or_default(),
                    obs.obscured.map(|b| b.to_string()).unwrap_or_default(),
                    obs.geoprivacy.clone().unwrap_or_default(),
                    obs.taxon_geoprivacy.clone().unwrap_or_default(),
                    obs.updated_at.clone().unwrap_or_default(),
                    obs.captive.map(|b| b.to_string()).unwrap_or_default(),
                    obs.time_observed_at.clone().unwrap_or_default(),
                    obs.observed_on_string.clone().unwrap_or_default(),
                    obs.place_guess.clone().unwrap_or_default(),
                ];
                updates.insert(id.to_string(), row);
            }
        }
        rate_limiter.wait_for_next_request().await;
    }

    // Merge into existing CSV
    let tmp = tempfile::NamedTempFile::new()?;
    merge_csv(
        std::path::Path::new(csv_path),
        tmp.path(),
        &updates,
        id_col_idx,
    )?;
    std::fs::copy(tmp.path(), csv_path)?;

    println!("Updated {} observations in {csv_path}", updates.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_fetch_params_from_url_sets_taxon_id() {
        let p = build_fetch_params(&FetchObservationsOptions {
            url: Some("https://www.inaturalist.org/observations?taxon_id=47790".to_string()),
            ..Default::default()
        });
        assert_eq!(p.taxon_id, Some(vec!["47790".to_string()]));
    }

    #[test]
    fn test_build_fetch_params_from_url_drops_any() {
        let p = build_fetch_params(&FetchObservationsOptions {
            url: Some("https://www.inaturalist.org/observations?taxon_id=47790&place_id=any".to_string()),
            ..Default::default()
        });
        assert_eq!(p.taxon_id, Some(vec!["47790".to_string()]));
        assert_eq!(p.place_id, None);
    }

    #[test]
    fn test_build_fetch_params_from_fields_uses_build_params() {
        let p = build_fetch_params(&FetchObservationsOptions {
            taxon: Some("47790".to_string()),
            place_id: Some(1),
            ..Default::default()
        });
        assert_eq!(p.taxon_id, Some(vec!["47790".to_string()]));
        assert_eq!(p.place_id, Some(vec![1i32]));
    }

    #[test]
    fn test_build_fetch_params_url_without_scheme_still_works() {
        let p = build_fetch_params(&FetchObservationsOptions {
            url: Some("taxon_id=47790&user_id=kueda".to_string()),
            ..Default::default()
        });
        assert_eq!(p.taxon_id, Some(vec!["47790".to_string()]));
        assert_eq!(p.user_id, Some(vec!["kueda".to_string()]));
    }
}

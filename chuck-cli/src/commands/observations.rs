use tokio::sync::mpsc;
use inaturalist::models::ObservationsResponse;
use crate::output::{CsvOutput, DarwinCoreOutput, ObservationWriter};
use chuck_core::api::{client, params::{build_params, extract_criteria}, rate_limiter::get_rate_limiter};
use crate::progress::ProgressManager;
use chuck_core::darwin_core::Metadata;

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

pub async fn fetch_observations(
    file: Option<String>,
    taxon: Option<String>,
    place_id: Option<i32>,
    user: Option<String>,
    d1: Option<String>,
    d2: Option<String>,
    created_d1: Option<String>,
    created_d2: Option<String>,
    fetch_photos: bool,
    format: crate::OutputFormat,
    dwc_extensions: Vec<crate::DwcExtension>,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = client::get_config().await;
    let params = build_params(taxon, place_id, user, d1, d2, created_d1, created_d2);

    let show_progress = file.is_some();
    let progress_manager = ProgressManager::new(show_progress, fetch_photos);

    // Create channel for sending observations from fetcher to writer
    let (tx, rx) = mpsc::channel::<ObservationsResponse>(10);

    // Clone progress manager for the writer task
    let progress_manager_clone = ProgressManager {
        multi: progress_manager.multi.clone(),
        observations_bar: progress_manager.observations_bar.clone(),
        photos_bar: progress_manager.photos_bar.clone(),
    };

    // Spawn writer task based on format
    let writer_handle = match format {
        crate::OutputFormat::Csv => {
            let writer = CsvOutput::new(file).unwrap();
            spawn_observation_write_task(writer, rx, progress_manager_clone)
        }
        crate::OutputFormat::Dwc => {
            let output_path = file.unwrap_or_else(|| "observations.zip".to_string());

            // Extract criteria from params for use in metadata text description
            let mut abstract_lines = vec![
                "Observations exported from iNaturalist using the following criteria:".to_string()
            ];
            let param_criteria = extract_criteria(&params);
            for criterion in param_criteria {
                abstract_lines.push(format!("* {}", criterion));
            }
            if fetch_photos {
                abstract_lines.push("* Photos downloaded and included in archive".to_string());
            }

            let metadata = Metadata { abstract_lines };
            let core_extensions: Vec<chuck_core::DwcaExtension> = dwc_extensions.iter().map(|e| e.clone().into()).collect();
            let writer = DarwinCoreOutput::new(output_path, core_extensions, fetch_photos, metadata).unwrap();
            spawn_observation_write_task(writer, rx, progress_manager_clone)
        }
    };

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
                    eprintln!("API request failed: {}", e);
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

    Ok(())
}

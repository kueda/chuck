use inaturalist::models::{Observation, ShowTaxon};
use inaturalist::apis::taxa_api;
use chuck_core::darwin_core::{
    ArchiveBuilder,
    Audiovisual,
    Identification,
    Metadata,
    Multimedia,
    Occurrence,
    PhotoDownloader,
};
use super::ObservationWriter;
use crate::progress::ProgressManager;
use chuck_core::api::{client::get_config, rate_limiter::get_rate_limiter};
use std::collections::{HashMap, HashSet};
use tokio::time::{sleep, Duration};

/// Handles DarwinCore ArchiveBuilder output for observations
pub struct DarwinCoreOutput {
    archive: Option<ArchiveBuilder>,
    output_path: String,
    dwc_extensions: Vec<chuck_core::DwcExtension>,
    fetch_photos: bool,
}

impl DarwinCoreOutput {
    /// Create a new DarwinCore writer
    pub fn new(
        output_path: String,
        dwc_extensions: Vec<chuck_core::DwcExtension>,
        fetch_photos: bool,
        metadata: Metadata
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let archive = ArchiveBuilder::new(dwc_extensions.clone(), metadata)?;

        Ok(Self {
            archive: Some(archive),
            output_path,
            dwc_extensions,
            fetch_photos,
        })
    }
}

impl ObservationWriter for DarwinCoreOutput {
    fn write_observations(
        &mut self,
        observations: &[Observation],
        progress_manager: &ProgressManager,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn std::error::Error>>> + Send {
        async move {
            if let Some(archive) = &mut self.archive {
                // Collect unique taxon IDs from ancestor_ids for taxonomic hierarchy fetching
                let mut all_taxon_ids = HashSet::new();
                for obs in observations {
                    // Collect from observation taxon
                    if let Some(taxon) = &obs.taxon {
                        if let Some(ancestor_ids) = &taxon.ancestor_ids {
                            all_taxon_ids.extend(ancestor_ids.iter());
                        }
                    }
                    // Collect from identification taxa
                    if let Some(identifications) = &obs.identifications {
                        for identification in identifications {
                            if let Some(taxon) = &identification.taxon {
                                if let Some(ancestor_ids) = &taxon.ancestor_ids {
                                    all_taxon_ids.extend(ancestor_ids.iter());
                                }
                            }
                        }
                    }
                }

                // Fetch taxa in chunks of 500 to populate taxonomic hierarchy
                let taxon_ids_vec: Vec<i32> = all_taxon_ids.into_iter().collect();
                let taxa_hash = fetch_taxa_in_chunks(&taxon_ids_vec).await?;

                // Convert iNaturalist observations to DarwinCore occurrences.
                // Do this first so the obs progress bar updates first
                let occurrences: Vec<Occurrence> = observations
                    .iter()
                    .map(|obs| Occurrence::from((obs, &taxa_hash)))
                    .collect();

                archive.add_occurrences(&occurrences).await?;
                progress_manager.observations_bar.inc(observations.len() as u64);

                // Download photos if fetch_photos is enabled
                let photo_mapping = if self.fetch_photos {
                    // Prepare progress bar for photo downloads
                    let photos_count = observations.iter()
                        .filter_map(|o| o.photos.as_ref())
                        .flatten()
                        .count();
                    progress_manager.prepare_photos_inc(photos_count as u64);

                    // Create callback to increment photos progress bar
                    let photos_bar = progress_manager.photos_bar.clone();
                    let progress_callback = move |count: usize| {
                        if let Some(ref bar) = photos_bar {
                            bar.inc(count as u64);
                        }
                    };

                    PhotoDownloader::fetch_photos_to_dir(observations, archive.media_dir(), progress_callback).await?
                } else {
                    HashMap::new()
                };

                // Convert observation photos to multimedia records if SimpleMultimedia extension is enabled
                if self.dwc_extensions.contains(&chuck_core::DwcExtension::SimpleMultimedia) {
                    let multimedia: Vec<Multimedia> = observations
                        .iter()
                        .filter_map(|obs| {
                            let occurrence_id = obs.id.map(|id| format!("{}", id))?;
                            Some(obs.photos.as_ref()?.iter().map(|photo| {
                                Multimedia::from((photo, occurrence_id.as_str(), obs.user.as_deref(), &photo_mapping))
                            }).collect::<Vec<_>>())
                        })
                        .flatten()
                        .collect();

                    if !multimedia.is_empty() {
                        archive.add_multimedia(&multimedia).await?;
                    }
                }

                // Convert observation photos to audiovisual records if Audiovisual extension is enabled
                if self.dwc_extensions.contains(&chuck_core::DwcExtension::Audiovisual) {
                    let audiovisual: Vec<Audiovisual> = observations
                        .iter()
                        .filter_map(|obs| {
                            let occurrence_id = obs.id.map(|id| format!("{}", id))?;
                            Some(obs.photos.as_ref()?.iter().map(|photo| {
                                Audiovisual::from((photo, occurrence_id.as_str(), obs, &photo_mapping))
                            }).collect::<Vec<_>>())
                        })
                        .flatten()
                        .collect();

                    if !audiovisual.is_empty() {
                        archive.add_audiovisual(&audiovisual).await?;
                    }
                }

                // Convert observation identifications to identification records if Identifications extension is enabled
                if self.dwc_extensions.contains(&chuck_core::DwcExtension::Identifications) {
                    let identifications: Vec<Identification> = observations
                        .iter()
                        .filter_map(|obs| {
                            let occurrence_id = obs.id.map(|id| format!("{}", id))?;
                            Some(obs.identifications.as_ref()?.iter().map(|identification| {
                                Identification::from((identification, occurrence_id.as_str(), &taxa_hash))
                            }).collect::<Vec<_>>())
                        })
                        .flatten()
                        .collect();

                    if !identifications.is_empty() {
                        archive.add_identifications(&identifications).await?;
                    }
                }
            }

            Ok(())
        }
    }

    fn finalize(&mut self) -> impl std::future::Future<Output = Result<(), Box<dyn std::error::Error>>> + Send {
        let output_path = self.output_path.clone();
        let archive = self.archive.take();
        async move {
            if let Some(archive) = archive {
                archive.build(&output_path).await?;
            }
            Ok(())
        }
    }
}

/// Fetch taxa in chunks of 500 with exponential backoff retry logic
async fn fetch_taxa_in_chunks(taxon_ids: &[i32]) -> Result<HashMap<i32, ShowTaxon>, Box<dyn std::error::Error>> {
    let mut taxa_hash = HashMap::new();
    let config = get_config().await;
    let rate_limiter = get_rate_limiter().await;

    for chunk in taxon_ids.chunks(500) {
        // Rate limiting: coordinate with other API requests
        if !taxa_hash.is_empty() {
            rate_limiter.wait_for_next_request().await;
        }

        let params = taxa_api::TaxaGetParams {
            q: None,
            is_active: None,
            id: Some(chunk.to_vec()),
            parent_id: None,
            rank: None,
            rank_level: None,
            id_above: None,
            id_below: None,
            per_page: Some("500".to_string()),
            locale: None,
            preferred_place_id: None,
            only_id: None,
            all_names: None,
            order: None,
            order_by: None,
        };

        // Retry with exponential backoff (3 attempts total)
        let mut attempt = 0;
        let response = loop {
            attempt += 1;
            let config_read = config.read().await;
            match taxa_api::taxa_get(&*config_read, params.clone()).await {
                Ok(response) => break response,
                Err(e) => {
                    if attempt >= 3 {
                        return Err(format!("Failed to fetch taxa after 3 attempts: {}", e).into());
                    }
                    let backoff_ms = 1000 * (2_u64.pow(attempt - 1));
                    eprintln!("Taxa API request failed (attempt {}), retrying in {}ms: {}", attempt, backoff_ms, e);
                    sleep(Duration::from_millis(backoff_ms)).await;
                }
            }
        };

        for taxon in response.results {
            if let Some(id) = taxon.id {
                taxa_hash.insert(id, taxon);
            }
        }
    }

    Ok(taxa_hash)
}

// Map iNaturalist observation to a DarwinCore occurrence with taxonomic hierarchy

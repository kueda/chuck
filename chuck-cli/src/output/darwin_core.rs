use inaturalist::models::Observation;
use chuck_core::darwin_core::{
    ArchiveBuilder,
    Audiovisual,
    Identification,
    Metadata,
    Multimedia,
    Occurrence,
    PhotoDownloader,
    collect_taxon_ids,
    fetch_taxa_for_observations,
};
use super::ObservationWriter;
use crate::progress::ProgressManager;
use std::collections::HashMap;

/// Handles DarwinCore ArchiveBuilder output for observations
pub struct DarwinCoreOutput {
    archive: Option<ArchiveBuilder>,
    output_path: String,
    dwc_extensions: Vec<chuck_core::DwcaExtension>,
    fetch_photos: bool,
}

impl DarwinCoreOutput {
    /// Create a new DarwinCore writer
    pub fn new(
        output_path: String,
        dwc_extensions: Vec<chuck_core::DwcaExtension>,
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
                let taxon_ids = collect_taxon_ids(observations);
                let taxa_hash = fetch_taxa_for_observations(&taxon_ids, None::<fn(usize, usize)>).await?;

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
                if self.dwc_extensions.contains(&chuck_core::DwcaExtension::SimpleMultimedia) {
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
                if self.dwc_extensions.contains(&chuck_core::DwcaExtension::Audiovisual) {
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
                if self.dwc_extensions.contains(&chuck_core::DwcaExtension::Identifications) {
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

use futures::StreamExt;
use inaturalist::models::Observation;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Semaphore;
use crate::progress::ProgressManager;

pub struct PhotoDownloader;

impl PhotoDownloader {
    const MAX_RETRIES: usize = 3;
    const RETRY_BASE_DELAY: std::time::Duration = std::time::Duration::from_secs(2);

    /// Creates a date-based subdirectory path from observation date
    fn create_date_subdir(base_dir: &Path, observed_on: &Option<String>) -> PathBuf {
        match observed_on {
            Some(date_str) => {
                // Parse date string (format: "2024-01-15" or similar)
                if let Some(date_parts) = date_str.split('-').collect::<Vec<_>>().get(0..3) {
                    let year = date_parts[0];
                    let month = date_parts[1];
                    let day = date_parts[2];
                    base_dir.join(year).join(month).join(day)
                } else {
                    // Fallback to "unknown" if date parsing fails
                    base_dir.join("unknown")
                }
            }
            None => base_dir.join("unknown")
        }
    }

    async fn download_photo_with_retry(
        photo_url: String,
        file_path: PathBuf,
        photo_id: i32,
    ) -> Result<(), String> {
        let mut last_error = None;

        for attempt in 1..=Self::MAX_RETRIES {
            match Self::download_photo(&photo_url, &file_path).await {
                Ok(()) => return Ok(()),
                Err(error_msg) => {
                    last_error = Some(error_msg.clone());
                    if attempt < Self::MAX_RETRIES {
                        let delay = Self::RETRY_BASE_DELAY * (2_u32.pow(attempt as u32 - 1));
                        eprintln!("Download attempt {} failed for photo {}: {}. Retrying in {:?}...",
                                 attempt, photo_id, error_msg, delay);
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap())
    }

    async fn download_photo(
        photo_url: &str,
        file_path: &Path
    ) -> Result<(), String> {
        let tmp_file = tokio::fs::File::create(file_path).await
            .map_err(|e| format!("Failed to create file: {}", e))?;
        let response = reqwest::get(photo_url).await
            .map_err(|e| {
                let status = if let Some(status) = e.status() {
                    status.to_string()
                } else {
                    "unknown".to_string()
                };
                format!("Failed to fetch URL ({}): {}", status, e)
            })?;
        let mut byte_stream = response.bytes_stream();

        let mut tmp_file = tmp_file;
        while let Some(item) = byte_stream.next().await {
            let bytes = item.map_err(|e| format!("Failed to read response bytes: {}", e))?;
            tokio::io::copy(&mut bytes.as_ref(), &mut tmp_file).await
                .map_err(|e| format!("Failed to write to file: {}", e))?;
        }

        Ok(())
    }
    /// Downloads photos to a specific directory and returns a mapping of photo ID to filename
    pub async fn fetch_photos_to_dir(
        observations: &[Observation],
        output_dir: &Path,
        progress_manager: &ProgressManager,
    ) -> Result<HashMap<i32, String>, Box<dyn std::error::Error>> {
        // Create a mapping from photo to observation date for subdirectory organization
        let mut photo_to_date = HashMap::new();
        for obs in observations {
            if let Some(photos) = &obs.photos {
                for photo in photos {
                    if let Some(photo_id) = photo.id {
                        photo_to_date.insert(photo_id, obs.observed_on.clone());
                    }
                }
            }
        }

        let photos = observations.iter()
            .filter_map(|o| o.photos.as_ref())
            .flatten()
            .cloned()
            .collect::<Vec<_>>();

        // Prepare progress bar for photo downloads
        progress_manager.prepare_photos_inc(photos.len() as u64);

        let mut photo_mapping = HashMap::new();

        // Limit concurrent downloads to prevent "too many open files"
        let semaphore = Arc::new(Semaphore::new(20));

        let tasks: Vec<_> = photos.iter().map(|photo| {
            let photo = photo.clone();
            let photos_bar = progress_manager.photos_bar.clone();
            let output_dir = output_dir.to_path_buf();
            let semaphore = semaphore.clone();
            let photo_to_date = photo_to_date.clone();

            tokio::spawn(async move {
                let mut result = None;
                if let (Some(url), Some(id)) = (&photo.url, &photo.id) {
                    // Acquire permit before opening file
                    let _permit = semaphore.acquire().await.unwrap();

                    let photo_url = url.replace("square", "original");
                    let filename = format!("{}.jpg", id);

                    // Create date-based subdirectory. The intent is to create
                    // a human-readable directory structure that does not
                    // result in directories with too many files. One
                    // consequence of this is that photos associated with
                    // multiple observations will be written twice. Since
                    // this is rare, it seems worth it.
                    let observed_on = photo_to_date.get(id).unwrap_or(&None);
                    let date_subdir = Self::create_date_subdir(&output_dir, observed_on);

                    // Create the subdirectory if it doesn't exist
                    if let Err(e) = tokio::fs::create_dir_all(&date_subdir).await {
                        eprintln!("Failed to create directory {}: {}", date_subdir.display(), e);
                        if let Some(ref bar) = photos_bar {
                            bar.inc(1);
                        }
                        return None;
                    }

                    let file_path = date_subdir.join(&filename);

                    // Get the relative path for use in the hash map
                    let cmp_count = file_path.components().count();
                    let rel_path: PathBuf = file_path
                        .clone()
                        .components()
                        .skip(cmp_count.saturating_sub(4)) // Skip more components due to YYYY/MM/DD subdirs
                        .collect();

                    match Self::download_photo_with_retry(photo_url, file_path, *id).await {
                        Ok(()) => {
                            result = Some((
                                *id,
                                rel_path.to_string_lossy().to_string()
                            ));
                        }
                        Err(e) => eprintln!("Failed to download photo {} after {} retries: {}", id, Self::MAX_RETRIES, e),
                    }
                    // Permit is automatically released when _permit goes out of scope
                }
                if let Some(ref bar) = photos_bar {
                    bar.inc(1);
                }
                result
            })
        }).collect();

        let results = futures::future::join_all(tasks).await;

        for result in results {
            if let Ok(Some((photo_id, filename))) = result {
                photo_mapping.insert(photo_id, filename);
            }
        }

        Ok(photo_mapping)
    }
}

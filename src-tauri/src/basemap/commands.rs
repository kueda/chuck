use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};

use futures::stream::{self, StreamExt};
use pmtiles::{
    AsyncPmTilesReader, HashMapCache, HttpBackend, PmTilesWriter,
    TileCoord, TileType,
};
use serde::Serialize;
use tauri::{Emitter, Manager};

use super::protocol;

const PLANET_PMTILES_BASE: &str = "https://build.protomaps.com";

/// Discover the latest available Protomaps daily build URL.
/// Tries yesterday through 7 days ago, returns the first that responds 200.
async fn discover_planet_url() -> Result<String, String> {
    let client = pmtiles::reqwest::Client::builder()
        .user_agent("Chuck/0.1")
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    let today = chrono::Utc::now().date_naive();
    for days_ago in 1..=7 {
        let date = today - chrono::Duration::days(days_ago);
        let url = format!(
            "{}/{}.pmtiles",
            PLANET_PMTILES_BASE,
            date.format("%Y%m%d")
        );
        log::debug!("pmtiles url: {url}");
        match client.head(&url).send().await {
            Ok(resp) if resp.status().is_success() => return Ok(url),
            err => {
                log::error!("head failed: {err:?}");
                continue
            },
        }
    }
    Err("No recent Protomaps build found (tried last 7 days)".into())
}

static CANCEL_FLAG: LazyLock<Arc<AtomicBool>> =
    LazyLock::new(|| Arc::new(AtomicBool::new(false)));

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BasemapStatus {
    pub downloaded: bool,
    pub max_zoom: Option<u8>,
    pub file_size: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub tiles_downloaded: u64,
    pub tiles_total: u64,
    pub bytes_downloaded: u64,
    pub phase: String,
}

fn basemap_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    protocol::basemap_path(app)
}

fn basemap_metadata_path(
    app: &tauri::AppHandle,
) -> Result<PathBuf, String> {
    let base_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?;
    Ok(base_dir.join("basemap_metadata.json"))
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct BasemapMetadata {
    max_zoom: u8,
    download_date: String,
    source_url: String,
}

#[tauri::command]
pub async fn get_basemap_status(
    app: tauri::AppHandle,
) -> Result<BasemapStatus, String> {
    let path = basemap_path(&app)?;
    if !path.exists() {
        return Ok(BasemapStatus {
            downloaded: false,
            max_zoom: None,
            file_size: None,
        });
    }

    let file_size = std::fs::metadata(&path).map(|m| m.len()).ok();

    let max_zoom = match basemap_metadata_path(&app) {
        Ok(meta_path) if meta_path.exists() => {
            std::fs::read_to_string(&meta_path)
                .ok()
                .and_then(|s| {
                    serde_json::from_str::<BasemapMetadata>(&s).ok()
                })
                .map(|m| m.max_zoom)
        }
        _ => None,
    };

    Ok(BasemapStatus {
        downloaded: true,
        max_zoom,
        file_size,
    })
}

/// Count total tiles across zoom levels 0 through max_zoom.
fn count_tiles(max_zoom: u8) -> u64 {
    (0..=max_zoom as u32).map(|z| 4u64.pow(z)).sum()
}

#[tauri::command]
pub async fn download_basemap(
    app: tauri::AppHandle,
    max_zoom: u8,
) -> Result<(), String> {
    if max_zoom > 15 {
        return Err("Max zoom cannot exceed 15".to_string());
    }

    CANCEL_FLAG.store(false, Ordering::SeqCst);

    let path = basemap_path(&app)?;
    let meta_path = basemap_metadata_path(&app)?;

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory: {e}"))?;
    }

    // Use a temporary path during download
    let tmp_path = path.with_extension("pmtiles.tmp");

    let cancel_flag = CANCEL_FLAG.clone();
    let tiles_total = count_tiles(max_zoom);

    app.emit(
        "basemap-download-progress",
        DownloadProgress {
            tiles_downloaded: 0,
            tiles_total,
            bytes_downloaded: 0,
            phase: "connecting".to_string(),
        },
    )
    .ok();

    // Discover the latest available Protomaps build
    let planet_url = discover_planet_url().await?;
    log::debug!("got planet_url: {planet_url}");

    // Open remote PMTiles file using the pmtiles crate's re-exported reqwest
    let client = pmtiles::reqwest::Client::builder()
        .user_agent("Chuck/0.1")
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    let backend =
        HttpBackend::try_from(client, &planet_url)
            .map_err(|e| {
                format!("Failed to connect to remote PMTiles: {e}")
            })?;
    let remote_reader = AsyncPmTilesReader::try_from_cached_source(
        backend,
        HashMapCache::default(),
    )
    .await
    .map_err(|e| {
        format!("Failed to read remote PMTiles header: {e}")
    })?;
    let remote_reader = Arc::new(remote_reader);

    // Create local writer
    let output_file = std::fs::File::create(&tmp_path)
        .map_err(|e| format!("Failed to create output file: {e}"))?;
    let mut stream_writer = PmTilesWriter::new(TileType::Mvt)
        .max_zoom(max_zoom)
        .create(output_file)
        .map_err(|e| format!("Failed to create PMTiles writer: {e}"))?;

    let mut tiles_downloaded: u64 = 0;
    let mut bytes_downloaded: u64 = 0;

    app.emit(
        "basemap-download-progress",
        DownloadProgress {
            tiles_downloaded: 0,
            tiles_total,
            bytes_downloaded: 0,
            phase: "downloading".to_string(),
        },
    )
    .ok();

    // Build list of all tile coordinates up to max_zoom
    let mut coords = Vec::with_capacity(tiles_total as usize);
    for z in 0..=max_zoom {
        let num_tiles = 1u32 << z;
        for x in 0..num_tiles {
            for y in 0..num_tiles {
                let coord =
                    TileCoord::new(z, x, y).map_err(|e| {
                        format!("Invalid tile coord: {e}")
                    })?;
                coords.push(coord);
            }
        }
    }

    // Fetch tiles concurrently in batches
    const CONCURRENCY: usize = 32;
    let cancel = cancel_flag.clone();
    let reader = remote_reader.clone();
    let mut tile_stream = stream::iter(coords)
        .map(move |coord| {
            let reader = reader.clone();
            async move {
                (coord, reader.get_tile_decompressed(coord).await)
            }
        })
        .buffer_unordered(CONCURRENCY);

    while let Some((coord, result)) = tile_stream.next().await {
        if cancel.load(Ordering::SeqCst) {
            drop(stream_writer);
            std::fs::remove_file(&tmp_path).ok();
            return Err("Download cancelled".to_string());
        }

        match result {
            Ok(Some(tile_data)) => {
                bytes_downloaded += tile_data.len() as u64;
                stream_writer
                    .add_tile(coord, &tile_data)
                    .map_err(|e| {
                        format!("Failed to write tile: {e}")
                    })?;
            }
            Ok(None) => {
                // No tile at this coordinate (e.g., ocean)
            }
            Err(e) => {
                log::warn!("Failed to fetch tile {coord:?}: {e}");
            }
        }

        tiles_downloaded += 1;

        // Emit progress every 100 tiles or on first tile
        if tiles_downloaded % 100 == 0 || tiles_downloaded == 1 {
            app.emit(
                "basemap-download-progress",
                DownloadProgress {
                    tiles_downloaded,
                    tiles_total,
                    bytes_downloaded,
                    phase: "downloading".to_string(),
                },
            )
            .ok();
        }
    }

    // Finalize the PMTiles file
    app.emit(
        "basemap-download-progress",
        DownloadProgress {
            tiles_downloaded,
            tiles_total,
            bytes_downloaded,
            phase: "finalizing".to_string(),
        },
    )
    .ok();

    stream_writer
        .finalize()
        .map_err(|e| format!("Failed to finalize PMTiles file: {e}"))?;

    // Move temp file to final location
    std::fs::rename(&tmp_path, &path)
        .map_err(|e| format!("Failed to move basemap file: {e}"))?;

    // Save metadata
    let metadata = BasemapMetadata {
        max_zoom,
        download_date: chrono::Utc::now().to_rfc3339(),
        source_url: planet_url.clone(),
    };
    let meta_json = serde_json::to_string_pretty(&metadata)
        .map_err(|e| format!("Failed to serialize metadata: {e}"))?;
    std::fs::write(&meta_path, meta_json)
        .map_err(|e| format!("Failed to write metadata: {e}"))?;

    // Reset the reader cache so next tile request picks up the new file
    protocol::reset_reader_cache().await;

    app.emit(
        "basemap-download-progress",
        DownloadProgress {
            tiles_downloaded,
            tiles_total,
            bytes_downloaded,
            phase: "complete".to_string(),
        },
    )
    .ok();

    Ok(())
}

#[tauri::command]
pub fn cancel_basemap_download() -> Result<(), String> {
    CANCEL_FLAG.store(true, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
pub async fn delete_basemap(
    app: tauri::AppHandle,
) -> Result<(), String> {
    let path = basemap_path(&app)?;
    if path.exists() {
        // Reset reader cache first so no file handles remain
        protocol::reset_reader_cache().await;
        std::fs::remove_file(&path)
            .map_err(|e| format!("Failed to delete basemap: {e}"))?;
    }

    let meta_path = basemap_metadata_path(&app)?;
    if meta_path.exists() {
        std::fs::remove_file(&meta_path).ok();
    }

    Ok(())
}

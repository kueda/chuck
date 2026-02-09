use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};

use bytes::{Bytes, BytesMut};
use futures::stream::{self, StreamExt};
use pmtiles::reqwest::header::{HeaderValue, RANGE};
use pmtiles::reqwest::{Method, Request, StatusCode};
use pmtiles::{
    AsyncBackend, AsyncPmTilesReader, HashMapCache, PmTilesWriter,
    PmtError, PmtResult, TileCoord,
};
use serde::Serialize;
use tauri::{Emitter, Manager};
use tokio::sync::{OnceCell, RwLock};
use url::Url;

use super::protocol;

/// HTTP backend that downloads data in large fixed-size chunks (4 MB) and
/// caches them in memory. Nearby reads (e.g. clustered tiles) hit the same
/// cached chunk, dramatically reducing the number of HTTP round-trips
/// compared to one range request per tile.
struct ChunkedHttpBackend {
    client: pmtiles::reqwest::Client,
    url: Url,
    chunks: RwLock<HashMap<usize, Arc<OnceCell<Bytes>>>>,
    file_size: usize,
}

const CHUNK_SIZE: usize = 4 * 1024 * 1024; // 4 MB

impl ChunkedHttpBackend {
    /// Create a new chunked backend. Issues a HEAD request to determine
    /// the remote file size (needed to clamp the last chunk).
    async fn try_new(
        client: pmtiles::reqwest::Client,
        url: &str,
    ) -> PmtResult<Self> {
        let url: Url = url.parse().map_err(|_| {
            PmtError::Reading(std::io::Error::other(
                format!("Invalid URL: {url}"),
            ))
        })?;

        let response = client
            .head(url.clone())
            .send()
            .await?
            .error_for_status()?;

        let file_size = response
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<usize>().ok())
            .ok_or_else(|| {
                PmtError::Reading(std::io::Error::other(
                    "Server did not return Content-Length",
                ))
            })?;

        Ok(Self {
            client,
            url,
            chunks: RwLock::new(HashMap::new()),
            file_size,
        })
    }

    /// Get or download a single chunk. Uses OnceCell to ensure each chunk
    /// is downloaded exactly once even under concurrent access.
    async fn get_chunk(&self, chunk_idx: usize) -> PmtResult<Bytes> {
        // Fast path: check if the OnceCell already exists
        let cell = {
            let guard = self.chunks.read().await;
            guard.get(&chunk_idx).cloned()
        };

        let cell = match cell {
            Some(c) => c,
            None => {
                let mut guard = self.chunks.write().await;
                guard
                    .entry(chunk_idx)
                    .or_insert_with(|| Arc::new(OnceCell::new()))
                    .clone()
            }
        };

        cell.get_or_try_init(|| async {
            let start = chunk_idx * CHUNK_SIZE;
            let end =
                (start + CHUNK_SIZE).min(self.file_size).saturating_sub(1);
            let range = format!("bytes={start}-{end}");
            let range = HeaderValue::try_from(range)?;

            let mut req = Request::new(Method::GET, self.url.clone());
            req.headers_mut().insert(RANGE, range);

            let response =
                self.client.execute(req).await?.error_for_status()?;
            if response.status() != StatusCode::PARTIAL_CONTENT {
                return Err(PmtError::RangeRequestsUnsupported);
            }
            Ok(response.bytes().await?)
        })
        .await
        .cloned()
    }
}

impl AsyncBackend for ChunkedHttpBackend {
    async fn read(
        &self,
        offset: usize,
        length: usize,
    ) -> PmtResult<Bytes> {
        let start_chunk = offset / CHUNK_SIZE;
        let end_chunk = (offset + length - 1) / CHUNK_SIZE;

        if start_chunk == end_chunk {
            // Common case: read fits within a single chunk
            let chunk_data = self.get_chunk(start_chunk).await?;
            let local_offset = offset - start_chunk * CHUNK_SIZE;
            let available =
                chunk_data.len().saturating_sub(local_offset).min(length);
            Ok(chunk_data.slice(
                local_offset..local_offset + available,
            ))
        } else {
            // Read spans multiple chunks — assemble from each
            let mut buf = BytesMut::with_capacity(length);
            for idx in start_chunk..=end_chunk {
                let chunk_data = self.get_chunk(idx).await?;
                let chunk_start = idx * CHUNK_SIZE;
                let local_start = if idx == start_chunk {
                    offset - chunk_start
                } else {
                    0
                };
                let local_end = if idx == end_chunk {
                    (offset + length) - chunk_start
                } else {
                    chunk_data.len()
                };
                let local_end = local_end.min(chunk_data.len());
                buf.extend_from_slice(&chunk_data[local_start..local_end]);
            }
            Ok(buf.freeze())
        }
    }
}

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
        ChunkedHttpBackend::try_new(client, &planet_url)
            .await
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

    // Create local writer, matching source compression so we can
    // copy raw compressed tile bytes without re-encoding.
    let source_header = remote_reader.get_header();
    let source_metadata = remote_reader.get_metadata().await
        .map_err(|e| format!("Failed to read metadata: {e}"))?;
    let output_file = std::fs::File::create(&tmp_path)
        .map_err(|e| format!("Failed to create output file: {e}"))?;
    let mut stream_writer =
        PmTilesWriter::new(source_header.tile_type)
            .tile_compression(source_header.tile_compression)
            .max_zoom(max_zoom)
            .metadata(&source_metadata)
            .create(output_file)
            .map_err(|e| {
                format!("Failed to create PMTiles writer: {e}")
            })?;

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

    // High concurrency because the ChunkedHttpBackend deduplicates chunk
    // downloads via OnceCell. With ~87 tiles per 4 MB chunk, 1024 in-flight
    // lookups span ~12 chunks, so for typical downloads (zoom 6 ≈ 13 chunks)
    // nearly all chunks begin downloading immediately. Raw byte copying
    // (no decompression) means tile processing is fast enough that the
    // bottleneck is purely network I/O.
    const CONCURRENCY: usize = 1024;
    let cancel = cancel_flag.clone();
    let reader = remote_reader.clone();
    let mut tile_stream = stream::iter(coords)
        .map(move |coord| {
            let reader = reader.clone();
            async move {
                (coord, reader.get_tile(coord).await)
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
                    .add_raw_tile(coord, &tile_data)
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

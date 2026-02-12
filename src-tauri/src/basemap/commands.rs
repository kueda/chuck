use std::collections::HashMap;
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
use tauri::Emitter;
use tokio::sync::{Mutex, OnceCell, RwLock};
use tokio::time::Instant;
use url::Url;

use super::protocol::{
    self, BasemapInfo, Bounds, IndexEntry,
};

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
pub struct DownloadProgress {
    pub tiles_downloaded: u64,
    pub tiles_total: u64,
    pub bytes_downloaded: u64,
    pub phase: String,
}

/// Open a remote PMTiles reader for the latest Protomaps build.
async fn open_remote_reader() -> Result<
    (
        Arc<AsyncPmTilesReader<ChunkedHttpBackend, HashMapCache>>,
        String,
    ),
    String,
> {
    let planet_url = discover_planet_url().await?;
    log::debug!("got planet_url: {planet_url}");

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
    let reader = AsyncPmTilesReader::try_from_cached_source(
        backend,
        HashMapCache::default(),
    )
    .await
    .map_err(|e| {
        format!("Failed to read remote PMTiles header: {e}")
    })?;

    Ok((Arc::new(reader), planet_url))
}

/// Download tiles from a remote reader into a local PMTiles file.
async fn download_tiles(
    app: &tauri::AppHandle,
    remote_reader: &Arc<
        AsyncPmTilesReader<ChunkedHttpBackend, HashMapCache>,
    >,
    coords: Vec<TileCoord>,
    tmp_path: &std::path::Path,
    final_path: &std::path::Path,
    writer_config: WriterConfig,
) -> Result<(u64, u64), String> {
    let source_header = remote_reader.get_header();
    let source_metadata = remote_reader.get_metadata().await
        .map_err(|e| format!("Failed to read metadata: {e}"))?;

    let output_file = std::fs::File::create(tmp_path)
        .map_err(|e| format!("Failed to create output file: {e}"))?;

    let mut builder = PmTilesWriter::new(source_header.tile_type)
        .tile_compression(source_header.tile_compression)
        .max_zoom(writer_config.max_zoom)
        .metadata(&source_metadata);

    if let Some(b) = &writer_config.bounds {
        builder = builder.bounds(
            b.min_lon, b.min_lat, b.max_lon, b.max_lat,
        );
    }

    let mut stream_writer =
        builder.create(output_file).map_err(|e| {
            format!("Failed to create PMTiles writer: {e}")
        })?;

    let tiles_total = coords.len() as u64;
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

    // High concurrency because the ChunkedHttpBackend deduplicates
    // chunk downloads via OnceCell.
    const CONCURRENCY: usize = 1024;
    let cancel = CANCEL_FLAG.clone();
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
            std::fs::remove_file(tmp_path).ok();
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
            Ok(None) => {}
            Err(e) => {
                log::warn!("Failed to fetch tile {coord:?}: {e}");
            }
        }

        tiles_downloaded += 1;

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
        .map_err(|e| format!("Failed to finalize PMTiles: {e}"))?;

    std::fs::rename(tmp_path, final_path)
        .map_err(|e| format!("Failed to move basemap file: {e}"))?;

    Ok((tiles_downloaded, bytes_downloaded))
}

struct WriterConfig {
    max_zoom: u8,
    bounds: Option<Bounds>,
}

/// Count total tiles across zoom levels 0 through max_zoom.
fn count_tiles(max_zoom: u8) -> u64 {
    (0..=max_zoom as u32).map(|z| 4u64.pow(z)).sum()
}

/// Enumerate all tile coordinates within a bounding box up to max_zoom.
fn tiles_in_bounds(
    bounds: &Bounds,
    max_zoom: u8,
) -> Vec<TileCoord> {
    let mut coords = Vec::new();
    for z in 0..=max_zoom {
        let n = (1u64 << z) as f64;

        let x_min = ((bounds.min_lon + 180.0) / 360.0 * n)
            .floor() as u32;
        let x_max = ((bounds.max_lon + 180.0) / 360.0 * n)
            .floor()
            .min(n - 1.0) as u32;

        let y_min = lat_to_tile_y(bounds.max_lat, z);
        let y_max = lat_to_tile_y(bounds.min_lat, z);

        for x in x_min..=x_max {
            for y in y_min..=y_max {
                if let Ok(coord) = TileCoord::new(z, x, y) {
                    coords.push(coord);
                }
            }
        }
    }
    coords
}

/// Convert latitude to tile Y coordinate at a given zoom level.
fn lat_to_tile_y(lat_deg: f64, zoom: u8) -> u32 {
    let n = (1u64 << zoom) as f64;
    let lat_rad = lat_deg.to_radians();
    let y = (1.0 - lat_rad.tan().asinh() / std::f64::consts::PI)
        / 2.0
        * n;
    (y.floor() as u32).min((n as u32).saturating_sub(1))
}

/// Update the index.json, adding or replacing an entry by id.
fn upsert_index_entry(
    app: &tauri::AppHandle,
    entry: IndexEntry,
) -> Result<(), String> {
    let mut entries = protocol::load_index(app)?;
    if let Some(existing) = entries.iter_mut().find(|e| e.id == entry.id)
    {
        *existing = entry;
    } else {
        entries.push(entry);
    }
    protocol::save_index(app, &entries)
}

/// Remove an entry from index.json by id.
fn remove_index_entry(
    app: &tauri::AppHandle,
    id: &str,
) -> Result<(), String> {
    let entries = protocol::load_index(app)?;
    let filtered: Vec<_> =
        entries.into_iter().filter(|e| e.id != id).collect();
    protocol::save_index(app, &filtered)
}

#[tauri::command]
pub async fn list_basemaps(
    app: tauri::AppHandle,
) -> Result<Vec<BasemapInfo>, String> {
    protocol::migrate_legacy_basemap(&app)?;
    protocol::list_basemaps(&app).await
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

    let dir = protocol::basemaps_dir(&app)?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create basemaps dir: {e}"))?;

    let path = dir.join("global.pmtiles");
    let tmp_path = dir.join("global.pmtiles.tmp");

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

    let (remote_reader, planet_url) = open_remote_reader().await?;

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

    let (tiles_downloaded, bytes_downloaded) = download_tiles(
        &app,
        &remote_reader,
        coords,
        &tmp_path,
        &path,
        WriterConfig { max_zoom, bounds: None },
    )
    .await?;

    // Update index
    upsert_index_entry(
        &app,
        IndexEntry {
            id: "global".into(),
            name: "Global".into(),
            download_date: chrono::Utc::now().to_rfc3339(),
            source_url: planet_url,
        },
    )?;

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
pub async fn download_regional_basemap(
    app: tauri::AppHandle,
    bounds: Bounds,
    max_zoom: u8,
    name: Option<String>,
) -> Result<(), String> {
    if max_zoom > 15 {
        return Err("Max zoom cannot exceed 15".to_string());
    }

    CANCEL_FLAG.store(false, Ordering::SeqCst);

    let dir = protocol::basemaps_dir(&app)?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create basemaps dir: {e}"))?;

    let id = uuid::Uuid::new_v4().to_string();
    let path = dir.join(format!("{id}.pmtiles"));
    let tmp_path = dir.join(format!("{id}.pmtiles.tmp"));

    let coords = tiles_in_bounds(&bounds, max_zoom);
    let tiles_total = coords.len() as u64;

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

    let (remote_reader, planet_url) = open_remote_reader().await?;

    let display_name = name.unwrap_or_else(|| {
        format!(
            "{:.1},{:.1} to {:.1},{:.1}",
            bounds.min_lat, bounds.min_lon,
            bounds.max_lat, bounds.max_lon,
        )
    });

    let (tiles_downloaded, bytes_downloaded) = download_tiles(
        &app,
        &remote_reader,
        coords,
        &tmp_path,
        &path,
        WriterConfig {
            max_zoom,
            bounds: Some(bounds),
        },
    )
    .await?;

    upsert_index_entry(
        &app,
        IndexEntry {
            id,
            name: display_name,
            download_date: chrono::Utc::now().to_rfc3339(),
            source_url: planet_url,
        },
    )?;

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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TileEstimate {
    pub tiles: u64,
}

#[tauri::command]
pub fn estimate_regional_tiles(
    bounds: Bounds,
    max_zoom: u8,
) -> Result<TileEstimate, String> {
    if max_zoom > 15 {
        return Err("Max zoom cannot exceed 15".to_string());
    }
    let count = tiles_in_bounds(&bounds, max_zoom).len() as u64;
    Ok(TileEstimate { tiles: count })
}

#[tauri::command]
pub fn cancel_basemap_download() -> Result<(), String> {
    CANCEL_FLAG.store(true, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
pub async fn delete_basemap(
    app: tauri::AppHandle,
    id: String,
) -> Result<(), String> {
    let dir = protocol::basemaps_dir(&app)?;
    let filename = if id == "global" {
        "global.pmtiles".to_string()
    } else {
        format!("{id}.pmtiles")
    };
    let path = dir.join(&filename);

    // Reset reader cache first so no file handles remain
    protocol::reset_reader_cache().await;

    if path.exists() {
        std::fs::remove_file(&path)
            .map_err(|e| format!("Failed to delete basemap: {e}"))?;
    }

    remove_index_entry(&app, &id)?;

    Ok(())
}

/// Rate-limit guard for Nominatim (max 1 request per second).
static NOMINATIM_LAST_REQUEST: LazyLock<Mutex<Instant>> =
    LazyLock::new(|| Mutex::new(Instant::now() - std::time::Duration::from_secs(1)));

/// Extract a concise place name from a Nominatim reverse-geocode response.
/// Prefers the `name` field; falls back to the first two components of
/// `display_name`.
fn extract_place_name(json: &serde_json::Value) -> Option<String> {
    if let Some(name) = json.get("name").and_then(|v| v.as_str()) {
        let name = name.trim();
        if !name.is_empty() {
            return Some(name.to_string());
        }
    }
    if let Some(display) =
        json.get("display_name").and_then(|v| v.as_str())
    {
        let parts: Vec<&str> =
            display.split(',').map(|s| s.trim()).collect();
        let meaningful: Vec<&str> =
            parts.into_iter().filter(|s| !s.is_empty()).collect();
        if !meaningful.is_empty() {
            let take = meaningful.len().min(2);
            return Some(meaningful[..take].join(", "));
        }
    }
    None
}

#[tauri::command]
pub async fn reverse_geocode(
    lat: f64,
    lon: f64,
    zoom: u8,
) -> Result<String, String> {
    // Enforce 1 req/sec rate limit
    {
        let mut last = NOMINATIM_LAST_REQUEST.lock().await;
        let elapsed = last.elapsed();
        let min_interval = std::time::Duration::from_secs(1);
        if elapsed < min_interval {
            tokio::time::sleep(min_interval - elapsed).await;
        }
        *last = Instant::now();
    }

    let client = reqwest::Client::builder()
        .user_agent(
            "Chuck/0.2 (https://github.com/kueda/chuck)",
        )
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    let url = format!(
        "https://nominatim.openstreetmap.org/reverse\
         ?format=json\
         &lat={lat}\
         &lon={lon}\
         &zoom={zoom}\
         &layer=natural,poi,address\
         &accept-language=en",
    );

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Nominatim request failed: {e}"))?;

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Invalid JSON from Nominatim: {e}"))?;

    extract_place_name(&json)
        .ok_or_else(|| "No place name found".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_tiles() {
        assert_eq!(count_tiles(0), 1);
        assert_eq!(count_tiles(1), 5);
        assert_eq!(count_tiles(2), 21);
        assert_eq!(count_tiles(6), 5461);
    }

    #[test]
    fn test_lat_to_tile_y() {
        // Zoom 0: whole world is one tile
        assert_eq!(lat_to_tile_y(85.0, 0), 0);
        assert_eq!(lat_to_tile_y(-85.0, 0), 0);

        // Zoom 1: 2x2 grid
        assert_eq!(lat_to_tile_y(45.0, 1), 0);
        assert_eq!(lat_to_tile_y(-45.0, 1), 1);
    }

    #[test]
    fn test_tiles_in_bounds() {
        // Small bbox at zoom 0 should yield 1 tile
        let bounds = Bounds {
            min_lon: -10.0,
            min_lat: -10.0,
            max_lon: 10.0,
            max_lat: 10.0,
        };
        let tiles = tiles_in_bounds(&bounds, 0);
        assert_eq!(tiles.len(), 1);

        // At zoom 1, a bbox around the equator/prime meridian
        // should cover 4 tiles (all quadrants)
        let tiles = tiles_in_bounds(&bounds, 1);
        // zoom 0: 1 tile, zoom 1: 4 tiles = 5 total
        assert_eq!(tiles.len(), 5);
    }

    #[test]
    fn test_extract_place_name_with_name() {
        let json = serde_json::json!({
            "name": "San Francisco",
            "display_name": "San Francisco, California, US"
        });
        assert_eq!(
            extract_place_name(&json),
            Some("San Francisco".to_string()),
        );
    }

    #[test]
    fn test_extract_place_name_without_name() {
        let json = serde_json::json!({
            "display_name": "Tokyo, Kanto, Japan"
        });
        assert_eq!(
            extract_place_name(&json),
            Some("Tokyo, Kanto".to_string()),
        );
    }

    #[test]
    fn test_extract_place_name_empty_name_falls_back() {
        let json = serde_json::json!({
            "name": "",
            "display_name": "Pacific Ocean, Earth"
        });
        assert_eq!(
            extract_place_name(&json),
            Some("Pacific Ocean, Earth".to_string()),
        );
    }

    #[test]
    fn test_extract_place_name_empty_json() {
        let json = serde_json::json!({});
        assert_eq!(extract_place_name(&json), None);
    }

    #[test]
    fn test_extract_place_name_single_component() {
        let json = serde_json::json!({
            "display_name": "Antarctica"
        });
        assert_eq!(
            extract_place_name(&json),
            Some("Antarctica".to_string()),
        );
    }

    #[test]
    fn test_tiles_in_bounds_regional() {
        // A small region (roughly San Francisco area)
        let bounds = Bounds {
            min_lon: -122.5,
            min_lat: 37.7,
            max_lon: -122.3,
            max_lat: 37.8,
        };
        let tiles = tiles_in_bounds(&bounds, 10);
        // Should have tiles at each zoom level
        assert!(tiles.len() > 10);
        // At zoom 10, SF is roughly 1-2 tiles wide
        let zoom_10_tiles: Vec<_> = tiles
            .iter()
            .filter(|t| t.z() == 10)
            .collect();
        assert!(
            !zoom_10_tiles.is_empty(),
            "Should have tiles at zoom 10"
        );
        assert!(
            zoom_10_tiles.len() <= 4,
            "Small area should have few tiles at zoom 10"
        );
    }
}

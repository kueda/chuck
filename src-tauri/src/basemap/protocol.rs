use std::path::PathBuf;
use std::sync::{Arc, LazyLock};

use pmtiles::{AsyncPmTilesReader, MmapBackend, NoCache};
use serde::{Deserialize, Serialize};
use tauri::{Manager, Runtime};
use tokio::sync::RwLock;

pub type BasemapReader = AsyncPmTilesReader<MmapBackend, NoCache>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bounds {
    pub min_lon: f64,
    pub min_lat: f64,
    pub max_lon: f64,
    pub max_lat: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BasemapInfo {
    pub id: String,
    pub name: String,
    pub max_zoom: u8,
    pub bounds: Option<Bounds>,
    pub download_date: String,
    pub source_url: String,
    pub file_size: u64,
}

/// Entry stored in index.json — only fields not in PMTiles headers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexEntry {
    pub id: String,
    pub name: String,
    pub download_date: String,
    pub source_url: String,
}

struct CachedReader {
    info: BasemapInfo,
    reader: Arc<BasemapReader>,
}

static READERS: LazyLock<RwLock<Option<Vec<CachedReader>>>> =
    LazyLock::new(|| RwLock::new(None));

/// Path to the basemaps directory.
pub fn basemaps_dir<R: Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<PathBuf, String> {
    let base_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?;
    Ok(base_dir.join("basemaps"))
}

/// Path to the basemap index file.
pub fn index_path<R: Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<PathBuf, String> {
    Ok(basemaps_dir(app)?.join("index.json"))
}

/// Migrate from the old single-file layout if needed.
/// Moves `basemap.pmtiles` -> `basemaps/global.pmtiles` and
/// converts `basemap_metadata.json` -> `basemaps/index.json`.
pub fn migrate_legacy_basemap<R: Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<(), String> {
    let base_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?;
    let old_path = base_dir.join("basemap.pmtiles");
    let new_dir = basemaps_dir(app)?;

    if !old_path.exists() || new_dir.exists() {
        return Ok(());
    }

    std::fs::create_dir_all(&new_dir)
        .map_err(|e| format!("Failed to create basemaps dir: {e}"))?;

    let new_path = new_dir.join("global.pmtiles");
    std::fs::rename(&old_path, &new_path)
        .map_err(|e| format!("Failed to migrate basemap: {e}"))?;

    // Try to read old metadata and create index.json
    let old_meta_path = base_dir.join("basemap_metadata.json");
    let entry = if old_meta_path.exists() {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct OldMeta {
            download_date: String,
            source_url: String,
        }
        let meta: Option<OldMeta> = std::fs::read_to_string(&old_meta_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok());
        match meta {
            Some(m) => IndexEntry {
                id: "global".into(),
                name: "Global".into(),
                download_date: m.download_date,
                source_url: m.source_url,
            },
            None => IndexEntry {
                id: "global".into(),
                name: "Global".into(),
                download_date: String::new(),
                source_url: String::new(),
            },
        }
    } else {
        IndexEntry {
            id: "global".into(),
            name: "Global".into(),
            download_date: String::new(),
            source_url: String::new(),
        }
    };

    save_index(app, &[entry])?;

    // Clean up old metadata file
    if old_meta_path.exists() {
        std::fs::remove_file(&old_meta_path).ok();
    }

    log::info!("Migrated legacy basemap to basemaps/global.pmtiles");
    Ok(())
}

/// Read index.json entries.
pub fn load_index<R: Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<Vec<IndexEntry>, String> {
    let path = index_path(app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let data = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read index.json: {e}"))?;
    serde_json::from_str(&data)
        .map_err(|e| format!("Failed to parse index.json: {e}"))
}

/// Write index.json entries.
pub fn save_index<R: Runtime>(
    app: &tauri::AppHandle<R>,
    entries: &[IndexEntry],
) -> Result<(), String> {
    let dir = basemaps_dir(app)?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create basemaps dir: {e}"))?;
    let path = index_path(app)?;
    let json = serde_json::to_string_pretty(entries)
        .map_err(|e| format!("Failed to serialize index: {e}"))?;
    std::fs::write(&path, json)
        .map_err(|e| format!("Failed to write index.json: {e}"))
}

/// Scan basemaps/*.pmtiles, read headers, merge with index.json.
pub async fn list_basemaps<R: Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<Vec<BasemapInfo>, String> {
    let dir = basemaps_dir(app)?;
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let index = load_index(app)?;

    let mut results = Vec::new();
    let entries = std::fs::read_dir(&dir)
        .map_err(|e| format!("Failed to read basemaps dir: {e}"))?;

    for entry in entries {
        let entry =
            entry.map_err(|e| format!("Failed to read dir entry: {e}"))?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("pmtiles") {
            continue;
        }

        let file_name = match path.file_stem().and_then(|s| s.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        let id = if file_name == "global" {
            "global".to_string()
        } else {
            file_name.clone()
        };

        // Read PMTiles header for bounds and zoom
        let backend = match MmapBackend::try_from(path.as_path()).await {
            Ok(b) => b,
            Err(e) => {
                log::warn!("Skipping {file_name}.pmtiles: {e}");
                continue;
            }
        };
        let reader =
            match AsyncPmTilesReader::try_from_source(backend).await {
                Ok(r) => r,
                Err(e) => {
                    log::warn!("Skipping {file_name}.pmtiles: {e}");
                    continue;
                }
            };

        let header = reader.get_header();
        let max_zoom = header.max_zoom;

        // Extract bounds from header; treat global defaults as None
        let bounds = if id == "global" {
            None
        } else {
            let b = Bounds {
                min_lon: header.min_longitude,
                min_lat: header.min_latitude,
                max_lon: header.max_longitude,
                max_lat: header.max_latitude,
            };
            // Skip if bounds look like the default (whole world)
            if b.min_lon == -180.0
                && b.min_lat == -85.0
                && b.max_lon == 180.0
                && b.max_lat == 85.0
            {
                None
            } else {
                Some(b)
            }
        };

        let file_size = std::fs::metadata(&path)
            .map(|m| m.len())
            .unwrap_or(0);

        // Merge with index entry if available
        let idx_entry = index.iter().find(|e| e.id == id);
        let name = idx_entry
            .map(|e| e.name.clone())
            .unwrap_or_else(|| {
                if id == "global" {
                    "Global".into()
                } else {
                    file_name.clone()
                }
            });
        let download_date = idx_entry
            .map(|e| e.download_date.clone())
            .unwrap_or_default();
        let source_url = idx_entry
            .map(|e| e.source_url.clone())
            .unwrap_or_default();

        results.push(BasemapInfo {
            id,
            name,
            max_zoom,
            bounds,
            download_date,
            source_url,
            file_size,
        });
    }

    // Sort: global first, then by name
    results.sort_by(|a, b| {
        let a_global = a.id == "global";
        let b_global = b.id == "global";
        b_global.cmp(&a_global).then_with(|| a.name.cmp(&b.name))
    });

    Ok(results)
}

/// Reset all cached readers (call after downloading or deleting).
pub async fn reset_reader_cache() {
    let mut guard = READERS.write().await;
    *guard = None;
}

/// Compute the geographic bounds of a tile in Web Mercator.
fn tile_bounds(z: u8, x: u32, y: u32) -> Bounds {
    let n = (1u64 << z) as f64;
    let min_lon = (x as f64) / n * 360.0 - 180.0;
    let max_lon = ((x + 1) as f64) / n * 360.0 - 180.0;
    let min_lat_rad =
        std::f64::consts::PI * (1.0 - 2.0 * ((y + 1) as f64) / n);
    let max_lat_rad =
        std::f64::consts::PI * (1.0 - 2.0 * (y as f64) / n);
    let min_lat = min_lat_rad.sinh().atan().to_degrees();
    let max_lat = max_lat_rad.sinh().atan().to_degrees();
    Bounds { min_lon, min_lat, max_lon, max_lat }
}

fn bounds_overlap(a: &Bounds, b: &Bounds) -> bool {
    a.min_lon <= b.max_lon
        && a.max_lon >= b.min_lon
        && a.min_lat <= b.max_lat
        && a.max_lat >= b.min_lat
}

/// Initialize or return cached readers for all basemap files.
async fn get_or_init_readers<R: Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<Vec<(BasemapInfo, Arc<BasemapReader>)>, String> {
    // Fast path
    {
        let guard = READERS.read().await;
        if let Some(readers) = guard.as_ref() {
            return Ok(readers
                .iter()
                .map(|r| (r.info.clone(), r.reader.clone()))
                .collect());
        }
    }

    // Slow path: build readers
    let mut guard = READERS.write().await;
    if let Some(readers) = guard.as_ref() {
        return Ok(readers
            .iter()
            .map(|r| (r.info.clone(), r.reader.clone()))
            .collect());
    }

    let dir = basemaps_dir(app)?;
    if !dir.exists() {
        *guard = Some(Vec::new());
        return Ok(Vec::new());
    }

    let infos = list_basemaps(app).await?;
    let mut cached = Vec::with_capacity(infos.len());

    for info in infos {
        let filename = if info.id == "global" {
            "global.pmtiles".to_string()
        } else {
            format!("{}.pmtiles", info.id)
        };
        let path = dir.join(&filename);
        if !path.exists() {
            continue;
        }

        let backend =
            match MmapBackend::try_from(path.as_path()).await {
                Ok(b) => b,
                Err(e) => {
                    log::warn!("Failed to open {filename}: {e}");
                    continue;
                }
            };
        let reader = match AsyncPmTilesReader::try_from_source(
            backend,
        )
        .await
        {
            Ok(r) => r,
            Err(e) => {
                log::warn!("Failed to read {filename}: {e}");
                continue;
            }
        };

        cached.push(CachedReader {
            info,
            reader: Arc::new(reader),
        });
    }

    let result = cached
        .iter()
        .map(|r| (r.info.clone(), r.reader.clone()))
        .collect();
    *guard = Some(cached);
    Ok(result)
}

pub fn handle_basemap_request<R: Runtime>(
    ctx: tauri::UriSchemeContext<'_, R>,
    request: tauri::http::Request<Vec<u8>>,
    responder: tauri::UriSchemeResponder,
) {
    let app_handle = ctx.app_handle().clone();

    tauri::async_runtime::spawn(async move {
        // Ensure legacy migration has happened
        if let Err(e) = migrate_legacy_basemap(&app_handle) {
            log::warn!("Legacy migration failed: {e}");
        }

        // Parse z/x/y from URI path: "basemap://localhost/{z}/{x}/{y}"
        let uri = request.uri();
        let uri_path = uri.path();
        let parts: Vec<&str> =
            uri_path.trim_matches('/').split('/').collect();

        if parts.len() != 3 {
            respond_error(
                responder,
                400,
                "Invalid tile coordinates",
            );
            return;
        }

        let z: u8 = match parts[0].parse() {
            Ok(v) => v,
            Err(_) => {
                respond_error(
                    responder, 400, "Invalid zoom level",
                );
                return;
            }
        };
        let x: u32 = match parts[1].parse() {
            Ok(v) => v,
            Err(_) => {
                respond_error(
                    responder, 400, "Invalid x coordinate",
                );
                return;
            }
        };
        let y: u32 = match parts[2].parse() {
            Ok(v) => v,
            Err(_) => {
                respond_error(
                    responder, 400, "Invalid y coordinate",
                );
                return;
            }
        };

        let readers = match get_or_init_readers(&app_handle).await {
            Ok(r) => r,
            Err(e) => {
                respond_error(responder, 500, &e);
                return;
            }
        };

        if readers.is_empty() {
            respond_error(responder, 404, "No basemap downloaded");
            return;
        }

        let tile_coord = match pmtiles::TileCoord::new(z, x, y) {
            Ok(c) => c,
            Err(e) => {
                respond_error(
                    responder,
                    400,
                    &format!("Invalid tile coord: {e}"),
                );
                return;
            }
        };

        let tile_geo = tile_bounds(z, x, y);

        // Try regional readers first, then global
        let mut regional = Vec::new();
        let mut global = Vec::new();
        for (info, reader) in &readers {
            if info.bounds.is_some() {
                regional.push((info, reader));
            } else {
                global.push((info, reader));
            }
        }

        for (info, reader) in &regional {
            if z > info.max_zoom {
                continue;
            }
            if let Some(b) = &info.bounds {
                if !bounds_overlap(&tile_geo, b) {
                    continue;
                }
            }
            match reader.get_tile_decompressed(tile_coord).await {
                Ok(Some(data)) => {
                    respond_tile(responder, &data);
                    return;
                }
                Ok(None) => continue,
                Err(e) => {
                    log::warn!("Regional tile error: {e}");
                    continue;
                }
            }
        }

        // Fall back to global
        for (info, reader) in &global {
            if z > info.max_zoom {
                continue;
            }
            match reader.get_tile_decompressed(tile_coord).await {
                Ok(Some(data)) => {
                    respond_tile(responder, &data);
                    return;
                }
                Ok(None) => continue,
                Err(e) => {
                    log::warn!("Global tile error: {e}");
                    continue;
                }
            }
        }

        // No tile found in any reader
        responder.respond(
            tauri::http::Response::builder()
                .status(204)
                .header("Access-Control-Allow-Origin", "*")
                .body(Vec::new())
                .unwrap(),
        );
    });
}

fn respond_tile(
    responder: tauri::UriSchemeResponder,
    data: &[u8],
) {
    responder.respond(
        tauri::http::Response::builder()
            .status(200)
            .header(
                "Content-Type",
                "application/vnd.mapbox-vector-tile",
            )
            .header("Cache-Control", "public, max-age=86400")
            .header("Access-Control-Allow-Origin", "*")
            .body(data.to_vec())
            .unwrap(),
    );
}

fn respond_error(
    responder: tauri::UriSchemeResponder,
    status: u16,
    message: &str,
) {
    responder.respond(
        tauri::http::Response::builder()
            .status(status)
            .header("Access-Control-Allow-Origin", "*")
            .body(message.as_bytes().to_vec())
            .unwrap(),
    );
}

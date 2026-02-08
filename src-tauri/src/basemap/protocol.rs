use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};

use pmtiles::{AsyncPmTilesReader, MmapBackend, NoCache};
use tauri::{Manager, Runtime};
use tokio::sync::RwLock;

type BasemapReader = AsyncPmTilesReader<MmapBackend, NoCache>;

static READER: LazyLock<RwLock<Option<Arc<BasemapReader>>>> =
    LazyLock::new(|| RwLock::new(None));

/// Get the path where the basemap PMTiles file is stored.
pub fn basemap_path<R: Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<PathBuf, String> {
    let base_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?;
    Ok(base_dir.join("basemap.pmtiles"))
}

async fn get_or_init_reader(
    path: &Path,
) -> Result<Arc<BasemapReader>, String> {
    // Fast path: check if reader is already initialized
    {
        let guard = READER.read().await;
        if let Some(reader) = guard.as_ref() {
            return Ok(reader.clone());
        }
    }

    // Slow path: initialize reader
    let mut guard = READER.write().await;
    // Double-check after acquiring write lock
    if let Some(reader) = guard.as_ref() {
        return Ok(reader.clone());
    }

    let backend = MmapBackend::try_from(path)
        .await
        .map_err(|e| format!("Failed to open basemap file: {e}"))?;
    let reader = AsyncPmTilesReader::try_from_source(backend)
        .await
        .map_err(|e| format!("Failed to read basemap PMTiles: {e}"))?;
    let reader = Arc::new(reader);
    *guard = Some(reader.clone());
    Ok(reader)
}

/// Reset the cached reader (call after downloading a new basemap).
pub async fn reset_reader_cache() {
    let mut guard = READER.write().await;
    *guard = None;
}

pub fn handle_basemap_request<R: Runtime>(
    ctx: tauri::UriSchemeContext<'_, R>,
    request: tauri::http::Request<Vec<u8>>,
    responder: tauri::UriSchemeResponder,
) {
    let app_handle = ctx.app_handle().clone();

    // Use tokio runtime for async pmtiles operations
    tauri::async_runtime::spawn(async move {
        let path = match basemap_path(&app_handle) {
            Ok(p) => p,
            Err(e) => {
                respond_error(
                    responder,
                    500,
                    &format!("Path error: {e}"),
                );
                return;
            }
        };

        if !path.exists() {
            respond_error(responder, 404, "No basemap downloaded");
            return;
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
                respond_error(responder, 400, "Invalid zoom level");
                return;
            }
        };
        let x: u32 = match parts[1].parse() {
            Ok(v) => v,
            Err(_) => {
                respond_error(responder, 400, "Invalid x coordinate");
                return;
            }
        };
        let y: u32 = match parts[2].parse() {
            Ok(v) => v,
            Err(_) => {
                respond_error(responder, 400, "Invalid y coordinate");
                return;
            }
        };

        let reader = match get_or_init_reader(&path).await {
            Ok(r) => r,
            Err(e) => {
                respond_error(responder, 500, &e);
                return;
            }
        };

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

        match reader.get_tile_decompressed(tile_coord).await {
            Ok(Some(tile_data)) => {
                responder.respond(
                    tauri::http::Response::builder()
                        .status(200)
                        .header(
                            "Content-Type",
                            "application/vnd.mapbox-vector-tile",
                        )
                        .header(
                            "Cache-Control",
                            "public, max-age=86400",
                        )
                        .header("Access-Control-Allow-Origin", "*")
                        .body(tile_data.to_vec())
                        .unwrap(),
                );
            }
            Ok(None) => {
                // No tile at this coordinate - return empty response
                responder.respond(
                    tauri::http::Response::builder()
                        .status(204)
                        .header("Access-Control-Allow-Origin", "*")
                        .body(Vec::new())
                        .unwrap(),
                );
            }
            Err(e) => {
                log::error!("Basemap tile read error: {e}");
                respond_error(
                    responder,
                    500,
                    &format!("Tile read error: {e}"),
                );
            }
        }
    });
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

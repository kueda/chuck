use tauri::Runtime;
use crate::search_params::SearchParams;

use super::coords::{lat_lng_to_tile_coords};
use super::mvt::{OccurrencePoint, encode_tile};

/// Generate MVT tile for given coordinates and occurrence data
pub fn generate_tile(
    z: u8,
    x: u32,
    y: u32,
    occurrences: Vec<(String, f64, f64, Option<String>)>
) -> Vec<u8> {
    // Convert occurrences to tile coordinates
    let points: Vec<OccurrencePoint> = occurrences
        .into_iter()
        .filter_map(|(core_id, lat, lng, name)| {
            let (tile_x, tile_y) = lat_lng_to_tile_coords(lat, lng, z, x, y);

            // Only include points that are within the tile extent (0-4096)
            if (0.0..=4096.0).contains(&tile_x) && (0.0..=4096.0).contains(&tile_y) {
                Some(OccurrencePoint {
                    core_id,
                    x: tile_x,
                    y: tile_y,
                    scientific_name: name,
                })
            } else {
                None
            }
        })
        .collect();

    encode_tile(points)
}

pub fn handle_tile_request<R: Runtime>(
    app: tauri::UriSchemeContext<'_, R>,
    request: tauri::http::Request<Vec<u8>>,
    responder: tauri::UriSchemeResponder
) {
    let app_handle = app.app_handle().clone();

    std::thread::spawn(move || {
        // Parse z/x/y from URI path: "tiles://localhost/5/8/15"
        let uri = request.uri();
        let path = uri.path();
        let parts: Vec<&str> = path.trim_matches('/').split('/').collect();

        if parts.len() != 3 {
            responder.respond(
                tauri::http::Response::builder()
                    .status(400)
                    .header("Access-Control-Allow-Origin", "*")
                    .body(b"Invalid tile coordinates".to_vec())
                    .unwrap()
            );
            return;
        }

        let z: u8 = match parts[0].parse() {
            Ok(v) => v,
            Err(_) => {
                responder.respond(
                    tauri::http::Response::builder()
                        .status(400)
                        .header("Access-Control-Allow-Origin", "*")
                        .body(b"Invalid zoom level".to_vec())
                        .unwrap()
                );
                return;
            }
        };

        let x: u32 = match parts[1].parse() {
            Ok(v) => v,
            Err(_) => {
                responder.respond(
                    tauri::http::Response::builder()
                        .status(400)
                        .header("Access-Control-Allow-Origin", "*")
                        .body(b"Invalid x coordinate".to_vec())
                        .unwrap()
                );
                return;
            }
        };

        let y: u32 = match parts[2].parse() {
            Ok(v) => v,
            Err(_) => {
                responder.respond(
                    tauri::http::Response::builder()
                        .status(400)
                        .header("Access-Control-Allow-Origin", "*")
                        .body(b"Invalid y coordinate".to_vec())
                        .unwrap()
                );
                return;
            }
        };

        // Generate tile using existing stateless pattern
        let result = (|| -> Result<Vec<u8>, String> {
            let archives_dir = crate::commands::archive::get_archives_dir(app_handle.clone())
                .map_err(|e| e.to_string())?;
            let archive = crate::dwca::Archive::current(&archives_dir)
                .map_err(|e| e.to_string())?;

            // Calculate bounding box for this tile
            let bbox = super::coords::tile_to_bbox(z, x, y);

            // Query occurrences within bounds
            let occurrences = archive.query_tile(
                bbox.west,
                bbox.south,
                bbox.east,
                bbox.north,
                z,
                SearchParams::from_uri(uri),
            ).map_err(|e| e.to_string())?;

            // Generate MVT tile
            Ok(generate_tile(z, x, y, occurrences))
        })();

        match result {
            Ok(tile_bytes) => {
                responder.respond(
                    tauri::http::Response::builder()
                        .status(200)
                        .header("Content-Type", "application/vnd.mapbox-vector-tile")
                        .header("Cache-Control", "public, max-age=3600")
                        .header("Access-Control-Allow-Origin", "*")
                        .body(tile_bytes)
                        .unwrap()
                );
            }
            Err(e) => {
                log::error!("Tile generation error: {e}");
                responder.respond(
                    tauri::http::Response::builder()
                        .status(500)
                        .header("Access-Control-Allow-Origin", "*")
                        .body(format!("Error: {e}").into_bytes())
                        .unwrap()
                );
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_tile_returns_mvt() {
        // Test with empty data
        let tile = generate_tile(0, 0, 0, Vec::new());
        assert!(!tile.is_empty());
    }

    #[test]
    fn test_generate_tile_with_data() {
        let occurrences = vec![
            ("1".to_string(), 0.0, 0.0, Some("Test species".to_string())),
        ];
        let tile = generate_tile(0, 0, 0, occurrences);
        assert!(!tile.is_empty());
        assert!(tile.len() > 10);
    }
}

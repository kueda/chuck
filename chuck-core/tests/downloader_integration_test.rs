use chuck_core::downloader::{Downloader, DownloadProgress};
use chuck_core::DwcaExtension;
use httpmock::prelude::*;
use inaturalist::apis::observations_api;
use serial_test::serial;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tempfile::TempDir;

// Minimal 1x1 PNG image (67 bytes)
const MINIMAL_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A,
    0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
    0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
    0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41,
    0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00,
    0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE,
    0x42, 0x60, 0x82
];

#[tokio::test]
#[serial]
async fn test_downloader_execute_creates_archive() {
    // Set up mock server
    let server = MockServer::start();

    // Create custom configuration pointing to mock server
    let config = chuck_core::api::client::create_config_with_base_url_and_jwt(
        server.base_url(),
        Some("test_jwt".to_string())
    );

    // Mock observations API - returns observation on first call, empty on subsequent calls
    let observations_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/observations")
            .query_param_exists("id_below");  // When pagination parameter is present
        then.status(200)
            .header("content-type", "application/json")
            .json_body(serde_json::json!({
                "total_results": 1,
                "results": []  // Empty results to stop pagination
            }));
    });

    // Mock initial observations API call (no id_below parameter)
    let observations_initial_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/observations");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(serde_json::json!({
                "total_results": 1,
                "results": [{
                    "id": 123456,
                    "taxon": {
                        "id": 47126,
                        "name": "Plantae",
                        "rank": "kingdom",
                        "ancestor_ids": [48460, 47126]
                    },
                    "user": {
                        "id": 1,
                        "login": "testuser"
                    },
                    "observed_on": "2024-01-01",
                    "created_at": "2024-01-01T00:00:00Z",
                    "geojson": {
                        "type": "Point",
                        "coordinates": [-122.4194, 37.7749]
                    },
                    "photos": [{
                        "id": 78910,
                        "url": format!("{}/photo.jpg", server.base_url())
                    }]
                }]
            }));
    });

    // Mock taxa API - responds to any taxa ID request
    let taxa_mock = server.mock(|when, then| {
        when.method(GET)
            .path_contains("/taxa");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(serde_json::json!({
                "results": [{
                    "id": 47126,
                    "name": "Plantae",
                    "rank": "kingdom",
                    "ancestor_ids": [48460, 47126],
                    "ancestry": "48460",
                    "parent_id": 48460
                }, {
                    "id": 48460,
                    "name": "Life",
                    "rank": "stateofmatter",
                    "ancestor_ids": [48460]
                }]
            }));
    });

    // Mock photo download
    let photo_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/photo.jpg");
        then.status(200)
            .header("content-type", "image/jpeg")
            .body(MINIMAL_PNG);
    });

    // Create temp directory for output
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("test.zip");

    // Create downloader with test params
    let params = observations_api::ObservationsGetParams {
        taxon_id: Some(vec!["47126".to_string()]),
        per_page: Some("1".to_string()),
        ..chuck_core::api::params::DEFAULT_GET_PARAMS.clone()
    };

    let extensions = vec![DwcaExtension::SimpleMultimedia];
    // Create downloader with custom config
    let downloader = Downloader::with_config(params, extensions, true, config);

    // Track progress
    let progress_count = Arc::new(AtomicUsize::new(0));
    let progress_count_clone = progress_count.clone();

    let progress_callback = move |_progress: DownloadProgress| {
        progress_count_clone.fetch_add(1, Ordering::Relaxed);
    };

    // Execute download
    let result = downloader.execute(
        output_path.to_str().unwrap(),
        progress_callback,
        None,
    ).await;

    // Assertions
    assert!(result.is_ok(), "Download should succeed: {:?}", result.err());
    assert!(progress_count.load(Ordering::Relaxed) > 0, "Progress callback should be called");
    assert!(output_path.exists(), "Archive file should be created");

    // Verify mocks were called
    observations_initial_mock.assert();
    observations_mock.assert();
    taxa_mock.assert();
    photo_mock.assert();
}

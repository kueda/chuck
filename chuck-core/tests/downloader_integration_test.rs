use chuck_core::downloader::{Downloader, DownloadProgress};
use chuck_core::DwcaExtension;
use httpmock::prelude::*;
use inaturalist::apis::observations_api;
use serial_test::serial;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tempfile::TempDir;

// Minimal MP3 file bytes (just enough for a valid download test)
const MINIMAL_MP3: &[u8] = b"ID3\x03\x00\x00\x00\x00\x00\x00";

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

// Helper to create standard taxa JSON response body
fn taxa_response_json() -> serde_json::Value {
    serde_json::json!({
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
    })
}

// Helper to create observation JSON with photos
fn observation_json(id: i32, base_url: &str, photo_ids: &[i32]) -> serde_json::Value {
    let photos: Vec<serde_json::Value> = photo_ids.iter().map(|photo_id| {
        serde_json::json!({
            "id": photo_id,
            "url": format!("{}/photo{}.jpg", base_url, photo_id)
        })
    }).collect();

    serde_json::json!({
        "id": id,
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
        "photos": photos
    })
}

// Helper to create observation JSON with sounds
fn observation_with_sound_json(
    id: i32,
    base_url: &str,
    sounds: &[(i32, &str)],
) -> serde_json::Value {
    let sound_list: Vec<serde_json::Value> = sounds.iter().map(|(sound_id, ext)| {
        serde_json::json!({
            "id": sound_id,
            "file_url": format!("{}/sound{}.{}", base_url, sound_id, ext),
            "file_content_type": format!("audio/{}", if *ext == "mp3" { "mpeg" } else { ext }),
            "license_code": "cc-by",
            "attribution": "(c) testuser"
        })
    }).collect();

    serde_json::json!({
        "id": id,
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
        "sounds": sound_list
    })
}

// Helper to create observations response JSON
fn observations_response_json(total_results: i32, observations: Vec<serde_json::Value>) -> serde_json::Value {
    serde_json::json!({
        "total_results": total_results,
        "results": observations
    })
}


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
            .json_body(observations_response_json(
                1,
                vec![observation_json(123456, &server.base_url(), &[78910])]
            ));
    });

    // Mock taxa API
    let _taxa_mock = server.mock(|when, then| {
        when.method(GET).path_contains("/taxa");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(taxa_response_json());
    });

    // Mock photo download
    let photo_mock = server.mock(|when, then| {
        when.method(GET).path("/photo78910.jpg");
        then.status(200).header("content-type", "image/jpeg").body(MINIMAL_PNG);
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
    photo_mock.assert();
}

#[tokio::test]
#[serial]
async fn test_downloader_reports_photo_progress() {
    // Set up mock server
    let server = MockServer::start();

    // Create custom configuration pointing to mock server
    let config = chuck_core::api::client::create_config_with_base_url_and_jwt(
        server.base_url(),
        Some("test_jwt".to_string())
    );

    // Mock observations API - returns observation on first call, empty on subsequent calls
    let observations_pagination_mock = server.mock(|when, then| {
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
            .json_body(observations_response_json(
                1,
                vec![observation_json(123456, &server.base_url(), &[78910])]
            ));
    });

    // Mock taxa API
    let taxa_mock = server.mock(|when, then| {
        when.method(GET).path_contains("/taxa");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(taxa_response_json());
    });

    // Mock photo download
    let photo_mock = server.mock(|when, then| {
        when.method(GET).path("/photo78910.jpg");
        then.status(200).header("content-type", "image/jpeg").body(MINIMAL_PNG);
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
    let downloader = Downloader::with_config(params, extensions, true, config);

    // Track whether we received any photo progress events
    let photo_progress_emitted = Arc::new(AtomicUsize::new(0));
    let photo_progress_clone = photo_progress_emitted.clone();

    let progress_callback = move |progress: DownloadProgress| {
        if progress.media_current > 0 {
            photo_progress_clone.store(progress.media_current, Ordering::Relaxed);
        }
    };

    // Execute download
    let result = downloader.execute(
        output_path.to_str().unwrap(),
        progress_callback,
        None,
    ).await;

    // Assertions
    assert!(result.is_ok(), "Download should succeed: {:?}", result.err());

    let final_photos = photo_progress_emitted.load(Ordering::Relaxed);
    assert!(
        final_photos > 0,
        "Expected photo progress to be emitted, but media_current was never > 0"
    );

    // Verify mocks were called
    observations_initial_mock.assert();
    observations_pagination_mock.assert();
    taxa_mock.assert();
    photo_mock.assert();
}

#[tokio::test]
#[serial]
async fn test_downloader_emits_progress_for_each_page() {
    // Set up mock server
    let server = MockServer::start();

    // Create custom configuration pointing to mock server
    let config = chuck_core::api::client::create_config_with_base_url_and_jwt(
        server.base_url(),
        Some("test_jwt".to_string())
    );

    // Mock third page - empty to stop pagination
    let observations_page3_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/observations")
            .query_param("id_below", "100");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(serde_json::json!({
                "total_results": 2,
                "results": []
            }));
    });

    // Mock second page - 1 observation with 1 photo
    let observations_page2_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/observations")
            .query_param("id_below", "200");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(observations_response_json(
                2,
                vec![observation_json(100, &server.base_url(), &[2000])]
            ));
    });

    // Mock first page - 1 observation with 1 photo
    let observations_page1_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/observations");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(observations_response_json(
                2,
                vec![observation_json(200, &server.base_url(), &[1000])]
            ));
    });

    // Mock taxa API
    let _taxa_mock = server.mock(|when, then| {
        when.method(GET).path_contains("/taxa");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(taxa_response_json());
    });

    // Mock photo downloads
    let photo1_mock = server.mock(|when, then| {
        when.method(GET).path("/photo1000.jpg");
        then.status(200).header("content-type", "image/jpeg").body(MINIMAL_PNG);
    });
    let photo2_mock = server.mock(|when, then| {
        when.method(GET).path("/photo2000.jpg");
        then.status(200).header("content-type", "image/jpeg").body(MINIMAL_PNG);
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
    let downloader = Downloader::with_config(params, extensions, true, config);

    // Track progress events for page 1 and page 2
    let page1_photo_progress = Arc::new(AtomicUsize::new(0));
    let page2_photo_progress = Arc::new(AtomicUsize::new(0));
    let page1_clone = page1_photo_progress.clone();
    let page2_clone = page2_photo_progress.clone();

    let progress_callback = move |progress: DownloadProgress| {
        if progress.observations_current == 1 && progress.media_current > 0 {
            page1_clone.store(progress.media_current, Ordering::Relaxed);
        }
        if progress.observations_current == 2 && progress.media_current > 0 {
            page2_clone.store(progress.media_current, Ordering::Relaxed);
        }
    };

    // Execute download
    let result = downloader.execute(
        output_path.to_str().unwrap(),
        progress_callback,
        None,
    ).await;

    // Assertions
    assert!(result.is_ok(), "Download should succeed: {:?}", result.err());

    let page1_photos = page1_photo_progress.load(Ordering::Relaxed);
    let page2_photos = page2_photo_progress.load(Ordering::Relaxed);

    assert!(
        page1_photos > 0,
        "Expected photo progress for page 1, but got 0"
    );

    assert!(
        page2_photos > 0,
        "Expected photo progress for page 2, but got 0"
    );

    // Verify mocks were called
    observations_page1_mock.assert();
    observations_page2_mock.assert();
    observations_page3_mock.assert();
    photo1_mock.assert();
    photo2_mock.assert();
}

#[tokio::test]
#[serial]
async fn test_downloader_estimates_total_photos() {
    // Set up mock server
    let server = MockServer::start();

    // Create custom configuration pointing to mock server
    let config = chuck_core::api::client::create_config_with_base_url_and_jwt(
        server.base_url(),
        Some("test_jwt".to_string())
    );

    // Mock pagination - return empty to stop
    let observations_pagination_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/observations")
            .query_param_exists("id_below");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(observations_response_json(10, vec![]));
    });

    // Mock initial observations - total_results=10 but only return 1 observation with 2 photos
    let observations_initial_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/observations");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(observations_response_json(
                10,
                vec![observation_json(123, &server.base_url(), &[1000, 1001])]
            ));
    });

    // Mock taxa API
    let _taxa_mock = server.mock(|when, then| {
        when.method(GET)
            .path_contains("/taxa");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(taxa_response_json());
    });

    // Mock photo downloads
    let photo1_mock = server.mock(|when, then| {
        when.method(GET).path("/photo1000.jpg");
        then.status(200).header("content-type", "image/jpeg").body(MINIMAL_PNG);
    });
    let photo2_mock = server.mock(|when, then| {
        when.method(GET).path("/photo1001.jpg");
        then.status(200).header("content-type", "image/jpeg").body(MINIMAL_PNG);
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
    let downloader = Downloader::with_config(params, extensions, true, config);

    // Track media_total from progress events
    let media_total_estimate = Arc::new(AtomicUsize::new(0));
    let media_total_clone = media_total_estimate.clone();

    let progress_callback = move |progress: DownloadProgress| {
        if progress.media_total > 0 {
            media_total_clone.store(progress.media_total, Ordering::Relaxed);
        }
    };

    // Execute download
    let result = downloader.execute(
        output_path.to_str().unwrap(),
        progress_callback,
        None,
    ).await;

    // Assertions
    assert!(result.is_ok(), "Download should succeed: {:?}", result.err());

    let estimated_total = media_total_estimate.load(Ordering::Relaxed);

    // KEY ASSERTION: The estimated total photos should be > total observations
    // We have 10 total_results (observations) and this page has 2 photos for 1 observation
    // So the estimate should be roughly 2 * 10 = 20 photos (or some estimate > 10)
    assert!(
        estimated_total > 10,
        "Expected media_total estimate ({estimated_total}) to be greater than total observations (10)"
    );

    // Verify mocks were called
    observations_initial_mock.assert();
    observations_pagination_mock.assert();
    photo1_mock.assert();
    photo2_mock.assert();
}

#[tokio::test]
#[serial]
async fn test_downloader_media_current_accumulates_across_pages() {
    // Set up mock server
    let server = MockServer::start();

    // Create custom configuration pointing to mock server
    let config = chuck_core::api::client::create_config_with_base_url_and_jwt(
        server.base_url(),
        Some("test_jwt".to_string())
    );

    // Mock third page - empty results to stop pagination
    let observations_page3_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/observations")
            .query_param("id_below", "200");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(observations_response_json(2, vec![]));
    });

    // Mock second page - 1 observation with 1 photo
    let observations_page2_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/observations")
            .query_param("id_below", "300");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(observations_response_json(
                2,
                vec![observation_json(200, &server.base_url(), &[2000])]
            ));
    });

    // Mock first page - 1 observation with 1 photo
    let observations_page1_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/observations");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(observations_response_json(
                2,
                vec![observation_json(300, &server.base_url(), &[1000])]
            ));
    });

    // Mock taxa API
    let _taxa_mock = server.mock(|when, then| {
        when.method(GET).path_contains("/taxa");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(taxa_response_json());
    });

    // Mock photo downloads
    let photo1_mock = server.mock(|when, then| {
        when.method(GET).path("/photo1000.jpg");
        then.status(200).header("content-type", "image/jpeg").body(MINIMAL_PNG);
    });
    let photo2_mock = server.mock(|when, then| {
        when.method(GET).path("/photo2000.jpg");
        then.status(200).header("content-type", "image/jpeg").body(MINIMAL_PNG);
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
    let downloader = Downloader::with_config(params, extensions, true, config);

    // Track photo progress values - we want to see if they accumulate
    let page1_photos = Arc::new(AtomicUsize::new(0));
    let page2_photos = Arc::new(AtomicUsize::new(0));
    let page1_clone = page1_photos.clone();
    let page2_clone = page2_photos.clone();

    let progress_callback = move |progress: DownloadProgress| {
        // After page 1: observations_current=1, media_current should be 1
        if progress.observations_current == 1 && progress.media_current > 0 {
            page1_clone.store(progress.media_current, Ordering::Relaxed);
        }
        // After page 2: observations_current=2, media_current should be 2 (accumulated)
        if progress.observations_current == 2 && progress.media_current > 0 {
            page2_clone.store(progress.media_current, Ordering::Relaxed);
        }
    };

    // Execute download
    let result = downloader.execute(
        output_path.to_str().unwrap(),
        progress_callback,
        None,
    ).await;

    // Assertions
    assert!(result.is_ok(), "Download should succeed: {:?}", result.err());

    let page1_photos_count = page1_photos.load(Ordering::Relaxed);
    let page2_photos_count = page2_photos.load(Ordering::Relaxed);

    // KEY ASSERTIONS: Test that media_current accumulates across pages
    assert_eq!(
        page1_photos_count, 1,
        "After page 1 (1 obs with 1 photo), media_current should be 1"
    );
    assert_eq!(
        page2_photos_count, 2,
        "After page 2 (2 obs total with 2 photos total), media_current should be 2 (accumulated), not 1 (reset)"
    );

    // Verify mocks were called
    observations_page1_mock.assert();
    observations_page2_mock.assert();
    observations_page3_mock.assert();
    photo1_mock.assert();
    photo2_mock.assert();
}

#[tokio::test]
#[serial]
async fn test_cancellation_stops_photo_downloads() {
    // Set up mock server
    let server = MockServer::start();

    // Create custom configuration pointing to mock server
    let config = chuck_core::api::client::create_config_with_base_url_and_jwt(
        server.base_url(),
        Some("test_jwt".to_string()),
    );

    // Mock pagination stop
    let _observations_pagination_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/observations")
            .query_param_exists("id_below");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(observations_response_json(1, vec![]));
    });

    // Create observation with many photos so download takes a while
    let photo_ids: Vec<i32> = (1..=20).collect();
    let _observations_initial_mock = server.mock(|when, then| {
        when.method(GET).path("/observations");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(observations_response_json(
                1,
                vec![observation_json(500, &server.base_url(), &photo_ids)],
            ));
    });

    // Mock taxa API
    let _taxa_mock = server.mock(|when, then| {
        when.method(GET).path_contains("/taxa");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(taxa_response_json());
    });

    // Mock photo downloads with a long delay so cancellation fires mid-download
    for id in &photo_ids {
        let path = format!("/photo{id}.jpg");
        server.mock(|when, then| {
            when.method(GET).path(path);
            then.status(200)
                .header("content-type", "image/jpeg")
                .body(MINIMAL_PNG)
                .delay(std::time::Duration::from_secs(5));
        });
    }

    // Create temp directory for output
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("test.zip");

    // Create downloader
    let params = observations_api::ObservationsGetParams {
        taxon_id: Some(vec!["47126".to_string()]),
        per_page: Some("1".to_string()),
        ..chuck_core::api::params::DEFAULT_GET_PARAMS.clone()
    };

    let extensions = vec![DwcaExtension::SimpleMultimedia];
    let downloader = Downloader::with_config(params, extensions, true, config);

    // Set up cancellation token
    let cancel_token = Arc::new(AtomicBool::new(false));
    let cancel_token_clone = cancel_token.clone();

    // Cancel as soon as observations are fetched (before photo downloads complete)
    let progress_callback = move |progress: DownloadProgress| {
        if progress.observations_current > 0 {
            cancel_token_clone.store(true, Ordering::Relaxed);
        }
    };

    // Execute download
    let result = downloader
        .execute(
            output_path.to_str().unwrap(),
            progress_callback,
            Some(cancel_token),
        )
        .await;

    // Should return cancellation error
    assert!(result.is_err(), "Download should be cancelled");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("cancelled"),
        "Error should mention cancellation, got: {err}"
    );

    // Archive should NOT have been created
    assert!(
        !output_path.exists(),
        "Archive file should not be created when cancelled"
    );
}

#[tokio::test]
#[serial]
async fn test_downloader_includes_sounds_in_multimedia_csv() {
    let server = MockServer::start();

    let config = chuck_core::api::client::create_config_with_base_url_and_jwt(
        server.base_url(),
        Some("test_jwt".to_string()),
    );

    let _observations_pagination_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/observations")
            .query_param_exists("id_below");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(observations_response_json(1, vec![]));
    });

    let _observations_initial_mock = server.mock(|when, then| {
        when.method(GET).path("/observations");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(observations_response_json(
                1,
                vec![observation_with_sound_json(123456, &server.base_url(), &[(999, "mp3")])],
            ));
    });

    let _taxa_mock = server.mock(|when, then| {
        when.method(GET).path_contains("/taxa");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(taxa_response_json());
    });

    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("test.zip");

    let params = observations_api::ObservationsGetParams {
        taxon_id: Some(vec!["47126".to_string()]),
        per_page: Some("1".to_string()),
        ..chuck_core::api::params::DEFAULT_GET_PARAMS.clone()
    };

    let extensions = vec![DwcaExtension::SimpleMultimedia];
    let downloader = Downloader::with_config(params, extensions, false, config);

    let result = downloader
        .execute(output_path.to_str().unwrap(), |_| {}, None)
        .await;

    assert!(result.is_ok(), "Download should succeed: {:?}", result.err());

    // Read the archive and check multimedia.csv
    let zip_data = std::fs::read(&output_path).unwrap();
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(zip_data)).unwrap();

    let mut multimedia_csv = String::new();
    {
        use std::io::Read;
        let mut file = zip.by_name("multimedia.csv").expect("multimedia.csv should exist");
        file.read_to_string(&mut multimedia_csv).unwrap();
    }

    assert!(
        multimedia_csv.contains("Sound"),
        "multimedia.csv should contain Sound type, got:\n{multimedia_csv}"
    );
    assert!(
        multimedia_csv.contains("audio/mpeg"),
        "multimedia.csv should contain audio/mpeg format, got:\n{multimedia_csv}"
    );
    assert!(
        multimedia_csv.contains("sound999.mp3"),
        "multimedia.csv should contain sound URL, got:\n{multimedia_csv}"
    );
}

#[tokio::test]
#[serial]
async fn test_downloader_downloads_sound_files() {
    let server = MockServer::start();

    let config = chuck_core::api::client::create_config_with_base_url_and_jwt(
        server.base_url(),
        Some("test_jwt".to_string()),
    );

    let _observations_pagination_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/observations")
            .query_param_exists("id_below");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(observations_response_json(1, vec![]));
    });

    let _observations_initial_mock = server.mock(|when, then| {
        when.method(GET).path("/observations");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(observations_response_json(
                1,
                vec![observation_with_sound_json(123456, &server.base_url(), &[(999, "mp3")])],
            ));
    });

    let _taxa_mock = server.mock(|when, then| {
        when.method(GET).path_contains("/taxa");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(taxa_response_json());
    });

    let sound_mock = server.mock(|when, then| {
        when.method(GET).path("/sound999.mp3");
        then.status(200)
            .header("content-type", "audio/mpeg")
            .body(MINIMAL_MP3);
    });

    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("test.zip");

    let params = observations_api::ObservationsGetParams {
        taxon_id: Some(vec!["47126".to_string()]),
        per_page: Some("1".to_string()),
        ..chuck_core::api::params::DEFAULT_GET_PARAMS.clone()
    };

    let extensions = vec![DwcaExtension::SimpleMultimedia];
    // fetch_media = true to trigger sound downloads
    let downloader = Downloader::with_config(params, extensions, true, config);

    let result = downloader
        .execute(output_path.to_str().unwrap(), |_| {}, None)
        .await;

    assert!(result.is_ok(), "Download should succeed: {:?}", result.err());

    sound_mock.assert();

    // Read the archive and verify sound file is present in media/
    let zip_data = std::fs::read(&output_path).unwrap();
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(zip_data)).unwrap();

    let has_sound_file = (0..zip.len()).any(|i| {
        let file = zip.by_index(i).unwrap();
        file.name().contains("999.mp3")
    });

    assert!(
        has_sound_file,
        "Archive should contain downloaded sound file 999.mp3"
    );

    // multimedia.csv should use local path, not remote URL
    let mut multimedia_csv = String::new();
    {
        use std::io::Read;
        let mut file = zip.by_name("multimedia.csv").expect("multimedia.csv should exist");
        file.read_to_string(&mut multimedia_csv).unwrap();
    }

    assert!(
        !multimedia_csv.contains(&server.base_url()),
        "multimedia.csv should use local path, not remote URL"
    );
    assert!(
        multimedia_csv.contains("999.mp3"),
        "multimedia.csv should reference local sound file"
    );
}

use std::backtrace::Backtrace;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use serde::{Serialize};
use tauri::{Emitter, Manager};
#[cfg(target_os = "linux")]
use gtk::prelude::{BinExt, Cast, GtkWindowExt, HeaderBarExt};
#[cfg(target_os = "linux")]
use gtk::{EventBox, HeaderBar};

use crate::dwca::Archive;
use crate::error::{ChuckError, Result};
use crate::photo_cache::PhotoCache;
use crate::search_params::SearchParams;
use crate::ZipState;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum ArchiveOpenProgress {
    Importing,
    Extracting,
    CreatingDatabase,
    Complete { info: ArchiveInfo },
    Error { message: String },
}

#[derive(Debug, Clone, Serialize)]
pub struct ArchiveInfo {
    pub name: String,

    #[serde(rename = "coreCount")]
    pub core_count: usize,

    #[serde(rename = "coreIdColumn")]
    pub core_id_column: String,

    #[serde(rename = "availableColumns")]
    pub available_columns: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub total: usize,
    pub results: Vec<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Debug, Serialize)]
pub struct XmlFile {
    pub filename: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct ArchiveMetadata {
    pub xml_files: Vec<XmlFile>,
}

/// Internal function to get all XML metadata files from storage directory
fn get_metadata_from_storage(storage_dir: &Path) -> Result<ArchiveMetadata> {
    let mut xml_files = Vec::new();

    // Read all .xml files from the storage directory
    if let Ok(entries) = std::fs::read_dir(storage_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("xml") {
                if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                    match std::fs::read_to_string(&path) {
                        Ok(content) => {
                            xml_files.push(XmlFile {
                                filename: filename.to_string(),
                                content,
                            });
                        }
                        Err(e) => {
                            log::warn!("Failed to read XML file {filename}: {e}");
                        }
                    }
                }
            }
        }
    }

    // Sort by filename to ensure consistent ordering (meta.xml first, then alphabetically)
    xml_files.sort_by(|a, b| {
        if a.filename == "meta.xml" {
            std::cmp::Ordering::Less
        } else if b.filename == "meta.xml" {
            std::cmp::Ordering::Greater
        } else {
            a.filename.cmp(&b.filename)
        }
    });

    Ok(ArchiveMetadata { xml_files })
}

pub(crate) fn get_archives_dir<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> Result<PathBuf> {
    let base_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| ChuckError::Tauri(e.to_string()))?;
    // Use a dedicated subdirectory for archives to avoid conflicts with
    // other app data (e.g., WebView2's EBWebView directory on Windows)
    Ok(base_dir.join("archives"))
}

#[tauri::command]
pub async fn open_archive(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    path: String,
) -> Result<ArchiveInfo> {
    use std::sync::mpsc;

    let base_dir = get_archives_dir(app.clone())?;
    let path_clone = path.clone();

    // Emit initial importing status
    app.emit("archive-open-progress", ArchiveOpenProgress::Importing)
        .map_err(|e| ChuckError::Tauri(e.to_string()))?;

    // Drop the cached ZipArchive before opening a new archive. On Windows,
    // open file handles prevent deletion, so release it before the new archive
    // open removes old archive directories.
    if let Ok(mut guard) = app.state::<ZipState>().0.lock() {
        *guard = None;
    }

    // Create a channel for progress updates
    let (tx, rx) = mpsc::channel();

    // Spawn blocking task
    let app_for_thread = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let archive = Archive::open(
            Path::new(&path_clone),
            &base_dir,
            |stage| {
                let _ = tx.send(stage.to_string());
            },
        )?;
        // Parse the zip central directory once while still on a blocking thread.
        // Returns None on failure; get_photo will re-attempt lazily if needed.
        let zip_archive = build_zip_archive(&archive.storage_dir);
        Ok::<_, ChuckError>((archive, zip_archive))
    });

    // Listen for progress updates and emit events
    std::thread::spawn(move || {
        for stage in rx {
            let progress = match stage.as_str() {
                "importing" => ArchiveOpenProgress::Importing,
                "extracting" => ArchiveOpenProgress::Extracting,
                "creating_database" => ArchiveOpenProgress::CreatingDatabase,
                _ => continue,
            };
            let _ = app_for_thread.emit("archive-open-progress", progress);
        }
    });

    match result.await {
        Ok(Ok((archive, zip_archive))) => {
            let info = archive.info()?;

            if let Some(zip) = zip_archive {
                if let Ok(mut guard) = app.state::<ZipState>().0.lock() {
                    *guard = Some(zip);
                }
            }

            // Emit completion event
            app.emit(
                "archive-open-progress",
                ArchiveOpenProgress::Complete {
                    info: info.clone(),
                },
            )
            .map_err(|e| ChuckError::Tauri(e.to_string()))?;

            // Set window title from Rust for reliable cross-platform behavior.
            // The JS setTitle() call does not work on Linux (Ubuntu).
            set_archive_window_title(&window, &info);

            Ok(info)
        }
        Ok(Err(e)) => {
            log::debug!("Failed to open archive: {e}");

            // Emit error event
            app.emit(
                "archive-open-progress",
                ArchiveOpenProgress::Error {
                    message: e.to_string(),
                },
            )
            .map_err(|err| ChuckError::Tauri(err.to_string()))?;

            Err(e)
        }
        Err(e) => {
            let err_msg = format!("Task join error: {e}");
            log::error!("{err_msg}");

            app.emit(
                "archive-open-progress",
                ArchiveOpenProgress::Error {
                    message: err_msg.clone(),
                },
            )
            .ok();

            Err(ChuckError::Tauri(err_msg))
        }
    }
}

/// Returns and clears the file path passed via CLI args (file association on
/// Windows/Linux). Returns None if no file was passed or it was already consumed.
#[tauri::command]
pub fn get_opened_file(app: tauri::AppHandle) -> Option<String> {
    let state = app.state::<crate::OpenedFile>();
    state.0.lock().ok()?.take()
}

fn set_archive_window_title(window: &tauri::WebviewWindow, info: &ArchiveInfo) {
    let title = format!("{} \u{2013} {} occurrences", info.name, info.core_count);
    if let Err(e) = window.set_title(&title) {
        log::warn!("Failed to set window title: {e}");
    }

    // In GTK (default in Ubuntu), the above method of setting the window
    // title doesn't change the HeaderBar the user actually sees, so we're
    // using the approach described in
    // https://github.com/tauri-apps/tauri/issues/13749#issuecomment-3027697386.
    // This closure / ? approach is to avoid a ton of unwraps.
    #[cfg(target_os = "linux")]
    (|| -> Option<()> {
        let header_bar = window.gtk_window().ok()?
            .titlebar()?
            .downcast::<EventBox>().ok()?
            .child()?
            .downcast::<HeaderBar>().ok()?;
        header_bar.set_title(Some(&title));
        Some(())
    })();
}

#[tauri::command]
pub fn current_archive(app: tauri::AppHandle) -> Result<ArchiveInfo> {
    let info = Archive::current(&get_archives_dir(app.clone())?).map_err(|e| {
        log::error!(
            "Failed to get current archive: {}, backtrace: {}",
            e,
            Backtrace::capture()
        );
        e
    })?.info()?;
    // Set window title in a spawned task to avoid interfering with the command response.
    // Using WebviewWindow as a command parameter breaks the return value in Tauri 2.
    let app_clone = app;
    let info_clone = info.clone();
    tauri::async_runtime::spawn(async move {
        if let Some(window) = app_clone.get_webview_window("main") {
            set_archive_window_title(&window, &info_clone);
        }
    });
    Ok(info)
}

#[tauri::command]
pub fn search(
    app: tauri::AppHandle,
    limit: usize,
    offset: usize,
    search_params: SearchParams,
    fields: Option<Vec<String>>,
) -> Result<SearchResult> {
    let archive = Archive::current(&get_archives_dir(app)?).map_err(|e| {
        log::error!(
            "caught error opening current: {}, backtrace: {}",
            e,
            Backtrace::capture()
        );
        e
    })?;
    archive.search(limit, offset, search_params, fields).map_err(|e| {
        log::error!("caught search error: {}, backtrace: {}", e, Backtrace::capture());
        e
    })
}

#[tauri::command]
pub fn get_autocomplete_suggestions(
    app: tauri::AppHandle,
    column_name: String,
    search_term: String,
    limit: Option<usize>,
) -> Result<Vec<String>> {
    let archive = Archive::current(&get_archives_dir(app)?).map_err(|e| {
        log::error!(
            "caught error opening current: {}, backtrace: {}",
            e,
            Backtrace::capture()
        );
        e
    })?;
    archive.get_autocomplete_suggestions(&column_name, &search_term, limit.unwrap_or(50)).map_err(|e| {
        log::error!("caught autocomplete error: {}, backtrace: {}", e, Backtrace::capture());
        e
    })
}

#[tauri::command]
pub fn get_occurrence(
    app: tauri::AppHandle,
    occurrence_id: String,
) -> Result<serde_json::Map<String, serde_json::Value>> {
    let archive = Archive::current(&get_archives_dir(app)?)?;
    archive.get_occurrence(&occurrence_id)
}

/// Opens the archive zip and parses its central directory, returning a ZipArchive
/// ready for repeated photo lookups. Returns None and logs a warning on failure.
fn build_zip_archive(storage_dir: &Path) -> Option<zip::ZipArchive<std::fs::File>> {
    let zip_path = storage_dir.join("archive.zip");
    let file = match std::fs::File::open(&zip_path) {
        Ok(f) => f,
        Err(e) => {
            log::warn!("Failed to open archive.zip for zip index: {e}");
            return None;
        }
    };
    match zip::ZipArchive::new(file) {
        Ok(z) => {
            log::debug!("ZipArchive central directory cached ({} entries)", z.len());
            Some(z)
        }
        Err(e) => {
            log::warn!("Failed to parse zip central directory: {e}");
            None
        }
    }
}

#[tauri::command]
pub fn get_photo(
    app: tauri::AppHandle,
    zip_state: tauri::State<'_, ZipState>,
    photo_path: String,
) -> Result<String> {
    let archive = Archive::current(&get_archives_dir(app)?)?;

    let cache_dir = archive.storage_dir.join("photo_cache");
    std::fs::create_dir_all(&cache_dir).map_err(|e| ChuckError::DirectoryCreate {
        path: cache_dir.clone(),
        source: e,
    })?;
    let photo_cache = PhotoCache::new(&cache_dir);

    if let Some(cached_path) = photo_cache.get_cached_photo(&photo_path)? {
        photo_cache.touch_file(&cached_path)?;
        return Ok(cached_path.to_string_lossy().to_string());
    }

    let normalized_path = photo_path.replace('\\', "/");
    let cached_file_path = photo_cache.get_cache_path(&photo_path);

    if let Some(p) = cached_file_path.parent() {
        if !p.exists() {
            std::fs::create_dir_all(p).map_err(|e| ChuckError::DirectoryCreate {
                path: p.to_path_buf(),
                source: e,
            })?;
        }
    }

    // Use the shared ZipArchive so the central directory is only parsed once.
    // Initialise lazily here if open_archive hasn't run yet (e.g. after restart).
    {
        let mut guard = zip_state
            .0
            .lock()
            .map_err(|_| ChuckError::Tauri("ZipState mutex poisoned".to_string()))?;

        if guard.is_none() {
            *guard = build_zip_archive(&archive.storage_dir);
            if guard.is_none() {
                return Err(ChuckError::Tauri(
                    "Failed to open archive zip for photo extraction".to_string(),
                ));
            }
            log::debug!("ZipState initialised lazily in get_photo");
        }

        let zip = guard.as_mut().unwrap();
        let zip_file = zip
            .by_name(&normalized_path)
            .map_err(ChuckError::ArchiveExtraction)?;

        let outfile = std::fs::File::create(&cached_file_path).map_err(|e| ChuckError::FileOpen {
            path: cached_file_path.clone(),
            source: e,
        })?;

        let mut reader = BufReader::with_capacity(64 * 1024, zip_file);
        let mut writer = BufWriter::with_capacity(64 * 1024, outfile);
        std::io::copy(&mut reader, &mut writer).map_err(|e| ChuckError::FileRead {
            path: cached_file_path.clone(),
            source: e,
        })?;
    } // release the mutex before eviction

    const MAX_CACHE_SIZE: u64 = 2 * 1024 * 1024 * 1024;
    photo_cache.evict_lru(MAX_CACHE_SIZE)?;

    Ok(cached_file_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn aggregate_by_field(
    app: tauri::AppHandle,
    field_name: String,
    search_params: SearchParams,
    limit: usize,
) -> Result<Vec<crate::db::AggregationResult>> {
    let archive = Archive::current(&get_archives_dir(app)?).map_err(|e| {
        log::error!("caught error opening current: {}, backtrace: {}", e, Backtrace::capture());
        e
    })?;
    archive.aggregate_by_field(&field_name, &search_params, Some(limit)).map_err(|e| {
        log::error!("caught aggregate_by_field error: {}, backtrace: {}", e, Backtrace::capture());
        e
    })
}

#[tauri::command]
pub fn get_archive_metadata(app: tauri::AppHandle) -> Result<ArchiveMetadata> {
    let base_dir = get_archives_dir(app)?;
    let archive = Archive::current(&base_dir)?;
    get_metadata_from_storage(&archive.storage_dir)
}

#[tauri::command]
pub fn save_text_file(path: String, content: String) -> Result<()> {
    let p = std::path::PathBuf::from(&path);
    std::fs::write(&p, content.as_bytes()).map_err(|source| {
        crate::error::ChuckError::FileWrite { path: p, source }
    })
}

#[cfg(test)]
mod metadata_tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_get_archive_metadata_returns_both_files() {
        // Create temporary directory structure mimicking archive storage
        let temp_dir = std::env::temp_dir()
            .join("chuck_test_metadata_command");
        std::fs::create_dir_all(&temp_dir).unwrap();

        // Create storage subdirectory (simulating archive storage)
        let storage_dir = temp_dir.join("test-archive-abc123");
        std::fs::create_dir_all(&storage_dir).unwrap();

        // Create eml.xml
        let eml_content = r#"<?xml version="1.0"?>
<eml:eml xmlns:eml="eml://ecoinformatics.org/eml-2.1.1">
  <dataset>
    <title>Test Dataset</title>
  </dataset>
</eml:eml>"#;
        let mut eml_file = std::fs::File::create(storage_dir.join("eml.xml")).unwrap();
        eml_file.write_all(eml_content.as_bytes()).unwrap();

        // Create meta.xml
        let meta_content = r#"<?xml version="1.0"?>
<archive>
  <core>
    <files><location>occurrence.csv</location></files>
  </core>
</archive>"#;
        let mut meta_file = std::fs::File::create(storage_dir.join("meta.xml")).unwrap();
        meta_file.write_all(meta_content.as_bytes()).unwrap();

        // Create minimal CSV and database for Archive::current to work
        let csv_content = b"id\n1\n";
        std::fs::write(storage_dir.join("occurrence.csv"), csv_content).unwrap();
        let db_path = storage_dir.join("test-archive.db");
        let db = crate::db::Database::create_from_core_files(
            &[storage_dir.join("occurrence.csv")],
            &[],
            &db_path,
            "id"
        ).unwrap();
        drop(db);

        // Create mock app handle - this won't work with real AppHandle
        // We'll need to refactor to accept base_dir directly for testing
        // For now, test the internal function directly

        let result = get_metadata_from_storage(&storage_dir);
        assert!(result.is_ok());

        let metadata = result.unwrap();
        assert_eq!(metadata.xml_files.len(), 2);

        // meta.xml should be first
        assert_eq!(metadata.xml_files[0].filename, "meta.xml");
        assert_eq!(metadata.xml_files[0].content, meta_content);

        // Then eml.xml
        assert_eq!(metadata.xml_files[1].filename, "eml.xml");
        assert_eq!(metadata.xml_files[1].content, eml_content);

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_get_archive_metadata_handles_missing_eml() {
        let temp_dir = std::env::temp_dir()
            .join("chuck_test_metadata_missing_eml");
        std::fs::create_dir_all(&temp_dir).unwrap();

        let storage_dir = temp_dir.join("test-archive-xyz789");
        std::fs::create_dir_all(&storage_dir).unwrap();

        // Only create meta.xml, not eml.xml
        let meta_content = r#"<?xml version="1.0"?>
<archive>
  <core>
    <files><location>occurrence.csv</location></files>
  </core>
</archive>"#;
        std::fs::write(storage_dir.join("meta.xml"), meta_content).unwrap();

        let result = get_metadata_from_storage(&storage_dir);
        assert!(result.is_ok());

        let metadata = result.unwrap();
        assert_eq!(metadata.xml_files.len(), 1);
        assert_eq!(metadata.xml_files[0].filename, "meta.xml");
        assert_eq!(metadata.xml_files[0].content, meta_content);

        std::fs::remove_dir_all(&temp_dir).ok();
    }
}

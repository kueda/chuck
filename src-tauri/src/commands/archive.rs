use std::backtrace::Backtrace;
use std::path::{Path, PathBuf};
use serde::{Serialize};
use tauri::{Emitter, Manager};

use crate::dwca::Archive;
use crate::error::{ChuckError, Result};
use crate::search_params::SearchParams;

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
                            log::warn!("Failed to read XML file {}: {}", filename, e);
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
pub async fn open_archive(app: tauri::AppHandle, path: String) -> Result<ArchiveInfo> {
    use std::sync::mpsc;

    let base_dir = get_archives_dir(app.clone())?;
    let path_clone = path.clone();

    // Emit initial importing status
    app.emit("archive-open-progress", ArchiveOpenProgress::Importing)
        .map_err(|e| ChuckError::Tauri(e.to_string()))?;

    // Create a channel for progress updates
    let (tx, rx) = mpsc::channel();

    // Spawn blocking task
    let app_for_thread = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        Archive::open_with_progress(
            Path::new(&path_clone),
            &base_dir,
            |stage| {
                let _ = tx.send(stage.to_string());
            },
        )
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
        Ok(Ok(archive)) => {
            let info = archive.info()?;

            // Emit completion event
            app.emit(
                "archive-open-progress",
                ArchiveOpenProgress::Complete {
                    info: info.clone(),
                },
            )
            .map_err(|e| ChuckError::Tauri(e.to_string()))?;

            Ok(info)
        }
        Ok(Err(e)) => {
            log::debug!("Failed to open archive: {}", e);

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
            let err_msg = format!("Task join error: {}", e);
            log::error!("{}", err_msg);

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

#[tauri::command]
pub fn current_archive(app: tauri::AppHandle) -> Result<ArchiveInfo> {
    Archive::current(&get_archives_dir(app)?).map_err(|e| {
        log::error!(
            "Failed to get current archive: {}, backtrace: {}",
            e,
            Backtrace::capture()
        );
        e
    })?.info()
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

#[tauri::command]
pub fn get_photo(
    app: tauri::AppHandle,
    photo_path: String,
) -> Result<String> {
    let archive = Archive::current(&get_archives_dir(app)?)?;
    archive.get_photo(&photo_path)
}

#[tauri::command]
pub fn aggregate_by_field(
    app: tauri::AppHandle,
    field_name: String,
    search_params: SearchParams,
    limit: usize,
) -> Result<Vec<crate::db::AggregationResult>> {
    let archive = Archive::current(&get_archives_dir(app)?)?;
    archive.aggregate_by_field(&field_name, &search_params, limit)
}

#[tauri::command]
pub fn get_archive_metadata(app: tauri::AppHandle) -> Result<ArchiveMetadata> {
    let base_dir = get_archives_dir(app)?;
    let archive = Archive::current(&base_dir)?;
    get_metadata_from_storage(&archive.storage_dir)
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
            &vec![storage_dir.join("occurrence.csv")],
            &vec![],
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

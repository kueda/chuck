use std::backtrace::Backtrace;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};

use crate::dwca::Archive;
use crate::error::{ChuckError, Result};

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

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct SearchParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scientific_name: Option<String>,
    pub order_by: Option<String>,
    pub order: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub total: usize,
    pub results: Vec<serde_json::Map<String, serde_json::Value>>,
}

fn get_local_data_dir(app: tauri::AppHandle) -> Result<PathBuf> {
    app
        .path()
        .app_local_data_dir()
        .map_err(|e| ChuckError::Tauri(e.to_string()))
}

#[tauri::command]
pub async fn open_archive(app: tauri::AppHandle, path: String) -> Result<ArchiveInfo> {
    use std::sync::mpsc;

    let base_dir = get_local_data_dir(app.clone())?;
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
    Archive::current(&get_local_data_dir(app)?).map_err(|e| {
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
    let archive = Archive::current(&get_local_data_dir(app)?).map_err(|e| {
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
pub fn get_occurrence(
    app: tauri::AppHandle,
    occurrence_id: String,
) -> Result<serde_json::Map<String, serde_json::Value>> {
    let archive = Archive::current(&get_local_data_dir(app)?)?;
    archive.get_occurrence(&occurrence_id)
}

#[tauri::command]
pub fn get_photo(
    app: tauri::AppHandle,
    photo_path: String,
) -> Result<String> {
    let archive = Archive::current(&get_local_data_dir(app)?)?;
    archive.get_photo(&photo_path)
}

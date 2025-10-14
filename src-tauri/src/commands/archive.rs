use std::path::Path;
use serde::Serialize;
use crate::dwca::Archive;
use crate::error::{ChuckError, Result};

#[derive(Debug, Serialize)]
pub struct ArchiveInfo {
    pub name: String,
    pub core_count: usize,
}

#[tauri::command]
pub fn open_archive(app: tauri::AppHandle, path: &str) -> Result<ArchiveInfo> {
    use tauri::Manager;

    let archive_path = Path::new(path);
    let local_data_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| ChuckError::Tauri(e.to_string()))?;

    let archive = Archive::open(archive_path, &local_data_dir)?;
    archive.info()
}

#[tauri::command]
pub fn current_archive(app: tauri::AppHandle) -> Result<ArchiveInfo> {
    use tauri::Manager;

    let local_data_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| ChuckError::Tauri(e.to_string()))?;

    let archive = Archive::current(&local_data_dir)?;
    archive.info()
}

use std::path::Path;
use crate::dwca::Archive;
use crate::error::{ChuckError, Result};

#[tauri::command]
pub fn open_archive(app: tauri::AppHandle, path: &str) -> Result<usize> {
    use tauri::Manager;

    let archive_path = Path::new(path);
    let local_data_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| ChuckError::Tauri(e.to_string()))?;

    let archive = Archive::open(archive_path, &local_data_dir)?;
    let count = archive.core_count()?;

    Ok(count)
}

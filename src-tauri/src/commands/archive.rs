use std::path::{Path, PathBuf};
use serde::Serialize;
use tauri::Manager;

use crate::dwca::Archive;
use crate::error::{ChuckError, Result};

#[derive(Debug, Serialize)]
pub struct ArchiveInfo {
    pub name: String,

    #[serde(rename = "coreCount")]
    pub core_count: usize,
}

fn get_local_data_dir(app: tauri::AppHandle) -> Result<PathBuf> {
    app
        .path()
        .app_local_data_dir()
        .map_err(|e| ChuckError::Tauri(e.to_string()))
}

#[tauri::command]
pub fn open_archive(app: tauri::AppHandle, path: &str) -> Result<ArchiveInfo> {
    Archive::open(Path::new(path), &get_local_data_dir(app)?)?.info()
}

#[tauri::command]
pub fn current_archive(app: tauri::AppHandle) -> Result<ArchiveInfo> {
    Archive::current(&get_local_data_dir(app)?)?.info()
}

#[tauri::command]
pub fn search(app: tauri::AppHandle) -> Result<Vec<chuck_core::darwin_core::Occurrence>> {
    let archive = Archive::current(&get_local_data_dir(app)?)?;
    archive.search(100).map_err(|e| {
        println!("caught search error: {}", e);
        e
    })
}

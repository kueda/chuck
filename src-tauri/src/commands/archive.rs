use std::path::Path;
use crate::dwca::{Archive, create_storage_dir};
use crate::db::Database;
use crate::error::{ChuckError, Result};

#[tauri::command]
pub fn open_archive(app: tauri::AppHandle, path: &str) -> Result<usize> {
    use tauri::Manager;

    let archive_path = Path::new(path);
    let local_data_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| ChuckError::Tauri(e.to_string()))?;

    let storage_dir = create_storage_dir(archive_path, &local_data_dir)?;

    let archive = Archive::open(archive_path, &storage_dir)?;

    let db_name = archive.archive_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("archive");
    let db_path = archive.storage_dir.join(format!("{}.db", db_name));

    let db = Database::create_from_core_files(archive.core_files(), &db_path)?;

    let count = db.count_records()?;

    Ok(count)
}

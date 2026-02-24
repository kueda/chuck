mod csv;
mod dwca;
mod groups;
mod kml;

use crate::commands::archive::get_archives_dir;
use crate::error::Result;
use crate::search_params::SearchParams;

/// Escapes a CSV field value per RFC 4180
fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

#[tauri::command]
pub fn export_csv(
    app: tauri::AppHandle,
    search_params: SearchParams,
    path: String,
) -> Result<()> {
    csv::export_csv(app, search_params, path)
}

#[tauri::command]
pub fn export_kml(
    app: tauri::AppHandle,
    search_params: SearchParams,
    path: String,
) -> Result<()> {
    kml::export_kml(app, search_params, path)
}

#[tauri::command]
pub fn export_groups_csv(
    app: tauri::AppHandle,
    search_params: SearchParams,
    field_name: String,
    path: String,
) -> Result<()> {
    groups::export_groups_csv(app, search_params, field_name, path)
}

#[tauri::command]
pub fn export_dwca(
    app: tauri::AppHandle,
    search_params: SearchParams,
    path: String,
) -> Result<()> {
    dwca::export_dwca_inner(get_archives_dir(app)?, search_params, path)
}

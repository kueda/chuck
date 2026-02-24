use std::path::PathBuf;

use crate::commands::archive::get_archives_dir;
use crate::db::AggregationResult;
use crate::dwca::Archive;
use crate::error::{ChuckError, Result};
use crate::search_params::SearchParams;

use super::csv_escape;

/// Builds a CSV string from aggregation results, using the field name as the
/// first column header and `occurrence_count` as the second.
fn build_groups_csv(field_name: &str, rows: &[AggregationResult]) -> String {
    let mut output = String::new();
    output.push_str(&csv_escape(field_name));
    output.push_str(",occurrence_count\n");
    for row in rows {
        let value = row.value.as_deref().unwrap_or("");
        output.push_str(&csv_escape(value));
        output.push(',');
        output.push_str(&row.count.to_string());
        output.push('\n');
    }
    output
}

/// Exports aggregated group counts as a CSV file
pub(super) fn export_groups_csv(
    app: tauri::AppHandle,
    search_params: SearchParams,
    field_name: String,
    path: String,
) -> Result<()> {
    let archive = Archive::current(&get_archives_dir(app)?)?;
    let rows = archive.aggregate_by_field(&field_name, &search_params, None)?;
    let csv = build_groups_csv(&field_name, &rows);
    let dest = PathBuf::from(&path);
    std::fs::write(&dest, csv).map_err(|source| ChuckError::FileWrite {
        path: dest,
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_agg(value: Option<&str>, count: i64) -> AggregationResult {
        AggregationResult { value: value.map(|s| s.to_string()), count, photo_url: None }
    }

    #[test]
    fn test_build_groups_csv_writes_header_and_rows() {
        let rows = vec![
            make_agg(Some("Homo sapiens"), 12),
            make_agg(Some("Canis lupus"), 3),
        ];
        let csv = build_groups_csv("scientificName", &rows);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines[0], "scientificName,occurrence_count");
        assert_eq!(lines[1], "Homo sapiens,12");
        assert_eq!(lines[2], "Canis lupus,3");
    }

    #[test]
    fn test_build_groups_csv_handles_null_value() {
        let rows = vec![make_agg(None, 5)];
        let csv = build_groups_csv("scientificName", &rows);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines[0], "scientificName,occurrence_count");
        assert_eq!(lines[1], ",5");
    }

    #[test]
    fn test_build_groups_csv_escapes_commas_in_values() {
        let rows = vec![make_agg(Some("Smith, Jane"), 2)];
        let csv = build_groups_csv("recordedBy", &rows);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines[1], "\"Smith, Jane\",2");
    }

    #[test]
    fn test_build_groups_csv_empty_results() {
        let csv = build_groups_csv("scientificName", &[]);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], "scientificName,occurrence_count");
    }
}

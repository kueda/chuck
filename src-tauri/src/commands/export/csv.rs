use std::collections::BTreeSet;
use std::path::PathBuf;

use serde_json::{Map, Value};

use crate::commands::archive::get_archives_dir;
use crate::dwca::Archive;
use crate::error::{ChuckError, Result};
use crate::search_params::SearchParams;

use super::csv_escape;

/// Builds a CSV string from a slice of occurrence rows
fn build_csv(rows: &[Map<String, Value>]) -> String {
    let columns: BTreeSet<String> = rows
        .iter()
        .flat_map(|row| row.keys().cloned())
        .collect();

    let mut output = String::new();

    // Header row
    let header: Vec<String> = columns.iter().map(|c| csv_escape(c)).collect();
    output.push_str(&header.join(","));
    output.push('\n');

    // Data rows
    for row in rows {
        let fields: Vec<String> = columns
            .iter()
            .map(|col| match row.get(col) {
                None | Some(Value::Null) => String::new(),
                Some(Value::String(s)) => csv_escape(s),
                Some(other) => csv_escape(&other.to_string()),
            })
            .collect();
        output.push_str(&fields.join(","));
        output.push('\n');
    }

    output
}

/// Exports filtered occurrences as a CSV file
pub(super) fn export_csv(
    app: tauri::AppHandle,
    search_params: SearchParams,
    path: String,
) -> Result<()> {
    let archive = Archive::current(&get_archives_dir(app)?)?;
    let rows = archive.query_all_occurrences(search_params)?;
    let csv = build_csv(&rows);
    let dest = PathBuf::from(&path);
    std::fs::write(&dest, csv).map_err(|source| ChuckError::FileWrite {
        path: dest,
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_row(fields: &[(&str, serde_json::Value)]) -> Map<String, serde_json::Value> {
        fields.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()
    }

    #[test]
    fn test_build_csv_writes_header_and_rows() {
        let rows = vec![
            make_row(&[
                ("occurrenceID", json!("abc-1")),
                ("scientificName", json!("Homo sapiens")),
            ]),
            make_row(&[
                ("occurrenceID", json!("abc-2")),
                ("scientificName", json!("Canis lupus")),
            ]),
        ];
        let csv = build_csv(&rows);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines[0], "occurrenceID,scientificName");
        assert_eq!(lines[1], "abc-1,Homo sapiens");
        assert_eq!(lines[2], "abc-2,Canis lupus");
    }

    #[test]
    fn test_build_csv_escapes_commas_and_quotes() {
        let rows = vec![make_row(&[
            ("name", json!("Smith, John")),
            ("note", json!("said \"hello\"")),
        ])];
        let csv = build_csv(&rows);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines[1], "\"Smith, John\",\"said \"\"hello\"\"\"");
    }

    #[test]
    fn test_build_csv_uses_empty_string_for_null_or_missing() {
        let rows = vec![
            make_row(&[
                ("a", json!("present")),
                ("b", serde_json::Value::Null),
            ]),
            make_row(&[("a", json!("only_a"))]),
        ];
        let csv = build_csv(&rows);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines[0], "a,b");
        assert_eq!(lines[1], "present,");
        assert_eq!(lines[2], "only_a,");
    }
}

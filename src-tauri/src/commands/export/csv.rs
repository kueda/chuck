use std::io::{BufWriter, Write};
use std::path::PathBuf;

use serde_json::Value;

use crate::commands::archive::get_archives_dir;
use crate::dwca::Archive;
use crate::error::{ChuckError, Result};
use crate::search_params::SearchParams;

use super::csv_escape;

/// Exports filtered occurrences as a CSV file, streaming rows directly to
/// disk via BufWriter to avoid materialising the full result set in memory.
pub(super) fn export_csv(
    app: tauri::AppHandle,
    search_params: SearchParams,
    path: String,
) -> Result<()> {
    export_csv_inner(get_archives_dir(app)?, search_params, path)
}

pub(super) fn export_csv_inner(
    archives_dir: PathBuf,
    search_params: SearchParams,
    path: String,
) -> Result<()> {
    let archive = Archive::current(&archives_dir)?;
    let dest = PathBuf::from(&path);
    let file = std::fs::File::create(&dest).map_err(|e| ChuckError::FileOpen {
        path: dest.clone(),
        source: e,
    })?;
    let mut writer = BufWriter::new(file);
    let mut header_written = false;

    archive.for_each_occurrence(search_params, |columns, row| {
        if !header_written {
            let header = columns.iter().map(|c| csv_escape(c)).collect::<Vec<_>>().join(",");
            writer.write_all(header.as_bytes())
                .and_then(|_| writer.write_all(b"\n"))
                .map_err(|e| ChuckError::FileWrite { path: dest.clone(), source: e })?;
            header_written = true;
        }
        let fields: Vec<String> = columns
            .iter()
            .map(|col| match row.get(col) {
                None | Some(Value::Null) => String::new(),
                Some(Value::String(s)) => csv_escape(s),
                Some(other) => csv_escape(&other.to_string()),
            })
            .collect();
        writer.write_all(fields.join(",").as_bytes())
            .and_then(|_| writer.write_all(b"\n"))
            .map_err(|e| ChuckError::FileWrite { path: dest.clone(), source: e })
    })?;

    writer.flush().map_err(|e| ChuckError::FileWrite { path: dest, source: e })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    struct ArchiveFixture {
        _temp: tempfile::TempDir,
        archives_dir: PathBuf,
        output: PathBuf,
    }

    fn setup_archive(csv_content: &str) -> ArchiveFixture {
        let temp = tempfile::tempdir().unwrap();
        let archives_dir = temp.path().to_path_buf();
        let storage_dir = archives_dir.join("test.zip-abc123");
        std::fs::create_dir_all(&storage_dir).unwrap();

        let meta_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<archive xmlns="http://rs.tdwg.org/dwc/text/">
  <core rowType="http://rs.tdwg.org/dwc/terms/Occurrence" fieldsTerminatedBy=",">
    <files><location>occurrence.csv</location></files>
    <id index="0"/>
    <field index="0" term="http://rs.tdwg.org/dwc/terms/occurrenceID"/>
  </core>
</archive>"#;
        std::fs::write(storage_dir.join("meta.xml"), meta_xml).unwrap();
        std::fs::write(storage_dir.join("occurrence.csv"), csv_content).unwrap();

        let db_path = storage_dir.join("test.db");
        let db = Database::create_from_core_files(
            &[storage_dir.join("occurrence.csv")],
            &[],
            &db_path,
            "occurrenceID",
        )
        .unwrap();
        drop(db);

        let output = archives_dir.join("out.csv");
        ArchiveFixture { _temp: temp, archives_dir, output }
    }

    #[test]
    fn test_export_csv_writes_header_and_rows() {
        let csv = "occurrenceID,scientificName\nabc-1,Homo sapiens\nabc-2,Canis lupus\n";
        let fixture = setup_archive(csv);

        export_csv_inner(
            fixture.archives_dir.clone(),
            SearchParams::default(),
            fixture.output.to_string_lossy().to_string(),
        )
        .unwrap();

        let result = std::fs::read_to_string(&fixture.output).unwrap();
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines[0], "occurrenceID,scientificName");
        assert!(result.contains("abc-1,Homo sapiens"), "row 1 missing: {result}");
        assert!(result.contains("abc-2,Canis lupus"), "row 2 missing: {result}");
    }

    #[test]
    fn test_export_csv_escapes_commas_and_quotes() {
        // DuckDB reads the CSV (unquoting as needed) and stores the raw string
        // values. The exporter must re-escape them when writing the output CSV.
        let csv =
            "occurrenceID,name,note\nocc-1,\"Smith, John\",\"said \"\"hello\"\"\"\n";
        let fixture = setup_archive(csv);

        export_csv_inner(
            fixture.archives_dir.clone(),
            SearchParams::default(),
            fixture.output.to_string_lossy().to_string(),
        )
        .unwrap();

        let result = std::fs::read_to_string(&fixture.output).unwrap();
        assert!(
            result.contains("\"Smith, John\""),
            "comma not escaped: {result}"
        );
        assert!(
            result.contains("\"said \"\"hello\"\"\""),
            "quote not escaped: {result}"
        );
    }

    #[test]
    fn test_export_csv_uses_empty_string_for_null() {
        // Row 1 has a value for b (so DuckDB keeps the column); row 2 leaves
        // b empty. The exporter must produce an empty field for the NULL value.
        let csv = "occurrenceID,a,b\nocc-1,present,has_value\nocc-2,only_a,\n";
        let fixture = setup_archive(csv);

        export_csv_inner(
            fixture.archives_dir.clone(),
            SearchParams::default(),
            fixture.output.to_string_lossy().to_string(),
        )
        .unwrap();

        let result = std::fs::read_to_string(&fixture.output).unwrap();
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines[0], "occurrenceID,a,b", "header: {result}");
        assert!(lines[1].contains(",present,has_value"), "row 1: {}", lines[1]);
        assert!(lines[2].ends_with(",only_a,"), "row 2 b should be empty: {}", lines[2]);
    }
}

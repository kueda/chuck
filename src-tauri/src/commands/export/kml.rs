use std::collections::BTreeMap;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use serde_json::{Map, Value};

use crate::commands::archive::get_archives_dir;
use crate::dwca::Archive;
use crate::error::{ChuckError, Result};
use crate::search_params::SearchParams;

/// Escapes XML special characters in a string
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Formats one occurrence row as a KML `<Placemark>`, or returns `None` if
/// the row has no coordinates.
fn format_placemark(row: &Map<String, Value>, core_id_column: &str) -> Option<String> {
    let lat = row.get("decimalLatitude").and_then(|v| v.as_f64())?;
    let lng = row.get("decimalLongitude").and_then(|v| v.as_f64())?;

    let name = row
        .get("scientificName")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(xml_escape)
        .or_else(|| {
            row.get(core_id_column)
                .map(|v| xml_escape(v.as_str().unwrap_or(&v.to_string())))
        })
        .unwrap_or_default();

    let sorted: BTreeMap<&str, &Value> = row.iter().map(|(k, v)| (k.as_str(), v)).collect();
    let mut extended_data = String::new();
    for (key, value) in &sorted {
        match value {
            Value::Null => continue,
            Value::Array(_) | Value::Object(_) => continue,
            Value::String(s) if s.is_empty() => continue,
            _ => {
                let display = match value {
                    Value::String(s) => xml_escape(s),
                    other => xml_escape(&other.to_string()),
                };
                extended_data.push_str(&format!(
                    "      <Data name=\"{}\"><value>{}</value></Data>\n",
                    xml_escape(key),
                    display,
                ));
            }
        }
    }

    Some(format!(
        "  <Placemark>\n    <name>{name}</name>\n\
         <Point><coordinates>{lng},{lat},0</coordinates></Point>\n\
         <ExtendedData>\n{extended_data}    </ExtendedData>\n  </Placemark>\n"
    ))
}

/// Exports filtered occurrences as a KML file
pub(super) fn export_kml(
    app: tauri::AppHandle,
    search_params: SearchParams,
    path: String,
) -> Result<()> {
    export_kml_inner(get_archives_dir(app)?, search_params, path)
}

pub(super) fn export_kml_inner(
    archives_dir: PathBuf,
    search_params: SearchParams,
    path: String,
) -> Result<()> {
    let archive = Archive::current(&archives_dir)?;
    let core_id_column = archive.core_id_column.clone();
    let dest = PathBuf::from(&path);
    let file = std::fs::File::create(&dest).map_err(|e| ChuckError::FileOpen {
        path: dest.clone(),
        source: e,
    })?;
    let mut writer = BufWriter::new(file);

    writer
        .write_all(
            b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
              <kml xmlns=\"http://www.opengis.net/kml/2.2\">\n\
              <Document>\n",
        )
        .map_err(|e| ChuckError::FileWrite { path: dest.clone(), source: e })?;

    archive.for_each_occurrence(search_params, |_columns, row| {
        if let Some(placemark) = format_placemark(&row, &core_id_column) {
            writer
                .write_all(placemark.as_bytes())
                .map_err(|e| ChuckError::FileWrite { path: dest.clone(), source: e })?;
        }
        Ok(())
    })?;

    writer
        .write_all(b"</Document>\n</kml>\n")
        .and_then(|_| writer.flush())
        .map_err(|e| ChuckError::FileWrite { path: dest, source: e })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn setup_archive(test_name: &str, csv_content: &str) -> (PathBuf, PathBuf) {
        let base_dir =
            std::env::temp_dir().join(format!("chuck_test_export_kml_{test_name}"));
        std::fs::remove_dir_all(&base_dir).ok();
        let storage_dir = base_dir.join("test.zip-abc123");
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

        let output =
            std::env::temp_dir().join(format!("chuck_test_export_kml_{test_name}_out.kml"));
        (base_dir, output)
    }

    #[test]
    fn test_export_kml_includes_placemarks_with_coordinates() {
        let csv = "occurrenceID,decimalLatitude,decimalLongitude,scientificName\n\
                   obs1,37.5,-122.0,Homo sapiens\n";
        let (base_dir, out) = setup_archive("placemarks", csv);

        export_kml_inner(
            base_dir.clone(),
            SearchParams::default(),
            out.to_string_lossy().to_string(),
        )
        .unwrap();

        let result = std::fs::read_to_string(&out).unwrap();
        assert!(result.contains("<Placemark>"), "no placemark: {result}");
        assert!(result.contains("<name>Homo sapiens</name>"), "{result}");
        assert!(result.contains("<coordinates>-122,37.5,0</coordinates>"), "{result}");

        std::fs::remove_dir_all(&base_dir).ok();
        std::fs::remove_file(&out).ok();
    }

    #[test]
    fn test_export_kml_skips_occurrences_without_coordinates() {
        let csv = "occurrenceID,decimalLatitude,decimalLongitude,scientificName\n\
                   obs1,37.5,,With lat only\n\
                   obs2,,-122.0,With lng only\n\
                   obs3,,,No coords\n";
        let (base_dir, out) = setup_archive("no_coords", csv);

        export_kml_inner(
            base_dir.clone(),
            SearchParams::default(),
            out.to_string_lossy().to_string(),
        )
        .unwrap();

        let result = std::fs::read_to_string(&out).unwrap();
        assert!(!result.contains("<Placemark>"), "unexpected placemark: {result}");

        std::fs::remove_dir_all(&base_dir).ok();
        std::fs::remove_file(&out).ok();
    }

    #[test]
    fn test_export_kml_uses_core_id_as_name_when_no_scientific_name() {
        let csv = "occurrenceID,decimalLatitude,decimalLongitude\nobs-abc,10.0,20.0\n";
        let (base_dir, out) = setup_archive("core_id_name", csv);

        export_kml_inner(
            base_dir.clone(),
            SearchParams::default(),
            out.to_string_lossy().to_string(),
        )
        .unwrap();

        let result = std::fs::read_to_string(&out).unwrap();
        assert!(result.contains("<name>obs-abc</name>"), "{result}");

        std::fs::remove_dir_all(&base_dir).ok();
        std::fs::remove_file(&out).ok();
    }

    #[test]
    fn test_export_kml_escapes_xml_special_chars() {
        let csv = "occurrenceID,decimalLatitude,decimalLongitude,scientificName,occurrenceRemarks\n\
                   obs1,1.0,2.0,\"A & B <species>\",\"note with \"\"quotes\"\"\"\n";
        let (base_dir, out) = setup_archive("xml_escape", csv);

        export_kml_inner(
            base_dir.clone(),
            SearchParams::default(),
            out.to_string_lossy().to_string(),
        )
        .unwrap();

        let result = std::fs::read_to_string(&out).unwrap();
        assert!(result.contains("A &amp; B &lt;species&gt;"), "{result}");
        assert!(result.contains("note with &quot;quotes&quot;"), "{result}");

        std::fs::remove_dir_all(&base_dir).ok();
        std::fs::remove_file(&out).ok();
    }
}

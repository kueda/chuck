use std::collections::BTreeMap;
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

/// Builds a KML string from a slice of occurrence rows
fn build_kml(rows: &[Map<String, Value>], core_id_column: &str) -> String {
    let mut placemarks = String::new();

    for row in rows {
        let lat = match row.get("decimalLatitude").and_then(|v| v.as_f64()) {
            Some(v) => v,
            None => continue,
        };
        let lng = match row.get("decimalLongitude").and_then(|v| v.as_f64()) {
            Some(v) => v,
            None => continue,
        };

        let name = row
            .get("scientificName")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(xml_escape)
            .or_else(|| {
                row.get(core_id_column).map(|v| {
                    xml_escape(v.as_str().unwrap_or(&v.to_string()))
                })
            })
            .unwrap_or_default();

        // Build ExtendedData from all non-null, non-empty scalar fields
        // Sort keys for deterministic output
        let sorted: BTreeMap<&str, &Value> = row
            .iter()
            .map(|(k, v)| (k.as_str(), v))
            .collect();

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

        placemarks.push_str(&format!(
            "  <Placemark>\n    <name>{name}</name>\n    <Point><coordinates>{lng},{lat},0</coordinates></Point>\n    <ExtendedData>\n{extended_data}    </ExtendedData>\n  </Placemark>\n"
        ));
    }

    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<kml xmlns=\"http://www.opengis.net/kml/2.2\">\n<Document>\n{placemarks}</Document>\n</kml>\n"
    )
}

/// Exports filtered occurrences as a KML file
pub(super) fn export_kml(
    app: tauri::AppHandle,
    search_params: SearchParams,
    path: String,
) -> Result<()> {
    let archive = Archive::current(&get_archives_dir(app)?)?;
    let core_id_column = archive.core_id_column.clone();
    let rows = archive.query_all_occurrences(search_params)?;
    let kml = build_kml(&rows, &core_id_column);
    let dest = PathBuf::from(&path);
    std::fs::write(&dest, kml).map_err(|source| ChuckError::FileWrite {
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
    fn test_build_kml_includes_placemarks_with_coordinates() {
        let rows = vec![make_row(&[
            ("decimalLatitude", json!(37.5)),
            ("decimalLongitude", json!(-122.0)),
            ("scientificName", json!("Homo sapiens")),
        ])];
        let kml = build_kml(&rows, "occurrenceID");
        assert!(kml.contains("<Placemark>"));
        assert!(kml.contains("<name>Homo sapiens</name>"));
        assert!(kml.contains("<coordinates>-122,37.5,0</coordinates>"));
    }

    #[test]
    fn test_build_kml_skips_occurrences_without_coordinates() {
        let rows = vec![
            make_row(&[
                ("decimalLatitude", json!(37.5)),
                ("scientificName", json!("With lat only")),
            ]),
            make_row(&[
                ("decimalLongitude", json!(-122.0)),
                ("scientificName", json!("With lng only")),
            ]),
            make_row(&[("scientificName", json!("No coords"))]),
        ];
        let kml = build_kml(&rows, "occurrenceID");
        assert!(!kml.contains("<Placemark>"));
    }

    #[test]
    fn test_build_kml_uses_core_id_as_name_when_no_scientific_name() {
        let rows = vec![make_row(&[
            ("decimalLatitude", json!(10.0)),
            ("decimalLongitude", json!(20.0)),
            ("occurrenceID", json!("abc-123")),
        ])];
        let kml = build_kml(&rows, "occurrenceID");
        assert!(kml.contains("<name>abc-123</name>"));
    }

    #[test]
    fn test_build_kml_escapes_xml_special_chars() {
        let rows = vec![make_row(&[
            ("decimalLatitude", json!(1.0)),
            ("decimalLongitude", json!(2.0)),
            ("scientificName", json!("A & B <species>")),
            ("occurrenceRemarks", json!("note with \"quotes\"")),
        ])];
        let kml = build_kml(&rows, "occurrenceID");
        assert!(kml.contains("A &amp; B &lt;species&gt;"));
        assert!(kml.contains("note with &quot;quotes&quot;"));
    }
}

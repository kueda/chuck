use std::path::{Path, PathBuf};
use std::collections::HashMap;
use duckdb::Row;
use chuck_core::darwin_core::Occurrence;

use crate::error::{ChuckError, Result};
use crate::dwca::ExtensionInfo;

// Most DwC attributes are strings, but a few should have different types to
// enable better queries
const TYPE_OVERRIDES: [(&str, &str); 4] = [
    ("decimalLatitude", "DOUBLE"),
    ("decimalLonglongitude", "DOUBLE"),
    ("eventDate", "DATE"),
    ("gbifID", "BIGINT"),
];

/// Represents a DuckDB database for Darwin Core Archive data
pub struct Database {
    conn: duckdb::Connection,
    /// Extension table metadata: (table_name, core_id_column)
    extension_tables: Vec<(String, String)>,
}

impl Database {

    /// Creates a new database from core files and extension files
    pub fn create_from_core_files(
        core_files: &[PathBuf],
        extensions: &[ExtensionInfo],
        db_path: &Path,
    ) -> Result<Self> {
        if core_files.is_empty() {
            return Err(ChuckError::NoCoreFiles);
        }

        let conn = duckdb::Connection::open(db_path)?;

        // Try to create table from first file
        let first_file = core_files[0]
            .to_str()
            .ok_or(ChuckError::PathEncoding)?;

        // In order to set some of the column types using the types arg of
        // read_csv below, we need to know what columns are present in the
        // file, or read_csv will error out when we tell it to use types for
        // columns that don't exist
        let mut stmt = conn.prepare(&format!(
            "SELECT unnest(Columns).name FROM sniff_csv('{}')",
            first_file
        ))?;
        let column_names: Vec<String> = stmt.query_map([], |row| {
            row.get(0)
        })?.collect::<std::result::Result<Vec<_>, _>>()?;
        let type_map: HashMap<&str, &str> = TYPE_OVERRIDES
            .iter()
            .filter(|(col, _)| column_names.contains(&col.to_string()))
            .copied()
            .collect();
        // Convert to DuckDB's JSON format: {'col1': 'TYPE1', 'col2': 'TYPE2'}
        // Only include types parameter if we have overrides
        let types_param = if type_map.is_empty() {
            String::new()
        } else {
            let pairs: Vec<String> = type_map
                .iter()
                .map(|(col, typ)| format!("'{}': '{}'", col, typ))
                .collect();
            format!(", types = {{{}}}", pairs.join(", "))
        };

        // Try to create table with specific types for known columns if they exist
        let sql = format!(
            "CREATE TABLE occurrences AS SELECT * FROM read_csv('{}', all_varchar = true{})",
            first_file,
            types_param
        );
        let create_result = conn.execute(&sql, []);

        // If table already exists, insert from first file instead
        match create_result {
            Ok(_) => {
                // Insert remaining files with the same type overrides
                for core_file in &core_files[1..] {
                    let csv_path = core_file
                        .to_str()
                        .ok_or(ChuckError::PathEncoding)?;

                    conn.execute(
                        &format!(
                            "INSERT INTO occurrences SELECT * FROM read_csv('{}', all_varchar = true{})",
                            csv_path,
                            types_param
                        ),
                        [],
                    )?;
                }
            },
            Err(e) => {
                let error_msg = e.to_string();
                if error_msg.contains("already exists") || error_msg.contains("Table with name") {
                    // We've previously created this db file, nothing to do
                } else {
                    return Err(e.into());
                }
            }
        }

        // Create extension tables
        let extension_tables = Self::create_extension_tables(&conn, extensions)?;

        Ok(Self { conn, extension_tables })
    }

    /// Creates tables for DarwinCore Archive extensions
    fn create_extension_tables(
        conn: &duckdb::Connection,
        extensions: &[ExtensionInfo],
    ) -> Result<Vec<(String, String)>> {
        let mut created_tables = Vec::new();

        for ext in extensions {
            // Check if the CSV file exists
            if !ext.location.exists() {
                log::warn!(
                    "Extension file does not exist: {}. Skipping.",
                    ext.location.display()
                );
                continue;
            }

            let csv_path = ext
                .location
                .to_str()
                .ok_or(ChuckError::PathEncoding)?;

            // Sniff the CSV to get column names
            let mut stmt = conn.prepare(&format!(
                "SELECT unnest(Columns).name FROM sniff_csv('{}')",
                csv_path
            ))?;
            let column_names: Vec<String> = stmt
                .query_map([], |row| row.get(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;

            // Apply type overrides for known numeric/date columns
            let type_map: HashMap<&str, &str> = TYPE_OVERRIDES
                .iter()
                .filter(|(col, _)| column_names.contains(&col.to_string()))
                .copied()
                .collect();

            let types_param = if type_map.is_empty() {
                String::new()
            } else {
                let pairs: Vec<String> = type_map
                    .iter()
                    .map(|(col, typ)| format!("'{}': '{}'", col, typ))
                    .collect();
                format!(", types = {{{}}}", pairs.join(", "))
            };

            // Try to create the table
            let sql = format!(
                "CREATE TABLE {} AS SELECT * FROM read_csv('{}', all_varchar = true{})",
                ext.table_name, csv_path, types_param
            );

            let create_result = conn.execute(&sql, []);

            match create_result {
                Ok(_) => {
                    log::info!("Created extension table: {} (joins on {})", ext.table_name, ext.core_id_column);
                    created_tables.push((ext.table_name.clone(), ext.core_id_column.clone()));
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    if error_msg.contains("already exists") || error_msg.contains("Table with name") {
                        log::info!("Extension table already exists: {}", ext.table_name);
                        created_tables.push((ext.table_name.clone(), ext.core_id_column.clone()));
                    } else {
                        log::error!(
                            "Failed to create extension table {}: {}",
                            ext.table_name,
                            e
                        );
                        return Err(e.into());
                    }
                }
            }
        }

        Ok(created_tables)
    }

    /// Opens an existing database with extension metadata
    pub fn open(db_path: &Path, extensions: &[ExtensionInfo]) -> Result<Self> {
        let conn = duckdb::Connection::open(db_path)?;

        // Build extension_tables from provided extension info
        let extension_tables: Vec<(String, String)> = extensions
            .iter()
            .map(|ext| (ext.table_name.clone(), ext.core_id_column.clone()))
            .collect();

        Ok(Self { conn, extension_tables })
    }

    /// Counts the number of observations in the database
    pub fn count_records(&self) -> Result<usize> {
        let count: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM occurrences",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Helper to convert a DuckDB column value to serde_json::Value
    fn get_column_as_json(row: &Row, idx: usize) -> serde_json::Value {
        let col_type = row.as_ref().column_type(idx);
        let type_str = col_type.to_string();
        // let col_name = row.as_ref().column_name(idx);
        // println!("{:?} ({}) type: {}", col_name, idx, col_type);

        // Handle different DuckDB types
        match type_str.as_str() {
            "Integer" | "BigInt" | "Int64" => {
                if let Ok(val) = row.get::<_, Option<i64>>(idx) {
                    val.map(|v| serde_json::json!(v)).unwrap_or(serde_json::Value::Null)
                } else {
                    serde_json::Value::Null
                }
            }
            "Double" | "Float" | "Float64" => {
                if let Ok(val) = row.get::<_, Option<f64>>(idx) {
                    val.map(|v| serde_json::json!(v)).unwrap_or(serde_json::Value::Null)
                } else {
                    serde_json::Value::Null
                }
            }
            "Boolean" => {
                if let Ok(val) = row.get::<_, Option<bool>>(idx) {
                    val.map(|v| serde_json::json!(v)).unwrap_or(serde_json::Value::Null)
                } else {
                    serde_json::Value::Null
                }
            }
            "Date32" => {
                // Handle Date32 as days since epoch
                if let Ok(days) = row.get::<_, Option<i32>>(idx) {
                    days.map(|d| {
                        let epoch = chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
                        let date = epoch + chrono::Duration::days(d as i64);
                        serde_json::json!(date.format("%Y-%m-%d").to_string())
                    }).unwrap_or(serde_json::Value::Null)
                } else {
                    serde_json::Value::Null
                }
            }
            "Varchar" | _ => {
                // For VARCHAR columns, try to parse as number if possible
                if let Ok(Some(s)) = row.get::<_, Option<String>>(idx) {
                    // Try parsing as float first (handles both int and float)
                    if let Ok(f) = s.parse::<f64>() {
                        // Check if it's actually an integer
                        if f.fract() == 0.0 && f.abs() < (i64::MAX as f64) {
                            serde_json::json!(f as i64)
                        } else {
                            serde_json::json!(f)
                        }
                    } else {
                        serde_json::json!(s)
                    }
                } else {
                    serde_json::Value::Null
                }
            }
        }
    }

    pub fn sql_parts(
        search_params: crate::commands::archive::SearchParams,
        fields: Option<Vec<String>>,
        extension_tables: &Vec<(String, String)>,
    ) -> (String, String, Vec<Box<dyn duckdb::ToSql>>, String) {
        // Validate and filter requested fields against allowlist
        let core_select_fields = if let Some(ref requested) = fields {
            let validated: Vec<&str> = requested
                .iter()
                .filter(|f| Occurrence::FIELD_NAMES.contains(&f.as_str()))
                .map(|s| s.as_str())
                .collect();

            if validated.is_empty() {
                "occurrences.*".to_string()
            } else {
                validated.iter().map(|f| format!("occurrences.{}", f)).collect::<Vec<_>>().join(", ")
            }
        } else {
            "occurrences.*".to_string()
        };

        // Build extension subqueries for each extension table
        // Note: We aggregate rows into a list then convert to JSON. This
        // looks like it should be less effecient than loading extension rows
        // in subsequent queries, but benchmarking showed that it's
        // actually *faster* with larger result sets
        let extension_subqueries: Vec<String> = extension_tables
            .iter()
            .map(|(table_name, core_id_col)| {
                format!(
                    "(SELECT COALESCE(to_json(list({})), '[]') FROM {} WHERE {}.{} = occurrences.{}) as {}",
                    table_name, table_name, table_name, core_id_col, core_id_col, table_name
                )
            })
            .collect();

        let select_fields = if extension_subqueries.is_empty() {
            core_select_fields
        } else {
            format!("{}, {}", core_select_fields, extension_subqueries.join(", "))
        };

        // Build dynamic WHERE clause and SELECT fields
        let mut where_clauses = Vec::new();
        let mut where_interpolations: Vec<Box<dyn duckdb::ToSql>> = Vec::new();
        if let Some(ref name) = search_params.scientific_name {
            where_clauses.push("scientificName ILIKE ?");
            where_interpolations.push(Box::new(format!("%{}%", name)));
        }
        let where_clause = if where_clauses.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", where_clauses.join(" AND "))
        };

        // Build ORDER clause
        let order_clause = if let Some(order_by) = search_params.order_by {
            if Occurrence::FIELD_NAMES.contains(&order_by.as_str()) {
                format!(" ORDER BY {}", order_by)
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        (select_fields, where_clause, where_interpolations, order_clause)
    }

    /// Searches for occurrences, returning up to the specified limit starting at offset
    pub fn search(
        &self,
        limit: usize,
        offset: usize,
        search_params: crate::commands::archive::SearchParams,
        fields: Option<Vec<String>>,
    ) -> Result<crate::commands::archive::SearchResult> {
        let (
            select_fields,
            where_clause,
            mut where_interpolations,
            order_clause
        ) = Self::sql_parts(search_params, fields, self.extension_tables.as_ref());

        // Execute COUNT query
        let count_query = format!("SELECT COUNT(*) FROM occurrences{}", where_clause);
        let count_param_refs: Vec<&dyn duckdb::ToSql> = where_interpolations.iter()
            .map(|p| p.as_ref()).collect();
        let total: usize = self.conn.query_row(
            &count_query,
            count_param_refs.as_slice(), |row| row.get(0)
        )?;

        // Build SELECT query
        let select_query = format!(
            "SELECT {} FROM occurrences{}{} LIMIT ? OFFSET ?",
            select_fields, where_clause, order_clause
        );
        where_interpolations.push(Box::new(limit));
        where_interpolations.push(Box::new(offset));

        let mut stmt = self.conn.prepare(&select_query)?;

        // Convert params to references for query_map
        let select_param_refs: Vec<&dyn duckdb::ToSql> = where_interpolations.iter().map(|p| p.as_ref()).collect();

        let rows = stmt.query_map(select_param_refs.as_slice(), |row| {
            // Dynamically map columns to JSON
            let mut map = serde_json::Map::new();
            let column_count = row.as_ref().column_count();

            for i in 0..column_count {
                let name = row.as_ref().column_name(i)
                    .map_err(|_e| duckdb::Error::InvalidColumnIndex(i))?;
                let value = Self::get_column_as_json(&row, i);

                // For extension columns, parse JSON string into array
                let is_extension = self.extension_tables.iter().any(|(tbl, _)| tbl == name);
                if is_extension {
                    if let serde_json::Value::String(json_str) = &value {
                        match serde_json::from_str::<serde_json::Value>(json_str) {
                            Ok(parsed) => {
                                map.insert(name.to_string(), parsed);
                            }
                            Err(_) => {
                                // If parsing fails, insert empty array
                                map.insert(name.to_string(), serde_json::json!([]));
                            }
                        }
                    } else {
                        // If not a string, insert empty array
                        map.insert(name.to_string(), serde_json::json!([]));
                    }
                } else {
                    map.insert(name.to_string(), value);
                }
            }

            Ok(map)
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }

        Ok(crate::commands::archive::SearchResult {
            total,
            results,
        })
    }

    /// Retrieves a single occurrence by ID with all columns and extension data
    pub fn get_occurrence(
        &self,
        core_id_column: &str,
        occurrence_id: &str,
    ) -> Result<serde_json::Map<String, serde_json::Value>> {
        // Build extension subqueries (same pattern as search method)
        let extension_subqueries: Vec<String> = self.extension_tables
            .iter()
            .map(|(table_name, core_id_col)| {
                format!(
                    "(SELECT COALESCE(to_json(list({})), '[]') FROM {} WHERE {}.{} = occurrences.{}) as {}",
                    table_name, table_name, table_name, core_id_col, core_id_col, table_name
                )
            })
            .collect();

        let select_fields = if extension_subqueries.is_empty() {
            "occurrences.*".to_string()
        } else {
            format!("occurrences.*, {}", extension_subqueries.join(", "))
        };

        // Build query with WHERE clause on core_id_column
        let query = format!(
            "SELECT {} FROM occurrences WHERE {} = ?",
            select_fields, core_id_column
        );

        let mut stmt = self.conn.prepare(&query)?;

        let result = stmt.query_row([occurrence_id], |row| {
            let mut map = serde_json::Map::new();
            let column_count = row.as_ref().column_count();

            for i in 0..column_count {
                let name = row.as_ref().column_name(i)
                    .map_err(|_| duckdb::Error::InvalidColumnIndex(i))?;
                let value = Self::get_column_as_json(&row, i);

                // Parse extension JSON strings
                let is_extension = self.extension_tables.iter().any(|(tbl, _)| tbl == name);
                if is_extension {
                    if let serde_json::Value::String(json_str) = &value {
                        match serde_json::from_str::<serde_json::Value>(json_str) {
                            Ok(parsed) => {
                                map.insert(name.to_string(), parsed);
                            }
                            Err(_) => {
                                map.insert(name.to_string(), serde_json::json!([]));
                            }
                        }
                    } else {
                        map.insert(name.to_string(), serde_json::json!([]));
                    }
                } else {
                    map.insert(name.to_string(), value);
                }
            }

            Ok(map)
        })?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    struct TestFixture {
        temp_dir: PathBuf,
        csv_paths: Vec<PathBuf>,
        db_path: PathBuf,
    }

    impl TestFixture {
        fn new(test_name: &str, csv_data: Vec<&[u8]>) -> Self {
            let temp_dir = std::env::temp_dir()
                .join(format!("chuck_test_db_{}", test_name));

            // Clean up from any previous test runs
            std::fs::remove_dir_all(&temp_dir).ok();
            std::fs::create_dir_all(&temp_dir).unwrap();

            // Create CSV files
            let mut csv_paths = Vec::new();
            for (i, data) in csv_data.iter().enumerate() {
                let csv_path = temp_dir.join(format!("test{}.csv", i));
                let mut file = std::fs::File::create(&csv_path).unwrap();
                file.write_all(data).unwrap();
                csv_paths.push(csv_path);
            }

            let db_path = temp_dir.join("test.db");

            Self {
                temp_dir,
                csv_paths,
                db_path,
            }
        }
    }

    impl Drop for TestFixture {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.temp_dir).ok();
        }
    }

    #[test]
    fn test_create_with_existing_table() {
        let fixture = TestFixture::new(
            "existing_table",
            vec![b"id,name\n1,Alice\n2,Bob\n"]
        );

        // First call should succeed
        let result1 = Database::create_from_core_files(
            &fixture.csv_paths,
            &vec![],
            &fixture.db_path
        );
        assert!(result1.is_ok());
        let db1 = result1.unwrap();
        assert_eq!(db1.count_records().unwrap(), 2);

        // Second call should recognize existing table and not alter it
        let result2 = Database::create_from_core_files(
            &fixture.csv_paths,
            &vec![],
            &fixture.db_path
        );

        assert!(result2.is_ok());
        let db2 = result2.unwrap();
        assert_eq!(db2.count_records().unwrap(), 2);

        // Cleanup happens automatically via Drop
    }

    #[test]
    fn test_create_with_multiple_core_files() {
        let fixture = TestFixture::new(
            "multiple_cores",
            vec![
                b"id,name\n1,Collins\n2,Gardiner\n",
                b"3,Lizzy\n4,Jane\n"
            ]
        );

        let result = Database::create_from_core_files(
            &fixture.csv_paths,
            &vec![],
            &fixture.db_path
        );
        assert!(result.is_ok());
        let db = result.unwrap();
        assert_eq!(db.count_records().unwrap(), 4);

        // Cleanup happens automatically via Drop
    }

    #[test]
    fn test_search_returns_occurrence_records() {
        // Create a CSV with Darwin Core occurrence fields
        let csv_data = br#"occurrenceID,basisOfRecord,recordedBy,eventDate,decimalLatitude,decimalLongitude,scientificName,taxonRank,taxonomicStatus,vernacularName,kingdom,phylum,class,order,family,genus,specificEpithet,infraspecificEpithet,taxonID,occurrenceRemarks,establishmentMeans,georeferencedDate,georeferenceProtocol,coordinateUncertaintyInMeters,coordinatePrecision,geodeticDatum,accessRights,license,informationWithheld,modified,captive,eventTime,verbatimEventDate,verbatimLocality
123456,HumanObservation,John Doe,2024-01-15,34.0522,-118.2437,Quercus agrifolia,species,accepted,Coast Live Oak,Plantae,Tracheophyta,Magnoliopsida,Fagales,Fagaceae,Quercus,agrifolia,,12345,Observed in park,native,2024-01-15,GPS,10.0,0.0001,WGS84,public,CC-BY,,,false,14:30:00,January 15 2024,Golden Gate Park
789012,HumanObservation,Jane Smith,2024-01-16,37.7749,-122.4194,Pinus radiata,species,accepted,Monterey Pine,Plantae,Tracheophyta,Pinopsida,Pinales,Pinaceae,Pinus,radiata,,67890,Tall specimen,native,2024-01-16,GPS,5.0,0.0001,WGS84,public,CC-BY-NC,,,false,09:15:00,January 16 2024,Presidio
345678,HumanObservation,Bob Jones,2024-01-17,36.7783,-119.4179,Sequoiadendron giganteum,species,accepted,Giant Sequoia,Plantae,Tracheophyta,Pinopsida,Pinales,Cupressaceae,Sequoiadendron,giganteum,,11111,Ancient tree,native,2024-01-17,GPS,20.0,0.0001,WGS84,public,CC0,,,false,11:00:00,January 17 2024,Sequoia National Park
"#;

        let fixture = TestFixture::new(
            "search_occurrences",
            vec![csv_data]
        );

        let db = Database::create_from_core_files(
            &fixture.csv_paths,
            &vec![],
            &fixture.db_path
        ).unwrap();

        use crate::commands::archive::SearchParams;

        // Test searching for all records
        let search_result = db.search(10, 0, SearchParams::default(), None).unwrap();
        assert_eq!(search_result.total, 3);
        assert_eq!(search_result.results.len(), 3);

        // Verify first occurrence fields
        let first = &search_result.results[0];
        // occurrenceID is parsed as a number since it's all digits
        assert_eq!(first.get("occurrenceID").and_then(|v| v.as_i64()), Some(123456));
        assert_eq!(first.get("basisOfRecord").and_then(|v| v.as_str()), Some("HumanObservation"));
        assert_eq!(first.get("recordedBy").and_then(|v| v.as_str()), Some("John Doe"));
        assert_eq!(first.get("eventDate").and_then(|v| v.as_str()), Some("2024-01-15"));
        assert_eq!(first.get("decimalLatitude").and_then(|v| v.as_f64()), Some(34.0522));
        assert_eq!(first.get("decimalLongitude").and_then(|v| v.as_f64()), Some(-118.2437));
        assert_eq!(first.get("scientificName").and_then(|v| v.as_str()), Some("Quercus agrifolia"));
        assert_eq!(first.get("taxonRank").and_then(|v| v.as_str()), Some("species"));
        assert_eq!(first.get("kingdom").and_then(|v| v.as_str()), Some("Plantae"));
        assert_eq!(first.get("family").and_then(|v| v.as_str()), Some("Fagaceae"));

        // Test limit parameter
        let limited = db.search(2, 0, SearchParams::default(), None).unwrap();
        assert_eq!(limited.total, 3);
        assert_eq!(limited.results.len(), 2);

        // Test offset parameter
        let offset_result = db.search(2, 1, SearchParams::default(), None).unwrap();
        assert_eq!(offset_result.total, 3);
        assert_eq!(offset_result.results.len(), 2);
        assert_eq!(offset_result.results[0].get("occurrenceID").and_then(|v| v.as_i64()), Some(789012));

        // Test limit larger than available records
        let all = db.search(100, 0, SearchParams::default(), None).unwrap();
        assert_eq!(all.total, 3);
        assert_eq!(all.results.len(), 3);
    }

    #[test]
    fn test_search_by_scientific_name() {
        use crate::commands::archive::SearchParams;

        // Create test data with various scientific names
        let csv_data = br#"occurrenceID,basisOfRecord,recordedBy,eventDate,decimalLatitude,decimalLongitude,scientificName,taxonRank,taxonomicStatus,vernacularName,kingdom,phylum,class,order,family,genus,specificEpithet,infraspecificEpithet,taxonID,occurrenceRemarks,establishmentMeans,georeferencedDate,georeferenceProtocol,coordinateUncertaintyInMeters,coordinatePrecision,geodeticDatum,accessRights,license,informationWithheld,modified,captive,eventTime,verbatimEventDate,verbatimLocality
1,obs,John,2024-01-01,0,0,Foobar,species,accepted,,,,,,,,,,,,,,,,,,,,,,,,,
2,obs,Jane,2024-01-01,0,0,foo,species,accepted,,,,,,,,,,,,,,,,,,,,,,,,,
3,obs,Bob,2024-01-01,0,0,Foo,species,accepted,,,,,,,,,,,,,,,,,,,,,,,,,
4,obs,Alice,2024-01-01,0,0,Barfoo,species,accepted,,,,,,,,,,,,,,,,,,,,,,,,,
5,obs,Charlie,2024-01-01,0,0,Bar,species,accepted,,,,,,,,,,,,,,,,,,,,,,,,,
"#;

        let fixture = TestFixture::new("search_scientific_name", vec![csv_data]);

        let db = Database::create_from_core_files(&fixture.csv_paths, &vec![], &fixture.db_path).unwrap();

        // Search for "foo" (case-insensitive partial match)
        let search_result = db.search(10, 0, SearchParams {
            scientific_name: Some("foo".to_string()),
            order_by: Some("occurrenceID".to_string()),
        }, None).unwrap();

        // Should return 4 results: "Foobar", "foo", "Foo", "Barfoo"
        assert_eq!(search_result.total, 4, "Expected total count of 4");
        assert_eq!(search_result.results.len(), 4, "Expected 4 results containing 'foo'");

        // Verify the names contain "foo" (case-insensitive)
        for result in &search_result.results {
            let name = result.get("scientificName")
                .and_then(|v| v.as_str())
                .expect("scientificName should exist")
                .to_lowercase();
            assert!(
                name.contains("foo"),
                "Expected '{}' to contain 'foo'",
                result.get("scientificName").and_then(|v| v.as_str()).unwrap()
            );
        }

        // Should NOT return "Bar"
        for result in &search_result.results {
            let name = result.get("scientificName").and_then(|v| v.as_str()).unwrap();
            assert_ne!(
                name,
                "Bar",
                "Should not return 'Bar'"
            );
        }
    }

    #[test]
    fn test_search_with_field_selection() {
        use crate::commands::archive::SearchParams;

        // Create test data
        let csv_data = br#"occurrenceID,basisOfRecord,recordedBy,eventDate,decimalLatitude,decimalLongitude,scientificName
1,obs,John,2024-01-01,0,0,Test species
"#;

        let fixture = TestFixture::new("search_field_selection", vec![csv_data]);
        let db = Database::create_from_core_files(&fixture.csv_paths, &vec![], &fixture.db_path).unwrap();

        // Search with specific fields
        let search_result = db.search(10, 0, SearchParams::default(), Some(vec![
            "occurrenceID".to_string(),
            "scientificName".to_string(),
        ])).unwrap();

        assert_eq!(search_result.total, 1);
        assert_eq!(search_result.results.len(), 1);

        let first = &search_result.results[0];
        // Should have the requested fields
        assert!(first.contains_key("occurrenceID"));
        assert!(first.contains_key("scientificName"));
        // Should only have the requested fields (2 fields)
        assert_eq!(first.len(), 2);
    }

    #[test]
    fn test_create_with_extensions() {
        use crate::commands::archive::SearchParams;

        // Create occurrence CSV
        let occurrence_csv = br#"occurrenceID,scientificName
1,Species A
2,Species B
"#;

        // Create multimedia extension CSV
        let multimedia_csv = br#"occurrenceID,type,identifier
1,StillImage,http://example.com/img1.jpg
1,StillImage,http://example.com/img2.jpg
2,StillImage,http://example.com/img3.jpg
"#;

        // Set up fixture
        let temp_dir = std::env::temp_dir()
            .join("chuck_test_db_extensions");
        std::fs::remove_dir_all(&temp_dir).ok();
        std::fs::create_dir_all(&temp_dir).unwrap();

        let occurrence_path = temp_dir.join("occurrence.csv");
        let multimedia_path = temp_dir.join("multimedia.csv");
        let db_path = temp_dir.join("test.db");

        std::fs::write(&occurrence_path, occurrence_csv).unwrap();
        std::fs::write(&multimedia_path, multimedia_csv).unwrap();

        // Create extension info
        let extensions = vec![ExtensionInfo {
            row_type: "http://rs.gbif.org/terms/1.0/Multimedia".to_string(),
            location: multimedia_path.clone(),
            table_name: "multimedia".to_string(),
            core_id_column: "occurrenceID".to_string(),
        }];

        // Create database with extensions
        let db = Database::create_from_core_files(
            &vec![occurrence_path],
            &extensions,
            &db_path
        ).unwrap();

        // Verify occurrences table was created
        assert_eq!(db.count_records().unwrap(), 2);

        // Verify extension table is tracked
        assert_eq!(db.extension_tables.len(), 1);
        assert_eq!(db.extension_tables[0].0, "multimedia");
        assert_eq!(db.extension_tables[0].1, "occurrenceID");

        // Search and verify extensions are included
        let search_result = db.search(10, 0, SearchParams::default(), None).unwrap();
        assert_eq!(search_result.results.len(), 2);

        // Check first occurrence has multimedia array
        let first = &search_result.results[0];
        assert!(first.contains_key("multimedia"));

        let multimedia = first.get("multimedia").unwrap();
        assert!(multimedia.is_array());

        let multimedia_array = multimedia.as_array().unwrap();
        assert_eq!(multimedia_array.len(), 2); // Two images for occurrence 1

        // Check multimedia items have expected fields
        let first_image = &multimedia_array[0];

        // occurrenceID is stored as VARCHAR in extension tables, so it comes back as string
        assert_eq!(
            first_image.get("occurrenceID").and_then(|v| v.as_str()),
            Some("1")
        );
        assert_eq!(
            first_image.get("type").and_then(|v| v.as_str()),
            Some("StillImage")
        );
        assert_eq!(
            first_image.get("identifier").and_then(|v| v.as_str()),
            Some("http://example.com/img1.jpg")
        );

        // Check second occurrence has multimedia array
        let second = &search_result.results[1];
        let multimedia_second = second.get("multimedia").unwrap().as_array().unwrap();
        assert_eq!(multimedia_second.len(), 1); // One image for occurrence 2

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_open_database_detects_extensions() {
        // Create occurrence CSV
        let occurrence_csv = br#"occurrenceID,scientificName
1,Species A
"#;

        // Create extension CSV
        let multimedia_csv = br#"occurrenceID,identifier
1,http://example.com/img1.jpg
"#;

        let temp_dir = std::env::temp_dir()
            .join("chuck_test_db_open_extensions");
        std::fs::remove_dir_all(&temp_dir).ok();
        std::fs::create_dir_all(&temp_dir).unwrap();

        let occurrence_path = temp_dir.join("occurrence.csv");
        let multimedia_path = temp_dir.join("multimedia.csv");
        let db_path = temp_dir.join("test.db");

        std::fs::write(&occurrence_path, occurrence_csv).unwrap();
        std::fs::write(&multimedia_path, multimedia_csv).unwrap();

        let extensions = vec![ExtensionInfo {
            row_type: "http://rs.gbif.org/terms/1.0/Multimedia".to_string(),
            location: multimedia_path,
            table_name: "multimedia".to_string(),
            core_id_column: "occurrenceID".to_string(),
        }];

        // Create database
        let _db = Database::create_from_core_files(
            &vec![occurrence_path],
            &extensions,
            &db_path
        ).unwrap();

        // Reopen the database with extension info
        let reopened_db = Database::open(&db_path, &extensions).unwrap();

        // Verify it has the extension table info
        assert_eq!(reopened_db.extension_tables.len(), 1);
        assert_eq!(reopened_db.extension_tables[0].0, "multimedia");
        assert_eq!(reopened_db.extension_tables[0].1, "occurrenceID");

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_sql_parts_only_allows_known_order_by() {
        let params = crate::commands::archive::SearchParams {
            scientific_name: None,
            order_by: Some("foo".to_string()),
        };
        let (_, _, _, order_clause) = Database::sql_parts(params, None, &vec![]);
        assert_eq!(order_clause, "");
    }

    #[test]
    fn test_get_occurrence_with_extensions() {
        // Create occurrence and multimedia test data
        let occurrence_csv = br#"occurrenceID,scientificName,decimalLatitude,decimalLongitude
1,Species A,34.05,-118.24
2,Species B,37.77,-122.41
"#;

        let multimedia_csv = br#"occurrenceID,type,identifier
1,StillImage,http://example.com/img1.jpg
1,StillImage,http://example.com/img2.jpg
"#;

        let temp_dir = std::env::temp_dir().join("chuck_test_get_occurrence");
        std::fs::remove_dir_all(&temp_dir).ok();
        std::fs::create_dir_all(&temp_dir).unwrap();

        let occurrence_path = temp_dir.join("occurrence.csv");
        let multimedia_path = temp_dir.join("multimedia.csv");
        let db_path = temp_dir.join("test.db");

        std::fs::write(&occurrence_path, occurrence_csv).unwrap();
        std::fs::write(&multimedia_path, multimedia_csv).unwrap();

        let extensions = vec![ExtensionInfo {
            row_type: "http://rs.gbif.org/terms/1.0/Multimedia".to_string(),
            location: multimedia_path,
            table_name: "multimedia".to_string(),
            core_id_column: "occurrenceID".to_string(),
        }];

        let db = Database::create_from_core_files(
            &vec![occurrence_path],
            &extensions,
            &db_path
        ).unwrap();

        // Get occurrence by ID
        let occurrence = db.get_occurrence("occurrenceID", "1").unwrap();

        // Verify core fields
        assert_eq!(
            occurrence.get("scientificName").and_then(|v| v.as_str()),
            Some("Species A")
        );
        assert_eq!(
            occurrence.get("decimalLatitude").and_then(|v| v.as_f64()),
            Some(34.05)
        );

        // Verify multimedia extension
        let multimedia = occurrence.get("multimedia").unwrap();
        assert!(multimedia.is_array());
        let multimedia_array = multimedia.as_array().unwrap();
        assert_eq!(multimedia_array.len(), 2);

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }
}

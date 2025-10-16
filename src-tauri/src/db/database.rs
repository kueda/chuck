use std::path::{Path, PathBuf};
use duckdb::Row;

use crate::error::{ChuckError, Result};

/// Represents a DuckDB database for Darwin Core Archive data
pub struct Database {
    path: PathBuf,
    conn: duckdb::Connection,
}

impl Database {
    /// Creates a new database from core files
    pub fn create_from_core_files(
        core_files: &[PathBuf],
        db_path: &Path,
    ) -> Result<Self> {
        if core_files.is_empty() {
            return Err(ChuckError::NoCoreFiles);
        }

        let conn = duckdb::Connection::open(db_path)?;

        // Try to create table from first file
        let first_csv = core_files[0]
            .to_str()
            .ok_or(ChuckError::PathEncoding)?;

        let create_result = conn.execute(
            &format!(
                "CREATE TABLE occurrences AS SELECT * FROM read_csv_auto('{}')",
                first_csv
            ),
            [],
        );

        // If table already exists, insert from first file instead
        match create_result {
            Ok(_) => {
                // Insert remaining files
                for core_file in &core_files[1..] {
                    let csv_path = core_file
                        .to_str()
                        .ok_or(ChuckError::PathEncoding)?;

                    conn.execute(
                        &format!(
                            "INSERT INTO occurrences SELECT * FROM read_csv_auto('{}')",
                            csv_path
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

        Ok(Self {
            path: db_path.to_path_buf(),
            conn,
        })
    }

    /// Opens an existing database
    pub fn open(db_path: &Path) -> Result<Self> {
        let conn = duckdb::Connection::open(db_path)?;
        Ok(Self {
            path: db_path.to_path_buf(),
            conn,
        })
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

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Helper to get column index from a row given a name
    fn col_index(row: &Row, name: &str) -> std::result::Result<usize, duckdb::Error> {
        row.as_ref().column_index(name)
    }

    /// Helper to get a required string value, converting from i64 if necessary
    fn col_string(row: &Row, name: &str) -> std::result::Result<String, duckdb::Error> {
        let idx = Self::col_index(&row, name)?;
        // Try as string first
        if let Ok(s) = row.get::<_, Option<String>>(idx) {
            Ok(s.unwrap_or_default())
        } else if let Ok(i) = row.get::<_, Option<i64>>(idx) {
            // Fall back to converting i64 to string
            Ok(i.map(|v| v.to_string()).unwrap_or_default())
        } else {
            Ok(String::new())
        }
    }

    fn col_opt_string(row: &Row, name: &str) -> std::result::Result<Option<String>, duckdb::Error> {
        let idx = Self::col_index(&row, name)?;

        // Check if this is a Date32 column
        let col_type = row.as_ref().column_type(idx);
        if col_type.to_string() == "Date32" {
            if let Ok(days) = row.get::<_, Option<i32>>(idx) {
                return Ok(days.map(|d| {
                    let epoch = chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
                    let date = epoch + chrono::Duration::days(d as i64);
                    date.format("%Y-%m-%d").to_string()
                }));
            }
        }

        // Try different types that DuckDB might infer
        if let Ok(s) = row.get::<_, Option<String>>(idx) {
            Ok(s)
        } else if let Ok(i) = row.get::<_, Option<i64>>(idx) {
            Ok(i.map(|v| v.to_string()))
        } else if let Ok(d) = row.get::<_, Option<f64>>(idx) {
            Ok(d.map(|v| v.to_string()))
        } else {
            // For types we still can't convert, return None
            Ok(None)
        }
    }

    /// Searches for occurrences, returning up to the specified limit starting at offset
    pub fn search(
        &self,
        limit: usize,
        offset: usize,
        search_params: crate::commands::archive::SearchParams,
    ) -> Result<crate::commands::archive::SearchResult> {
        // Build dynamic WHERE clause
        let mut where_clauses = Vec::new();
        let mut count_params: Vec<Box<dyn duckdb::ToSql>> = Vec::new();

        if let Some(ref name) = search_params.scientific_name {
            where_clauses.push("scientificName ILIKE ?");
            count_params.push(Box::new(format!("%{}%", name)));
        }

        let where_clause = if where_clauses.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", where_clauses.join(" AND "))
        };

        // Execute COUNT query
        let count_query = format!("SELECT COUNT(*) FROM occurrences{}", where_clause);
        let count_param_refs: Vec<&dyn duckdb::ToSql> = count_params.iter().map(|p| p.as_ref()).collect();
        let total: usize = self.conn.query_row(&count_query, count_param_refs.as_slice(), |row| row.get(0))?;

        // Build SELECT query
        let select_query = format!("SELECT * FROM occurrences{} LIMIT ? OFFSET ?", where_clause);

        // Rebuild params for SELECT query (reuse where params + add limit/offset)
        let mut select_params: Vec<Box<dyn duckdb::ToSql>> = Vec::new();
        if let Some(ref name) = search_params.scientific_name {
            select_params.push(Box::new(format!("%{}%", name)));
        }
        select_params.push(Box::new(limit));
        select_params.push(Box::new(offset));

        let mut stmt = self.conn.prepare(&select_query)?;

        // Convert params to references for query_map
        let select_param_refs: Vec<&dyn duckdb::ToSql> = select_params.iter().map(|p| p.as_ref()).collect();

        let rows = stmt.query_map(select_param_refs.as_slice(), |row| {
            // Map DuckDB row to Occurrence struct
            Ok(chuck_core::darwin_core::Occurrence {
                occurrence_id: Self::col_string(&row, "occurrenceID")?,
                basis_of_record: Self::col_string(&row, "basisOfRecord")?,
                recorded_by: Self::col_string(&row, "recordedBy")?,
                event_date: Self::col_opt_string(&row, "eventDate")?,
                decimal_latitude: row.get(Self::col_index(&row, "decimalLatitude")?)?,
                decimal_longitude: row.get(Self::col_index(&row, "decimalLongitude")?)?,
                scientific_name: Self::col_opt_string(&row, "scientificName")?,
                taxon_rank: Self::col_opt_string(&row, "taxonRank")?,
                taxonomic_status: Self::col_opt_string(&row, "taxonomicStatus")?,
                vernacular_name: Self::col_opt_string(&row, "vernacularName")?,
                kingdom: Self::col_opt_string(&row, "kingdom")?,
                phylum: Self::col_opt_string(&row, "phylum")?,
                class: Self::col_opt_string(&row, "class")?,
                order: Self::col_opt_string(&row, "order")?,
                family: Self::col_opt_string(&row, "family")?,
                genus: Self::col_opt_string(&row, "genus")?,
                specific_epithet: Self::col_opt_string(&row, "specificEpithet")?,
                infraspecific_epithet: Self::col_opt_string(&row, "infraspecificEpithet")?,
                taxon_id: row.get(Self::col_index(&row, "taxonID")?)?,
                occurrence_remarks: Self::col_opt_string(&row, "occurrenceRemarks")?,
                establishment_means: Self::col_opt_string(&row, "establishmentMeans")?,
                georeferenced_date: Self::col_opt_string(&row, "georeferencedDate")?,
                georeference_protocol: Self::col_opt_string(&row, "georeferenceProtocol")?,
                coordinate_uncertainty_in_meters: row.get(Self::col_index(&row, "coordinateUncertaintyInMeters")?)?,
                coordinate_precision: row.get(Self::col_index(&row, "coordinatePrecision")?)?,
                geodetic_datum: Self::col_opt_string(&row, "geodeticDatum")?,
                access_rights: Self::col_opt_string(&row, "accessRights")?,
                license: Self::col_opt_string(&row, "license")?,
                information_withheld: Self::col_opt_string(&row, "informationWithheld")?,
                modified: Self::col_opt_string(&row, "modified")?,
                captive: row.get(Self::col_index(&row, "captive")?)?,
                event_time: Self::col_opt_string(&row, "eventTime")?,
                verbatim_event_date: Self::col_opt_string(&row, "verbatimEventDate")?,
                verbatim_locality: Self::col_opt_string(&row, "verbatimLocality")?,
            })
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
            &fixture.db_path
        );
        assert!(result1.is_ok());
        let db1 = result1.unwrap();
        assert_eq!(db1.count_records().unwrap(), 2);

        // Second call should recognize existing table and not alter it
        let result2 = Database::create_from_core_files(
            &fixture.csv_paths,
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
            &fixture.db_path
        ).unwrap();

        use crate::commands::archive::SearchParams;

        // Test searching for all records
        let search_result = db.search(10, 0, SearchParams::default()).unwrap();
        assert_eq!(search_result.total, 3);
        assert_eq!(search_result.results.len(), 3);

        // Verify first occurrence fields
        let first = &search_result.results[0];
        assert_eq!(first.occurrence_id, "123456");
        assert_eq!(first.basis_of_record, "HumanObservation");
        assert_eq!(first.recorded_by, "John Doe");
        assert_eq!(first.event_date, Some("2024-01-15".to_string()));
        assert_eq!(first.decimal_latitude, Some(34.0522));
        assert_eq!(first.decimal_longitude, Some(-118.2437));
        assert_eq!(first.scientific_name, Some("Quercus agrifolia".to_string()));
        assert_eq!(first.taxon_rank, Some("species".to_string()));
        assert_eq!(first.kingdom, Some("Plantae".to_string()));
        assert_eq!(first.family, Some("Fagaceae".to_string()));

        // Test limit parameter
        let limited = db.search(2, 0, SearchParams::default()).unwrap();
        assert_eq!(limited.total, 3);
        assert_eq!(limited.results.len(), 2);

        // Test offset parameter
        let offset_result = db.search(2, 1, SearchParams::default()).unwrap();
        assert_eq!(offset_result.total, 3);
        assert_eq!(offset_result.results.len(), 2);
        assert_eq!(offset_result.results[0].occurrence_id, "789012");

        // Test limit larger than available records
        let all = db.search(100, 0, SearchParams::default()).unwrap();
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

        let db = Database::create_from_core_files(&fixture.csv_paths, &fixture.db_path).unwrap();

        // Search for "foo" (case-insensitive partial match)
        let search_result = db.search(10, 0, SearchParams {
            scientific_name: Some("foo".to_string()),
        }).unwrap();

        // Should return 4 results: "Foobar", "foo", "Foo", "Barfoo"
        assert_eq!(search_result.total, 4, "Expected total count of 4");
        assert_eq!(search_result.results.len(), 4, "Expected 4 results containing 'foo'");

        // Verify the names contain "foo" (case-insensitive)
        for result in &search_result.results {
            let name = result.scientific_name.as_ref().unwrap().to_lowercase();
            assert!(
                name.contains("foo"),
                "Expected '{}' to contain 'foo'",
                result.scientific_name.as_ref().unwrap()
            );
        }

        // Should NOT return "Bar"
        for result in &search_result.results {
            assert_ne!(
                result.scientific_name.as_ref().unwrap(),
                "Bar",
                "Should not return 'Bar'"
            );
        }
    }
}


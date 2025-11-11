use std::path::{Path, PathBuf};
use std::collections::HashMap;
use duckdb::{params, Row};
use chuck_core::darwin_core::Occurrence;

use crate::error::{ChuckError, Result};
use crate::dwca::ExtensionInfo;
use crate::search_params::SearchParams;

// Most DwC attributes are strings, but a few should have different types to
// enable better queries
const TYPE_OVERRIDES: [(&str, &str); 4] = [
    ("decimalLatitude", "DOUBLE"),
    ("decimalLongitude", "DOUBLE"),
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
        core_id_column: &str,
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

        // Drop columns that are entirely null or empty strings
        Self::drop_empty_columns(&conn, core_id_column)?;

        // Create indices on coordinate columns for fast spatial queries
        // (Do this after dropping columns in case lat/lng were dropped)
        let updated_columns = Self::get_column_names(&conn, "occurrences")?;
        if updated_columns.contains(&"decimalLatitude".to_string()) {
            conn.execute("CREATE INDEX IF NOT EXISTS idx_lat ON occurrences(decimalLatitude)", [])?;
        }
        if updated_columns.contains(&"decimalLongitude".to_string()) {
            conn.execute("CREATE INDEX IF NOT EXISTS idx_lng ON occurrences(decimalLongitude)", [])?;
        }

        // Create extension tables
        let extension_tables = Self::create_extension_tables(&conn, extensions)?;

        Ok(Self { conn, extension_tables })
    }

    /// Helper to get column names for a table
    fn get_column_names(conn: &duckdb::Connection, table_name: &str) -> Result<Vec<String>> {
        let mut stmt = conn.prepare(&format!(
            "SELECT column_name FROM information_schema.columns WHERE table_name = '{}' ORDER BY column_name",
            table_name
        ))?;
        let columns: Vec<String> = stmt.query_map([], |row| {
            row.get(0)
        })?.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(columns)
    }

    /// Drops columns from the occurrences table that contain only NULL or empty strings
    fn drop_empty_columns(conn: &duckdb::Connection, core_id_column: &str) -> Result<()> {
        // Get all column names
        let column_names = Self::get_column_names(conn, "occurrences")?;

        // Check each column to see if it's entirely empty
        for column_name in &column_names {
            // Skip the core ID column
            if column_name == core_id_column {
                continue;
            }

            // Get the column type to determine how to check for emptiness
            let type_override = TYPE_OVERRIDES.iter()
                .find(|(col, _)| col == &column_name.as_str())
                .map(|(_, typ)| *typ);

            // Quote column name to handle reserved keywords like "order"
            let quoted_column = format!("\"{}\"", column_name);

            let query = match type_override {
                Some("DOUBLE") | Some("BIGINT") => {
                    // For numeric types, just check for NULL
                    format!("SELECT COUNT(*) FROM occurrences WHERE {} IS NOT NULL", quoted_column)
                }
                Some("DATE") => {
                    // For date types, just check for NULL
                    format!("SELECT COUNT(*) FROM occurrences WHERE {} IS NOT NULL", quoted_column)
                }
                _ => {
                    // For VARCHAR (default), check for NULL and empty strings
                    format!(
                        "SELECT COUNT(*) FROM occurrences WHERE {} IS NOT NULL AND {} != ''",
                        quoted_column, quoted_column
                    )
                }
            };

            let count: usize = conn.query_row(&query, [], |row| row.get(0))?;

            // If no non-empty values, drop the column
            if count == 0 {
                log::info!("Dropping empty column: {}", column_name);
                conn.execute(&format!("ALTER TABLE occurrences DROP COLUMN {}", quoted_column), [])?;
            }
        }

        Ok(())
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

    /// Returns a list of all column names in the occurrences table
    pub fn get_available_columns(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT column_name FROM information_schema.columns \
             WHERE table_name = 'occurrences' \
             ORDER BY column_name"
        )?;

        let columns: Vec<String> = stmt.query_map([], |row| {
            row.get(0)
        })?.collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(columns)
    }

    /// Gets a reference to the database connection for use with PhotoCache
    pub fn connection(&self) -> &duckdb::Connection {
        &self.conn
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
        search_params: SearchParams,
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

        // Build dynamic WHERE clause from filters HashMap
        let mut where_clauses = Vec::new();
        let mut where_interpolations: Vec<Box<dyn duckdb::ToSql>> = Vec::new();

        for (column_name, filter_value) in &search_params.filters {
            // Validate column name against allowlist
            if Occurrence::FIELD_NAMES.contains(&column_name.as_str()) {
                // Check if this column has a type override
                let type_override = TYPE_OVERRIDES.iter()
                    .find(|(col, _)| col == &column_name.as_str())
                    .map(|(_, typ)| *typ);

                match type_override {
                    Some("DOUBLE") | Some("BIGINT") => {
                        // For numeric types, use range matching (e.g., "3" matches 3.0 to 3.9999...)
                        if let Ok(lower_bound) = filter_value.parse::<f64>() {
                            // Calculate upper bound by incrementing at the precision level
                            // "3" -> [3.0, 4.0), "3.1" -> [3.1, 3.2), "3.14" -> [3.14, 3.15)
                            let decimal_places = filter_value
                                .split('.')
                                .nth(1)
                                .map(|s| s.len())
                                .unwrap_or(0);
                            let increment = 10_f64.powi(-(decimal_places as i32));
                            let upper_bound = lower_bound + increment;

                            where_clauses.push(format!("{} >= ? AND {} < ?", column_name, column_name));
                            where_interpolations.push(Box::new(lower_bound));
                            where_interpolations.push(Box::new(upper_bound));
                        }
                        // If parse fails, skip this filter
                    }
                    Some("DATE") => {
                        // For date types, cast to string and use prefix matching
                        where_clauses.push(format!("CAST({} AS VARCHAR) LIKE ?", column_name));
                        where_interpolations.push(Box::new(format!("{}%", filter_value)));
                    }
                    _ => {
                        // For VARCHAR (default), use ILIKE with substring matching
                        where_clauses.push(format!("{} ILIKE ?", column_name));
                        where_interpolations.push(Box::new(format!("%{}%", filter_value)));
                    }
                }
            }
        }

        let where_clause = if where_clauses.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", where_clauses.join(" AND "))
        };

        // Build ORDER clause
        let order_clause = if let Some(order_by) = search_params.order_by {
            if Occurrence::FIELD_NAMES.contains(&order_by.as_str()) {
                let direction = search_params.order
                    .as_ref()
                    .and_then(|d| {
                        let upper = d.to_uppercase();
                        if upper == "ASC" || upper == "DESC" {
                            Some(upper)
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| "ASC".to_string());
                format!(" ORDER BY {} {}", order_by, direction)
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
        search_params: SearchParams,
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

    /// Get autocomplete suggestions for a column
    pub fn get_autocomplete_suggestions(
        &self,
        column_name: &str,
        search_term: &str,
        limit: usize,
    ) -> Result<Vec<String>> {
        // Validate column name against allowlist
        if !Occurrence::FIELD_NAMES.contains(&column_name) {
            return Err(crate::error::ChuckError::Database(
                duckdb::Error::InvalidColumnName(column_name.to_string())
            ));
        }

        // Check if column has a non-VARCHAR type override
        if let Some((_, column_type)) = TYPE_OVERRIDES.iter().find(|(col, _)| col == &column_name) {
            return Err(crate::error::ChuckError::AutocompleteNotAvailable {
                column: column_name.to_string(),
                column_type: column_type.to_string(),
            });
        }

        let query = format!(
            "SELECT DISTINCT {} FROM occurrences WHERE {} IS NOT NULL AND {} ILIKE ? ORDER BY {} LIMIT ?",
            column_name, column_name, column_name, column_name
        );

        let mut stmt = self.conn.prepare(&query)?;
        let search_pattern = format!("%{}%", search_term);
        let mut rows = stmt.query(params![search_pattern, limit as i64])?;

        let mut suggestions = Vec::new();
        while let Some(row) = rows.next()? {
            if let Ok(Some(value)) = row.get::<_, Option<String>>(0) {
                suggestions.push(value);
            }
        }

        Ok(suggestions)
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
            &fixture.db_path,
            "id"
        );
        assert!(result1.is_ok());
        let db1 = result1.unwrap();
        assert_eq!(db1.count_records().unwrap(), 2);

        // Second call should recognize existing table and not alter it
        let result2 = Database::create_from_core_files(
            &fixture.csv_paths,
            &vec![],
            &fixture.db_path,
            "id"
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
            &fixture.db_path,
            "id"
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
            &fixture.db_path,
            "occurrenceID"
        ).unwrap();

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
        // Create test data with various scientific names
        let csv_data = br#"occurrenceID,basisOfRecord,recordedBy,eventDate,decimalLatitude,decimalLongitude,scientificName,taxonRank,taxonomicStatus,vernacularName,kingdom,phylum,class,order,family,genus,specificEpithet,infraspecificEpithet,taxonID,occurrenceRemarks,establishmentMeans,georeferencedDate,georeferenceProtocol,coordinateUncertaintyInMeters,coordinatePrecision,geodeticDatum,accessRights,license,informationWithheld,modified,captive,eventTime,verbatimEventDate,verbatimLocality
1,obs,John,2024-01-01,0,0,Foobar,species,accepted,,,,,,,,,,,,,,,,,,,,,,,,,
2,obs,Jane,2024-01-01,0,0,foo,species,accepted,,,,,,,,,,,,,,,,,,,,,,,,,
3,obs,Bob,2024-01-01,0,0,Foo,species,accepted,,,,,,,,,,,,,,,,,,,,,,,,,
4,obs,Alice,2024-01-01,0,0,Barfoo,species,accepted,,,,,,,,,,,,,,,,,,,,,,,,,
5,obs,Charlie,2024-01-01,0,0,Bar,species,accepted,,,,,,,,,,,,,,,,,,,,,,,,,
"#;

        let fixture = TestFixture::new("search_scientific_name", vec![csv_data]);

        let db = Database::create_from_core_files(&fixture.csv_paths, &vec![], &fixture.db_path, "occurrenceID").unwrap();

        // Search for "foo" (case-insensitive partial match)
        let mut filters = HashMap::new();
        filters.insert("scientificName".to_string(), "foo".to_string());
        let search_result = db.search(10, 0, SearchParams {
            filters,
            order_by: Some("occurrenceID".to_string()),
            order: None,
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
        // Create test data
        let csv_data = br#"occurrenceID,basisOfRecord,recordedBy,eventDate,decimalLatitude,decimalLongitude,scientificName
1,obs,John,2024-01-01,0,0,Test species
"#;

        let fixture = TestFixture::new("search_field_selection", vec![csv_data]);
        let db = Database::create_from_core_files(&fixture.csv_paths, &vec![], &fixture.db_path, "occurrenceID").unwrap();

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
            &db_path,
            "occurrenceID"
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
            &db_path,
            "occurrenceID"
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
        let params = crate::search_params::SearchParams {
            filters: HashMap::new(),
            order_by: Some("foo".to_string()),
            order: None,
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
            &db_path,
            "occurrenceID"
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

    #[test]
    fn test_get_available_columns() {
        // Create a temporary directory for test database
        let temp_dir = std::env::temp_dir().join("chuck_test_available_columns");
        std::fs::remove_dir_all(&temp_dir).ok();
        std::fs::create_dir_all(&temp_dir).unwrap();
        let db_path = temp_dir.join("test.db");

        // Create test CSV file
        let csv_path = temp_dir.join("test.csv");
        std::fs::write(
            &csv_path,
            "id,scientificName,decimalLatitude,eventDate\n\
             1,Species A,10.5,2023-01-01\n\
             2,Species B,20.3,2023-01-02\n"
        ).unwrap();

        // Create database from CSV
        let db = Database::create_from_core_files(
            &[csv_path],
            &[],
            &db_path,
            "id"
        ).unwrap();

        // Test getting available columns
        let columns = db.get_available_columns().unwrap();

        assert_eq!(columns.len(), 4);
        assert!(columns.contains(&"id".to_string()));
        assert!(columns.contains(&"scientificName".to_string()));
        assert!(columns.contains(&"decimalLatitude".to_string()));
        assert!(columns.contains(&"eventDate".to_string()));

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_search_with_order() {
        let temp_dir = std::env::temp_dir().join("chuck_test_search_order");
        std::fs::remove_dir_all(&temp_dir).ok();
        std::fs::create_dir_all(&temp_dir).unwrap();
        let db_path = temp_dir.join("test.db");

        let csv_path = temp_dir.join("test.csv");
        std::fs::write(
            &csv_path,
            "id,scientificName\n\
             3,Zebra\n\
             1,Apple\n\
             2,Banana\n"
        ).unwrap();

        let db = Database::create_from_core_files(&[csv_path], &[], &db_path, "id").unwrap();

        // Test ASC order
        let params_asc = SearchParams {
            filters: HashMap::new(),
            order_by: Some("scientificName".to_string()),
            order: Some("ASC".to_string()),
        };
        let result_asc = db.search(10, 0, params_asc, Some(vec!["scientificName".to_string()])).unwrap();
        let first_name = result_asc.results[0].get("scientificName").unwrap().as_str().unwrap();
        assert_eq!(first_name, "Apple");

        // Test DESC order
        let params_desc = SearchParams {
            filters: HashMap::new(),
            order_by: Some("scientificName".to_string()),
            order: Some("DESC".to_string()),
        };
        let result_desc = db.search(10, 0, params_desc, Some(vec!["scientificName".to_string()])).unwrap();
        let first_name_desc = result_desc.results[0].get("scientificName").unwrap().as_str().unwrap();
        assert_eq!(first_name_desc, "Zebra");

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_search_with_numeric_order() {
        let temp_dir = std::env::temp_dir().join("chuck_test_numeric_order");
        std::fs::remove_dir_all(&temp_dir).ok();
        std::fs::create_dir_all(&temp_dir).unwrap();
        let db_path = temp_dir.join("test.db");

        let csv_path = temp_dir.join("test.csv");
        std::fs::write(
            &csv_path,
            "occurrenceID,scientificName,decimalLatitude\n\
             1,Species A,10.5\n\
             2,Species B,-5.3\n\
             3,Species C,2.1\n\
             4,Species D,-15.7\n"
        ).unwrap();

        let db = Database::create_from_core_files(&[csv_path], &[], &db_path, "id").unwrap();

        // Test ASC order - should be numeric ordering
        let params_asc = SearchParams {
            filters: HashMap::new(),
            order_by: Some("decimalLatitude".to_string()),
            order: Some("ASC".to_string()),
        };
        let result_asc = db.search(10, 0, params_asc, Some(vec!["occurrenceID".to_string(), "decimalLatitude".to_string()])).unwrap();

        // If sorted numerically: -15.7, -5.3, 2.1, 10.5 (ids: 4, 2, 3, 1)
        // If sorted alphabetically: -15.7, -5.3, 10.5, 2.1 (ids: 4, 2, 1, 3)
        // Values come back as numbers, so use as_i64()
        let first_id = result_asc.results[0].get("occurrenceID").unwrap().as_i64().unwrap();
        let second_id = result_asc.results[1].get("occurrenceID").unwrap().as_i64().unwrap();
        let third_id = result_asc.results[2].get("occurrenceID").unwrap().as_i64().unwrap();
        let fourth_id = result_asc.results[3].get("occurrenceID").unwrap().as_i64().unwrap();

        assert_eq!(first_id, 4, "Expected -15.7 first (numeric sort)");
        assert_eq!(second_id, 2, "Expected -5.3 second (numeric sort)");
        assert_eq!(third_id, 3, "Expected 2.1 third (numeric sort)");
        assert_eq!(fourth_id, 1, "Expected 10.5 fourth (numeric sort)");

        // Test DESC order
        let params_desc = SearchParams {
            filters: HashMap::new(),
            order_by: Some("decimalLatitude".to_string()),
            order: Some("DESC".to_string()),
        };
        let result_desc = db.search(10, 0, params_desc, Some(vec!["occurrenceID".to_string(), "decimalLatitude".to_string()])).unwrap();
        let first_id_desc = result_desc.results[0].get("occurrenceID").unwrap().as_i64().unwrap();
        assert_eq!(first_id_desc, 1, "Expected 10.5 first in DESC order");

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_search_with_decimal_latitude_range_filter() {
        // Create test data with various decimalLatitude values
        let csv_data = br#"occurrenceID,scientificName,decimalLatitude,decimalLongitude
1,Species A,3.0,0.0
2,Species B,3.1,0.0
3,Species C,3.14,0.0
4,Species D,3.141,0.0
5,Species E,30.0,0.0
6,Species F,301.3,0.0
7,Species G,2.9,0.0
"#;

        let fixture = TestFixture::new("search_decimal_latitude_range", vec![csv_data]);
        let db = Database::create_from_core_files(&fixture.csv_paths, &vec![], &fixture.db_path, "occurrenceID").unwrap();

        // Test 1: Search for "3" should match 3.0-3.9999 (ids: 1, 2, 3, 4)
        let mut filters = HashMap::new();
        filters.insert("decimalLatitude".to_string(), "3".to_string());
        let search_result = db.search(10, 0, SearchParams {
            filters: filters.clone(),
            order_by: Some("occurrenceID".to_string()),
            order: None,
        }, None).unwrap();

        assert_eq!(search_result.total, 4, "Search for '3' should match 3.0, 3.1, 3.14, 3.141");
        let ids: Vec<i64> = search_result.results.iter()
            .map(|r| r.get("occurrenceID").and_then(|v| v.as_i64()).unwrap())
            .collect();
        assert_eq!(ids, vec![1, 2, 3, 4], "Should match records with lat 3.x");

        // Test 2: Search for "3.1" should match 3.1-3.19999 (ids: 2, 3, 4)
        let mut filters = HashMap::new();
        filters.insert("decimalLatitude".to_string(), "3.1".to_string());
        let search_result = db.search(10, 0, SearchParams {
            filters: filters.clone(),
            order_by: Some("occurrenceID".to_string()),
            order: None,
        }, None).unwrap();

        assert_eq!(search_result.total, 3, "Search for '3.1' should match 3.1, 3.14, 3.141");
        let ids: Vec<i64> = search_result.results.iter()
            .map(|r| r.get("occurrenceID").and_then(|v| v.as_i64()).unwrap())
            .collect();
        assert_eq!(ids, vec![2, 3, 4], "Should match records with lat 3.1x");

        // Test 3: Search for "3.14" should match 3.14-3.149999 (ids: 3, 4)
        let mut filters = HashMap::new();
        filters.insert("decimalLatitude".to_string(), "3.14".to_string());
        let search_result = db.search(10, 0, SearchParams {
            filters: filters.clone(),
            order_by: Some("occurrenceID".to_string()),
            order: None,
        }, None).unwrap();

        assert_eq!(search_result.total, 2, "Search for '3.14' should match 3.14, 3.141");
        let ids: Vec<i64> = search_result.results.iter()
            .map(|r| r.get("occurrenceID").and_then(|v| v.as_i64()).unwrap())
            .collect();
        assert_eq!(ids, vec![3, 4], "Should match records with lat 3.14x");

        // Test 4: Search for "30" should NOT match "3.x" (should only match 30.x)
        let mut filters = HashMap::new();
        filters.insert("decimalLatitude".to_string(), "30".to_string());
        let search_result = db.search(10, 0, SearchParams {
            filters: filters.clone(),
            order_by: Some("occurrenceID".to_string()),
            order: None,
        }, None).unwrap();

        assert_eq!(search_result.total, 1, "Search for '30' should only match 30.0");
        let ids: Vec<i64> = search_result.results.iter()
            .map(|r| r.get("occurrenceID").and_then(|v| v.as_i64()).unwrap())
            .collect();
        assert_eq!(ids, vec![5], "Should match only record with lat 30.0");
    }

    #[test]
    fn test_get_autocomplete_suggestions_rejects_non_varchar_columns() {
        // Create test data
        let csv_data = br#"occurrenceID,scientificName,decimalLatitude,decimalLongitude
1,Species A,3.0,0.0
"#;

        let fixture = TestFixture::new("autocomplete_type_check", vec![csv_data]);
        let db = Database::create_from_core_files(&fixture.csv_paths, &vec![], &fixture.db_path, "occurrenceID").unwrap();

        // Test that VARCHAR column works
        let result = db.get_autocomplete_suggestions("scientificName", "Spec", 10);
        assert!(result.is_ok(), "scientificName (VARCHAR) should work for autocomplete");

        // Test that DOUBLE column is rejected with informative error
        let result = db.get_autocomplete_suggestions("decimalLatitude", "3", 10);
        assert!(result.is_err(), "decimalLatitude (DOUBLE) should be rejected");

        let err = result.unwrap_err();
        let err_msg = err.to_string();
        assert!(
            err_msg.contains("decimalLatitude") && err_msg.contains("not available for autocomplete"),
            "Error should mention column name and that it's not available for autocomplete. Got: {}",
            err_msg
        );
        assert!(
            err_msg.contains("DOUBLE"),
            "Error should mention the column type. Got: {}",
            err_msg
        );

        // Test that BIGINT column is also rejected
        let csv_data_gbif = br#"gbifID,scientificName
12345,Species B
"#;
        let fixture_gbif = TestFixture::new("autocomplete_gbif_check", vec![csv_data_gbif]);
        let db_gbif = Database::create_from_core_files(&fixture_gbif.csv_paths, &vec![], &fixture_gbif.db_path, "gbifID").unwrap();

        let result = db_gbif.get_autocomplete_suggestions("gbifID", "123", 10);
        assert!(result.is_err(), "gbifID (BIGINT) should be rejected");

        let err = result.unwrap_err();
        let err_msg = err.to_string();
        assert!(
            err_msg.contains("gbifID") && err_msg.contains("BIGINT"),
            "Error should mention gbifID and BIGINT. Got: {}",
            err_msg
        );
    }
}

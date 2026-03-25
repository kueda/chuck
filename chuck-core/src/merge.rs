use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};

/// Stream-based CSV merge: reads from `existing` and writes to `output`,
/// replacing any row whose value at `id_col_index` appears in `updates`.
/// Rows in `updates` not found in `existing` are appended at the end.
/// The header row is always preserved. Both CSVs must share the same schema.
///
/// Accepts generic `Read`/`Write` so callers can stream directly between ZIP
/// entries without materialising files on disk.
pub fn merge_csv_streams<R: Read, W: Write>(
    existing: R,
    output: W,
    updates: &HashMap<String, Vec<String>>,
    id_col_index: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(existing);
    let mut writer = csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(output);

    writer.write_record(reader.headers()?)?;

    let mut seen: HashSet<String> = HashSet::new();
    for result in reader.records() {
        let record = result?;
        let id = record.get(id_col_index).unwrap_or("").to_string();
        if let Some(updated_row) = updates.get(&id) {
            writer.write_record(updated_row)?;
            seen.insert(id);
        } else {
            writer.write_record(&record)?;
        }
    }

    // Append rows whose IDs were not in the existing file (new records)
    for (id, row) in updates {
        if !seen.contains(id) {
            writer.write_record(row)?;
        }
    }

    writer.flush()?;
    Ok(())
}

/// Group-replace CSV merge for extension CSVs (one-to-many relationships).
///
/// Unlike `merge_csv_streams`, `updates` maps each id to ALL rows for that id.
/// When a coreid is encountered in `existing`, ALL its existing rows are replaced
/// by the corresponding group from `updates` (written on the first encounter,
/// subsequent rows for the same coreid are silently dropped). Coreids present in
/// `updates` but absent from `existing` are appended at the end.
pub fn merge_extension_csv_streams<R: Read, W: Write>(
    existing: R,
    output: W,
    updates: &HashMap<String, Vec<Vec<String>>>,
    id_col_index: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(existing);
    let mut writer = csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(output);

    writer.write_record(reader.headers()?)?;

    let mut written: HashSet<String> = HashSet::new();
    for result in reader.records() {
        let record = result?;
        let id = record.get(id_col_index).unwrap_or("").to_string();
        if let Some(update_rows) = updates.get(&id) {
            if !written.contains(&id) {
                for row in update_rows {
                    writer.write_record(row)?;
                }
                written.insert(id);
            }
            // Remaining existing rows for this id are dropped (replaced by update_rows)
        } else {
            writer.write_record(&record)?;
        }
    }

    // Append update groups for coreids not present in existing
    for (id, rows) in updates {
        if !written.contains(id) {
            for row in rows {
                writer.write_record(row)?;
            }
        }
    }

    writer.flush()?;
    Ok(())
}

/// Convenience wrapper around `merge_csv_streams` for file paths.
pub fn merge_csv(
    existing_path: &std::path::Path,
    output_path: &std::path::Path,
    updates: &HashMap<String, Vec<String>>,
    id_col_index: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let reader = std::fs::File::open(existing_path)?;
    let writer = std::fs::File::create(output_path)?;
    merge_csv_streams(reader, writer, updates, id_col_index)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_csv(path: &std::path::Path, rows: &[&[&str]]) {
        let mut wtr = csv::WriterBuilder::new()
            .has_headers(false)
            .from_path(path)
            .unwrap();
        for row in rows {
            wtr.write_record(*row).unwrap();
        }
        wtr.flush().unwrap();
    }

    fn read_csv(path: &std::path::Path) -> Vec<Vec<String>> {
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_path(path)
            .unwrap();
        rdr.records()
            .map(|r| r.unwrap().iter().map(String::from).collect())
            .collect()
    }

    #[test]
    fn test_updates_matching_row_in_place() {
        let dir = tempfile::tempdir().unwrap();
        let existing = dir.path().join("existing.csv");
        let output = dir.path().join("output.csv");

        write_csv(&existing, &[
            &["id", "name"],
            &["1", "Alice"],
            &["2", "Bob"],
            &["3", "Carol"],
        ]);

        let updates: HashMap<String, Vec<String>> = [(
            "2".to_string(),
            vec!["2".to_string(), "Robert".to_string()],
        )]
        .into();
        merge_csv(&existing, &output, &updates, 0).unwrap();

        let rows = read_csv(&output);
        assert_eq!(rows, vec![
            vec!["id", "name"],
            vec!["1", "Alice"],
            vec!["2", "Robert"], // updated in place, not moved to end
            vec!["3", "Carol"],
        ]);
    }

    #[test]
    fn test_appends_new_row_not_in_existing() {
        let dir = tempfile::tempdir().unwrap();
        let existing = dir.path().join("existing.csv");
        let output = dir.path().join("output.csv");

        write_csv(&existing, &[
            &["id", "name"],
            &["1", "Alice"],
        ]);

        let updates: HashMap<String, Vec<String>> = [(
            "2".to_string(),
            vec!["2".to_string(), "Bob".to_string()],
        )]
        .into();
        merge_csv(&existing, &output, &updates, 0).unwrap();

        let rows = read_csv(&output);
        assert_eq!(rows, vec![
            vec!["id", "name"],
            vec!["1", "Alice"],
            vec!["2", "Bob"],
        ]);
    }

    #[test]
    fn test_empty_updates_copies_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let existing = dir.path().join("existing.csv");
        let output = dir.path().join("output.csv");

        write_csv(&existing, &[
            &["id", "val"],
            &["1", "a"],
            &["2", "b"],
        ]);

        merge_csv(&existing, &output, &HashMap::new(), 0).unwrap();

        let rows = read_csv(&output);
        assert_eq!(rows, vec![
            vec!["id", "val"],
            vec!["1", "a"],
            vec!["2", "b"],
        ]);
    }

    #[test]
    fn test_updates_by_coreid_in_place() {
        let dir = tempfile::tempdir().unwrap();
        let existing = dir.path().join("existing.csv");
        let output = dir.path().join("output.csv");

        write_csv(&existing, &[
            &["coreid", "url"],
            &["1", "http://example.com/a"],
            &["2", "http://example.com/b"],
        ]);

        let updates: HashMap<String, Vec<String>> = [(
            "1".to_string(),
            vec!["1".to_string(), "http://example.com/a-updated".to_string()],
        )]
        .into();
        merge_csv(&existing, &output, &updates, 0).unwrap();

        let rows = read_csv(&output);
        assert_eq!(rows, vec![
            vec!["coreid", "url"],
            vec!["1", "http://example.com/a-updated"], // updated in place
            vec!["2", "http://example.com/b"],
        ]);
    }

    #[test]
    fn test_mix_of_updates_and_new_rows() {
        let dir = tempfile::tempdir().unwrap();
        let existing = dir.path().join("existing.csv");
        let output = dir.path().join("output.csv");

        write_csv(&existing, &[
            &["id", "name"],
            &["1", "Alice"],
            &["2", "Bob"],
        ]);

        let updates: HashMap<String, Vec<String>> = [
            ("2".to_string(), vec!["2".to_string(), "Robert".to_string()]),
            ("3".to_string(), vec!["3".to_string(), "Carol".to_string()]),
        ]
        .into();
        merge_csv(&existing, &output, &updates, 0).unwrap();

        let rows = read_csv(&output);
        // "2" updated in place, "3" appended
        assert_eq!(rows[0], vec!["id", "name"]);
        assert_eq!(rows[1], vec!["1", "Alice"]);
        assert_eq!(rows[2], vec!["2", "Robert"]);
        assert_eq!(rows[3], vec!["3", "Carol"]);
    }
}

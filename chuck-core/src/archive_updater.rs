use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, atomic::AtomicBool};
use chrono::NaiveDate;

use crate::chuck_metadata::{read_chuck_metadata, read_pub_date};
use crate::api::params::parse_url_params;
use crate::darwin_core::{
    meta::generate_eml,
    meta::Metadata,
    occurrence::Occurrence,
    multimedia::Multimedia,
    audiovisual::Audiovisual,
    identification::Identification,
    comment::Comment,
};
use crate::downloader::{Downloader, DownloadProgress};
use crate::merge::merge_csv;
use crate::DwcaExtension;

/// Infer which DwC-A extensions are present in a ZIP archive by checking for
/// the corresponding CSV filenames.
pub fn infer_extensions(zip_path: &str) -> Result<Vec<DwcaExtension>, Box<dyn std::error::Error>> {
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let names: HashSet<String> = (0..archive.len())
        .map(|i| archive.by_index(i).map(|e| e.name().to_string()))
        .collect::<Result<_, _>>()?;

    let mut extensions = Vec::new();
    if names.contains(Multimedia::FILENAME) {
        extensions.push(DwcaExtension::SimpleMultimedia);
    }
    if names.contains(Audiovisual::FILENAME) {
        extensions.push(DwcaExtension::Audiovisual);
    }
    if names.contains(Identification::FILENAME) {
        extensions.push(DwcaExtension::Identifications);
    }
    if names.contains(Comment::FILENAME) {
        extensions.push(DwcaExtension::Comments);
    }
    Ok(extensions)
}

/// Returns true if the ZIP archive contains any files under `media/`.
pub fn archive_has_media(zip_path: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    for i in 0..archive.len() {
        if archive.by_index(i)?.name().starts_with("media/") {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Extract all entries from a ZIP archive into `dest_dir`.
fn extract_zip(zip_path: &str, dest_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let out_path = dest_dir.join(entry.name());
        if entry.is_dir() {
            std::fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut out_file = std::fs::File::create(&out_path)?;
            std::io::copy(&mut entry, &mut out_file)?;
        }
    }
    Ok(())
}

/// Recursively copy all files from `src` into `dest`, overwriting on conflict.
fn copy_dir_into(src: &Path, dest: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if !src.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_into(&src_path, &dest_path)?;
        } else {
            std::fs::copy(&src_path, &dest_path)?;
        }
    }
    Ok(())
}

/// Repack a directory of files into a ZIP archive.
fn repack_zip(src_dir: &Path, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    use zip::write::{FileOptions, ZipWriter};
    use zip::CompressionMethod;

    let zip_file = std::fs::File::create(output_path)?;
    let mut zip = ZipWriter::new(zip_file);

    let options: FileOptions<()> = FileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);
    let media_options: FileOptions<()> = FileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .unix_permissions(0o644);

    fn add_dir(
        zip: &mut ZipWriter<std::fs::File>,
        dir: &Path,
        prefix: &str,
        options: FileOptions<()>,
        media_options: FileOptions<()>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = if prefix.is_empty() {
                entry.file_name().to_string_lossy().to_string()
            } else {
                format!("{}/{}", prefix, entry.file_name().to_string_lossy())
            };
            if path.is_dir() {
                add_dir(zip, &path, &name, options, media_options)?;
            } else {
                let is_media = name.starts_with("media/");
                zip.start_file(&name, if is_media { media_options } else { options })?;
                let mut file = std::fs::File::open(&path)?;
                std::io::copy(&mut file, zip)?;
            }
        }
        Ok(())
    }

    add_dir(&mut zip, src_dir, "", options, media_options)?;
    zip.finish()?;
    Ok(())
}

/// Compute `updated_since` as `pub_date - 1 day`, formatted as `YYYY-MM-DD`.
pub fn updated_since_from_pub_date(pub_date: &str) -> Result<String, Box<dyn std::error::Error>> {
    let date = NaiveDate::parse_from_str(pub_date, "%Y-%m-%d")?;
    let updated_since = date - chrono::Duration::days(1);
    Ok(updated_since.format("%Y-%m-%d").to_string())
}

/// Update a Chuck DwC-A archive in place by fetching observations updated since
/// the archive's `pubDate` and merging them into the existing records.
///
/// Errors if:
/// - The archive has no `chuck.json` (not a Chuck archive)
/// - The archive has no `pubDate` in `eml.xml`
/// - The `inat_query` in `chuck.json` is absent or unparseable
pub async fn update_archive<F>(
    zip_path: &str,
    progress_callback: F,
    jwt: Option<String>,
    cancel_token: Option<Arc<AtomicBool>>,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: Fn(DownloadProgress) + Send + Sync + Clone + 'static,
{
    // --- Read archive metadata ---
    let chuck_meta = read_chuck_metadata(zip_path)?
        .ok_or("Not a Chuck archive: chuck.json not found")?;
    let original_inat_query = chuck_meta.inat_query
        .ok_or("chuck.json is missing inat_query")?;
    let pub_date = read_pub_date(zip_path)?
        .ok_or("eml.xml is missing pubDate")?;
    let updated_since = updated_since_from_pub_date(&pub_date)?;
    let extensions = infer_extensions(zip_path)?;
    let fetch_media = archive_has_media(zip_path)?;

    // --- Build update params ---
    let mut params = parse_url_params(&original_inat_query);
    params.updated_since = Some(updated_since);

    // --- Download updates to a temp archive ---
    let updates_tmp = tempfile::NamedTempFile::new()?;
    let updates_path = updates_tmp.path().to_str().unwrap().to_string();
    let downloader = Downloader::new(params, extensions, fetch_media, jwt);
    downloader.execute(&updates_path, progress_callback, cancel_token).await?;

    merge_archive_into(zip_path, &updates_path, zip_path, &original_inat_query)?;

    Ok(())
}

/// Merge `updates_zip` into `existing_zip`, writing the result to `output_path`.
///
/// Existing records are updated in-place if the same ID appears in `updates_zip`;
/// records in `updates_zip` with IDs not found in `existing_zip` are appended.
/// Media files are merged with updates taking precedence on conflict.
fn merge_archive_into(
    existing_zip: &str,
    updates_zip: &str,
    output_path: &str,
    original_inat_query: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // --- Extract both archives to temp dirs ---
    let existing_dir = tempfile::tempdir()?;
    let updates_dir = tempfile::tempdir()?;
    extract_zip(existing_zip, existing_dir.path())?;
    extract_zip(updates_zip, updates_dir.path())?;

    // --- Build updates map from updates occurrence.csv: id → row ---
    let updates_occurrence_csv = updates_dir.path().join(Occurrence::FILENAME);

    // Read a CSV into a HashMap<id_at_col, row>
    let read_updates_map = |csv_path: &Path, id_col: usize|
        -> Result<HashMap<String, Vec<String>>, Box<dyn std::error::Error>>
    {
        if !csv_path.exists() {
            return Ok(HashMap::new());
        }
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_path(csv_path)?;
        Ok(rdr.records()
            .filter_map(|r| r.ok())
            .filter_map(|r| {
                let id = r.get(id_col).map(String::from)?;
                Some((id, r.iter().map(String::from).collect()))
            })
            .collect())
    };

    // --- Merge CSVs ---
    let merge_dir = tempfile::tempdir()?;

    // Merge occurrence.csv
    let existing_occ = existing_dir.path().join(Occurrence::FILENAME);
    let merged_occ = merge_dir.path().join(Occurrence::FILENAME);
    let occ_updates = read_updates_map(&updates_occurrence_csv, 0)?;
    merge_csv(&existing_occ, &merged_occ, &occ_updates, 0)?;

    // Build a coreid-keyed map for each extension CSV (coreid is column 0)
    let extension_filenames = [
        Multimedia::FILENAME,
        Audiovisual::FILENAME,
        Identification::FILENAME,
        Comment::FILENAME,
    ];
    for filename in &extension_filenames {
        let existing_csv = existing_dir.path().join(filename);
        let updates_csv = updates_dir.path().join(filename);
        if !existing_csv.exists() {
            continue;
        }
        let merged_csv = merge_dir.path().join(filename);
        let ext_updates = read_updates_map(&updates_csv, 0)?;
        merge_csv(&existing_csv, &merged_csv, &ext_updates, 0)?;
    }

    // Copy meta.xml from existing (schema hasn't changed)
    std::fs::copy(
        existing_dir.path().join("meta.xml"),
        merge_dir.path().join("meta.xml"),
    )?;

    // Write updated eml.xml (pubDate = today) with original inat_query preserved
    let new_metadata = Metadata {
        abstract_lines: vec![],
        inat_query: Some(original_inat_query.to_string()),
    };
    let eml_xml = generate_eml(&new_metadata);
    std::fs::write(merge_dir.path().join("eml.xml"), &eml_xml)?;

    // Write chuck.json with original inat_query (not the updated_since version)
    let chuck_json = serde_json::json!({ "inat_query": original_inat_query }).to_string();
    std::fs::write(merge_dir.path().join("chuck.json"), &chuck_json)?;

    // Merge media dirs: existing first, then overlay updates
    let merged_media = merge_dir.path().join("media");
    copy_dir_into(&existing_dir.path().join("media"), &merged_media)?;
    copy_dir_into(&updates_dir.path().join("media"), &merged_media)?;

    // --- Repack ZIP to output path atomically ---
    // Write to a temp file in the same directory first, then rename over the
    // target so a crash mid-write never leaves a corrupt or truncated archive.
    let output_path_obj = Path::new(output_path);
    let output_dir = output_path_obj.parent().unwrap_or(Path::new("."));
    let tmp_output = tempfile::NamedTempFile::new_in(output_dir)?;
    let tmp_path = tmp_output
        .path()
        .to_str()
        .ok_or("temp file path is not valid UTF-8")?
        .to_string();
    repack_zip(merge_dir.path(), &tmp_path)?;
    tmp_output.persist(output_path_obj).map_err(|e| e.error)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal ZIP with an occurrence.csv (given as raw CSV text),
    /// a stub meta.xml, an eml.xml with the given pubDate, and a chuck.json.
    fn build_test_zip(path: &str, occurrence_csv: &str, inat_query: &str, pub_date: &str) {
        use std::io::Write;
        use zip::CompressionMethod;
        use zip::write::FileOptions;

        let file = std::fs::File::create(path).unwrap();
        let mut zip = zip::write::ZipWriter::new(file);
        let opts: FileOptions<()> =
            FileOptions::default().compression_method(CompressionMethod::Deflated);

        zip.start_file("occurrence.csv", opts).unwrap();
        zip.write_all(occurrence_csv.as_bytes()).unwrap();

        zip.start_file("meta.xml", opts).unwrap();
        zip.write_all(b"<archive/>").unwrap();

        let eml = format!(
            "<eml><dataset><pubDate>{pub_date}</pubDate></dataset></eml>"
        );
        zip.start_file("eml.xml", opts).unwrap();
        zip.write_all(eml.as_bytes()).unwrap();

        let chuck = format!(r#"{{"inat_query":"{inat_query}"}}"#);
        zip.start_file("chuck.json", opts).unwrap();
        zip.write_all(chuck.as_bytes()).unwrap();

        zip.finish().unwrap();
    }

    /// Read occurrence.csv from a ZIP and return its data rows (excluding header).
    fn read_occ_rows(zip_path: &str) -> Vec<String> {
        use std::io::Read;
        let file = std::fs::File::open(zip_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut entry = archive.by_name(Occurrence::FILENAME).unwrap();
        let mut content = String::new();
        entry.read_to_string(&mut content).unwrap();
        content.lines().skip(1).map(String::from).collect()
    }

    #[test]
    fn test_merge_archive_into_updates_in_place_and_appends_new() {
        let existing_tmp = tempfile::NamedTempFile::new().unwrap();
        let updates_tmp = tempfile::NamedTempFile::new().unwrap();
        let output_tmp = tempfile::NamedTempFile::new().unwrap();

        let existing_path = existing_tmp.path().to_str().unwrap().to_string();
        let updates_path = updates_tmp.path().to_str().unwrap().to_string();
        let output_path = output_tmp.path().to_str().unwrap().to_string();

        // Existing archive: obs/1 (original value), obs/2
        build_test_zip(
            &existing_path,
            "id,name\n\
             https://www.inaturalist.org/observations/1,original\n\
             https://www.inaturalist.org/observations/2,unchanged\n",
            "taxon_id=47790",
            "2026-03-01",
        );

        // Updates archive: obs/1 (updated value), obs/3 (new)
        build_test_zip(
            &updates_path,
            "id,name\n\
             https://www.inaturalist.org/observations/1,updated\n\
             https://www.inaturalist.org/observations/3,new\n",
            "taxon_id=47790",
            "2026-03-24",
        );

        merge_archive_into(&existing_path, &updates_path, &output_path, "taxon_id=47790")
            .unwrap();

        let rows = read_occ_rows(&output_path);
        assert_eq!(rows.len(), 3, "expected 3 rows: updated obs/1, unchanged obs/2, new obs/3");
        // obs/1 updated in place (first position preserved)
        assert_eq!(rows[0], "https://www.inaturalist.org/observations/1,updated");
        // obs/2 unchanged, original position preserved
        assert_eq!(rows[1], "https://www.inaturalist.org/observations/2,unchanged");
        // obs/3 appended
        assert_eq!(rows[2], "https://www.inaturalist.org/observations/3,new");
    }

    #[test]
    fn test_merge_archive_into_in_place_update() {
        // Verify merge_archive_into works when output_path == existing_zip,
        // which is the path taken by update_archive. The atomic rename must not
        // corrupt the source file even though they share a path.
        let existing_tmp = tempfile::NamedTempFile::new().unwrap();
        let updates_tmp = tempfile::NamedTempFile::new().unwrap();
        let existing_path = existing_tmp.path().to_str().unwrap().to_string();
        let updates_path = updates_tmp.path().to_str().unwrap().to_string();

        build_test_zip(
            &existing_path,
            "id,name\n\
             https://www.inaturalist.org/observations/1,original\n",
            "taxon_id=1",
            "2026-01-01",
        );
        build_test_zip(
            &updates_path,
            "id,name\n\
             https://www.inaturalist.org/observations/1,updated\n\
             https://www.inaturalist.org/observations/2,new\n",
            "taxon_id=1",
            "2026-01-02",
        );

        // output_path == existing_zip (in-place)
        merge_archive_into(&existing_path, &updates_path, &existing_path, "taxon_id=1")
            .unwrap();

        let rows = read_occ_rows(&existing_path);
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0],
            "https://www.inaturalist.org/observations/1,updated"
        );
        assert_eq!(rows[1], "https://www.inaturalist.org/observations/2,new");
    }

    #[test]
    fn test_updated_since_from_pub_date() {
        assert_eq!(
            updated_since_from_pub_date("2026-03-24").unwrap(),
            "2026-03-23"
        );
    }

    #[test]
    fn test_updated_since_crosses_month_boundary() {
        assert_eq!(
            updated_since_from_pub_date("2026-03-01").unwrap(),
            "2026-02-28"
        );
    }

    #[tokio::test]
    async fn test_infer_extensions_detects_multimedia() {
        use crate::darwin_core::meta::Metadata;
        use crate::darwin_core::archive::ArchiveBuilder;

        let metadata = Metadata::default();
        let builder = ArchiveBuilder::new(
            vec![DwcaExtension::SimpleMultimedia, DwcaExtension::Identifications],
            metadata,
        ).unwrap();
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        builder.build(&path).await.unwrap();

        let exts = infer_extensions(&path).unwrap();
        assert!(exts.contains(&DwcaExtension::SimpleMultimedia));
        assert!(exts.contains(&DwcaExtension::Identifications));
        assert!(!exts.contains(&DwcaExtension::Audiovisual));
        assert!(!exts.contains(&DwcaExtension::Comments));
    }

    #[tokio::test]
    async fn test_archive_has_media_false_for_empty_archive() {
        use crate::darwin_core::meta::Metadata;
        use crate::darwin_core::archive::ArchiveBuilder;

        let metadata = Metadata::default();
        let builder = ArchiveBuilder::new(vec![], metadata).unwrap();
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        builder.build(&path).await.unwrap();

        assert!(!archive_has_media(&path).unwrap());
    }
}

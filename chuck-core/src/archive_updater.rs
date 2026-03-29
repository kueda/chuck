use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, atomic::AtomicBool};
use chrono::NaiveDate;

use crate::chuck_metadata::{ChuckMetadata, parse_pub_date_from_xml};
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
use crate::downloader::{Downloader, DownloadProgress, DownloadStage};
use crate::merge::{merge_csv_streams, merge_extension_csv_streams};
use crate::DwcaExtension;

/// Infer which DwC-A extensions are present in a ZIP archive by checking for
/// the corresponding CSV filenames.
///
/// This approach was chosen over parsing `meta.xml` because it confirms the
/// extension file actually exists, whereas `meta.xml` could declare an extension
/// whose CSV is absent in a corrupt or partial archive.
///
/// Trade-off: detection is coupled to the `FILENAME` constants on each type
/// (e.g. `Multimedia::FILENAME`). If a constant is renamed, this function will
/// silently stop detecting that extension. The alternative — matching on the
/// stable `rowType` URI in `meta.xml` — would be resilient to filename changes
/// but would trust the manifest over the actual contents.
pub fn infer_extensions(zip_path: &str) -> Result<Vec<DwcaExtension>, Box<dyn std::error::Error>> {
    let file = std::fs::File::open(zip_path)?;
    let archive = zip::ZipArchive::new(file)?;
    Ok(extensions_from_zip(&archive))
}

/// Infer extensions from an already-open ZipArchive using the central directory.
/// Uses `file_names()` to avoid seeking to each local file header.
fn extensions_from_zip<R: std::io::Read + std::io::Seek>(
    archive: &zip::ZipArchive<R>,
) -> Vec<DwcaExtension> {
    let names: HashSet<&str> = archive.file_names().collect();
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
    extensions
}

/// Returns true if the ZIP archive contains any files under `media/`.
pub fn archive_has_media(zip_path: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let file = std::fs::File::open(zip_path)?;
    let archive = zip::ZipArchive::new(file)?;
    Ok(archive.file_names().any(|name| name.starts_with("media/")))
}

/// All metadata needed by the update UI, read in a single zip open.
pub struct ArchivePreview {
    pub inat_query: Option<String>,
    pub pub_date: Option<String>,
    pub extensions: Vec<DwcaExtension>,
    pub has_media: bool,
}

/// Read all archive metadata needed to populate the update UI in a single zip
/// open. Uses `file_names()` to scan the central directory without seeking to
/// each local file header, which is critical for large archives with many media
/// files.
pub fn read_archive_preview(
    zip_path: &str,
) -> Result<ArchivePreview, Box<dyn std::error::Error>> {
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    // Collect names from the central directory (already in memory after
    // ZipArchive::new). This avoids the per-entry seeks that by_index() causes.
    let extensions = extensions_from_zip(&archive);
    let has_media = archive.file_names().any(|name| name.starts_with("media/"));

    let inat_query = match archive.by_name("chuck.json") {
        Ok(mut entry) => {
            let mut contents = String::new();
            std::io::Read::read_to_string(&mut entry, &mut contents)?;
            let meta: ChuckMetadata = serde_json::from_str(&contents)?;
            meta.inat_query
        }
        Err(zip::result::ZipError::FileNotFound) => None,
        Err(e) => return Err(e.into()),
    };

    let pub_date = match archive.by_name("eml.xml") {
        Ok(mut entry) => {
            let mut contents = String::new();
            std::io::Read::read_to_string(&mut entry, &mut contents)?;
            parse_pub_date_from_xml(&contents)
        }
        Err(zip::result::ZipError::FileNotFound) => None,
        Err(e) => return Err(e.into()),
    };

    Ok(ArchivePreview { inat_query, pub_date, extensions, has_media })
}

/// Read a CSV stream into a `HashMap<id_at_col, row>` (one row per id).
/// Used for occurrence.csv where each observation has exactly one row.
fn read_updates_map_from_reader<R: std::io::Read>(
    reader: R,
    id_col: usize,
) -> Result<HashMap<String, Vec<String>>, Box<dyn std::error::Error>> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(reader);
    Ok(rdr
        .records()
        .filter_map(|r| r.ok())
        .filter_map(|r| {
            let id = r.get(id_col).map(String::from)?;
            Some((id, r.iter().map(String::from).collect()))
        })
        .collect())
}

/// Read a CSV stream into a `HashMap<id_at_col, Vec<rows>>`, grouping all rows
/// with the same id. Used for extension CSVs where one observation (coreid) has
/// multiple rows (e.g. one row per photo in multimedia.csv).
type GroupedMap = HashMap<String, Vec<Vec<String>>>;
fn read_grouped_updates_from_reader<R: std::io::Read>(
    reader: R,
    id_col: usize,
) -> Result<GroupedMap, Box<dyn std::error::Error>> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(reader);
    let mut map: HashMap<String, Vec<Vec<String>>> = HashMap::new();
    for result in rdr.records() {
        let record = result?;
        let id = match record.get(id_col) {
            Some(id) => id.to_string(),
            None => continue,
        };
        map.entry(id).or_default().push(record.iter().map(String::from).collect());
    }
    Ok(map)
}

/// Read `<para>` lines from the `<abstract>` section of an EML stream.
/// Returns an empty vec if the stream is unreadable or has no `<abstract>`.
fn read_abstract_lines_from_reader<R: std::io::Read>(mut reader: R) -> Vec<String> {
    let mut content = String::new();
    if reader.read_to_string(&mut content).is_err() {
        return vec![];
    }
    let abs_start = match content.find("<abstract>") {
        Some(i) => i + "<abstract>".len(),
        None => return vec![],
    };
    let abs_end = match content[abs_start..].find("</abstract>") {
        Some(i) => abs_start + i,
        None => return vec![],
    };
    let section = &content[abs_start..abs_end];
    let mut lines = Vec::new();
    let mut rest = section;
    while let Some(start) = rest.find("<para>") {
        rest = &rest[start + "<para>".len()..];
        if let Some(end) = rest.find("</para>") {
            let text = rest[..end].trim().to_string();
            if !text.is_empty() {
                lines.push(text);
            }
            rest = &rest[end + "</para>".len()..];
        } else {
            break;
        }
    }
    lines
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
    let preview = read_archive_preview(zip_path)?;
    let original_inat_query = preview.inat_query
        .ok_or("Not a Chuck archive: chuck.json not found or missing inat_query")?;
    let pub_date = preview.pub_date
        .ok_or("eml.xml is missing pubDate")?;
    let updated_since = updated_since_from_pub_date(&pub_date)?;
    let extensions = preview.extensions;
    let fetch_media = preview.has_media;

    // --- Build update params ---
    let mut params = parse_url_params(&original_inat_query);
    params.updated_since = Some(updated_since);

    // --- Download updates to a temp archive ---
    let updates_tmp = tempfile::NamedTempFile::new()?;
    let updates_path = updates_tmp.path().to_str().unwrap().to_string();
    let downloader = Downloader::new(params, extensions, fetch_media, jwt);
    let callback_for_merge = progress_callback.clone();
    downloader.execute(&updates_path, progress_callback, cancel_token).await?;

    merge_archive_append(zip_path, &updates_path, &original_inat_query, &callback_for_merge)?;

    Ok(())
}

/// Append-based merge: rewrites only CSVs and metadata, keeping existing media
/// local file entries in place by truncating the old central directory and writing
/// a new one.
///
/// This is O(update_size + CSV_size) I/O rather than O(archive_size), making it
/// much faster for large archives where only a small fraction of observations changed.
///
/// How it works:
/// 1. Read CSV updates into memory (small).
/// 2. Collect central-directory metadata for every existing media/meta.xml entry
///    (no media data is read — only the CD record, already in RAM after
///    `ZipArchive::new`).
/// 3. Seek to `central_directory_start`, truncating the old CD in place.
/// 4. Write new local file entries for merged CSVs, fresh eml.xml/chuck.json,
///    and any new media from the updates.
/// 5. Write a combined central directory: existing media entries (original offsets,
///    untouched) + new entries (new offsets).
/// 6. Write the End-of-Central-Directory record.
///
/// Known limitations vs `merge_archive_into`:
/// - Not atomic: modifies the file in place (no temp-file + rename).
/// - Dead space accumulates: old CSV local entries are unreachable but remain in
///   the file, growing it slightly with each update.
/// - Superseded media (re-uploaded photo for an existing observation) keeps its
///   old local entry as dead space; only the CSV reference changes.
fn merge_archive_append(
    zip_path: &str,
    updates_zip: &str,
    original_inat_query: &str,
    progress_callback: &impl Fn(DownloadProgress),
) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::{Seek, SeekFrom, Write};

    // ---- Binary helpers (non-ZIP64; assumes sizes < 4 GiB) ----

    fn write_local_header(
        w: &mut impl Write,
        name_bytes: &[u8],
        method: u16,
        crc32: u32,
        compressed: u32,
        uncompressed: u32,
    ) -> std::io::Result<()> {
        // DOS timestamp: 2000-01-01 00:00:00
        let dos_date: u16 = 0x2821;
        let dos_time: u16 = 0x0000;
        w.write_all(&0x04034b50u32.to_le_bytes())?; // signature
        w.write_all(&20u16.to_le_bytes())?;          // version needed
        w.write_all(&0u16.to_le_bytes())?;           // flags
        w.write_all(&method.to_le_bytes())?;
        w.write_all(&dos_time.to_le_bytes())?;
        w.write_all(&dos_date.to_le_bytes())?;
        w.write_all(&crc32.to_le_bytes())?;
        w.write_all(&compressed.to_le_bytes())?;
        w.write_all(&uncompressed.to_le_bytes())?;
        w.write_all(&(name_bytes.len() as u16).to_le_bytes())?;
        w.write_all(&0u16.to_le_bytes())?; // extra len
        w.write_all(name_bytes)?;
        Ok(())
    }

    /// Returns the number of bytes written.
    fn write_cd_entry(
        w: &mut impl Write,
        name_bytes: &[u8],
        method: u16,
        crc32: u32,
        compressed: u32,
        uncompressed: u32,
        local_offset: u32,
    ) -> std::io::Result<usize> {
        let dos_date: u16 = 0x2821;
        let dos_time: u16 = 0x0000;
        w.write_all(&0x02014b50u32.to_le_bytes())?; // signature
        w.write_all(&20u16.to_le_bytes())?;          // version made by
        w.write_all(&20u16.to_le_bytes())?;          // version needed
        w.write_all(&0u16.to_le_bytes())?;           // flags
        w.write_all(&method.to_le_bytes())?;
        w.write_all(&dos_time.to_le_bytes())?;
        w.write_all(&dos_date.to_le_bytes())?;
        w.write_all(&crc32.to_le_bytes())?;
        w.write_all(&compressed.to_le_bytes())?;
        w.write_all(&uncompressed.to_le_bytes())?;
        w.write_all(&(name_bytes.len() as u16).to_le_bytes())?;
        w.write_all(&0u16.to_le_bytes())?; // extra len
        w.write_all(&0u16.to_le_bytes())?; // comment len
        w.write_all(&0u16.to_le_bytes())?; // disk start
        w.write_all(&0u16.to_le_bytes())?; // internal attrs
        // External attrs: regular file, rw-r--r--
        w.write_all(&(0o100644u32 << 16).to_le_bytes())?;
        w.write_all(&local_offset.to_le_bytes())?;
        w.write_all(name_bytes)?;
        Ok(46 + name_bytes.len())
    }

    fn write_eocd(
        w: &mut impl Write,
        num_entries: u16,
        cd_size: u32,
        cd_offset: u32,
    ) -> std::io::Result<()> {
        w.write_all(&0x06054b50u32.to_le_bytes())?; // signature
        w.write_all(&0u16.to_le_bytes())?;           // disk number
        w.write_all(&0u16.to_le_bytes())?;           // disk with start of CD
        w.write_all(&num_entries.to_le_bytes())?;    // entries on this disk
        w.write_all(&num_entries.to_le_bytes())?;    // total entries
        w.write_all(&cd_size.to_le_bytes())?;
        w.write_all(&cd_offset.to_le_bytes())?;
        w.write_all(&0u16.to_le_bytes())?; // comment len
        Ok(())
    }

    // Metadata for an existing entry to preserve in the new CD (no data read).
    struct PreservedEntry {
        name: Vec<u8>,
        method: u16,
        crc32: u32,
        compressed_size: u32,
        uncompressed_size: u32,
        local_offset: u32,
    }

    // Metadata for a newly written entry.
    struct NewEntry {
        name: Vec<u8>,
        crc32: u32,
        size: u32,
        local_offset: u32,
    }

    let csv_filenames: HashSet<&str> = [
        Occurrence::FILENAME,
        Multimedia::FILENAME,
        Audiovisual::FILENAME,
        Identification::FILENAME,
        Comment::FILENAME,
    ]
    .into_iter()
    .collect();

    // ---- Phase 1: Read CSV updates from the updates ZIP ----
    progress_callback(DownloadProgress {
        stage: DownloadStage::Merging { current: 0, total: 3 },
        ..Default::default()
    });

    let mut occ_map: HashMap<String, Vec<String>> = HashMap::new();
    let mut ext_maps: HashMap<String, GroupedMap> = HashMap::new();
    let mut new_media: Vec<(Vec<u8>, Vec<u8>)> = Vec::new(); // (name_bytes, data)
    {
        let updates_file = std::fs::File::open(updates_zip)?;
        let mut updates_archive = zip::ZipArchive::new(updates_file)?;
        for i in 0..updates_archive.len() {
            let mut entry = updates_archive.by_index(i)?;
            let name = entry.name().to_string();
            if name == Occurrence::FILENAME {
                occ_map = read_updates_map_from_reader(&mut entry, 0)?;
            } else if csv_filenames.contains(name.as_str()) {
                ext_maps.insert(name, read_grouped_updates_from_reader(&mut entry, 0)?);
            } else if name.starts_with("media/") {
                let mut data = Vec::new();
                std::io::Read::read_to_end(&mut entry, &mut data)?;
                new_media.push((name.into_bytes(), data));
            }
        }
    }

    progress_callback(DownloadProgress {
        stage: DownloadStage::Merging { current: 1, total: 3 },
        ..Default::default()
    });

    // ---- Phase 2: Read existing archive; collect preserved entries + merge CSVs ----
    // Build a set of media filenames present in the updates so we can exclude them
    // from `preserved` — an existing entry superseded by an update must not appear
    // twice in the central directory.
    let new_media_names: HashSet<&[u8]> = new_media.iter().map(|(n, _)| n.as_slice()).collect();

    let mut preserved: Vec<PreservedEntry> = Vec::new();
    let mut merged_csvs: Vec<(Vec<u8>, Vec<u8>)> = Vec::new(); // (name_bytes, data)
    let eml_abstract_lines;
    let cd_start;
    {
        let existing_file = std::fs::File::open(zip_path)?;
        let mut archive = zip::ZipArchive::new(existing_file)?;
        cd_start = archive.central_directory_start();

        for i in 0..archive.len() {
            // Use by_index_raw first to get name and metadata without decompressing.
            let (name, is_preserve) = {
                let entry = archive.by_index_raw(i)?;
                let name = entry.name().to_string();
                // Preserve media/meta.xml unless superseded by the update.
                let keep = (name.starts_with("media/") || name == "meta.xml")
                    && !new_media_names.contains(entry.name().as_bytes());
                if keep {
                    preserved.push(PreservedEntry {
                        name: entry.name().as_bytes().to_vec(),
                        method: match entry.compression() {
                            zip::CompressionMethod::Deflated => 8,
                            _ => 0, // Stored
                        },
                        crc32: entry.crc32(),
                        compressed_size: entry.compressed_size() as u32,
                        uncompressed_size: entry.size() as u32,
                        local_offset: entry.header_start() as u32,
                    });
                }
                (name, keep)
            };
            if is_preserve {
                continue;
            }
            // Decompressed access for entries we need to read.
            if name == Occurrence::FILENAME {
                let mut entry = archive.by_index(i)?;
                let mut buf = Vec::new();
                merge_csv_streams(&mut entry, &mut buf, &occ_map, 0)?;
                merged_csvs.push((name.into_bytes(), buf));
            } else if csv_filenames.contains(name.as_str()) {
                let empty_map = HashMap::new();
                let updates = ext_maps.get(&name).unwrap_or(&empty_map);
                let mut entry = archive.by_index(i)?;
                let mut buf = Vec::new();
                merge_extension_csv_streams(&mut entry, &mut buf, updates, 0)?;
                merged_csvs.push((name.into_bytes(), buf));
            }
            // eml.xml and chuck.json are skipped here; fresh versions written below.
        }

        eml_abstract_lines = match archive.by_name("eml.xml") {
            Ok(entry) => read_abstract_lines_from_reader(entry),
            Err(_) => vec![],
        };
    }

    progress_callback(DownloadProgress {
        stage: DownloadStage::Merging { current: 2, total: 3 },
        ..Default::default()
    });

    // ---- Phase 3: Truncate at cd_start and write new local entries ----
    let mut file = std::fs::OpenOptions::new().read(true).write(true).open(zip_path)?;
    file.seek(SeekFrom::Start(cd_start))?;

    let mut write_pos: u64 = cd_start;
    let mut new_entries: Vec<NewEntry> = Vec::new();

    // Write one STORED (uncompressed) local entry; update write_pos and new_entries.
    let mut write_stored = |name_bytes: &[u8], data: &[u8]| -> Result<(), Box<dyn std::error::Error>> {
        let crc = crc32fast::hash(data);
        let size = data.len() as u32;
        let offset = write_pos as u32;
        write_local_header(&mut file, name_bytes, 0, crc, size, size)?;
        file.write_all(data)?;
        write_pos += 30 + name_bytes.len() as u64 + size as u64;
        new_entries.push(NewEntry {
            name: name_bytes.to_vec(),
            crc32: crc,
            size,
            local_offset: offset,
        });
        Ok(())
    };

    for (name_bytes, data) in &merged_csvs {
        write_stored(name_bytes, data)?;
    }

    let new_metadata = Metadata {
        abstract_lines: eml_abstract_lines,
        inat_query: Some(original_inat_query.to_string()),
    };
    let eml_bytes = generate_eml(&new_metadata).into_bytes();
    write_stored(b"eml.xml", &eml_bytes)?;

    let chuck_json = serde_json::json!({ "inat_query": original_inat_query })
        .to_string()
        .into_bytes();
    write_stored(b"chuck.json", &chuck_json)?;

    for (name_bytes, data) in &new_media {
        write_stored(name_bytes, data)?;
    }

    // ---- Phase 4: Write combined central directory ----
    let new_cd_start = write_pos;
    let mut cd_size: u32 = 0;

    for e in &preserved {
        cd_size += write_cd_entry(
            &mut file,
            &e.name,
            e.method,
            e.crc32,
            e.compressed_size,
            e.uncompressed_size,
            e.local_offset,
        )? as u32;
    }
    for e in &new_entries {
        cd_size += write_cd_entry(
            &mut file,
            &e.name,
            0, // STORED
            e.crc32,
            e.size,
            e.size,
            e.local_offset,
        )? as u32;
    }

    let total_entries = (preserved.len() + new_entries.len()) as u16;
    write_eocd(&mut file, total_entries, cd_size, new_cd_start as u32)?;

    // Truncate to exact final size (handles case where new content is shorter).
    let final_size = new_cd_start + cd_size as u64 + 22;
    file.set_len(final_size)?;

    progress_callback(DownloadProgress {
        stage: DownloadStage::Merging { current: 3, total: 3 },
        ..Default::default()
    });

    Ok(())
}

/// Merge `updates_zip` into `existing_zip`, writing the result to `output_path`.
///
/// Streams directly between ZIP files without extracting to disk:
///
/// - Pass 1: scan `updates_zip` to build in-memory CSV update maps and a set
///   of media filenames present in the updates.
/// - Pass 2: stream `existing_zip` → output ZIP, merging each CSV and skipping
///   media files that are superseded by the updates.
/// - Pass 3: stream update media from `updates_zip` → output ZIP.
///
/// The output is written atomically: a temp file in the same directory as
/// `output_path` is used, then renamed over the target.
fn merge_archive_into(
    existing_zip: &str,
    updates_zip: &str,
    output_path: &str,
    original_inat_query: &str,
    progress_callback: &impl Fn(DownloadProgress),
) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write;
    use zip::write::{FileOptions, ZipWriter};
    use zip::CompressionMethod;

    let csv_filenames: HashSet<&str> = [
        Occurrence::FILENAME,
        Multimedia::FILENAME,
        Audiovisual::FILENAME,
        Identification::FILENAME,
        Comment::FILENAME,
    ]
    .into_iter()
    .collect();

    let options: FileOptions<()> = FileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);
    let media_options: FileOptions<()> = FileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .unix_permissions(0o644);

    // --- Pass 1: Build CSV update maps and media filename set from updates ZIP ---
    // occurrence.csv: one row per id → HashMap<id, row>
    // extension CSVs: many rows per coreid → HashMap<coreid, Vec<rows>>
    let mut occ_map: HashMap<String, Vec<String>> = HashMap::new();
    let mut ext_maps: HashMap<String, GroupedMap> = HashMap::new();
    let mut media_in_updates: HashSet<String> = HashSet::new();
    {
        let updates_file = std::fs::File::open(updates_zip)?;
        let mut updates_archive = zip::ZipArchive::new(updates_file)?;
        for i in 0..updates_archive.len() {
            let mut entry = updates_archive.by_index(i)?;
            let name = entry.name().to_string();
            if name == Occurrence::FILENAME {
                occ_map = read_updates_map_from_reader(&mut entry, 0)?;
            } else if csv_filenames.contains(name.as_str()) {
                ext_maps.insert(name, read_grouped_updates_from_reader(&mut entry, 0)?);
            } else if name.starts_with("media/") {
                media_in_updates.insert(name);
            }
        }
    }

    // --- Passes 2 & 3: Stream to output ZIP (atomically via temp file) ---
    let output_path_obj = Path::new(output_path);
    let output_dir = output_path_obj.parent().unwrap_or(Path::new("."));
    let tmp_output = tempfile::NamedTempFile::new_in(output_dir)?;
    let mut zip_out = ZipWriter::new(tmp_output);

    // Pass 2: Stream existing ZIP → output, merging CSVs, skipping superseded media
    {
        let existing_file = std::fs::File::open(existing_zip)?;
        let mut existing_archive = zip::ZipArchive::new(existing_file)?;
        let total = existing_archive.len();
        // Emit at start and every ~1% of entries so the UI stays responsive
        // without flooding the event channel.
        let step = (total / 100).max(1);
        progress_callback(DownloadProgress {
            stage: DownloadStage::Merging { current: 0, total },
            ..Default::default()
        });
        for i in 0..total {
            let mut entry = existing_archive.by_index(i)?;
            let name = entry.name().to_string();
            if name == "chuck.json" {
                // Written fresh below
            } else if name == "eml.xml" {
                let abstract_lines = read_abstract_lines_from_reader(&mut entry);
                let new_metadata = Metadata {
                    abstract_lines,
                    inat_query: Some(original_inat_query.to_string()),
                };
                zip_out.start_file(&name, options)?;
                zip_out.write_all(generate_eml(&new_metadata).as_bytes())?;
            } else if name == "meta.xml" {
                zip_out.start_file(&name, options)?;
                std::io::copy(&mut entry, &mut zip_out)?;
            } else if name == Occurrence::FILENAME {
                zip_out.start_file(&name, options)?;
                merge_csv_streams(&mut entry, &mut zip_out, &occ_map, 0)?;
            } else if csv_filenames.contains(name.as_str()) {
                let empty_map = HashMap::new();
                let updates = ext_maps.get(&name).unwrap_or(&empty_map);
                zip_out.start_file(&name, options)?;
                merge_extension_csv_streams(&mut entry, &mut zip_out, updates, 0)?;
            } else if name.starts_with("media/") && !media_in_updates.contains(&name) {
                zip_out.start_file(&name, media_options)?;
                std::io::copy(&mut entry, &mut zip_out)?;
            }
            // media superseded by updates and unknown entries are skipped

            let processed = i + 1;
            if processed % step == 0 || processed == total {
                progress_callback(DownloadProgress {
                    stage: DownloadStage::Merging { current: processed, total },
                    ..Default::default()
                });
            }
        }
    }

    // Pass 3: Stream update media → output ZIP (updates take precedence)
    {
        let updates_file = std::fs::File::open(updates_zip)?;
        let mut updates_archive = zip::ZipArchive::new(updates_file)?;
        for i in 0..updates_archive.len() {
            let mut entry = updates_archive.by_index(i)?;
            let name = entry.name().to_string();
            if name.starts_with("media/") {
                zip_out.start_file(&name, media_options)?;
                std::io::copy(&mut entry, &mut zip_out)?;
            }
        }
    }

    // Write fresh chuck.json preserving the original (non-updated_since) query
    let chuck_json = serde_json::json!({ "inat_query": original_inat_query }).to_string();
    zip_out.start_file("chuck.json", options)?;
    zip_out.write_all(chuck_json.as_bytes())?;

    let tmp_output = zip_out.finish()?;
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

    /// Build a minimal ZIP with abstract content in eml.xml.
    fn build_test_zip_with_abstract(
        path: &str,
        occurrence_csv: &str,
        inat_query: &str,
        pub_date: &str,
        abstract_lines: &[&str],
    ) {
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

        let paras: String = abstract_lines
            .iter()
            .map(|l| format!("<para>{l}</para>"))
            .collect::<Vec<_>>()
            .join("");
        let eml = format!(
            "<eml><dataset><pubDate>{pub_date}</pubDate>\
             <abstract>{paras}</abstract></dataset></eml>"
        );
        zip.start_file("eml.xml", opts).unwrap();
        zip.write_all(eml.as_bytes()).unwrap();

        let chuck = format!(r#"{{"inat_query":"{inat_query}"}}"#);
        zip.start_file("chuck.json", opts).unwrap();
        zip.write_all(chuck.as_bytes()).unwrap();

        zip.finish().unwrap();
    }

    /// Build a ZIP with an extra CSV (e.g. multimedia.csv) in addition to the
    /// standard occurrence.csv / meta.xml / eml.xml / chuck.json.
    fn build_test_zip_with_extra_csv(
        path: &str,
        occurrence_csv: &str,
        inat_query: &str,
        pub_date: &str,
        extra_csv_name: &str,
        extra_csv: &str,
    ) {
        use std::io::Write;
        use zip::CompressionMethod;
        use zip::write::FileOptions;

        let file = std::fs::File::create(path).unwrap();
        let mut zip = zip::write::ZipWriter::new(file);
        let opts: FileOptions<()> =
            FileOptions::default().compression_method(CompressionMethod::Deflated);

        zip.start_file("occurrence.csv", opts).unwrap();
        zip.write_all(occurrence_csv.as_bytes()).unwrap();

        zip.start_file(extra_csv_name, opts).unwrap();
        zip.write_all(extra_csv.as_bytes()).unwrap();

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

    /// Read a named CSV from a ZIP and return its data rows (excluding header).
    fn read_csv_rows_from_zip(zip_path: &str, csv_name: &str) -> Vec<String> {
        use std::io::Read;
        let file = std::fs::File::open(zip_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut entry = archive.by_name(csv_name).unwrap();
        let mut content = String::new();
        entry.read_to_string(&mut content).unwrap();
        content.lines().skip(1).map(String::from).collect()
    }

    /// Build a minimal ZIP with media files.
    fn build_test_zip_with_media(
        path: &str,
        occurrence_csv: &str,
        inat_query: &str,
        pub_date: &str,
        media_files: &[(&str, &[u8])],
    ) {
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

        for (name, content) in media_files {
            zip.start_file(name, opts).unwrap();
            zip.write_all(content).unwrap();
        }

        zip.finish().unwrap();
    }

    /// Read all media entries from a ZIP, returning a map of name → bytes.
    fn read_media_from_zip(zip_path: &str) -> std::collections::HashMap<String, Vec<u8>> {
        use std::io::Read;
        let file = std::fs::File::open(zip_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut result = std::collections::HashMap::new();
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).unwrap();
            if entry.name().starts_with("media/") {
                let mut content = Vec::new();
                entry.read_to_end(&mut content).unwrap();
                result.insert(entry.name().to_string(), content);
            }
        }
        result
    }

    /// Read eml.xml from a ZIP and return its full content.
    fn read_eml_from_zip(zip_path: &str) -> String {
        use std::io::Read;
        let file = std::fs::File::open(zip_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut entry = archive.by_name("eml.xml").unwrap();
        let mut content = String::new();
        entry.read_to_string(&mut content).unwrap();
        content
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

        merge_archive_into(&existing_path, &updates_path, &output_path, "taxon_id=47790", &|_| {})
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
    fn test_merge_archive_into_preserves_multiple_multimedia_rows_per_observation() {
        // Regression: when an observation has multiple multimedia rows (one per
        // photo), the update map must not collapse them into a single row.
        let existing_tmp = tempfile::NamedTempFile::new().unwrap();
        let updates_tmp = tempfile::NamedTempFile::new().unwrap();
        let output_tmp = tempfile::NamedTempFile::new().unwrap();

        let existing_path = existing_tmp.path().to_str().unwrap().to_string();
        let updates_path = updates_tmp.path().to_str().unwrap().to_string();
        let output_path = output_tmp.path().to_str().unwrap().to_string();

        // Existing: obs/1 has photo_a and photo_b
        // Updates:  obs/1 has photo_c and photo_d (both changed)
        // Expected: output has exactly photo_c and photo_d — not photo_a/b,
        //           and not both rows collapsed to photo_d.
        let obs_id = "https://www.inaturalist.org/observations/1";

        build_test_zip_with_extra_csv(
            &existing_path,
            &format!("id,name\n{obs_id},original\n"),
            "taxon_id=1",
            "2026-01-01",
            Multimedia::FILENAME,
            &format!(
                "coreid,identifier\n\
                 {obs_id},http://photo_a\n\
                 {obs_id},http://photo_b\n"
            ),
        );
        build_test_zip_with_extra_csv(
            &updates_path,
            &format!("id,name\n{obs_id},updated\n"),
            "taxon_id=1",
            "2026-01-02",
            Multimedia::FILENAME,
            &format!(
                "coreid,identifier\n\
                 {obs_id},http://photo_c\n\
                 {obs_id},http://photo_d\n"
            ),
        );

        merge_archive_into(&existing_path, &updates_path, &output_path, "taxon_id=1", &|_| {})
            .unwrap();

        let rows = read_csv_rows_from_zip(&output_path, Multimedia::FILENAME);
        assert_eq!(
            rows.len(),
            2,
            "expected 2 multimedia rows, got {}: {rows:?}",
            rows.len()
        );
        assert!(
            rows.iter().any(|r| r.contains("photo_c")),
            "photo_c missing from output: {rows:?}"
        );
        assert!(
            rows.iter().any(|r| r.contains("photo_d")),
            "photo_d missing from output: {rows:?}"
        );
        assert!(
            !rows.iter().any(|r| r.contains("photo_a")),
            "photo_a should have been replaced: {rows:?}"
        );
    }

    #[test]
    fn test_merge_archive_into_streams_media_correctly() {
        let existing_tmp = tempfile::NamedTempFile::new().unwrap();
        let updates_tmp = tempfile::NamedTempFile::new().unwrap();
        let output_tmp = tempfile::NamedTempFile::new().unwrap();

        let existing_path = existing_tmp.path().to_str().unwrap().to_string();
        let updates_path = updates_tmp.path().to_str().unwrap().to_string();
        let output_path = output_tmp.path().to_str().unwrap().to_string();

        build_test_zip_with_media(
            &existing_path,
            "id,name\nhttps://www.inaturalist.org/observations/1,original\n",
            "taxon_id=1",
            "2026-01-01",
            &[
                ("media/photo_a.jpg", b"existing_photo_a"),
                ("media/photo_b.jpg", b"photo_b"),
            ],
        );
        build_test_zip_with_media(
            &updates_path,
            "id,name\nhttps://www.inaturalist.org/observations/1,updated\n",
            "taxon_id=1",
            "2026-01-02",
            &[
                ("media/photo_a.jpg", b"updated_photo_a"),
                ("media/photo_c.jpg", b"photo_c"),
            ],
        );

        merge_archive_into(&existing_path, &updates_path, &output_path, "taxon_id=1", &|_| {})
            .unwrap();

        let media = read_media_from_zip(&output_path);
        assert_eq!(media.len(), 3, "expected 3 media files");
        assert_eq!(
            media.get("media/photo_a.jpg").unwrap().as_slice(),
            b"updated_photo_a",
            "photo_a should be replaced by update"
        );
        assert_eq!(
            media.get("media/photo_b.jpg").unwrap().as_slice(),
            b"photo_b",
            "photo_b should be preserved from existing"
        );
        assert_eq!(
            media.get("media/photo_c.jpg").unwrap().as_slice(),
            b"photo_c",
            "photo_c should be added from updates"
        );
    }

    #[test]
    fn test_merge_archive_into_preserves_abstract_lines() {
        let existing_tmp = tempfile::NamedTempFile::new().unwrap();
        let updates_tmp = tempfile::NamedTempFile::new().unwrap();
        let output_tmp = tempfile::NamedTempFile::new().unwrap();

        let existing_path = existing_tmp.path().to_str().unwrap().to_string();
        let updates_path = updates_tmp.path().to_str().unwrap().to_string();
        let output_path = output_tmp.path().to_str().unwrap().to_string();

        build_test_zip_with_abstract(
            &existing_path,
            "id,name\nhttps://www.inaturalist.org/observations/1,original\n",
            "taxon_id=1",
            "2026-01-01",
            &["Observations of birds in California", "Filtered by user kueda"],
        );
        build_test_zip(
            &updates_path,
            "id,name\nhttps://www.inaturalist.org/observations/1,updated\n",
            "taxon_id=1",
            "2026-01-02",
        );

        merge_archive_into(&existing_path, &updates_path, &output_path, "taxon_id=1", &|_| {})
            .unwrap();

        let eml = read_eml_from_zip(&output_path);
        assert!(
            eml.contains("Observations of birds in California"),
            "first abstract line not preserved in eml.xml"
        );
        assert!(
            eml.contains("Filtered by user kueda"),
            "second abstract line not preserved in eml.xml"
        );
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
        merge_archive_into(&existing_path, &updates_path, &existing_path, "taxon_id=1", &|_| {})
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
    fn test_merge_archive_into_updates_pub_date_to_today() {
        let existing_tmp = tempfile::NamedTempFile::new().unwrap();
        let updates_tmp = tempfile::NamedTempFile::new().unwrap();
        let output_tmp = tempfile::NamedTempFile::new().unwrap();

        let existing_path = existing_tmp.path().to_str().unwrap().to_string();
        let updates_path = updates_tmp.path().to_str().unwrap().to_string();
        let output_path = output_tmp.path().to_str().unwrap().to_string();

        build_test_zip(
            &existing_path,
            "id,name\n",
            "taxon_id=1",
            "2020-01-01",
        );
        build_test_zip(
            &updates_path,
            "id,name\n",
            "taxon_id=1",
            "2020-01-02",
        );

        merge_archive_into(&existing_path, &updates_path, &output_path, "taxon_id=1", &|_| {})
            .unwrap();

        let eml = read_eml_from_zip(&output_path);
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        assert!(
            eml.contains(&format!("<pubDate>{today}</pubDate>")),
            "pubDate not updated to today ({today}) in eml.xml: {eml}"
        );
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

        let tmp = tempfile::NamedTempFile::new().unwrap();
        let metadata = Metadata::default();
        let builder = ArchiveBuilder::new(
            vec![DwcaExtension::SimpleMultimedia, DwcaExtension::Identifications],
            metadata,
            tmp.path(),
        ).unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        builder.build().await.unwrap();

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

        let tmp = tempfile::NamedTempFile::new().unwrap();
        let metadata = Metadata::default();
        let builder = ArchiveBuilder::new(vec![], metadata, tmp.path()).unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        builder.build().await.unwrap();

        assert!(!archive_has_media(&path).unwrap());
    }

    #[test]
    fn test_read_archive_preview_returns_all_fields() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap();
        build_test_zip_with_extra_csv(
            path,
            "id\n1\n",
            "taxon_id=47792",
            "2025-01-15",
            Multimedia::FILENAME,
            "coreid\n1\n",
        );
        let preview = read_archive_preview(path).unwrap();
        assert_eq!(preview.inat_query, Some("taxon_id=47792".to_string()));
        assert_eq!(preview.pub_date, Some("2025-01-15".to_string()));
        assert!(preview.extensions.contains(&DwcaExtension::SimpleMultimedia));
        assert!(!preview.has_media);
    }

    #[test]
    fn test_read_archive_preview_detects_media() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap();
        build_test_zip_with_media(
            path,
            "id\n1\n",
            "taxon_id=47792",
            "2025-01-15",
            &[("media/photo.jpg", b"fake-jpeg")],
        );
        let preview = read_archive_preview(path).unwrap();
        assert!(preview.has_media);
        assert!(preview.extensions.is_empty());
    }

    #[test]
    fn test_merge_emits_merging_progress_events() {
        use std::sync::{Arc, Mutex};

        let existing_tmp = tempfile::NamedTempFile::new().unwrap();
        let updates_tmp = tempfile::NamedTempFile::new().unwrap();
        let output_tmp = tempfile::NamedTempFile::new().unwrap();

        build_test_zip(existing_tmp.path().to_str().unwrap(), "id\n1\n2\n", "taxon_id=1", "2025-01-01");
        build_test_zip(updates_tmp.path().to_str().unwrap(), "id\n1\n", "taxon_id=1", "2025-02-01");

        let events: Arc<Mutex<Vec<DownloadProgress>>> = Arc::new(Mutex::new(Vec::new()));
        let events_clone = Arc::clone(&events);
        let callback = move |p: DownloadProgress| {
            events_clone.lock().unwrap().push(p);
        };

        merge_archive_into(
            existing_tmp.path().to_str().unwrap(),
            updates_tmp.path().to_str().unwrap(),
            output_tmp.path().to_str().unwrap(),
            "taxon_id=1",
            &callback,
        ).unwrap();

        let captured = events.lock().unwrap();
        assert!(!captured.is_empty(), "no progress events emitted");
        assert!(captured.iter().all(|p| matches!(
            p.stage, DownloadStage::Merging { .. }
        )));
        // First event must be current=0
        assert!(matches!(captured[0].stage, DownloadStage::Merging { current: 0, .. }));
        // Last event must have current == total
        let last = captured.last().unwrap();
        assert!(matches!(last.stage, DownloadStage::Merging { current, total } if current == total));
    }

    #[test]
    fn test_read_archive_preview_no_chuck_json() {
        use std::io::Write;
        use zip::CompressionMethod;
        use zip::write::FileOptions;

        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap();

        let file = std::fs::File::create(path).unwrap();
        let mut zip = zip::write::ZipWriter::new(file);
        let opts: FileOptions<()> =
            FileOptions::default().compression_method(CompressionMethod::Stored);
        let eml = "<eml><dataset><pubDate>2025-06-01</pubDate></dataset></eml>";
        zip.start_file("eml.xml", opts).unwrap();
        zip.write_all(eml.as_bytes()).unwrap();
        zip.finish().unwrap();

        let preview = read_archive_preview(path).unwrap();
        assert_eq!(preview.inat_query, None);
        assert_eq!(preview.pub_date, Some("2025-06-01".to_string()));
        assert!(preview.extensions.is_empty());
        assert!(!preview.has_media);
    }

    #[test]
    fn test_merge_append_updates_occurrence_csv() {
        let existing_tmp = tempfile::NamedTempFile::new().unwrap();
        let updates_tmp = tempfile::NamedTempFile::new().unwrap();
        let existing_path = existing_tmp.path().to_str().unwrap();
        let updates_path = updates_tmp.path().to_str().unwrap();

        build_test_zip(existing_path, "id,name\n1,Alice\n2,Bob\n", "taxon_id=1", "2025-01-01");
        build_test_zip(updates_path, "id,name\n2,Robert\n3,Carol\n", "taxon_id=1", "2025-02-01");

        merge_archive_append(existing_path, updates_path, "taxon_id=1", &|_| {}).unwrap();

        let rows = read_csv_rows_from_zip(existing_path, Occurrence::FILENAME);
        assert!(rows.contains(&"1,Alice".to_string()), "row 1 should be preserved");
        assert!(rows.contains(&"2,Robert".to_string()), "row 2 should be updated");
        assert!(rows.contains(&"3,Carol".to_string()), "row 3 should be appended");
        assert_eq!(rows.len(), 3);
    }

    #[test]
    fn test_merge_append_preserves_existing_media() {
        let existing_tmp = tempfile::NamedTempFile::new().unwrap();
        let updates_tmp = tempfile::NamedTempFile::new().unwrap();
        let existing_path = existing_tmp.path().to_str().unwrap();
        let updates_path = updates_tmp.path().to_str().unwrap();

        build_test_zip_with_media(
            existing_path,
            "id\n1\n",
            "taxon_id=1",
            "2025-01-01",
            &[("media/old.jpg", b"old-jpeg")],
        );
        build_test_zip_with_media(
            updates_path,
            "id\n2\n",
            "taxon_id=1",
            "2025-02-01",
            &[("media/new.jpg", b"new-jpeg")],
        );

        merge_archive_append(existing_path, updates_path, "taxon_id=1", &|_| {}).unwrap();

        let media = read_media_from_zip(existing_path);
        assert!(media.contains_key("media/old.jpg"), "existing media should be preserved");
        assert!(media.contains_key("media/new.jpg"), "new media should be appended");
        assert_eq!(media["media/old.jpg"], b"old-jpeg");
        assert_eq!(media["media/new.jpg"], b"new-jpeg");
    }

    #[test]
    fn test_merge_append_superseded_media_uses_new_version() {
        // When the same media filename exists in both archives, the updated version
        // should win and there must be exactly one CD entry for that filename
        // (no duplicate that would make the ZIP invalid).
        let existing_tmp = tempfile::NamedTempFile::new().unwrap();
        let updates_tmp = tempfile::NamedTempFile::new().unwrap();
        let existing_path = existing_tmp.path().to_str().unwrap();
        let updates_path = updates_tmp.path().to_str().unwrap();

        build_test_zip_with_media(
            existing_path,
            "id\n1\n",
            "taxon_id=1",
            "2025-01-01",
            &[("media/photo.jpg", b"old-bytes")],
        );
        build_test_zip_with_media(
            updates_path,
            "id\n1\n",
            "taxon_id=1",
            "2025-02-01",
            &[("media/photo.jpg", b"new-bytes")],
        );

        merge_archive_append(existing_path, updates_path, "taxon_id=1", &|_| {}).unwrap();

        let media = read_media_from_zip(existing_path);
        assert_eq!(
            media["media/photo.jpg"].as_slice(),
            b"new-bytes",
            "superseded media should use the updated version"
        );

        // Ensure no duplicate filenames in the central directory.
        let file = std::fs::File::open(existing_path).unwrap();
        let archive = zip::ZipArchive::new(file).unwrap();
        let names: Vec<&str> = archive.file_names().collect();
        let unique: std::collections::HashSet<&str> = names.iter().copied().collect();
        assert_eq!(
            names.len(),
            unique.len(),
            "duplicate filenames in central directory: {names:?}"
        );
    }
}

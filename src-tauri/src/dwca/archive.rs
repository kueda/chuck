use rayon::prelude::*;
use std::collections::HashSet;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::search_params::SearchParams;
use crate::db::Database;
use crate::error::{ChuckError, Result};

/// Information about an extension in a DarwinCore Archive
#[derive(Debug, Clone)]
pub struct ExtensionInfo {
    /// The rowType from meta.xml (e.g., "http://rs.gbif.org/terms/1.0/Multimedia")
    pub row_type: String,
    /// Path to the extension CSV file
    pub location: PathBuf,
    /// The DwcaExtension enum variant for this extension
    pub extension: chuck_core::DwcaExtension,
    /// The core ID column name that extensions reference (e.g., "gbifID" or "occurrenceID")
    pub core_id_column: String,
}


#[derive(Debug)]
struct ZipFileInfo {
    index: usize,
    path: PathBuf,
    is_dir: bool,
    #[cfg(unix)]
    unix_mode: Option<u32>,
}

/// Represents a Darwin Core Archive
pub struct Archive {
    /// Directory where archive contents are stored
    pub storage_dir: PathBuf,

    pub name: String,

    pub core_id_column: String,

    /// Internal database for querying archive data
    db: Database,
}

impl Archive {
    /// Opens and extracts a Darwin Core Archive
    pub fn open(archive_path: &Path, base_dir: &Path) -> Result<Self> {
        Self::open_with_progress(archive_path, base_dir, |_| {})
    }

    /// Opens and extracts a Darwin Core Archive with progress callback
    pub fn open_with_progress<F>(
        archive_path: &Path,
        base_dir: &Path,
        mut progress_callback: F,
    ) -> Result<Self>
    where
        F: FnMut(&str),
    {
        // Create storage directory based on archive hash
        progress_callback("importing");
        let storage_dir = create_storage_dir(archive_path, base_dir)?;

        // Remove all other archive directories in the base directory
        remove_other_archives(base_dir, &storage_dir)?;

        // Create a hard link to the original archive for lazy photo extraction
        // This is instant and doesn't copy data, but keeps the file accessible
        // even if the user deletes the original
        let archive_copy_path = storage_dir.join("archive.zip");
        std::fs::hard_link(archive_path, &archive_copy_path).map_err(|e| ChuckError::FileOpen {
            path: archive_copy_path.clone(),
            source: e,
        })?;

        progress_callback("extracting");
        extract_archive(archive_path, &storage_dir)?;

        let (core_files, core_id_column, extensions) = parse_meta_xml(&storage_dir)?;

        // Create database from core files and extensions
        progress_callback("creating_database");
        let db_name = archive_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("archive");
        let db_path = storage_dir.join(format!("{}.db", db_name));
        let db = Database::create_from_core_files(&core_files, &extensions, &db_path, &core_id_column)?;

        // Initialize photo cache table
        let photo_cache = crate::photo_cache::PhotoCache::new(db.connection());
        photo_cache.create_table()?;

        Ok(Self {
            // archive_path: archive_path.to_path_buf(),
            storage_dir,
            name: archive_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string(),
            core_id_column,
            db,
        })
    }

    /// Returns an Archive representing the currently-open archive
    /// (i.e. the archive that is already unzipped and has a DuckDB database)
    pub fn current(base_dir: &Path) -> Result<Self> {
        if !base_dir.exists() {
            return Err(ChuckError::NoArchiveFound(base_dir.to_path_buf()));
        }

        // Find the first directory in base_dir
        let storage_dir = std::fs::read_dir(base_dir)
            .map_err(|e| ChuckError::FileRead {
                path: base_dir.to_path_buf(),
                source: e,
            })?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .find(|p| p.is_dir())
            .ok_or_else(|| ChuckError::NoArchiveFound(base_dir.to_path_buf()))?;

        // Extract name from the directory name (format: "filename-hash")
        let dir_name = storage_dir
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| ChuckError::InvalidFileName(storage_dir.clone()))?;

        let name = dir_name
            .rsplit_once('-')
            .map(|(name, _everything_else)| name)
            .unwrap_or("unknown")
            .to_string();

        // Find the first .db file in the storage directory
        let db_path = std::fs::read_dir(&storage_dir)
            .map_err(|e| ChuckError::FileRead {
                path: storage_dir.clone(),
                source: e,
            })?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .find(|p| p.extension().and_then(|s| s.to_str()) == Some("db"))
            .ok_or_else(|| ChuckError::NoArchiveFound(storage_dir.clone()))?;

        // Parse meta.xml to get extension information
        let (_core_files, core_id_column, extensions) = parse_meta_xml(&storage_dir)?;

        let db = Database::open(&db_path, &extensions)?;

        Ok(Self {
            storage_dir,
            name,
            core_id_column,
            db,
        })
    }

    /// Returns the number of core records in the archive
    pub fn core_count(&self) -> Result<usize> {
        self.db.count_records()
    }

    /// Returns archive information
    pub fn info(&self) -> Result<crate::commands::archive::ArchiveInfo> {
        let available_columns = self.db.get_available_columns()?;

        Ok(crate::commands::archive::ArchiveInfo {
            name: self.name.clone(),
            core_count: self.core_count()?,
            core_id_column: self.core_id_column.clone(),
            available_columns,
        })
    }

    /// Searches for occurrences in the archive
    pub fn search(
        &self,
        limit: usize,
        offset: usize,
        search_params: SearchParams,
        fields: Option<Vec<String>>,
    ) -> Result<crate::commands::archive::SearchResult> {
        let params = SearchParams {
            order_by: search_params.order_by.clone().or(Some(self.core_id_column.clone())),
            ..search_params
        };
        self.db.search(
            limit,
            offset,
            params,
            fields
        )
    }

    /// Get autocomplete suggestions for a given column
    pub fn get_autocomplete_suggestions(
        &self,
        column_name: &str,
        search_term: &str,
        limit: usize,
    ) -> Result<Vec<String>> {
        self.db.get_autocomplete_suggestions(column_name, search_term, limit)
    }

    /// Aggregates occurrences by a field (GROUP BY)
    pub fn aggregate_by_field(
        &self,
        field_name: &str,
        search_params: &SearchParams,
        limit: usize,
    ) -> Result<Vec<crate::db::AggregationResult>> {
        self.db.aggregate_by_field(field_name, search_params, limit, &self.core_id_column)
    }

    /// Retrieves a single occurrence by its core ID with all fields and extensions
    pub fn get_occurrence(
        &self,
        occurrence_id: &str,
    ) -> Result<serde_json::Map<String, serde_json::Value>> {
        self.db.get_occurrence(&self.core_id_column, occurrence_id)
    }

    /// Query occurrences within a bounding box for tile generation
    /// Returns (core_id, latitude, longitude, scientificName) tuples
    ///
    /// Uses grid-based sampling at low zoom levels to reduce data volume while
    /// preserving spatial extent (showing where observations exist across the tile)
    pub fn query_tile(
        &self,
        west: f64,
        south: f64,
        east: f64,
        north: f64,
        zoom: u8,
        search_params: SearchParams,
    ) -> Result<Vec<(i64, f64, f64, Option<String>)>> {
        let conn = self.db.connection();

        let (
            _,
            where_clause,
            mut where_interpolations,
            _
        ) = Database::sql_parts(search_params, None, &vec![]);

        // Determine grid cell size based on zoom level
        // At low zoom, use coarse grid to reduce points while preserving spatial extent
        // At high zoom, return all points (no sampling)
        let grid_size = match zoom {
            0..=2 => Some(1.0),    // ~111km cells - very coarse sampling
            3..=5 => Some(0.1),    // ~11km cells - moderate sampling
            6..=8 => Some(0.01),   // ~1km cells - fine sampling
            _ => None              // No sampling at zoom 9+
        };

        let query = if let Some(grid) = grid_size {
            // Grid-based sampling: pick one point per grid cell
            format!(
                "SELECT
                    ANY_VALUE({}) as core_id,
                    ANY_VALUE(decimalLatitude) as decimalLatitude,
                    ANY_VALUE(decimalLongitude) as decimalLongitude,
                    ANY_VALUE(scientificName) as scientificName
                 FROM occurrences
                 {}
                     decimalLatitude BETWEEN ? AND ?
                     AND decimalLongitude BETWEEN ? AND ?
                     AND decimalLatitude IS NOT NULL
                     AND decimalLongitude IS NOT NULL
                 GROUP BY
                     FLOOR(decimalLatitude / {}),
                     FLOOR(decimalLongitude / {})",
                self.core_id_column,
                if where_clause.len() == 0 {
                    String::from("WHERE")
                } else {
                    format!("{where_clause} AND")
                },
                grid,
                grid
            )
        } else {
            // No sampling at high zoom - return all points
            format!(
                "SELECT {}, decimalLatitude, decimalLongitude, scientificName
                 FROM occurrences
                 {}
                     decimalLatitude BETWEEN ? AND ?
                     AND decimalLongitude BETWEEN ? AND ?
                     AND decimalLatitude IS NOT NULL
                     AND decimalLongitude IS NOT NULL",
                self.core_id_column,
                if where_clause.len() == 0 {
                    String::from("WHERE")
                } else {
                    format!("{where_clause} AND")
                }
            )
        };
        // log::debug!("query_tile, query: {}", query);

        let mut stmt = conn.prepare(&query).map_err(ChuckError::Database)?;
        where_interpolations.push(Box::new(south));
        where_interpolations.push(Box::new(north));
        where_interpolations.push(Box::new(west));
        where_interpolations.push(Box::new(east));
        let select_param_refs: Vec<&dyn duckdb::ToSql> = where_interpolations
            .iter()
            .map(|p| p.as_ref())
            .collect();
        let rows = stmt
            // .query_map([south, north, west, east], |row| {
            .query_map(select_param_refs.as_slice(), |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, f64>(1)?,
                    row.get::<_, f64>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })
            .map_err(ChuckError::Database)?;

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(ChuckError::Database)
    }

    /// Gets a photo from the cache or extracts it from the archive
    /// Returns the absolute path to the cached photo file
    pub fn get_photo(&self, photo_path: &str) -> Result<String> {
        // Create photo cache
        let photo_cache = crate::photo_cache::PhotoCache::new(self.db.connection());

        // Check if photo is already cached
        if let Some(cached_path) = photo_cache.get_cached_photo(photo_path)? {
            // Update access time and return cached path
            photo_cache.update_access_time(photo_path)?;
            return Ok(cached_path);
        }

        // Photo not cached - extract it from the archive
        let archive_zip_path = self.storage_dir.join("archive.zip");
        let cache_dir = self.storage_dir.join("photo_cache");
        std::fs::create_dir_all(&cache_dir).map_err(|e| ChuckError::DirectoryCreate {
            path: cache_dir.clone(),
            source: e,
        })?;

        // Create a unique filename based on the photo path to avoid conflicts
        let safe_filename = photo_path.replace(['/', '\\'], "_");
        let cached_file_path = cache_dir.join(&safe_filename);

        // Extract the photo from the ZIP using the path from the multimedia table
        let bytes_written = extract_single_file(
            &archive_zip_path,
            photo_path,
            &cached_file_path,
        )?;

        // Add to cache
        photo_cache.add_photo(
            photo_path,
            cached_file_path.to_str().ok_or_else(|| ChuckError::InvalidFileName(cached_file_path.clone()))?,
            bytes_written as i64,
        )?;

        // Evict LRU photos if cache is too large (2GB default)
        const MAX_CACHE_SIZE: i64 = 2 * 1024 * 1024 * 1024; // 2GB
        photo_cache.evict_lru(MAX_CACHE_SIZE)?;

        Ok(cached_file_path.to_string_lossy().to_string())
    }
}

fn create_storage_dir(archive_path: &Path, base_dir: &Path) -> Result<PathBuf> {
    let fname = archive_path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| ChuckError::InvalidFileName(archive_path.to_path_buf()))?;

    // Generate unique ID using timestamp (microseconds since epoch)
    // SystemTimeError is extremely unlikely (only if clock is before Unix epoch)
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System clock should be after Unix epoch")
        .as_micros();

    let unique_dir_name = format!("{}-{:x}", fname, timestamp);
    let target_dir = base_dir.join(unique_dir_name);

    std::fs::create_dir_all(&target_dir).map_err(|e| ChuckError::DirectoryCreate {
        path: target_dir.clone(),
        source: e,
    })?;

    Ok(target_dir)
}

fn remove_other_archives(base_dir: &Path, current_storage_dir: &Path) -> Result<()> {
    if !base_dir.exists() {
        return Ok(());
    }

    let entries = std::fs::read_dir(base_dir).map_err(|e| ChuckError::FileRead {
        path: base_dir.to_path_buf(),
        source: e,
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| ChuckError::FileRead {
            path: base_dir.to_path_buf(),
            source: e,
        })?;

        let path = entry.path();
        if path.is_dir() && path != current_storage_dir {
            std::fs::remove_dir_all(&path).map_err(|e| ChuckError::DirectoryCreate {
                path: path.clone(),
                source: e,
            })?;
        }
    }

    Ok(())
}

fn extract_archive(archive_path: &Path, target_dir: &Path) -> Result<()> {
    let files_to_extract = get_files_to_extract(archive_path, target_dir)?;
    let archive_path = Arc::new(archive_path.to_path_buf());
    let errors: Arc<Mutex<Vec<ChuckError>>> = Arc::new(Mutex::new(Vec::new()));

    // Extract files in parallel
    files_to_extract.par_iter().for_each(|file_info| {
        let result = (|| -> Result<()> {
            if file_info.is_dir {
                std::fs::create_dir_all(&file_info.path).map_err(|e| ChuckError::DirectoryCreate {
                    path: file_info.path.clone(),
                    source: e,
                })?;
            } else {
                // Create parent directories if needed
                if let Some(p) = file_info.path.parent() {
                    std::fs::create_dir_all(p).map_err(|e| ChuckError::DirectoryCreate {
                        path: p.to_path_buf(),
                        source: e,
                    })?;
                }

                // Open a new archive instance for this thread
                let file = std::fs::File::open(&*archive_path).map_err(|e| ChuckError::FileOpen {
                    path: archive_path.as_ref().clone(),
                    source: e,
                })?;

                let mut archive = zip::ZipArchive::new(file).map_err(ChuckError::ArchiveExtraction)?;
                let zip_file = archive.by_index(file_info.index).map_err(ChuckError::ArchiveExtraction)?;

                let outfile = std::fs::File::create(&file_info.path).map_err(|e| ChuckError::FileOpen {
                    path: file_info.path.clone(),
                    source: e,
                })?;

                // Use buffered I/O with 64KB buffers for better performance
                let mut reader = BufReader::with_capacity(64 * 1024, zip_file);
                let mut writer = BufWriter::with_capacity(64 * 1024, outfile);
                std::io::copy(&mut reader, &mut writer).map_err(|e| ChuckError::FileRead {
                    path: file_info.path.clone(),
                    source: e,
                })?;

                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Some(mode) = file_info.unix_mode {
                        std::fs::set_permissions(&file_info.path, std::fs::Permissions::from_mode(mode))
                            .ok();
                    }
                }
            }
            Ok(())
        })();

        if let Err(e) = result {
            errors.lock().unwrap().push(e);
        }
    });

    // Check for errors
    let errors = Arc::try_unwrap(errors).unwrap().into_inner().unwrap();
    if let Some(err) = errors.into_iter().next() {
        return Err(err);
    }

    Ok(())
}

/// Extracts a single file from the archive to a target path
/// Returns the size of the extracted file in bytes
fn extract_single_file(
    archive_path: &Path,
    file_path_in_zip: &str,
    target_path: &Path,
) -> Result<u64> {
    let file = std::fs::File::open(archive_path).map_err(|e| ChuckError::FileOpen {
        path: archive_path.to_path_buf(),
        source: e,
    })?;

    let mut archive = zip::ZipArchive::new(file).map_err(ChuckError::ArchiveExtraction)?;

    // Find the file in the archive by name
    let zip_file = archive
        .by_name(file_path_in_zip)
        .map_err(ChuckError::ArchiveExtraction)?;

    // Create parent directories if needed
    if let Some(p) = target_path.parent() {
        if !p.exists() {
            std::fs::create_dir_all(p).map_err(|e| ChuckError::DirectoryCreate {
                path: p.to_path_buf(),
                source: e,
            })?;
        }
    }

    // Extract the file with buffered I/O
    let outfile = std::fs::File::create(target_path).map_err(|e| ChuckError::FileOpen {
        path: target_path.to_path_buf(),
        source: e,
    })?;

    // Use buffered I/O with 64KB buffers for better performance
    let mut reader = BufReader::with_capacity(64 * 1024, zip_file);
    let mut writer = BufWriter::with_capacity(64 * 1024, outfile);
    let bytes_written = std::io::copy(&mut reader, &mut writer).map_err(|e| {
        ChuckError::FileRead {
            path: target_path.to_path_buf(),
            source: e,
        }
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Some(mode) = reader.get_ref().unix_mode() {
            std::fs::set_permissions(target_path, std::fs::Permissions::from_mode(mode)).ok();
        }
    }

    Ok(bytes_written)
}

/// Metadata of files in the archive that we need to extract
fn get_files_to_extract(archive_path: &Path, target_dir: &Path) -> Result<Vec<ZipFileInfo>> {
    // Phase 1: Extract meta.xml first to determine which files we need
    let meta_path = target_dir.join("meta.xml");
    extract_single_file(archive_path, "meta.xml", meta_path.as_path())?;

    // Also extract all other .xml files in the archive root (potential metadata files)
    let file = std::fs::File::open(archive_path).map_err(|e| ChuckError::FileOpen {
        path: archive_path.to_path_buf(),
        source: e,
    })?;
    let mut archive = zip::ZipArchive::new(file).map_err(ChuckError::ArchiveExtraction)?;

    for i in 0..archive.len() {
        if let Ok(file) = archive.by_index(i) {
            if let Some(enclosed_path) = file.enclosed_name() {
                let path_str = enclosed_path.to_string_lossy().to_string();
                // Extract .xml files that are in the root directory (not meta.xml, already extracted)
                if path_str.ends_with(".xml")
                    && path_str != "meta.xml"
                    && !path_str.contains('/')
                    && !file.is_dir()
                {
                    let outpath = target_dir.join(&path_str);
                    let _ = extract_single_file(archive_path, &path_str, outpath.as_path());
                }
            }
        }
    }

    // Parse meta.xml to determine needed files
    let needed_files = get_needed_files_from_meta(target_dir)?;

    // Phase 2: Collect information about files to extract
    let file = std::fs::File::open(archive_path).map_err(|e| ChuckError::FileOpen {
        path: archive_path.to_path_buf(),
        source: e,
    })?;

    let mut archive = zip::ZipArchive::new(file).map_err(ChuckError::ArchiveExtraction)?;

    let files_to_extract: Vec<ZipFileInfo> = (0..archive.len())
        .filter_map(|i| {
            let file = archive.by_index(i).ok()?;
            let path = file.enclosed_name()?.to_path_buf();
            let outpath = target_dir.join(&path);

            // Only extract files we actually need
            if !file.is_dir() && !needed_files.contains(path.to_str()?) {
                return None;
            }

            Some(ZipFileInfo {
                index: i,
                path: outpath,
                is_dir: file.is_dir(),
                #[cfg(unix)]
                unix_mode: file.unix_mode(),
            })
        })
        .collect();
    Ok(files_to_extract)
}

/// Parses meta.xml to determine which files we need to extract
/// Returns a HashSet of file paths (relative to archive root)
fn get_needed_files_from_meta(storage_dir: &Path) -> Result<std::collections::HashSet<String>> {
    let meta_path = storage_dir.join("meta.xml");
    let contents = std::fs::read_to_string(&meta_path).map_err(|e| ChuckError::FileRead {
        path: meta_path.clone(),
        source: e,
    })?;

    let doc = roxmltree::Document::parse(&contents).map_err(|e| ChuckError::XmlParse {
        path: meta_path,
        source: e,
    })?;

    let mut needed_files = HashSet::new();

    // Get core files
    if let Some(core_node) = doc.descendants().find(|n| n.has_tag_name("core")) {
        for location_node in core_node.descendants().filter(|n| n.has_tag_name("location")) {
            if let Some(text) = location_node.text() {
                needed_files.insert(text.to_string());
            }
        }
    }

    // Get extension files (only for supported types)
    for ext_node in doc.descendants().filter(|n| n.has_tag_name("extension")) {
        if let Some(row_type) = ext_node.attribute("rowType") {
            // Only include supported extension types
            if chuck_core::DwcaExtension::from_row_type(row_type).is_some() {
                for location_node in ext_node.descendants().filter(|n| n.has_tag_name("location")) {
                    if let Some(text) = location_node.text() {
                        needed_files.insert(text.to_string());
                    }
                }
            }
        }
    }

    Ok(needed_files)
}

fn parse_meta_xml(storage_dir: &Path) -> Result<(Vec<PathBuf>, String, Vec<ExtensionInfo>)> {
    let meta_path = storage_dir.join("meta.xml");
    let contents = std::fs::read_to_string(&meta_path).map_err(|e| ChuckError::FileRead {
        path: meta_path.clone(),
        source: e,
    })?;

    let doc = roxmltree::Document::parse(&contents).map_err(|e| ChuckError::XmlParse {
        path: meta_path,
        source: e,
    })?;

    // Find the core element
    let core_node = doc
        .descendants()
        .find(|n| n.has_tag_name("core"))
        .ok_or(ChuckError::NoCoreFiles)?;

    // Parse core files
    let core_files: Vec<PathBuf> = core_node
        .descendants()
        .filter(|n| n.has_tag_name("location"))
        .filter_map(|n| n.text())
        .map(|text| storage_dir.join(text))
        .collect();

    if core_files.is_empty() {
        return Err(ChuckError::NoCoreFiles);
    }

    // Parse core ID field - this is the column that extensions will reference
    let core_id_index = core_node
        .descendants()
        .find(|n| n.has_tag_name("id"))
        .and_then(|id_node| id_node.attribute("index"))
        .and_then(|idx| idx.parse::<usize>().ok());

    // Find the core ID column name by looking up the field at that index
    let core_id_column = if let Some(id_index) = core_id_index {
        core_node
            .descendants()
            .filter(|n| n.has_tag_name("field"))
            .find(|field_node| {
                field_node
                    .attribute("index")
                    .and_then(|idx| idx.parse::<usize>().ok())
                    == Some(id_index)
            })
            .and_then(|field_node| field_node.attribute("term"))
            .and_then(|term| {
                // Extract the column name from the term URL
                // e.g., "http://rs.gbif.org/terms/1.0/gbifID" -> "gbifID"
                term.rsplit('/')
                    .next()
                    .or_else(|| term.rsplit('#').next())
            })
            .map(|s| s.to_string())
    } else {
        None
    };

    let core_id_column = core_id_column.unwrap_or_else(|| {
        log::warn!("Could not determine core ID column from meta.xml, defaulting to 'occurrenceID'");
        "occurrenceID".to_string()
    });

    // Parse extensions (only supported types)
    let extensions: Vec<ExtensionInfo> = doc
        .descendants()
        .filter(|n| n.has_tag_name("extension"))
        .filter_map(|ext_node| {
            let row_type = ext_node.attribute("rowType")?;

            // Try to map rowType to DwcaExtension
            let extension = chuck_core::DwcaExtension::from_row_type(row_type)?;

            // Extract location from <files><location>...</location></files>
            let location_text = ext_node
                .descendants()
                .filter(|n| n.has_tag_name("location"))
                .filter_map(|n| n.text())
                .next()?;

            let location = storage_dir.join(location_text);

            Some(ExtensionInfo {
                row_type: row_type.to_string(),
                location,
                extension,
                core_id_column: core_id_column.clone(),
            })
        })
        .collect();

    Ok((core_files, core_id_column, extensions))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    struct UnzippedArchiveFixture {
        base_dir: PathBuf,
        storage_dir: PathBuf,
    }

    impl UnzippedArchiveFixture {
        fn new(test_name: &str, meta_xml: &str) -> Self {
            let temp_dir = std::env::temp_dir()
                .join(format!("chuck_test_dwca_archive_{}", test_name));
            std::fs::create_dir_all(&temp_dir).unwrap();
            let meta_path = temp_dir.join("meta.xml");
            let mut file = std::fs::File::create(&meta_path).unwrap();
            file.write_all(meta_xml.as_bytes()).unwrap();
            Self {
                base_dir: temp_dir.clone(),
                storage_dir: temp_dir,
            }
        }

        fn with_files(test_name: &str, files: &[(&str, &[u8])]) -> Self {
            let temp_dir = std::env::temp_dir()
                .join(format!("chuck_test_dwca_archive_{}", test_name));
            std::fs::create_dir_all(&temp_dir).unwrap();

            for (filename, content) in files {
                let file_path = temp_dir.join(filename);
                let mut file = std::fs::File::create(&file_path).unwrap();
                file.write_all(content).unwrap();
            }

            Self {
                base_dir: temp_dir.clone(),
                storage_dir: temp_dir,
            }
        }

        fn with_structure(
            test_name: &str,
            archive_name: &str,
            files: &[(&str, &[u8])],
            create_db: bool,
        ) -> Self {
            let base_dir = std::env::temp_dir()
                .join(format!("chuck_test_dwca_archive_{}", test_name));
            std::fs::create_dir_all(&base_dir).unwrap();

            let storage_dir = base_dir.join(format!("{}-abc123", archive_name));
            std::fs::create_dir_all(&storage_dir).unwrap();

            for (filename, content) in files {
                let file_path = storage_dir.join(filename);
                let mut file = std::fs::File::create(&file_path).unwrap();
                file.write_all(content).unwrap();
            }

            if create_db {
                let csv_paths: Vec<PathBuf> = files
                    .iter()
                    .filter(|(name, _)| name.ends_with(".csv"))
                    .map(|(name, _)| storage_dir.join(name))
                    .collect();

                if !csv_paths.is_empty() {
                    let db_name = archive_name
                        .strip_suffix(".zip")
                        .unwrap_or(archive_name);
                    let db_path = storage_dir.join(format!("{}.db", db_name));
                    let db = Database::create_from_core_files(&csv_paths, &vec![], &db_path, "occurrenceID").unwrap();
                    drop(db);
                }
            }

            Self {
                base_dir,
                storage_dir,
            }
        }

        fn dir(&self) -> &Path {
            &self.storage_dir
        }

        fn base_dir(&self) -> &Path {
            &self.base_dir
        }
    }

    impl Drop for UnzippedArchiveFixture {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.base_dir).ok();
        }
    }

    struct ZippedArchiveFixture {
        _unzipped_fixture: UnzippedArchiveFixture,
        archive_path: PathBuf,
        base_dir: PathBuf,
    }

    impl ZippedArchiveFixture {
        fn new(test_name: &str, files: Option<&[(&str, &[u8])]>) -> Self {
            let files = files.unwrap_or(&[
                ("meta.xml", br#"<?xml version="1.0" encoding="UTF-8"?>
<archive>
  <core>
    <files>
      <location>occurrence.csv</location>
    </files>
  </core>
</archive>"#),
                ("occurrence.csv", b"id,name\n1,test\n"),
            ]);

            let unzipped_fixture = UnzippedArchiveFixture::with_files(test_name, files);

            let temp_dir = std::env::temp_dir();
            let archive_path = temp_dir.join(format!("{}.zip", test_name));
            let base_dir = temp_dir.join(format!("{}_base", test_name));

            // Zip up the unzipped fixture's directory
            let archive_file = std::fs::File::create(&archive_path).unwrap();
            let mut zip = zip::ZipWriter::new(archive_file);
            let options = zip::write::FileOptions::<()>::default();

            for (filename, content) in files {
                zip.start_file(*filename, options).unwrap();
                zip.write_all(content).unwrap();
            }
            zip.finish().unwrap();

            Self {
                _unzipped_fixture: unzipped_fixture,
                archive_path,
                base_dir,
            }
        }

        fn archive_path(&self) -> &Path {
            &self.archive_path
        }

        fn base_dir(&self) -> &Path {
            &self.base_dir
        }
    }

    impl Drop for ZippedArchiveFixture {
        fn drop(&mut self) {
            std::fs::remove_file(&self.archive_path).ok();
            std::fs::remove_dir_all(&self.base_dir).ok();
        }
    }

    #[test]
    fn test_parse_meta_xml_recognizes_single_core() {
        let meta_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<archive>
  <core>
    <files>
      <location>occurrence.csv</location>
    </files>
  </core>
</archive>"#;
        let fixture = UnzippedArchiveFixture::new(
            "parse_meta_xml_recognizes_single_core",
            meta_xml
        );

        let result = parse_meta_xml(fixture.dir());
        assert!(result.is_ok());

        let (core_files, core_id_column, extensions) = result.unwrap();
        assert_eq!(core_files.len(), 1);
        assert_eq!(
            core_files[0].file_name().unwrap(),
            "occurrence.csv"
        );
        assert_eq!(core_id_column, "occurrenceID");
        assert_eq!(extensions.len(), 0);
    }

    #[test]
    fn test_parse_meta_xml_multiple_cores() {
        let meta_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<archive>
  <core>
    <files>
      <location>occurrence1.csv</location>
      <location>occurrence2.csv</location>
    </files>
  </core>
</archive>"#;
        let fixture = UnzippedArchiveFixture::new(
            "parse_meta_xml_multiple_cores",
            meta_xml
        );

        let result = parse_meta_xml(fixture.dir());
        assert!(result.is_ok());

        let (core_files, _core_id_column, extensions) = result.unwrap();
        assert_eq!(core_files.len(), 2);
        assert_eq!(
            core_files[0].file_name().unwrap(),
            "occurrence1.csv"
        );
        assert_eq!(
            core_files[1].file_name().unwrap(),
            "occurrence2.csv"
        );
        assert_eq!(extensions.len(), 0);
    }

    #[test]
    fn test_parse_meta_xml_no_core_files() {
        let meta_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<archive>
</archive>"#;
        let fixture = UnzippedArchiveFixture::new(
            "parse_meta_xml_no_core_files",
            meta_xml
        );

        let result = parse_meta_xml(fixture.dir());
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_meta_xml_with_extensions() {
        let meta_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<archive>
  <core>
    <files>
      <location>occurrence.csv</location>
    </files>
  </core>
  <extension encoding="UTF-8" rowType="http://rs.gbif.org/terms/1.0/Multimedia">
    <files>
      <location>multimedia.csv</location>
    </files>
  </extension>
  <extension encoding="UTF-8" rowType="http://rs.tdwg.org/ac/terms/Multimedia">
    <files>
      <location>audiovisual.csv</location>
    </files>
  </extension>
  <extension encoding="UTF-8" rowType="http://rs.tdwg.org/dwc/terms/Identification">
    <files>
      <location>identification.csv</location>
    </files>
  </extension>
</archive>"#;
        let fixture = UnzippedArchiveFixture::new(
            "parse_meta_xml_with_extensions",
            meta_xml
        );

        let result = parse_meta_xml(fixture.dir());
        assert!(result.is_ok());

        let (core_files, _core_id_column, extensions) = result.unwrap();
        assert_eq!(core_files.len(), 1);
        assert_eq!(extensions.len(), 3);

        // Check that extensions are mapped correctly
        assert_eq!(extensions[0].extension, chuck_core::DwcaExtension::SimpleMultimedia);
        assert_eq!(extensions[0].extension.table_name(), "multimedia");
        assert_eq!(extensions[0].row_type, "http://rs.gbif.org/terms/1.0/Multimedia");
        assert_eq!(extensions[0].core_id_column, "occurrenceID");
        assert_eq!(
            extensions[0].location.file_name().unwrap(),
            "multimedia.csv"
        );

        assert_eq!(extensions[1].extension, chuck_core::DwcaExtension::Audiovisual);
        assert_eq!(extensions[1].extension.table_name(), "audiovisual");
        assert_eq!(extensions[1].row_type, "http://rs.tdwg.org/ac/terms/Multimedia");
        assert_eq!(extensions[1].core_id_column, "occurrenceID");
        assert_eq!(
            extensions[1].location.file_name().unwrap(),
            "audiovisual.csv"
        );

        assert_eq!(extensions[2].extension, chuck_core::DwcaExtension::Identifications);
        assert_eq!(extensions[2].extension.table_name(), "identifications");
        assert_eq!(extensions[2].row_type, "http://rs.tdwg.org/dwc/terms/Identification");
        assert_eq!(extensions[2].core_id_column, "occurrenceID");
        assert_eq!(
            extensions[2].location.file_name().unwrap(),
            "identification.csv"
        );
    }

    #[test]
    fn test_parse_meta_xml_filters_unsupported_extensions() {
        let meta_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<archive>
  <core>
    <files>
      <location>occurrence.csv</location>
    </files>
  </core>
  <extension encoding="UTF-8" rowType="http://rs.gbif.org/terms/1.0/Multimedia">
    <files>
      <location>multimedia.csv</location>
    </files>
  </extension>
  <extension encoding="UTF-8" rowType="http://rs.tdwg.org/dwc/terms/Occurrence">
    <files>
      <location>verbatim.csv</location>
    </files>
  </extension>
  <extension encoding="UTF-8" rowType="http://rs.tdwg.org/dwc/terms/Identification">
    <files>
      <location>identification.csv</location>
    </files>
  </extension>
</archive>"#;
        let fixture = UnzippedArchiveFixture::new(
            "parse_meta_xml_filters_unsupported",
            meta_xml
        );

        let result = parse_meta_xml(fixture.dir());
        assert!(result.is_ok());

        let (core_files, _core_id_column, extensions) = result.unwrap();
        assert_eq!(core_files.len(), 1);

        // Should only have 2 extensions (Multimedia and Identification)
        // Occurrence extension should be filtered out as unsupported
        assert_eq!(extensions.len(), 2);

        assert_eq!(extensions[0].extension, chuck_core::DwcaExtension::SimpleMultimedia);
        assert_eq!(extensions[0].extension.table_name(), "multimedia");
        assert_eq!(extensions[0].core_id_column, "occurrenceID");
        assert_eq!(extensions[1].extension, chuck_core::DwcaExtension::Identifications);
        assert_eq!(extensions[1].extension.table_name(), "identifications");
        assert_eq!(extensions[1].core_id_column, "occurrenceID");

        // Verify Occurrence extension was not included
        assert!(!extensions.iter().any(|ext| ext.row_type == "http://rs.tdwg.org/dwc/terms/Occurrence"));
    }

    #[test]
    fn test_parse_meta_xml_detects_gbif_id_column() {
        let meta_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<archive>
  <core encoding="UTF-8" rowType="http://rs.tdwg.org/dwc/terms/Occurrence">
    <files>
      <location>occurrence.csv</location>
    </files>
    <id index="0" />
    <field index="0" term="http://rs.gbif.org/terms/1.0/gbifID"/>
    <field index="1" term="http://rs.tdwg.org/dwc/terms/occurrenceID"/>
  </core>
  <extension encoding="UTF-8" rowType="http://rs.gbif.org/terms/1.0/Multimedia">
    <files>
      <location>multimedia.csv</location>
    </files>
  </extension>
</archive>"#;
        let fixture = UnzippedArchiveFixture::new(
            "parse_meta_xml_detects_gbif_id",
            meta_xml
        );

        let result = parse_meta_xml(fixture.dir());
        assert!(result.is_ok());

        let (core_files, core_id_column, extensions) = result.unwrap();
        assert_eq!(core_files.len(), 1);
        assert_eq!(core_id_column, "gbifID");
        assert_eq!(extensions.len(), 1);

        // Should detect gbifID as the core ID column
        assert_eq!(extensions[0].core_id_column, "gbifID");
    }

    #[test]
    fn test_opening_new_archive_removes_other_archive_directories() {
        let fixture1 = ZippedArchiveFixture::new("test_archive1", None);
        let fixture2 = ZippedArchiveFixture::new("test_archive2", None);

        // Open first archive
        let archive1 = Archive::open(fixture1.archive_path(), fixture1.base_dir()).unwrap();
        let storage_dir1 = archive1.storage_dir.clone();
        assert!(storage_dir1.exists());

        // Open second archive using the same base directory
        let archive2 = Archive::open(fixture2.archive_path(), fixture1.base_dir()).unwrap();
        let storage_dir2 = archive2.storage_dir.clone();

        // After opening the second archive, the first archive's directory should be removed
        assert!(
            !storage_dir1.exists(),
            "First archive directory should be removed after opening second archive"
        );
        assert!(
            storage_dir2.exists(),
            "Second archive directory should exist"
        );
    }

    #[test]
    fn test_create_storage_dir() {
        let temp_dir = std::env::temp_dir();
        let test_archive = temp_dir.join("test_archive.zip");
        let base_dir = temp_dir.join("chuck_test_storage");

        // Create a test file
        let mut file = std::fs::File::create(&test_archive).unwrap();
        file.write_all(b"test content").unwrap();

        let result = create_storage_dir(&test_archive, &base_dir);
        assert!(result.is_ok());

        let storage_dir = result.unwrap();
        assert!(storage_dir.exists());
        assert!(storage_dir.starts_with(&base_dir));

        // Cleanup
        std::fs::remove_dir_all(&base_dir).ok();
        std::fs::remove_file(&test_archive).ok();
    }

    #[test]
    fn test_current_with_existing_archive() {
        let fixture = UnzippedArchiveFixture::with_structure(
            "current_existing",
            "test_archive.zip",
            &[
                ("meta.xml", br#"<?xml version="1.0" encoding="UTF-8"?>
<archive>
  <core>
    <files>
      <location>occurrence.csv</location>
    </files>
  </core>
</archive>"#),
                ("occurrence.csv", b"id,name\n1,test\n"),
            ],
            true,
        );

        let current_archive = Archive::current(fixture.base_dir()).unwrap();
        assert_eq!(current_archive.name, "test_archive.zip");
        assert_eq!(current_archive.storage_dir, fixture.dir());
        assert_eq!(current_archive.core_count().unwrap(), 1);
    }

    #[test]
    fn test_current_with_no_archive() {
        let temp_dir = std::env::temp_dir();
        let base_dir = temp_dir.join("chuck_test_current_no_archive");

        // Ensure base_dir doesn't exist
        std::fs::remove_dir_all(&base_dir).ok();

        let result = Archive::current(&base_dir);
        assert!(result.is_err());

        // Also test with empty directory
        std::fs::create_dir_all(&base_dir).unwrap();
        let result = Archive::current(&base_dir);
        assert!(result.is_err());

        // Cleanup
        std::fs::remove_dir_all(&base_dir).ok();
    }

    #[test]
    fn test_current_extracts_name_with_dashes() {
        let fixture = UnzippedArchiveFixture::with_structure(
            "current_name_extraction",
            "kueda-2017.zip",
            &[
                ("meta.xml", br#"<?xml version="1.0" encoding="UTF-8"?>
<archive>
  <core>
    <files>
      <location>occurrence.csv</location>
    </files>
  </core>
</archive>"#),
                ("occurrence.csv", b"id,name\n1,test\n"),
            ],
            true,
        );

        let current_archive = Archive::current(fixture.base_dir()).unwrap();
        assert_eq!(current_archive.name, "kueda-2017.zip");
    }

    #[test]
    fn test_current_returns_core_id_column() {
        let fixture = UnzippedArchiveFixture::with_structure(
            "core_id_column_extraction",
            "kueda-2017.zip",
            &[
                ("meta.xml", br#"<?xml version="1.0" encoding="UTF-8"?>
<archive>
  <core>
    <files>
      <location>occurrence.csv</location>
    </files>
    <id index="0" />
    <field index="1" term="http://rs.tdwg.org/dwc/terms/occurrenceID"/>
    <field index="0" term="http://rs.gbif.org/terms/1.0/gbifID"/>
  </core>
</archive>"#),
                ("occurrence.csv", b"gbifID,occurrenceID\n1,2\n"),
            ],
            true,
        );

        let current_archive = Archive::current(fixture.base_dir()).unwrap();
        assert_eq!(current_archive.core_id_column, "gbifID");
    }

    #[test]
    fn test_lazy_photo_extraction() {
        use std::io::Write;

        // Create a test archive with photos
        let test_name = "lazy_photo_extraction";
        let temp_dir = std::env::temp_dir().join(format!("chuck_test_{}", test_name));
        std::fs::remove_dir_all(&temp_dir).ok();
        std::fs::create_dir_all(&temp_dir).unwrap();

        // Create a simple ZIP archive with photos
        let archive_path = temp_dir.join("test.zip");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&archive_path).unwrap());
        let options: zip::write::FileOptions<()> = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .unix_permissions(0o644);

        // Add meta.xml
        let meta_xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<archive xmlns="http://rs.tdwg.org/dwc/text/">
  <core rowType="http://rs.tdwg.org/dwc/terms/Occurrence" encoding="UTF-8" fieldsTerminatedBy="," linesTerminatedBy="\n" fieldsEnclosedBy='"' ignoreHeaderLines="1">
    <files>
      <location>occurrence.csv</location>
    </files>
    <id index="0" />
    <field index="0" term="http://rs.tdwg.org/dwc/terms/occurrenceID"/>
  </core>
  <extension rowType="http://rs.gbif.org/terms/1.0/Multimedia" encoding="UTF-8" fieldsTerminatedBy="," linesTerminatedBy="\n" fieldsEnclosedBy='"' ignoreHeaderLines="1">
    <files>
      <location>multimedia.csv</location>
    </files>
    <coreid index="0" />
    <field index="0" term="http://rs.tdwg.org/dwc/terms/occurrenceID"/>
    <field index="1" term="http://purl.org/dc/terms/identifier"/>
  </extension>
</archive>"#;
        zip.start_file("meta.xml", options.clone()).unwrap();
        zip.write_all(meta_xml).unwrap();

        // Add occurrence.csv
        let occurrence_csv = b"occurrenceID\n1\n";
        zip.start_file("occurrence.csv", options.clone()).unwrap();
        zip.write_all(occurrence_csv).unwrap();

        // Add multimedia.csv with photo reference
        let multimedia_csv = b"occurrenceID,identifier\n1,media/2024/01/01/test.jpg\n";
        zip.start_file("multimedia.csv", options.clone()).unwrap();
        zip.write_all(multimedia_csv).unwrap();

        // Add a photo file
        let photo_data = b"fake jpeg data";
        zip.start_file("media/2024/01/01/test.jpg", options).unwrap();
        zip.write_all(photo_data).unwrap();

        zip.finish().unwrap();

        // Open the archive (photos should NOT be extracted)
        let base_dir = temp_dir.join("storage");
        let archive = Archive::open_with_progress(&archive_path, &base_dir, |_| {}).unwrap();

        // Verify photos were not extracted during open
        let media_dir = archive.storage_dir.join("media");
        assert!(!media_dir.exists(), "Photos should not be extracted during archive open");

        // Verify archive.zip was created (hard link)
        let archive_zip = archive.storage_dir.join("archive.zip");
        assert!(archive_zip.exists(), "archive.zip hard link should exist");

        // Verify photo cache table exists
        let photo_cache = crate::photo_cache::PhotoCache::new(archive.db.connection());
        let cache_size = photo_cache.get_cache_size().unwrap();
        assert_eq!(cache_size, 0, "Cache should be empty initially");

        // Request the photo (should extract on demand)
        let cached_path = archive.get_photo("media/2024/01/01/test.jpg").unwrap();
        assert!(std::path::Path::new(&cached_path).exists(), "Photo should be extracted to cache");

        // Verify it's in the cache directory
        assert!(cached_path.contains("photo_cache"), "Photo should be in photo_cache directory");

        // Verify cache size is updated
        let cache_size = photo_cache.get_cache_size().unwrap();
        assert!(cache_size > 0, "Cache size should be non-zero after extraction");

        // Request the same photo again (should come from cache)
        let cached_path2 = archive.get_photo("media/2024/01/01/test.jpg").unwrap();
        assert_eq!(cached_path, cached_path2, "Second request should return same cached path");

        // Verify the photo content
        let content = std::fs::read(&cached_path).unwrap();
        assert_eq!(content, photo_data, "Cached photo content should match original");

        // Clean up
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_archive_info_includes_available_columns() {
        let meta_xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<archive xmlns="http://rs.tdwg.org/dwc/text/">
  <core rowType="http://rs.tdwg.org/dwc/terms/Occurrence">
    <files><location>occurrence.csv</location></files>
    <id index="0"/>
  </core>
</archive>"#;

        let csv_content = b"id,scientificName,eventDate\n\
                            1,Test species,2023-01-01\n";

        let files = &[
            ("meta.xml", &meta_xml[..]),
            ("occurrence.csv", &csv_content[..]),
        ];

        let fixture = ZippedArchiveFixture::new("info_columns", Some(files));
        let archive = Archive::open(fixture.archive_path(), fixture.base_dir()).unwrap();

        let info = archive.info().unwrap();

        assert!(info.available_columns.len() >= 3);
        assert!(info.available_columns.contains(&"id".to_string()));
        assert!(info.available_columns.contains(&"scientificName".to_string()));
        assert!(info.available_columns.contains(&"eventDate".to_string()));
    }
}


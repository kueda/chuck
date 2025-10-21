use std::path::{Path, PathBuf};
use sha2::{Digest, Sha256};
use crate::error::{ChuckError, Result};
use crate::commands::archive::SearchParams;
use crate::db::Database;

/// Information about an extension in a DarwinCore Archive
#[derive(Debug, Clone)]
pub struct ExtensionInfo {
    /// The rowType from meta.xml (e.g., "http://rs.gbif.org/terms/1.0/Multimedia")
    pub row_type: String,
    /// Path to the extension CSV file
    pub location: PathBuf,
    /// Table name derived from rowType (e.g., "multimedia")
    pub table_name: String,
    /// The core ID column name that extensions reference (e.g., "gbifID" or "occurrenceID")
    pub core_id_column: String,
}

/// Supported DarwinCore Archive extension rowTypes
const SUPPORTED_ROW_TYPES: &[&str] = &[
    "http://rs.gbif.org/terms/1.0/Multimedia",           // Simple Multimedia
    "http://rs.tdwg.org/ac/terms/Multimedia",             // Audiovisual
    "http://rs.tdwg.org/dwc/terms/Identification",        // Identification
];

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
        let db = Database::create_from_core_files(&core_files, &extensions, &db_path)?;

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
        Ok(crate::commands::archive::ArchiveInfo {
            name: self.name.clone(),
            core_count: self.core_count()?,
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
            order_by: Some(self.core_id_column.clone()),
            ..search_params
        };
        self.db.search(
            limit,
            offset,
            params,
            fields
        )
    }
}

fn create_storage_dir(archive_path: &Path, base_dir: &Path) -> Result<PathBuf> {
    let mut file = std::fs::File::open(archive_path).map_err(|e| ChuckError::FileOpen {
        path: archive_path.to_path_buf(),
        source: e,
    })?;

    let fname = archive_path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| ChuckError::InvalidFileName(archive_path.to_path_buf()))?;

    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher).map_err(|e| ChuckError::FileRead {
        path: archive_path.to_path_buf(),
        source: e,
    })?;

    let file_hash = hasher.finalize();
    let file_hash_string = format!("{}-{:x}", fname, file_hash);
    let target_dir = base_dir.join(file_hash_string);

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
    let file = std::fs::File::open(archive_path).map_err(|e| ChuckError::FileOpen {
        path: archive_path.to_path_buf(),
        source: e,
    })?;

    let mut archive = zip::ZipArchive::new(file).map_err(ChuckError::ArchiveExtraction)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(ChuckError::ArchiveExtraction)?;
        let outpath = match file.enclosed_name() {
            Some(path) => target_dir.join(path),
            None => continue,
        };

        if file.is_dir() {
            std::fs::create_dir_all(&outpath).map_err(|e| ChuckError::DirectoryCreate {
                path: outpath,
                source: e,
            })?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    std::fs::create_dir_all(p).map_err(|e| ChuckError::DirectoryCreate {
                        path: p.to_path_buf(),
                        source: e,
                    })?;
                }
            }

            let mut outfile =
                std::fs::File::create(&outpath).map_err(|e| ChuckError::FileOpen {
                    path: outpath.clone(),
                    source: e,
                })?;
            std::io::copy(&mut file, &mut outfile).map_err(|e| ChuckError::FileRead {
                path: outpath.clone(),
                source: e,
            })?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = file.unix_mode() {
                    std::fs::set_permissions(&outpath, std::fs::Permissions::from_mode(mode))
                        .ok();
                }
            }
        }
    }

    Ok(())
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

            // Only process supported extension types
            if !SUPPORTED_ROW_TYPES.contains(&row_type) {
                log::debug!("Skipping unsupported extension type: {}", row_type);
                return None;
            }

            // Extract location from <files><location>...</location></files>
            let location_text = ext_node
                .descendants()
                .filter(|n| n.has_tag_name("location"))
                .filter_map(|n| n.text())
                .next()?;

            let location = storage_dir.join(location_text);

            // Derive table name from rowType
            // e.g., "http://rs.gbif.org/terms/1.0/Multimedia" -> "multimedia"
            // or "http://rs.tdwg.org/ac/terms/Multimedia" -> "multimedia"
            let table_name = row_type
                .rsplit('/')
                .next()
                .or_else(|| row_type.rsplit('#').next())
                .unwrap_or("unknown")
                .to_lowercase();

            Some(ExtensionInfo {
                row_type: row_type.to_string(),
                location,
                table_name,
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
                    let db = Database::create_from_core_files(&csv_paths, &vec![], &db_path).unwrap();
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

        // Check that table names are derived correctly
        assert_eq!(extensions[0].table_name, "multimedia");
        assert_eq!(extensions[0].row_type, "http://rs.gbif.org/terms/1.0/Multimedia");
        assert_eq!(extensions[0].core_id_column, "occurrenceID");
        assert_eq!(
            extensions[0].location.file_name().unwrap(),
            "multimedia.csv"
        );

        assert_eq!(extensions[1].table_name, "multimedia");
        assert_eq!(extensions[1].row_type, "http://rs.tdwg.org/ac/terms/Multimedia");
        assert_eq!(extensions[1].core_id_column, "occurrenceID");
        assert_eq!(
            extensions[1].location.file_name().unwrap(),
            "audiovisual.csv"
        );

        assert_eq!(extensions[2].table_name, "identification");
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

        assert_eq!(extensions[0].table_name, "multimedia");
        assert_eq!(extensions[0].core_id_column, "occurrenceID");
        assert_eq!(extensions[1].table_name, "identification");
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
}


use std::path::{Path, PathBuf};
use crate::error::{ChuckError, Result};

/// Represents a Darwin Core Archive
#[derive(Debug)]
pub struct Archive {
    /// Path to the original archive file
    pub archive_path: PathBuf,
    /// Directory where archive contents are stored
    pub storage_dir: PathBuf,
    /// Paths to core data files
    pub core_files: Vec<PathBuf>,
}

impl Archive {
    /// Opens and extracts a Darwin Core Archive
    pub fn open(archive_path: &Path, storage_dir: &Path) -> Result<Self> {
        // Remove all other archive directories in the base directory
        if let Some(base_dir) = storage_dir.parent() {
            remove_other_archives(base_dir, storage_dir)?;
        }

        extract_archive(archive_path, storage_dir)?;
        let core_files = parse_meta_xml(storage_dir)?;

        Ok(Self {
            archive_path: archive_path.to_path_buf(),
            storage_dir: storage_dir.to_path_buf(),
            core_files,
        })
    }

    pub fn core_files(&self) -> &[PathBuf] {
        &self.core_files
    }
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

fn parse_meta_xml(storage_dir: &Path) -> Result<Vec<PathBuf>> {
    let meta_path = storage_dir.join("meta.xml");
    let contents = std::fs::read_to_string(&meta_path).map_err(|e| ChuckError::FileRead {
        path: meta_path.clone(),
        source: e,
    })?;

    let doc = roxmltree::Document::parse(&contents).map_err(|e| ChuckError::XmlParse {
        path: meta_path,
        source: e,
    })?;

    let core_files: Vec<PathBuf> = doc
        .descendants()
        .filter(|n| n.has_tag_name("core"))
        .flat_map(|core| core.descendants())
        .filter(|n| n.has_tag_name("location"))
        .filter_map(|n| n.text())
        .map(|text| storage_dir.join(text))
        .collect();

    if core_files.is_empty() {
        return Err(ChuckError::NoCoreFiles);
    }

    Ok(core_files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    struct UnzippedArchiveFixture {
        temp_dir: PathBuf
    }

    impl UnzippedArchiveFixture {
        fn new(test_name: &str, meta_xml: &str) -> Self {
            let temp_dir = std::env::temp_dir()
                .join(format!("chuck_test_dwca_archive_{}", test_name));
            std::fs::create_dir_all(&temp_dir).unwrap();
            let meta_path = temp_dir.join("meta.xml");
            let mut file = std::fs::File::create(&meta_path).unwrap();
            file.write_all(meta_xml.as_bytes()).unwrap();
            Self { temp_dir }
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

            Self { temp_dir }
        }

        fn dir(&self) -> &Path {
            &self.temp_dir
        }
    }

    impl Drop for UnzippedArchiveFixture {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.temp_dir).ok();
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

        let core_files = result.unwrap();
        assert_eq!(core_files.len(), 1);
        assert_eq!(
            core_files[0].file_name().unwrap(),
            "occurrence.csv"
        );
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

        let core_files = result.unwrap();
        assert_eq!(core_files.len(), 2);
        assert_eq!(
            core_files[0].file_name().unwrap(),
            "occurrence1.csv"
        );
        assert_eq!(
            core_files[1].file_name().unwrap(),
            "occurrence2.csv"
        );
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
    fn test_opening_new_archive_removes_other_archive_directories() {
        let fixture1 = ZippedArchiveFixture::new("test_archive1", None);
        let fixture2 = ZippedArchiveFixture::new("test_archive2", None);

        // Open first archive
        let storage_dir1 = crate::dwca::create_storage_dir(
            fixture1.archive_path(),
            fixture1.base_dir()
        ).unwrap();
        let _archive1 = Archive::open(fixture1.archive_path(), &storage_dir1).unwrap();
        assert!(storage_dir1.exists());

        // Open second archive using the same base directory
        let storage_dir2 = crate::dwca::create_storage_dir(
            fixture2.archive_path(),
            fixture1.base_dir()
        ).unwrap();
        let _archive2 = Archive::open(fixture2.archive_path(), &storage_dir2).unwrap();

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
}


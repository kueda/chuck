use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use zip::write::{FileOptions, ZipWriter};
use zip::CompressionMethod;
use crate::darwin_core::{
    audiovisual::Audiovisual,
    comment::Comment,
    identification::Identification,
    meta::{self, Metadata},
    multimedia::Multimedia,
    occurrence::Occurrence,
};

/// A DarwinCore Archive builder that can stream occurrence records and generate a compliant ZIP archive
pub struct ArchiveBuilder {
    temp_dir: TempDir,
    zip: ZipWriter<File>,
    /// The final destination path; the ZIP is written to a temp file and renamed here on success.
    output_path: PathBuf,
    occurrence_writer: csv::Writer<File>,
    multimedia_writer: Option<csv::Writer<File>>,
    audiovisual_writer: Option<csv::Writer<File>>,
    identification_writer: Option<csv::Writer<File>>,
    comment_writer: Option<csv::Writer<File>>,
    enabled_extensions: Vec<crate::DwcaExtension>,
    record_count: u64,
    multimedia_count: u64,
    audiovisual_count: u64,
    identification_count: u64,
    comment_count: u64,
    occurrence_file_path: PathBuf,
    multimedia_file_path: PathBuf,
    audiovisual_file_path: PathBuf,
    identification_file_path: PathBuf,
    comment_file_path: PathBuf,
    metadata: Metadata,
}

impl ArchiveBuilder {
    /// Create a new DarwinCore Archive builder.
    /// Opens the output ZIP file immediately; pass the final output path so that
    /// temp files land on the same filesystem (important on Linux where /tmp may be tmpfs).
    pub fn new(
        dwc_extensions: Vec<crate::DwcaExtension>,
        metadata: Metadata,
        output_path: &Path,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let base_dir = output_path.parent().unwrap_or(Path::new("."));
        let temp_dir = TempDir::new_in(base_dir)?;
        let occurrence_file_path = temp_dir.path().join("occurrence.csv");
        let multimedia_file_path = temp_dir.path().join("multimedia.csv");
        let audiovisual_file_path = temp_dir.path().join("audiovisual.csv");
        let identification_file_path = temp_dir.path().join("identification.csv");
        let comment_file_path = temp_dir.path().join("comment.csv");

        // Create media staging directory inside temp dir
        let media_dir_path = temp_dir.path().join("media");
        std::fs::create_dir_all(&media_dir_path)?;

        // Write the ZIP to a temp file inside the temp dir so that a cancelled or failed
        // download leaves no partial file at the final output path. On successful build()
        // the temp ZIP is renamed to output_path (same filesystem → atomic rename).
        let zip_temp_path = temp_dir.path().join("archive.zip");
        let zip_file = File::create(&zip_temp_path)?;
        let zip = ZipWriter::new(zip_file);

        // Create CSV writer for occurrence records
        let occurrence_file = File::create(&occurrence_file_path)?;
        let mut occurrence_writer = csv::WriterBuilder::new()
            .has_headers(true)
            .from_writer(occurrence_file);

        // Write CSV headers
        occurrence_writer.write_record(Occurrence::csv_headers())?;
        occurrence_writer.flush()?;

        Ok(Self {
            temp_dir,
            zip,
            output_path: output_path.to_path_buf(),
            occurrence_writer,
            multimedia_writer: None,
            audiovisual_writer: None,
            identification_writer: None,
            comment_writer: None,
            enabled_extensions: dwc_extensions,
            record_count: 0,
            multimedia_count: 0,
            audiovisual_count: 0,
            identification_count: 0,
            comment_count: 0,
            occurrence_file_path,
            multimedia_file_path,
            audiovisual_file_path,
            identification_file_path,
            comment_file_path,
            metadata,
        })
    }

    /// Get the media staging directory path for downloading files before adding to the ZIP.
    pub fn media_dir(&self) -> PathBuf {
        self.temp_dir.path().join("media")
    }

    /// Stream a staged media file into the open ZIP and remove it from the staging directory.
    /// `rel_zip_path` is the path as it should appear in the ZIP (e.g. `"media/2024/01/15/12345.jpg"`).
    /// The file must exist at `temp_dir / rel_zip_path`.
    pub fn add_media_from_temp(
        &mut self,
        rel_zip_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // ZIP spec requires forward slashes; normalize Windows backslashes.
        // Using the normalized form for the local path too ensures this works cross-platform
        // (on non-Windows, Path::join does not treat backslashes as separators).
        let normalized = rel_zip_path.replace('\\', "/");
        let local_path = self.temp_dir.path().join(&normalized);
        if !local_path.exists() {
            return Ok(());
        }
        let zip_opts: FileOptions<()> = FileOptions::default()
            .compression_method(CompressionMethod::Stored)
            .unix_permissions(0o644);
        self.zip.start_file(&normalized, zip_opts)?;
        let mut file = File::open(&local_path)?;
        std::io::copy(&mut file, &mut self.zip)?;
        std::fs::remove_file(&local_path)?;
        Ok(())
    }

    /// Add a batch of DarwinCore occurrences to the archive
    pub async fn add_occurrences(&mut self, occurrences: &[Occurrence]) -> Result<(), Box<dyn std::error::Error>> {
        for occurrence in occurrences {
            self.occurrence_writer.write_record(occurrence.to_csv_record())?;
            self.record_count += 1;
        }

        // Flush after each batch to ensure data is written
        self.occurrence_writer.flush()?;
        Ok(())
    }

    /// Add a batch of DarwinCore multimedia records to the archive
    pub async fn add_multimedia(&mut self, multimedia: &[Multimedia]) -> Result<(), Box<dyn std::error::Error>> {
        if multimedia.is_empty() {
            return Ok(());
        }

        // Initialize multimedia writer if this is the first multimedia batch
        if self.multimedia_writer.is_none() {
            let multimedia_file = File::create(&self.multimedia_file_path)?;
            let mut writer = csv::WriterBuilder::new()
                .has_headers(true)
                .from_writer(multimedia_file);

            // Write CSV headers
            writer.write_record(Multimedia::csv_headers())?;
            writer.flush()?;
            self.multimedia_writer = Some(writer);
        }

        if let Some(writer) = &mut self.multimedia_writer {
            for media in multimedia {
                writer.write_record(media.to_csv_record())?;
                self.multimedia_count += 1;
            }

            // Flush after each batch to ensure data is written
            writer.flush()?;
        }

        Ok(())
    }

    /// Add a batch of DarwinCore audiovisual records to the archive
    pub async fn add_audiovisual(&mut self, audiovisual: &[Audiovisual]) -> Result<(), Box<dyn std::error::Error>> {
        if audiovisual.is_empty() {
            return Ok(());
        }

        // Initialize audiovisual writer if this is the first audiovisual batch
        if self.audiovisual_writer.is_none() {
            let audiovisual_file = File::create(&self.audiovisual_file_path)?;
            let mut writer = csv::WriterBuilder::new()
                .has_headers(true)
                .from_writer(audiovisual_file);

            // Write CSV headers
            writer.write_record(Audiovisual::csv_headers())?;
            writer.flush()?;
            self.audiovisual_writer = Some(writer);
        }

        if let Some(writer) = &mut self.audiovisual_writer {
            for media in audiovisual {
                writer.write_record(media.to_csv_record())?;
                self.audiovisual_count += 1;
            }

            // Flush after each batch to ensure data is written
            writer.flush()?;
        }

        Ok(())
    }

    /// Add a batch of DarwinCore identification records to the archive
    pub async fn add_identifications(&mut self, identifications: &[Identification]) -> Result<(), Box<dyn std::error::Error>> {
        if identifications.is_empty() {
            return Ok(());
        }

        // Initialize identification writer if this is the first identification batch
        if self.identification_writer.is_none() {
            let identification_file = File::create(&self.identification_file_path)?;
            let mut writer = csv::WriterBuilder::new()
                .has_headers(true)
                .from_writer(identification_file);

            // Write CSV headers
            writer.write_record(Identification::csv_headers())?;
            writer.flush()?;
            self.identification_writer = Some(writer);
        }

        if let Some(writer) = &mut self.identification_writer {
            for identification in identifications {
                writer.write_record(identification.to_csv_record())?;
                self.identification_count += 1;
            }

            // Flush after each batch to ensure data is written
            writer.flush()?;
        }

        Ok(())
    }

    /// Add a batch of DarwinCore comment records to the archive
    pub async fn add_comments(
        &mut self,
        comments: &[Comment],
    ) -> Result<(), Box<dyn std::error::Error>> {
        if comments.is_empty() {
            return Ok(());
        }

        // Initialize comment writer if this is the first comment batch
        if self.comment_writer.is_none() {
            let comment_file = File::create(&self.comment_file_path)?;
            let mut writer = csv::WriterBuilder::new()
                .has_headers(true)
                .from_writer(comment_file);

            // Write CSV headers
            writer.write_record(Comment::csv_headers())?;
            writer.flush()?;
            self.comment_writer = Some(writer);
        }

        if let Some(writer) = &mut self.comment_writer {
            for comment in comments {
                writer.write_record(comment.to_csv_record())?;
                self.comment_count += 1;
            }

            // Flush after each batch to ensure data is written
            writer.flush()?;
        }

        Ok(())
    }

    /// Finish writing the archive. All media must have been added via `add_media_from_temp`
    /// before calling this; only CSV files and metadata are written here.
    pub async fn build(mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Ensure all CSV data is written
        self.occurrence_writer.flush()?;
        drop(self.occurrence_writer); // Close the file

        // Close multimedia writer if it exists
        if let Some(mut writer) = self.multimedia_writer.take() {
            writer.flush()?;
            drop(writer);
        }

        // Close audiovisual writer if it exists
        if let Some(mut writer) = self.audiovisual_writer.take() {
            writer.flush()?;
            drop(writer);
        }

        // Close identification writer if it exists
        if let Some(mut writer) = self.identification_writer.take() {
            writer.flush()?;
            drop(writer);
        }

        // Close comment writer if it exists
        if let Some(mut writer) = self.comment_writer.take() {
            writer.flush()?;
            drop(writer);
        }

        // Generate meta.xml (includes extensions based on enabled extensions and record counts)
        let meta_xml = meta::generate_meta_xml(&self.enabled_extensions);
        let meta_file_path = self.temp_dir.path().join("meta.xml");
        std::fs::write(&meta_file_path, meta_xml)?;

        // Generate EML metadata
        let eml_xml = meta::generate_eml(&self.metadata);
        let eml_file_path = self.temp_dir.path().join("eml.xml");
        std::fs::write(&eml_file_path, eml_xml)?;

        let options: FileOptions<()> = FileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .unix_permissions(0o644);

        // Add meta.xml to ZIP
        self.zip.start_file("meta.xml", options)?;
        let meta_content = std::fs::read(&meta_file_path)?;
        self.zip.write_all(&meta_content)?;

        // Add eml.xml to ZIP
        self.zip.start_file("eml.xml", options)?;
        let eml_content = std::fs::read(&eml_file_path)?;
        self.zip.write_all(&eml_content)?;

        // Add chuck.json if inat_query is set
        if let Some(ref inat_query) = self.metadata.inat_query {
            let chuck_json = serde_json::json!({ "inat_query": inat_query }).to_string();
            self.zip.start_file("chuck.json", options)?;
            self.zip.write_all(chuck_json.as_bytes())?;
        }

        // Add occurrence.csv to ZIP
        self.zip.start_file("occurrence.csv", options)?;
        let occurrence_content = std::fs::read(&self.occurrence_file_path)?;
        self.zip.write_all(&occurrence_content)?;

        // Add extension CSVs to ZIP for all enabled extensions, even if empty
        let ext_specs: &[(crate::DwcaExtension, &str, &std::path::Path, Vec<&str>)] = &[
            (
                crate::DwcaExtension::SimpleMultimedia,
                "multimedia.csv",
                &self.multimedia_file_path,
                Multimedia::csv_headers(),
            ),
            (
                crate::DwcaExtension::Audiovisual,
                "audiovisual.csv",
                &self.audiovisual_file_path,
                Audiovisual::csv_headers(),
            ),
            (
                crate::DwcaExtension::Identifications,
                "identification.csv",
                &self.identification_file_path,
                Identification::csv_headers(),
            ),
            (
                crate::DwcaExtension::Comments,
                "comment.csv",
                &self.comment_file_path,
                Comment::csv_headers(),
            ),
        ];

        for (ext, zip_name, file_path, headers) in ext_specs {
            if !self.enabled_extensions.contains(ext) {
                continue;
            }
            // Write header-only file if no records were written for this extension
            if !file_path.exists() {
                let mut wtr = csv::WriterBuilder::new()
                    .has_headers(true)
                    .from_path(file_path)?;
                wtr.write_record(headers)?;
                wtr.flush()?;
            }
            self.zip.start_file(*zip_name, options)?;
            self.zip.write_all(&std::fs::read(file_path)?)?;
        }

        // Finish ZIP (writes central directory)
        let zip_temp_path = self.temp_dir.path().join("archive.zip");
        self.zip.finish()?;

        // Rename the temp ZIP to the final output path. Both are on the same filesystem
        // so this is an atomic rename on most systems.
        std::fs::rename(&zip_temp_path, &self.output_path)?;

        log::info!(
            "DarwinCore Archive complete: {} records, {} multimedia, {} audiovisual, \
            {} identifications, {} comments",
            self.record_count, self.multimedia_count, self.audiovisual_count,
            self.identification_count, self.comment_count,
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::darwin_core::meta::Metadata;
    use crate::DwcaExtension;
    use zip::ZipArchive;

    #[test]
    fn test_archive_temp_files_in_same_dir_as_output() {
        let output_dir = tempfile::TempDir::new().unwrap();
        let output_path = output_dir.path().join("test.zip");
        let builder = ArchiveBuilder::new(vec![], Metadata::default(), &output_path).unwrap();
        // media_dir lives inside temp_dir, which must be under the output's parent dir
        assert!(
            builder.media_dir().starts_with(output_dir.path()),
            "expected media_dir {:?} to be under {:?}",
            builder.media_dir(),
            output_dir.path()
        );
    }

    #[tokio::test]
    async fn test_add_media_from_temp_adds_to_zip_and_removes_local_file() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let mut builder = ArchiveBuilder::new(vec![], Metadata::default(), tmp.path()).unwrap();

        // Create a fake media file in the staging dir
        let media_dir = builder.media_dir();
        std::fs::create_dir_all(media_dir.join("2024/01/15")).unwrap();
        let staged = media_dir.join("2024/01/15/99999.jpg");
        std::fs::write(&staged, b"fake image data").unwrap();

        builder.add_media_from_temp("media/2024/01/15/99999.jpg").unwrap();

        // Staging file must be gone
        assert!(!staged.exists(), "staged file should have been removed");

        builder.build().await.unwrap();

        let file = std::fs::File::open(tmp.path()).unwrap();
        let mut archive = ZipArchive::new(file).unwrap();
        let mut entry = archive.by_name("media/2024/01/15/99999.jpg")
            .expect("media entry missing from zip");
        let mut contents = vec![];
        std::io::Read::read_to_end(&mut entry, &mut contents).unwrap();
        assert_eq!(contents, b"fake image data");
    }

    #[tokio::test]
    async fn test_add_media_from_temp_normalizes_backslash_paths() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let mut builder = ArchiveBuilder::new(vec![], Metadata::default(), tmp.path()).unwrap();

        let media_dir = builder.media_dir();
        std::fs::create_dir_all(media_dir.join("2024/01/15")).unwrap();
        let staged = media_dir.join("2024/01/15/99999.jpg");
        std::fs::write(&staged, b"fake image data").unwrap();

        // Simulate what happens on Windows: rel_zip_path has backslash separators
        builder.add_media_from_temp(r"media\2024\01\15\99999.jpg").unwrap();

        builder.build().await.unwrap();

        let file = std::fs::File::open(tmp.path()).unwrap();
        let mut archive = ZipArchive::new(file).unwrap();
        // The entry must be stored with forward slashes so extract_single_file can find it
        let mut entry = archive.by_name("media/2024/01/15/99999.jpg")
            .expect("media entry should be stored with forward slashes");
        let mut contents = vec![];
        std::io::Read::read_to_end(&mut entry, &mut contents).unwrap();
        assert_eq!(contents, b"fake image data");
    }

    /// Build a minimal archive with no occurrences and the given extensions enabled,
    /// return the list of file names present in the ZIP.
    async fn zip_file_names(extensions: Vec<DwcaExtension>) -> Vec<String> {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let builder = ArchiveBuilder::new(extensions, Metadata::default(), tmp.path()).unwrap();
        builder.build().await.unwrap();
        let file = std::fs::File::open(tmp.path()).unwrap();
        let mut archive = ZipArchive::new(file).unwrap();
        (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_string())
            .collect()
    }

    #[tokio::test]
    async fn test_chuck_json_written_when_inat_query_present() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let metadata = Metadata {
            inat_query: Some("taxon_id=47790".to_string()),
            ..Default::default()
        };
        let builder = ArchiveBuilder::new(vec![], metadata, tmp.path()).unwrap();
        builder.build().await.unwrap();
        let file = std::fs::File::open(tmp.path()).unwrap();
        let mut archive = ZipArchive::new(file).unwrap();
        let mut chuck_file = archive.by_name("chuck.json").expect("chuck.json missing");
        let mut contents = String::new();
        std::io::Read::read_to_string(&mut chuck_file, &mut contents).unwrap();
        let json: serde_json::Value = serde_json::from_str(&contents).unwrap();
        assert_eq!(json["inat_query"], "taxon_id=47790");
    }

    #[tokio::test]
    async fn test_chuck_json_absent_when_no_inat_query() {
        let names = zip_file_names(vec![]).await;
        assert!(!names.contains(&"chuck.json".to_string()), "chuck.json should be absent");
    }

    #[tokio::test]
    async fn test_enabled_extensions_with_no_records_produce_csv_files() {
        let names = zip_file_names(vec![
            DwcaExtension::Comments,
            DwcaExtension::Identifications,
            DwcaExtension::SimpleMultimedia,
            DwcaExtension::Audiovisual,
        ])
        .await;

        assert!(names.contains(&"comment.csv".to_string()),
            "comment.csv missing from ZIP: {names:?}");
        assert!(names.contains(&"identification.csv".to_string()),
            "identification.csv missing from ZIP: {names:?}");
        assert!(names.contains(&"multimedia.csv".to_string()),
            "multimedia.csv missing from ZIP: {names:?}");
        assert!(names.contains(&"audiovisual.csv".to_string()),
            "audiovisual.csv missing from ZIP: {names:?}");
    }

    #[tokio::test]
    async fn test_disabled_extensions_produce_no_csv_files() {
        let names = zip_file_names(vec![]).await;
        assert!(!names.contains(&"comment.csv".to_string()));
        assert!(!names.contains(&"identification.csv".to_string()));
        assert!(!names.contains(&"multimedia.csv".to_string()));
        assert!(!names.contains(&"audiovisual.csv".to_string()));
    }
}

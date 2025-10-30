use std::fs::File;
use std::io::Write;
use std::path::{PathBuf};
use tempfile::TempDir;
use zip::write::{FileOptions, ZipWriter};
use zip::CompressionMethod;
use crate::darwin_core::{
    audiovisual::Audiovisual,
    identification::Identification,
    meta::{self, Metadata},
    multimedia::Multimedia,
    occurrence::Occurrence,
};

/// A DarwinCore Archive builder that can stream occurrence records and generate a compliant ZIP archive
pub struct ArchiveBuilder {
    temp_dir: TempDir,
    occurrence_writer: csv::Writer<File>,
    multimedia_writer: Option<csv::Writer<File>>,
    audiovisual_writer: Option<csv::Writer<File>>,
    identification_writer: Option<csv::Writer<File>>,
    enabled_extensions: Vec<crate::DwcExtension>,
    record_count: u64,
    multimedia_count: u64,
    audiovisual_count: u64,
    identification_count: u64,
    occurrence_file_path: PathBuf,
    multimedia_file_path: PathBuf,
    audiovisual_file_path: PathBuf,
    identification_file_path: PathBuf,
    media_dir_path: PathBuf,
    metadata: Metadata,
}

impl ArchiveBuilder {
    /// Create a new DarwinCore Archive builder
    pub fn new(dwc_extensions: Vec<crate::DwcExtension>, metadata: Metadata) -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let occurrence_file_path = temp_dir.path().join("occurrence.csv");
        let multimedia_file_path = temp_dir.path().join("multimedia.csv");
        let audiovisual_file_path = temp_dir.path().join("audiovisual.csv");
        let identification_file_path = temp_dir.path().join("identification.csv");
        let media_dir_path = temp_dir.path().join("media");

        // Create media directory
        std::fs::create_dir_all(&media_dir_path)?;

        // Create CSV writer for occurrence records
        let occurrence_file = File::create(&occurrence_file_path)?;
        let mut occurrence_writer = csv::WriterBuilder::new()
            .has_headers(true)
            .from_writer(occurrence_file);

        // Write CSV headers
        occurrence_writer.write_record(&Occurrence::csv_headers())?;
        occurrence_writer.flush()?;

        Ok(Self {
            temp_dir,
            occurrence_writer,
            multimedia_writer: None,
            audiovisual_writer: None,
            identification_writer: None,
            enabled_extensions: dwc_extensions,
            record_count: 0,
            multimedia_count: 0,
            audiovisual_count: 0,
            identification_count: 0,
            occurrence_file_path,
            multimedia_file_path,
            audiovisual_file_path,
            identification_file_path,
            media_dir_path,
            metadata,
        })
    }

    /// Get the media directory path for downloading photos
    pub fn media_dir(&self) -> &std::path::Path {
        &self.media_dir_path
    }

    /// Add a batch of DarwinCore occurrences to the archive
    pub async fn add_occurrences(&mut self, occurrences: &[Occurrence]) -> Result<(), Box<dyn std::error::Error>> {
        for occurrence in occurrences {
            self.occurrence_writer.write_record(&occurrence.to_csv_record())?;
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
            writer.write_record(&Multimedia::csv_headers())?;
            writer.flush()?;
            self.multimedia_writer = Some(writer);
        }

        if let Some(writer) = &mut self.multimedia_writer {
            for media in multimedia {
                writer.write_record(&media.to_csv_record())?;
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
            writer.write_record(&Audiovisual::csv_headers())?;
            writer.flush()?;
            self.audiovisual_writer = Some(writer);
        }

        if let Some(writer) = &mut self.audiovisual_writer {
            for media in audiovisual {
                writer.write_record(&media.to_csv_record())?;
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
            writer.write_record(&Identification::csv_headers())?;
            writer.flush()?;
            self.identification_writer = Some(writer);
        }

        if let Some(writer) = &mut self.identification_writer {
            for identification in identifications {
                writer.write_record(&identification.to_csv_record())?;
                self.identification_count += 1;
            }

            // Flush after each batch to ensure data is written
            writer.flush()?;
        }

        Ok(())
    }

    /// Build the final archive and create the ZIP file
    pub async fn build(mut self, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
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

        // Generate meta.xml (includes extensions based on enabled extensions and record counts)
        let meta_xml = meta::generate_meta_xml(&self.enabled_extensions);
        let meta_file_path = self.temp_dir.path().join("meta.xml");
        std::fs::write(&meta_file_path, meta_xml)?;

        // Generate EML metadata
        let eml_xml = meta::generate_eml(&self.metadata);
        let eml_file_path = self.temp_dir.path().join("eml.xml");
        std::fs::write(&eml_file_path, eml_xml)?;

        // Create ZIP archive
        let zip_file = File::create(output_path)?;
        let mut zip = ZipWriter::new(zip_file);
        let options: FileOptions<()> = FileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .unix_permissions(0o644);

        // Add meta.xml to ZIP
        zip.start_file("meta.xml", options)?;
        let meta_content = std::fs::read(&meta_file_path)?;
        zip.write_all(&meta_content)?;

        // Add eml.xml to ZIP

        zip.start_file("eml.xml", options)?;
        let eml_content = std::fs::read(&eml_file_path)?;
        zip.write_all(&eml_content)?;

        // Add occurrence.csv to ZIP
        zip.start_file("occurrence.csv", options)?;
        let occurrence_content = std::fs::read(&self.occurrence_file_path)?;
        zip.write_all(&occurrence_content)?;

        // Add multimedia.csv to ZIP if we have multimedia records
        if self.multimedia_count > 0 {
            zip.start_file("multimedia.csv", options)?;
            let multimedia_content = std::fs::read(&self.multimedia_file_path)?;
            zip.write_all(&multimedia_content)?;
        }

        // Add audiovisual.csv to ZIP if we have audiovisual records
        if self.audiovisual_count > 0 {
            zip.start_file("audiovisual.csv", options)?;
            let audiovisual_content = std::fs::read(&self.audiovisual_file_path)?;
            zip.write_all(&audiovisual_content)?;
        }

        // Add identification.csv to ZIP if we have identification records
        if self.identification_count > 0 {
            zip.start_file("identification.csv", options)?;
            let identification_content = std::fs::read(&self.identification_file_path)?;
            zip.write_all(&identification_content)?;
        }

        // Add media directory contents to ZIP if it exists and has files
        println!("Zipping media...");
        Self::add_directory_to_zip(&mut zip, &self.media_dir_path, "media")?;

        // Finish ZIP
        zip.finish()?;

        println!("DarwinCore Archive created: {}", output_path);
        println!("Records exported: {}", self.record_count);
        if self.multimedia_count > 0 {
            println!("Multimedia records exported: {}", self.multimedia_count);
        }
        if self.audiovisual_count > 0 {
            println!("Audiovisual records exported: {}", self.audiovisual_count);
        }
        if self.identification_count > 0 {
            println!("Identification records exported: {}", self.identification_count);
        }

        Ok(())
    }

    /// Add directory contents to ZIP archive using streaming to avoid loading entire files into memory
    fn add_directory_to_zip(
        zip: &mut ZipWriter<File>,
        dir_path: &std::path::Path,
        zip_prefix: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if !dir_path.exists() || !dir_path.is_dir() {
            return Ok(());
        }

        // There's no point in trying to compress JPGs
        let zip_opts: FileOptions<()> = FileOptions::default()
            .compression_method(CompressionMethod::Stored)
            .unix_permissions(0o644);

        for entry in std::fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                let file_name = path.file_name()
                    .and_then(|name| name.to_str())
                    .ok_or("Invalid filename")?;

                let zip_path = format!("{}/{}", zip_prefix, file_name);
                zip.start_file(zip_path, zip_opts)?;

                // Stream the file contents instead of reading entirely into memory
                let mut file = File::open(&path)?;
                std::io::copy(&mut file, zip)?;
            } else if path.is_dir() {
                // Recursively add subdirectory contents
                let dir_name = path.file_name()
                    .and_then(|name| name.to_str())
                    .ok_or("Invalid directory name")?;

                let subdir_zip_prefix = format!("{}/{}", zip_prefix, dir_name);
                Self::add_directory_to_zip(zip, &path, &subdir_zip_prefix)?;
            }
        }

        Ok(())
    }
}

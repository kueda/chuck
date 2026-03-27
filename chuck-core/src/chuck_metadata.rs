use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChuckMetadata {
    pub inat_query: Option<String>,
}

/// Read Chuck-specific metadata from a DwC-A ZIP archive.
/// Returns `None` if the archive was not produced by Chuck (no `chuck.json`).
pub fn read_chuck_metadata(
    zip_path: &str,
) -> Result<Option<ChuckMetadata>, Box<dyn std::error::Error>> {
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    match archive.by_name("chuck.json") {
        Ok(mut entry) => {
            let mut contents = String::new();
            std::io::Read::read_to_string(&mut entry, &mut contents)?;
            let meta: ChuckMetadata = serde_json::from_str(&contents)?;
            Ok(Some(meta))
        }
        Err(zip::result::ZipError::FileNotFound) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Read the `pubDate` value from `eml.xml` inside a DwC-A ZIP archive.
/// Returns `None` if `eml.xml` is absent or contains no `<pubDate>` element.
pub fn read_pub_date(
    zip_path: &str,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut eml_entry = match archive.by_name("eml.xml") {
        Ok(e) => e,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let mut contents = String::new();
    std::io::Read::read_to_string(&mut eml_entry, &mut contents)?;
    let tag = "<pubDate>";
    if let Some(start) = contents.find(tag) {
        let rest = &contents[start + tag.len()..];
        if let Some(end) = rest.find("</pubDate>") {
            return Ok(Some(rest[..end].trim().to_string()));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::darwin_core::meta::Metadata;
    use crate::darwin_core::archive::ArchiveBuilder;

    async fn build_archive(inat_query: Option<String>) -> tempfile::NamedTempFile {
        let metadata = Metadata { inat_query, ..Default::default() };
        let builder = ArchiveBuilder::new(vec![], metadata, &std::env::temp_dir()).unwrap();
        let tmp = tempfile::NamedTempFile::new().unwrap();
        builder.build(tmp.path().to_str().unwrap()).await.unwrap();
        tmp
    }

    #[tokio::test]
    async fn test_reads_inat_query() {
        let tmp = build_archive(Some("taxon_id=47790".to_string())).await;
        let meta = read_chuck_metadata(tmp.path().to_str().unwrap())
            .unwrap()
            .expect("expected Some(ChuckMetadata)");
        assert_eq!(meta.inat_query, Some("taxon_id=47790".to_string()));
    }

    #[tokio::test]
    async fn test_returns_none_when_no_chuck_json() {
        let tmp = build_archive(None).await;
        let meta = read_chuck_metadata(tmp.path().to_str().unwrap()).unwrap();
        assert!(meta.is_none());
    }

    #[tokio::test]
    async fn test_reads_pub_date() {
        let tmp = build_archive(None).await;
        let pub_date = read_pub_date(tmp.path().to_str().unwrap()).unwrap();
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        assert_eq!(pub_date, Some(today));
    }

    #[tokio::test]
    async fn test_read_pub_date_returns_none_when_no_eml() {
        // Build a zip with no eml.xml
        let tmp = tempfile::NamedTempFile::new().unwrap();
        {
            let file = std::fs::File::create(tmp.path()).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            let options: zip::write::FileOptions<()> = zip::write::FileOptions::default();
            zip.start_file("meta.xml", options).unwrap();
            zip.finish().unwrap();
        }
        let pub_date = read_pub_date(tmp.path().to_str().unwrap()).unwrap();
        assert!(pub_date.is_none());
    }
}

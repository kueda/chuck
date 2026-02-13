use std::io::Write;
use std::path::{Path, PathBuf};

use chuck_lib::dwca::Archive;
use chuck_lib::search_params::SearchParams;

struct ZipFixture {
    archive_path: PathBuf,
    base_dir: PathBuf,
}

impl ZipFixture {
    fn new(test_name: &str, files: &[(&str, &[u8])]) -> Self {
        let temp_dir = std::env::temp_dir()
            .join(format!("chuck_integration_{test_name}"));
        std::fs::remove_dir_all(&temp_dir).ok();
        std::fs::create_dir_all(&temp_dir).unwrap();

        let archive_path = temp_dir.join("archive.zip");
        let base_dir = temp_dir.join("storage");

        let archive_file = std::fs::File::create(&archive_path).unwrap();
        let mut zip = zip::ZipWriter::new(archive_file);
        let options = zip::write::FileOptions::<()>::default();

        for (filename, content) in files {
            zip.start_file(*filename, options).unwrap();
            zip.write_all(content).unwrap();
        }
        zip.finish().unwrap();

        Self { archive_path, base_dir }
    }

    fn archive_path(&self) -> &Path {
        &self.archive_path
    }

    fn base_dir(&self) -> &Path {
        &self.base_dir
    }
}

impl Drop for ZipFixture {
    fn drop(&mut self) {
        if let Some(parent) = self.archive_path.parent() {
            std::fs::remove_dir_all(parent).ok();
        }
    }
}

#[test]
fn test_open_archive_with_audiovisual_extension() {
    let meta_xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<archive xmlns="http://rs.tdwg.org/dwc/text/">
  <core encoding="UTF-8" fieldsTerminatedBy="," linesTerminatedBy="\n"
        fieldsEnclosedBy='"' ignoreHeaderLines="1"
        rowType="http://rs.tdwg.org/dwc/terms/Occurrence">
    <files>
      <location>occurrence.csv</location>
    </files>
    <id index="0" />
    <field index="0" term="http://rs.tdwg.org/dwc/terms/occurrenceID"/>
    <field index="1" term="http://rs.tdwg.org/dwc/terms/scientificName"/>
  </core>
  <extension encoding="UTF-8" fieldsTerminatedBy="," linesTerminatedBy="\n"
             fieldsEnclosedBy='"' ignoreHeaderLines="1"
             rowType="http://rs.tdwg.org/ac/terms/Multimedia">
    <files>
      <location>audiovisual.csv</location>
    </files>
    <coreid index="0" />
    <field index="0" term="http://rs.tdwg.org/dwc/terms/occurrenceID"/>
    <field index="1" term="http://purl.org/dc/terms/identifier"/>
    <field index="2" term="http://purl.org/dc/terms/type"/>
    <field index="3" term="http://purl.org/dc/terms/title"/>
  </extension>
</archive>"#;

    let occurrence_csv = b"occurrenceID,scientificName\n\
        1,Quercus agrifolia\n\
        2,Sequoia sempervirens\n\
        3,Pinus coulteri\n";

    let audiovisual_csv = b"occurrenceID,identifier,type,title\n\
        1,https://example.com/photo1.jpg,StillImage,Oak photo\n\
        1,https://example.com/photo2.jpg,StillImage,Oak photo 2\n\
        2,https://example.com/video.mp4,MovingImage,Redwood video\n";

    let files: &[(&str, &[u8])] = &[
        ("meta.xml", meta_xml),
        ("occurrence.csv", occurrence_csv),
        ("audiovisual.csv", audiovisual_csv),
    ];

    let fixture = ZipFixture::new("audiovisual_extension", files);
    let archive = Archive::open(
        fixture.archive_path(),
        fixture.base_dir(),
        |_| {},
    ).unwrap();

    // Should have 3 occurrence records
    assert_eq!(archive.core_count().unwrap(), 3);

    // Search all records
    let result = archive.search(
        10,
        0,
        SearchParams {
            sort_by: Some("occurrenceID".to_string()),
            ..SearchParams::default()
        },
        None,
    ).unwrap();
    assert_eq!(result.total, 3);

    // Find each occurrence by its ID
    let occ1 = result.results.iter()
        .find(|r| r.get("occurrenceID").and_then(|v| v.as_i64()) == Some(1))
        .expect("occurrence 1 not found");
    let occ2 = result.results.iter()
        .find(|r| r.get("occurrenceID").and_then(|v| v.as_i64()) == Some(2))
        .expect("occurrence 2 not found");
    let occ3 = result.results.iter()
        .find(|r| r.get("occurrenceID").and_then(|v| v.as_i64()) == Some(3))
        .expect("occurrence 3 not found");

    // Occurrence 1: 2 audiovisual entries
    let av1 = occ1.get("audiovisual").unwrap().as_array().unwrap();
    assert_eq!(av1.len(), 2);
    assert_eq!(
        av1[0].get("identifier").and_then(|v| v.as_str()),
        Some("https://example.com/photo1.jpg")
    );
    assert_eq!(
        av1[0].get("type").and_then(|v| v.as_str()),
        Some("StillImage")
    );
    assert_eq!(
        av1[0].get("title").and_then(|v| v.as_str()),
        Some("Oak photo")
    );
    assert_eq!(
        av1[1].get("identifier").and_then(|v| v.as_str()),
        Some("https://example.com/photo2.jpg")
    );

    // Occurrence 2: 1 audiovisual entry
    let av2 = occ2.get("audiovisual").unwrap().as_array().unwrap();
    assert_eq!(av2.len(), 1);
    assert_eq!(
        av2[0].get("identifier").and_then(|v| v.as_str()),
        Some("https://example.com/video.mp4")
    );
    assert_eq!(
        av2[0].get("type").and_then(|v| v.as_str()),
        Some("MovingImage")
    );
    assert_eq!(
        av2[0].get("title").and_then(|v| v.as_str()),
        Some("Redwood video")
    );

    // Occurrence 3: empty audiovisual array
    let av3 = occ3.get("audiovisual").unwrap().as_array().unwrap();
    assert_eq!(av3.len(), 0);
}

#[test]
fn test_aggregate_by_field_with_audiovisual_extension() {
    let _ = env_logger::try_init();
    let meta_xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<archive xmlns="http://rs.tdwg.org/dwc/text/">
  <core encoding="UTF-8" fieldsTerminatedBy="," linesTerminatedBy="\n"
        fieldsEnclosedBy='"' ignoreHeaderLines="1"
        rowType="http://rs.tdwg.org/dwc/terms/Occurrence">
    <files>
      <location>occurrence.csv</location>
    </files>
    <id index="0" />
    <field index="0" term="http://rs.tdwg.org/dwc/terms/occurrenceID"/>
    <field index="1" term="http://rs.tdwg.org/dwc/terms/scientificName"/>
  </core>
  <extension encoding="UTF-8" fieldsTerminatedBy="," linesTerminatedBy="\n"
             fieldsEnclosedBy='"' ignoreHeaderLines="1"
             rowType="http://rs.tdwg.org/ac/terms/Multimedia">
    <files>
      <location>audiovisual.csv</location>
    </files>
    <coreid index="0" />
    <field index="0" term="http://rs.tdwg.org/dwc/terms/occurrenceID"/>
    <field index="1" term="http://rs.tdwg.org/ac/terms/accessURI"/>
    <field index="2" term="http://purl.org/dc/terms/type"/>
  </extension>
</archive>"#;

    // 4 occurrences: 2 oaks, 1 redwood, 1 pine
    let occurrence_csv = b"occurrenceID,scientificName\n\
        1,Quercus agrifolia\n\
        2,Quercus agrifolia\n\
        3,Sequoia sempervirens\n\
        4,Pinus coulteri\n";

    // Photos for occurrences 1 and 3 only
    let audiovisual_csv = b"occurrenceID,accessURI,type\n\
        1,https://example.com/oak.jpg,StillImage\n\
        3,https://example.com/redwood.jpg,StillImage\n";

    let files: &[(&str, &[u8])] = &[
        ("meta.xml", meta_xml),
        ("occurrence.csv", occurrence_csv),
        ("audiovisual.csv", audiovisual_csv),
    ];

    let fixture = ZipFixture::new("aggregate_audiovisual", files);
    let archive = Archive::open(
        fixture.archive_path(),
        fixture.base_dir(),
        |_| {},
    ).unwrap();

    let results = archive.aggregate_by_field(
        "scientificName",
        &SearchParams::default(),
        10,
    ).unwrap();

    // Should have 3 distinct scientificName values
    assert_eq!(results.len(), 3);

    // Results are ordered by count DESC
    let oak = results.iter()
        .find(|r| r.value.as_deref() == Some("Quercus agrifolia"))
        .expect("Quercus agrifolia not found");
    let redwood = results.iter()
        .find(|r| r.value.as_deref() == Some("Sequoia sempervirens"))
        .expect("Sequoia sempervirens not found");
    let pine = results.iter()
        .find(|r| r.value.as_deref() == Some("Pinus coulteri"))
        .expect("Pinus coulteri not found");

    assert_eq!(oak.count, 2);
    assert_eq!(redwood.count, 1);
    assert_eq!(pine.count, 1);

    // Oak has a photo (occurrence 1 is MIN core_id for Quercus agrifolia)
    assert_eq!(
        oak.photo_url.as_deref(),
        Some("https://example.com/oak.jpg")
    );

    // Redwood has a photo
    assert_eq!(
        redwood.photo_url.as_deref(),
        Some("https://example.com/redwood.jpg")
    );

    // Pine has no audiovisual entries
    assert!(pine.photo_url.is_none());
}

#[test]
fn test_aggregate_by_field_with_mismatched_csv_headers() {
    // CSV headers differ from the term names declared in meta.xml.
    // The rename-during-import logic should normalise them so queries
    // that reference canonical term names still work.
    let _ = env_logger::try_init();

    let meta_xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<archive xmlns="http://rs.tdwg.org/dwc/text/">
  <core encoding="UTF-8" fieldsTerminatedBy="," linesTerminatedBy="\n"
        fieldsEnclosedBy='"' ignoreHeaderLines="1"
        rowType="http://rs.tdwg.org/dwc/terms/Occurrence">
    <files>
      <location>occurrence.csv</location>
    </files>
    <id index="0" />
    <field index="0" term="http://rs.tdwg.org/dwc/terms/occurrenceID"/>
    <field index="1" term="http://rs.tdwg.org/dwc/terms/scientificName"/>
  </core>
  <extension encoding="UTF-8" fieldsTerminatedBy="," linesTerminatedBy="\n"
             fieldsEnclosedBy='"' ignoreHeaderLines="1"
             rowType="http://rs.tdwg.org/ac/terms/Multimedia">
    <files>
      <location>audiovisual.csv</location>
    </files>
    <coreid index="0" />
    <field index="0" term="http://rs.tdwg.org/dwc/terms/occurrenceID"/>
    <field index="1" term="http://rs.tdwg.org/ac/terms/accessURI"/>
    <field index="2" term="http://purl.org/dc/terms/type"/>
  </extension>
</archive>"#;

    let occurrence_csv = b"occurrenceID,scientificName\n\
        1,Quercus agrifolia\n\
        2,Sequoia sempervirens\n";

    // CSV header is "photo_url" but meta.xml declares term accessURI at index 1
    let audiovisual_csv = b"occurrenceID,photo_url,type\n\
        1,https://example.com/oak.jpg,StillImage\n\
        2,https://example.com/redwood.jpg,StillImage\n";

    let files: &[(&str, &[u8])] = &[
        ("meta.xml", meta_xml),
        ("occurrence.csv", occurrence_csv),
        ("audiovisual.csv", audiovisual_csv),
    ];

    let fixture = ZipFixture::new("aggregate_mismatched_headers", files);
    let archive = Archive::open(
        fixture.archive_path(),
        fixture.base_dir(),
        |_| {},
    ).unwrap();

    // aggregate_by_field references "accessURI" — this only works if the
    // column was renamed from "photo_url" during import
    let results = archive.aggregate_by_field(
        "scientificName",
        &SearchParams::default(),
        10,
    ).unwrap();

    assert_eq!(results.len(), 2);

    let oak = results.iter()
        .find(|r| r.value.as_deref() == Some("Quercus agrifolia"))
        .expect("Quercus agrifolia not found");
    assert_eq!(
        oak.photo_url.as_deref(),
        Some("https://example.com/oak.jpg")
    );

    let redwood = results.iter()
        .find(|r| r.value.as_deref() == Some("Sequoia sempervirens"))
        .expect("Sequoia sempervirens not found");
    assert_eq!(
        redwood.photo_url.as_deref(),
        Some("https://example.com/redwood.jpg")
    );
}

#[test]
fn test_open_archive_created_by_chuck() {
    // Sanity check: a valid Chuck-style DwC-A (with <id index="0"/> and
    // <field index="0" term="...occurrenceID"/> both on column 0) can be
    // opened and searched.
    let meta_xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<archive xmlns="http://rs.tdwg.org/dwc/text/"
         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
         xsi:schemaLocation="http://rs.tdwg.org/dwc/text/ http://rs.tdwg.org/dwc/text/tdwg_dwc_text.xsd">
  <core encoding="UTF-8" fieldsTerminatedBy="," linesTerminatedBy="\n"
        fieldsEnclosedBy='"' ignoreHeaderLines="1"
        rowType="http://rs.tdwg.org/dwc/terms/Occurrence">
    <files>
      <location>occurrence.csv</location>
    </files>
    <id index="0"/>
    <field index="0" term="http://rs.tdwg.org/dwc/terms/occurrenceID"/>
    <field index="1" term="http://rs.tdwg.org/dwc/terms/scientificName"/>
    <field index="2" term="http://rs.tdwg.org/dwc/terms/decimalLatitude"/>
    <field index="3" term="http://rs.tdwg.org/dwc/terms/decimalLongitude"/>
  </core>
</archive>"#;

    let occurrence_csv =
        b"occurrenceID,scientificName,decimalLatitude,decimalLongitude\n\
        1,Quercus agrifolia,37.7749,-122.4194\n\
        2,Sequoia sempervirens,37.2350,-122.0578\n";

    let files: &[(&str, &[u8])] = &[
        ("meta.xml", meta_xml),
        ("occurrence.csv", occurrence_csv),
    ];

    let fixture = ZipFixture::new("chuck_created", files);
    let archive = Archive::open(
        fixture.archive_path(),
        fixture.base_dir(),
        |_| {},
    ).unwrap();

    assert_eq!(archive.core_count().unwrap(), 2);

    let result = archive.search(
        10,
        0,
        SearchParams {
            sort_by: Some("occurrenceID".to_string()),
            ..SearchParams::default()
        },
        None,
    ).unwrap();
    assert_eq!(result.total, 2);

    let occ1 = result.results.iter()
        .find(|r| r.get("occurrenceID").and_then(|v| v.as_i64()) == Some(1))
        .expect("occurrence 1 not found");
    assert_eq!(
        occ1.get("scientificName").and_then(|v| v.as_str()),
        Some("Quercus agrifolia")
    );
}

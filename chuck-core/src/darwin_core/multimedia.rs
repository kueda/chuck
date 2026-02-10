/// Represents a multimedia record in a DarwinCore Archive following the Simple Multimedia extension
/// https://rs.gbif.org/extension/gbif/1.0/multimedia.xml
#[derive(Debug, Clone)]
pub struct Multimedia {
    pub coreid: Option<String>,
    pub occurrence_id: String,
    pub r#type: Option<String>,
    pub format: Option<String>,
    pub identifier: Option<String>,
    pub references: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub created: Option<String>,
    pub creator: Option<String>,
    pub contributor: Option<String>,
    pub publisher: Option<String>,
    pub audience: Option<String>,
    pub source: Option<String>,
    pub license: Option<String>,
    pub rights_holder: Option<String>,
    pub dataset_id: Option<String>,
}

impl Multimedia {
    /// Row type URI for Simple Multimedia extension
    pub const ROW_TYPE: &'static str =
        "http://rs.gbif.org/terms/1.0/Multimedia";

    /// CSV filename for the multimedia extension
    pub const FILENAME: &'static str = "multimedia.csv";

    /// Fields written to CSV when exporting, paired with their term URIs
    pub const WRITE_FIELDS: &'static [(&'static str, &'static str)] = &[
        ("occurrenceID", "http://rs.tdwg.org/dwc/terms/occurrenceID"),
        ("type", "http://purl.org/dc/terms/type"),
        ("format", "http://purl.org/dc/terms/format"),
        ("identifier", "http://purl.org/dc/terms/identifier"),
        ("references", "http://purl.org/dc/terms/references"),
        ("title", "http://purl.org/dc/terms/title"),
        ("description", "http://purl.org/dc/terms/description"),
        ("created", "http://purl.org/dc/terms/created"),
        ("creator", "http://purl.org/dc/terms/creator"),
        ("contributor", "http://purl.org/dc/terms/contributor"),
        ("publisher", "http://purl.org/dc/terms/publisher"),
        ("audience", "http://purl.org/dc/terms/audience"),
        ("source", "http://purl.org/dc/terms/source"),
        ("license", "http://purl.org/dc/terms/license"),
        ("rightsHolder", "http://purl.org/dc/terms/rightsHolder"),
        ("datasetID", "http://rs.tdwg.org/dwc/terms/datasetID"),
    ];

    /// Returns the CSV headers for multimedia records
    pub fn csv_headers() -> Vec<&'static str> {
        Self::WRITE_FIELDS.iter().map(|(name, _)| *name).collect()
    }

    /// Converts the multimedia record to a CSV record
    pub fn to_csv_record(&self) -> Vec<String> {
        vec![
            self.occurrence_id.clone(),
            self.r#type.clone().unwrap_or_default(),
            self.format.clone().unwrap_or_default(),
            self.identifier.clone().unwrap_or_default(),
            self.references.clone().unwrap_or_default(),
            self.title.clone().unwrap_or_default(),
            self.description.clone().unwrap_or_default(),
            self.created.clone().unwrap_or_default(),
            self.creator.clone().unwrap_or_default(),
            self.contributor.clone().unwrap_or_default(),
            self.publisher.clone().unwrap_or_default(),
            self.audience.clone().unwrap_or_default(),
            self.source.clone().unwrap_or_default(),
            self.license.clone().unwrap_or_default(),
            self.rights_holder.clone().unwrap_or_default(),
            self.dataset_id.clone().unwrap_or_default(),
        ]
    }
}

/// Represents a multimedia record in a DarwinCore Archive following the Simple Multimedia extension
/// https://rs.gbif.org/extension/gbif/1.0/multimedia.xml
#[derive(Debug, Clone)]
pub struct Multimedia {
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
    /// Returns the CSV headers for multimedia records
    pub fn csv_headers() -> Vec<&'static str> {
        vec![
            "occurrenceID",
            "type",
            "format",
            "identifier",
            "references",
            "title",
            "description",
            "created",
            "creator",
            "contributor",
            "publisher",
            "audience",
            "source",
            "license",
            "rightsHolder",
            "datasetID",
        ]
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

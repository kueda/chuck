// Darwin Core Comment extension
// https://schema.org/Comment

use serde::Serialize;

/// DarwinCore Comment record for the Comment extension
#[derive(Debug, Serialize)]
pub struct Comment {
    #[serde(rename = "coreid")]
    pub coreid: Option<String>,
    #[serde(rename = "occurrenceID")]
    pub occurrence_id: String,
    #[serde(rename = "identifier")]
    pub identifier: Option<String>,
    #[serde(rename = "text")]
    pub text: Option<String>,
    #[serde(rename = "author")]
    pub author: Option<String>,
    #[serde(rename = "authorID")]
    pub author_id: Option<String>,
    #[serde(rename = "created")]
    pub created: Option<String>,
    #[serde(rename = "modified")]
    pub modified: Option<String>,
}

impl Comment {
    /// Row type URI for the Comment extension
    pub const ROW_TYPE: &'static str = "https://schema.org/Comment";

    /// CSV filename for the comment extension
    pub const FILENAME: &'static str = "comment.csv";

    /// Fields written to CSV when exporting, paired with their term URIs
    pub const WRITE_FIELDS: &'static [(&'static str, &'static str)] = &[
        ("occurrenceID", "http://rs.tdwg.org/dwc/terms/occurrenceID"),
        ("identifier", "http://purl.org/dc/terms/identifier"),
        ("text", "https://schema.org/text"),
        ("author", "https://schema.org/author"),
        ("authorID", "https://chuck.kueda.net/terms/authorID"),
        ("created", "http://purl.org/dc/terms/created"),
        ("modified", "http://purl.org/dc/terms/modified"),
    ];

    /// Get the CSV header row for comment records
    pub fn csv_headers() -> Vec<&'static str> {
        Self::WRITE_FIELDS.iter().map(|(name, _)| *name).collect()
    }

    /// Convert to CSV record for writing
    pub fn to_csv_record(&self) -> Vec<String> {
        vec![
            self.occurrence_id.clone(),
            self.identifier.clone().unwrap_or_default(),
            self.text.clone().unwrap_or_default(),
            self.author.clone().unwrap_or_default(),
            self.author_id.clone().unwrap_or_default(),
            self.created.clone().unwrap_or_default(),
            self.modified.clone().unwrap_or_default(),
        ]
    }
}

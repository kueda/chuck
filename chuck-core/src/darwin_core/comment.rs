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
    /// Get the CSV header row for comment records
    pub fn csv_headers() -> Vec<&'static str> {
        vec![
            "coreid",
            "occurrenceID",
            "identifier",
            "text",
            "author",
            "authorID",
            "created",
            "modified",
        ]
    }

    /// Convert to CSV record for writing
    pub fn to_csv_record(&self) -> Vec<String> {
        vec![
            self.coreid.clone().unwrap_or_default(),
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

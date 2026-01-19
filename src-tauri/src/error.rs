use std::path::PathBuf;
use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum ChuckError {
    #[error("Failed to open file: {path}")]
    FileOpen {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to read file: {path}")]
    FileRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to create directory: {path}")]
    DirectoryCreate {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to extract archive")]
    ArchiveExtraction(#[source] zip::result::ZipError),

    #[error("Invalid file name in path: {0}")]
    InvalidFileName(PathBuf),

    #[error("Failed to parse XML: {path}")]
    XmlParse {
        path: PathBuf,
        #[source]
        source: roxmltree::Error,
    },

    #[error("No core files found in meta.xml")]
    NoCoreFiles,

    #[error("No archive found in directory: {0}")]
    NoArchiveFound(PathBuf),

    #[error("Database error: {0}")]
    Database(#[from] duckdb::Error),

    #[error("Invalid path encoding")]
    PathEncoding,

    #[error("Tauri error: {0}")]
    Tauri(String),

    #[error("Column '{column}' is not available for autocomplete (type: {column_type})")]
    AutocompleteNotAvailable {
        column: String,
        column_type: String,
    },

    #[error("Extension missing core ID: {0}")]
    NoExtensionCoreId(String),

    #[error("TYPE_OVERRIDES contains core_id column '{0}' - core ID must always be VARCHAR to handle all ID formats (numeric, text, UUIDs, etc.)")]
    CoreIdTypeOverride(String),
}

impl Serialize for ChuckError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, ChuckError>;

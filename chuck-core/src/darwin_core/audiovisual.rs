/// Represents an audiovisual record in a DarwinCore Archive following the Audiovisual Media Description extension
/// https://rs.gbif.org/extension/ac/audiovisual_2024_11_07.xml
#[derive(Debug, Clone)]
pub struct Audiovisual {
    // Core identifier and linkage
    pub coreid: Option<String>,
    pub occurrence_id: String,
    pub identifier: Option<String>,

    // Management vocabulary
    pub r#type: Option<String>,
    pub title: Option<String>,
    pub modified: Option<String>,
    pub metadata_language_literal: Option<String>,
    pub available: Option<String>,

    // Attribution vocabulary
    pub rights: Option<String>,
    pub owner: Option<String>,
    pub usage_terms: Option<String>,
    pub credit: Option<String>,
    pub attribution_link_url: Option<String>,
    pub source: Option<String>,

    // Content and context vocabulary
    pub description: Option<String>,
    pub caption: Option<String>,
    pub comments: Option<String>,

    // Taxonomic coverage vocabulary
    pub scientific_name: Option<String>,
    pub common_name: Option<String>,
    pub life_stage: Option<String>,
    pub part_of_organism: Option<String>,

    // Geography vocabulary
    pub location_shown: Option<String>,
    pub location_created: Option<String>,
    pub continent: Option<String>,
    pub country: Option<String>,
    pub country_code: Option<String>,
    pub state_province: Option<String>,
    pub locality: Option<String>,
    pub decimal_latitude: Option<f64>,
    pub decimal_longitude: Option<f64>,

    // Service access point vocabulary
    pub access_uri: Option<String>,
    pub format: Option<String>,
    pub extent: Option<String>,
    pub pixel_x_dimension: Option<i32>,
    pub pixel_y_dimension: Option<i32>,

    // Date and time vocabulary
    pub created: Option<String>,
    pub date_time_original: Option<String>,
    pub temporal_coverage: Option<String>,
}

impl Audiovisual {
    /// Returns the CSV headers for audiovisual records
    pub fn csv_headers() -> Vec<&'static str> {
        vec![
            "coreid",
            "occurrenceID",
            "identifier",
            "type",
            "title",
            "modified",
            "metadataLanguageLiteral",
            "available",
            "rights",
            "owner",
            "usageTerms",
            "credit",
            "attributionLinkURL",
            "source",
            "description",
            "caption",
            "comments",
            "scientificName",
            "commonName",
            "lifeStage",
            "partOfOrganism",
            "locationShown",
            "locationCreated",
            "continent",
            "country",
            "countryCode",
            "stateProvince",
            "locality",
            "decimalLatitude",
            "decimalLongitude",
            "accessURI",
            "format",
            "extent",
            "pixelXDimension",
            "pixelYDimension",
            "created",
            "dateTimeOriginal",
            "temporalCoverage",
        ]
    }

    /// Converts the audiovisual record to a CSV record
    pub fn to_csv_record(&self) -> Vec<String> {
        vec![
            self.coreid.clone().unwrap_or_default(),
            self.occurrence_id.clone(),
            self.identifier.clone().unwrap_or_default(),
            self.r#type.clone().unwrap_or_default(),
            self.title.clone().unwrap_or_default(),
            self.modified.clone().unwrap_or_default(),
            self.metadata_language_literal.clone().unwrap_or_default(),
            self.available.clone().unwrap_or_default(),
            self.rights.clone().unwrap_or_default(),
            self.owner.clone().unwrap_or_default(),
            self.usage_terms.clone().unwrap_or_default(),
            self.credit.clone().unwrap_or_default(),
            self.attribution_link_url.clone().unwrap_or_default(),
            self.source.clone().unwrap_or_default(),
            self.description.clone().unwrap_or_default(),
            self.caption.clone().unwrap_or_default(),
            self.comments.clone().unwrap_or_default(),
            self.scientific_name.clone().unwrap_or_default(),
            self.common_name.clone().unwrap_or_default(),
            self.life_stage.clone().unwrap_or_default(),
            self.part_of_organism.clone().unwrap_or_default(),
            self.location_shown.clone().unwrap_or_default(),
            self.location_created.clone().unwrap_or_default(),
            self.continent.clone().unwrap_or_default(),
            self.country.clone().unwrap_or_default(),
            self.country_code.clone().unwrap_or_default(),
            self.state_province.clone().unwrap_or_default(),
            self.locality.clone().unwrap_or_default(),
            self.decimal_latitude.map(|v| v.to_string()).unwrap_or_default(),
            self.decimal_longitude.map(|v| v.to_string()).unwrap_or_default(),
            self.access_uri.clone().unwrap_or_default(),
            self.format.clone().unwrap_or_default(),
            self.extent.clone().unwrap_or_default(),
            self.pixel_x_dimension.map(|v| v.to_string()).unwrap_or_default(),
            self.pixel_y_dimension.map(|v| v.to_string()).unwrap_or_default(),
            self.created.clone().unwrap_or_default(),
            self.date_time_original.clone().unwrap_or_default(),
            self.temporal_coverage.clone().unwrap_or_default(),
        ]
    }
}

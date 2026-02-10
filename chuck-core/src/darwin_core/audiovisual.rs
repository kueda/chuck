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
    /// Row type URI for Audiovisual Media Description extension
    pub const ROW_TYPE: &'static str =
        "http://rs.tdwg.org/ac/terms/Multimedia";

    /// CSV filename for the audiovisual extension
    pub const FILENAME: &'static str = "audiovisual.csv";

    /// Fields written to CSV when exporting, paired with their term URIs
    pub const WRITE_FIELDS: &'static [(&'static str, &'static str)] = &[
        ("occurrenceID", "http://rs.tdwg.org/dwc/terms/occurrenceID"),
        ("identifier", "http://purl.org/dc/terms/identifier"),
        ("type", "http://purl.org/dc/terms/type"),
        ("title", "http://purl.org/dc/terms/title"),
        ("modified", "http://purl.org/dc/terms/modified"),
        ("metadataLanguageLiteral", "http://rs.tdwg.org/ac/terms/metadataLanguageLiteral"),
        ("available", "http://purl.org/dc/terms/available"),
        ("rights", "http://purl.org/dc/terms/rights"),
        ("owner", "http://ns.adobe.com/xap/1.0/rights/Owner"),
        ("usageTerms", "http://ns.adobe.com/xap/1.0/rights/UsageTerms"),
        ("credit", "http://ns.adobe.com/photoshop/1.0/Credit"),
        ("attributionLinkURL", "http://rs.tdwg.org/ac/terms/attributionLinkURL"),
        ("source", "http://purl.org/dc/terms/source"),
        ("description", "http://purl.org/dc/terms/description"),
        ("caption", "http://rs.tdwg.org/ac/terms/caption"),
        ("comments", "http://rs.tdwg.org/ac/terms/comments"),
        ("scientificName", "http://rs.tdwg.org/dwc/terms/scientificName"),
        ("commonName", "http://rs.tdwg.org/ac/terms/commonName"),
        ("lifeStage", "http://rs.tdwg.org/dwc/terms/lifeStage"),
        ("partOfOrganism", "http://rs.tdwg.org/ac/terms/partOfOrganism"),
        ("locationShown", "http://rs.tdwg.org/ac/terms/locationShown"),
        ("locationCreated", "http://rs.tdwg.org/ac/terms/locationCreated"),
        ("continent", "http://rs.tdwg.org/dwc/terms/continent"),
        ("country", "http://rs.tdwg.org/dwc/terms/country"),
        ("countryCode", "http://rs.tdwg.org/dwc/terms/countryCode"),
        ("stateProvince", "http://rs.tdwg.org/dwc/terms/stateProvince"),
        ("locality", "http://rs.tdwg.org/dwc/terms/locality"),
        ("decimalLatitude", "http://rs.tdwg.org/dwc/terms/decimalLatitude"),
        ("decimalLongitude", "http://rs.tdwg.org/dwc/terms/decimalLongitude"),
        ("accessURI", "http://rs.tdwg.org/ac/terms/accessURI"),
        ("format", "http://purl.org/dc/terms/format"),
        ("extent", "http://purl.org/dc/terms/extent"),
        ("pixelXDimension", "http://rs.tdwg.org/ac/terms/pixelXDimension"),
        ("pixelYDimension", "http://rs.tdwg.org/ac/terms/pixelYDimension"),
        ("created", "http://purl.org/dc/terms/created"),
        ("dateTimeOriginal", "http://rs.tdwg.org/ac/terms/dateTimeOriginal"),
        ("temporalCoverage", "http://purl.org/dc/terms/temporal"),
    ];

    /// Returns the CSV headers for audiovisual records
    pub fn csv_headers() -> Vec<&'static str> {
        Self::WRITE_FIELDS.iter().map(|(name, _)| *name).collect()
    }

    /// Converts the audiovisual record to a CSV record
    pub fn to_csv_record(&self) -> Vec<String> {
        vec![
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

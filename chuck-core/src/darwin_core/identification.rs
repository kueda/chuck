// Darwin Core Identification History extension
// https://rs.gbif.org/extension/dwc/identification_history_2025-07-10.xml

use serde::Serialize;

/// DarwinCore Identification record for the Identification History extension
#[derive(Debug, Serialize)]
pub struct Identification {
    #[serde(rename = "coreid")]
    pub coreid: Option<String>,
    #[serde(rename = "occurrenceID")]
    pub occurrence_id: String,
    #[serde(rename = "identificationID")]
    pub identification_id: Option<String>,
    #[serde(rename = "identifiedBy")]
    pub identified_by: Option<String>,
    #[serde(rename = "identifiedByID")]
    pub identified_by_id: Option<String>,
    #[serde(rename = "dateIdentified")]
    pub date_identified: Option<String>,
    #[serde(rename = "identificationRemarks")]
    pub identification_remarks: Option<String>,
    #[serde(rename = "taxonID")]
    pub taxon_id: Option<String>,
    #[serde(rename = "scientificName")]
    pub scientific_name: Option<String>,
    #[serde(rename = "taxonRank")]
    pub taxon_rank: Option<String>,
    #[serde(rename = "vernacularName")]
    pub vernacular_name: Option<String>,
    #[serde(rename = "taxonomicStatus")]
    pub taxonomic_status: Option<String>,
    #[serde(rename = "higherClassification")]
    pub higher_classification: Option<String>,
    #[serde(rename = "kingdom")]
    pub kingdom: Option<String>,
    #[serde(rename = "phylum")]
    pub phylum: Option<String>,
    #[serde(rename = "class")]
    pub class: Option<String>,
    #[serde(rename = "order")]
    pub order: Option<String>,
    #[serde(rename = "superfamily")]
    pub superfamily: Option<String>,
    #[serde(rename = "family")]
    pub family: Option<String>,
    #[serde(rename = "subfamily")]
    pub subfamily: Option<String>,
    #[serde(rename = "tribe")]
    pub tribe: Option<String>,
    #[serde(rename = "subtribe")]
    pub subtribe: Option<String>,
    #[serde(rename = "genus")]
    pub genus: Option<String>,
    #[serde(rename = "subgenus")]
    pub subgenus: Option<String>,
    #[serde(rename = "infragenericEpithet")]
    pub infrageneric_epithet: Option<String>,
    #[serde(rename = "specificEpithet")]
    pub specific_epithet: Option<String>,
    #[serde(rename = "infraspecificEpithet")]
    pub infraspecific_epithet: Option<String>,
    #[serde(rename = "identificationVerificationStatus")]
    pub identification_verification_status: Option<String>,
    #[serde(rename = "identificationCurrent")]
    pub identification_current: Option<bool>,
}

impl Identification {
    /// Row type URI for the Identification extension
    pub const ROW_TYPE: &'static str =
        "http://rs.tdwg.org/dwc/terms/Identification";

    /// CSV filename for the identification extension
    pub const FILENAME: &'static str = "identification.csv";

    /// Fields written to CSV when exporting, paired with their term URIs
    pub const WRITE_FIELDS: &'static [(&'static str, &'static str)] = &[
        ("occurrenceID", "http://rs.tdwg.org/dwc/terms/occurrenceID"),
        ("identificationID", "http://rs.tdwg.org/dwc/terms/identificationID"),
        ("identifiedBy", "http://rs.tdwg.org/dwc/terms/identifiedBy"),
        ("identifiedByID", "http://rs.tdwg.org/dwc/terms/identifiedByID"),
        ("dateIdentified", "http://rs.tdwg.org/dwc/terms/dateIdentified"),
        (
            "identificationRemarks",
            "http://rs.tdwg.org/dwc/terms/identificationRemarks",
        ),
        ("taxonID", "http://rs.tdwg.org/dwc/terms/taxonID"),
        ("scientificName", "http://rs.tdwg.org/dwc/terms/scientificName"),
        ("taxonRank", "http://rs.tdwg.org/dwc/terms/taxonRank"),
        ("vernacularName", "http://rs.tdwg.org/dwc/terms/vernacularName"),
        ("taxonomicStatus", "http://rs.tdwg.org/dwc/terms/taxonomicStatus"),
        ("higherClassification", "http://rs.tdwg.org/dwc/terms/higherClassification"),
        ("kingdom", "http://rs.tdwg.org/dwc/terms/kingdom"),
        ("phylum", "http://rs.tdwg.org/dwc/terms/phylum"),
        ("class", "http://rs.tdwg.org/dwc/terms/class"),
        ("order", "http://rs.tdwg.org/dwc/terms/order"),
        ("superfamily", "http://rs.tdwg.org/dwc/terms/superfamily"),
        ("family", "http://rs.tdwg.org/dwc/terms/family"),
        ("subfamily", "http://rs.tdwg.org/dwc/terms/subfamily"),
        ("tribe", "http://rs.tdwg.org/dwc/terms/tribe"),
        ("subtribe", "http://rs.tdwg.org/dwc/terms/subtribe"),
        ("genus", "http://rs.tdwg.org/dwc/terms/genus"),
        ("subgenus", "http://rs.tdwg.org/dwc/terms/subgenus"),
        ("infragenericEpithet", "http://rs.tdwg.org/dwc/terms/infragenericEpithet"),
        ("specificEpithet", "http://rs.tdwg.org/dwc/terms/specificEpithet"),
        ("infraspecificEpithet", "http://rs.tdwg.org/dwc/terms/infraspecificEpithet"),
        (
            "identificationVerificationStatus",
            "http://rs.tdwg.org/dwc/terms/identificationVerificationStatus",
        ),
        (
            "identificationCurrent",
            "https://www.inaturalist.org/terms/identificationCurrent",
        ),
    ];

    /// Get the CSV header row for identification records
    pub fn csv_headers() -> Vec<&'static str> {
        Self::WRITE_FIELDS.iter().map(|(name, _)| *name).collect()
    }

    /// Convert to CSV record for writing
    pub fn to_csv_record(&self) -> Vec<String> {
        vec![
            self.occurrence_id.clone(),
            self.identification_id.clone().unwrap_or_default(),
            self.identified_by.clone().unwrap_or_default(),
            self.identified_by_id.clone().unwrap_or_default(),
            self.date_identified.clone().unwrap_or_default(),
            self.identification_remarks.clone().unwrap_or_default(),
            self.taxon_id.clone().unwrap_or_default(),
            self.scientific_name.clone().unwrap_or_default(),
            self.taxon_rank.clone().unwrap_or_default(),
            self.vernacular_name.clone().unwrap_or_default(),
            self.taxonomic_status.clone().unwrap_or_default(),
            self.higher_classification.clone().unwrap_or_default(),
            self.kingdom.clone().unwrap_or_default(),
            self.phylum.clone().unwrap_or_default(),
            self.class.clone().unwrap_or_default(),
            self.order.clone().unwrap_or_default(),
            self.superfamily.clone().unwrap_or_default(),
            self.family.clone().unwrap_or_default(),
            self.subfamily.clone().unwrap_or_default(),
            self.tribe.clone().unwrap_or_default(),
            self.subtribe.clone().unwrap_or_default(),
            self.genus.clone().unwrap_or_default(),
            self.subgenus.clone().unwrap_or_default(),
            self.infrageneric_epithet.clone().unwrap_or_default(),
            self.specific_epithet.clone().unwrap_or_default(),
            self.infraspecific_epithet.clone().unwrap_or_default(),
            self.identification_verification_status.clone().unwrap_or_default(),
            self.identification_current.map(|b| b.to_string()).unwrap_or_default(),
        ]
    }
}

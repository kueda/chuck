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
    /// Get the CSV header row for identification records
    pub fn csv_headers() -> Vec<&'static str> {
        vec![
            "coreid",
            "occurrenceID",
            "identificationID",
            "identifiedBy",
            "identifiedByID",
            "dateIdentified",
            "identificationRemarks",
            "taxonID",
            "scientificName",
            "taxonRank",
            "vernacularName",
            "taxonomicStatus",
            "higherClassification",
            "kingdom",
            "phylum",
            "class",
            "order",
            "superfamily",
            "family",
            "subfamily",
            "tribe",
            "subtribe",
            "genus",
            "subgenus",
            "infragenericEpithet",
            "specificEpithet",
            "infraspecificEpithet",
            "identificationVerificationStatus",
            "identificationCurrent",
        ]
    }

    /// Convert to CSV record for writing
    pub fn to_csv_record(&self) -> Vec<String> {
        vec![
            self.coreid.clone().unwrap_or_default(),
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

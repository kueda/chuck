use serde::Serialize;

/// Represents a DarwinCore Occurrence record
/// Based on the DarwinCore Occurrence standard: https://dwc.tdwg.org/terms/#occurrence
#[derive(Debug, Serialize)]
pub struct Occurrence {
    /// An identifier for the Occurrence
    #[serde(rename = "occurrenceID")]
    pub occurrence_id: String,

    /// The specific nature of the data record
    #[serde(rename = "basisOfRecord")]
    pub basis_of_record: String,

    /// A person, group, or organization responsible for recording the original Occurrence
    #[serde(rename = "recordedBy")]
    pub recorded_by: String,

    /// The date-time or interval during which an Event occurred
    #[serde(rename = "eventDate")]
    pub event_date: Option<String>,

    /// The geographic latitude of the geographic center of a Location
    #[serde(rename = "decimalLatitude")]
    pub decimal_latitude: Option<f64>,

    /// The geographic longitude of the geographic center of a Location
    #[serde(rename = "decimalLongitude")]
    pub decimal_longitude: Option<f64>,

    /// The full scientific name, with authorship and date information if known
    #[serde(rename = "scientificName")]
    pub scientific_name: Option<String>,

    /// The taxonomic rank of the most specific name in the scientificName
    #[serde(rename = "taxonRank")]
    pub taxon_rank: Option<String>,

    /// The status of the use of the scientificName as a label for a taxon
    #[serde(rename = "taxonomicStatus")]
    pub taxonomic_status: Option<String>,

    /// A common or vernacular name
    #[serde(rename = "vernacularName")]
    pub vernacular_name: Option<String>,

    /// The name of the kingdom in which the taxon is classified
    pub kingdom: Option<String>,

    /// The name of the phylum (or division) in which the taxon is classified
    pub phylum: Option<String>,

    /// The name of the class in which the taxon is classified
    pub class: Option<String>,

    /// The name of the order in which the taxon is classified
    pub order: Option<String>,

    /// The name of the family in which the taxon is classified
    pub family: Option<String>,

    /// The name of the genus in which the taxon is classified
    pub genus: Option<String>,

    /// The name of the first or species epithet of the scientificName
    #[serde(rename = "specificEpithet")]
    pub specific_epithet: Option<String>,

    /// The name of the lowest or terminal infraspecific epithet of the scientificName
    #[serde(rename = "infraspecificEpithet")]
    pub infraspecific_epithet: Option<String>,

    /// iNaturalist taxon ID
    #[serde(rename = "taxonID")]
    pub taxon_id: Option<i32>,

    /// Comments or notes about the Occurrence
    #[serde(rename = "occurrenceRemarks")]
    pub occurrence_remarks: Option<String>,

    /// A statement about whether the organism or organisms have been introduced to a given place and time
    #[serde(rename = "establishmentMeans")]
    pub establishment_means: Option<String>,

    /// The date on which the Location was first georeferenced
    #[serde(rename = "georeferencedDate")]
    pub georeferenced_date: Option<String>,

    /// A description or reference to the methods used to determine the spatial footprint
    #[serde(rename = "georeferenceProtocol")]
    pub georeference_protocol: Option<String>,

    /// A measure of the applicability of the coordinate reference system to the Location
    #[serde(rename = "coordinateUncertaintyInMeters")]
    pub coordinate_uncertainty_in_meters: Option<f64>,

    /// The horizontal distance (in meters) from the given decimalLatitude and decimalLongitude
    #[serde(rename = "coordinatePrecision")]
    pub coordinate_precision: Option<f64>,

    /// The ellipsoid, geodetic datum, or spatial reference system (SRS) upon which coordinates are based
    #[serde(rename = "geodeticDatum")]
    pub geodetic_datum: Option<String>,

    /// Information about who can access the resource or an indication of its security status
    #[serde(rename = "accessRights")]
    pub access_rights: Option<String>,

    /// A legal document giving official permission to do something with the resource
    pub license: Option<String>,

    /// Additional information that exists, but that has not been shared in the given record
    #[serde(rename = "informationWithheld")]
    pub information_withheld: Option<String>,

    /// The most recent date-time on which the resource was changed
    pub modified: Option<String>,

    /// Whether the organism was captive or cultivated
    pub captive: Option<bool>,

    /// The time or time interval during which an Event occurred
    #[serde(rename = "eventTime")]
    pub event_time: Option<String>,

    /// The verbatim original representation of the date and time information for an Event
    #[serde(rename = "verbatimEventDate")]
    pub verbatim_event_date: Option<String>,

    /// The verbatim original representation of locality
    #[serde(rename = "verbatimLocality")]
    pub verbatim_locality: Option<String>,
}

impl Occurrence {
    /// Get the CSV headers for DarwinCore occurrence records
    pub fn csv_headers() -> Vec<&'static str> {
        vec![
            "occurrenceID",
            "basisOfRecord",
            "recordedBy",
            "eventDate",
            "decimalLatitude",
            "decimalLongitude",
            "scientificName",
            "taxonRank",
            "taxonomicStatus",
            "vernacularName",
            "kingdom",
            "phylum",
            "class",
            "order",
            "family",
            "genus",
            "specificEpithet",
            "infraspecificEpithet",
            "taxonID",
            "occurrenceRemarks",
            "establishmentMeans",
            "georeferencedDate",
            "georeferenceProtocol",
            "coordinateUncertaintyInMeters",
            "coordinatePrecision",
            "geodeticDatum",
            "accessRights",
            "license",
            "informationWithheld",
            "modified",
            "captive",
            "eventTime",
            "verbatimEventDate",
            "verbatimLocality"
        ]
    }

    /// Convert to CSV record values
    pub fn to_csv_record(&self) -> Vec<String> {
        vec![
            self.occurrence_id.clone(),
            self.basis_of_record.clone(),
            self.recorded_by.clone(),
            self.event_date.clone().unwrap_or_default(),
            self.decimal_latitude.map_or(String::new(), |lat| lat.to_string()),
            self.decimal_longitude.map_or(String::new(), |lng| lng.to_string()),
            self.scientific_name.clone().unwrap_or_default(),
            self.taxon_rank.clone().unwrap_or_default(),
            self.taxonomic_status.clone().unwrap_or_default(),
            self.vernacular_name.clone().unwrap_or_default(),
            self.kingdom.clone().unwrap_or_default(),
            self.phylum.clone().unwrap_or_default(),
            self.class.clone().unwrap_or_default(),
            self.order.clone().unwrap_or_default(),
            self.family.clone().unwrap_or_default(),
            self.genus.clone().unwrap_or_default(),
            self.specific_epithet.clone().unwrap_or_default(),
            self.infraspecific_epithet.clone().unwrap_or_default(),
            self.taxon_id.map_or(String::new(), |taxon_id| taxon_id.to_string()),
            self.occurrence_remarks.clone().unwrap_or_default(),
            self.establishment_means.clone().unwrap_or_default(),
            self.georeferenced_date.clone().unwrap_or_default(),
            self.georeference_protocol.clone().unwrap_or_default(),
            self.coordinate_uncertainty_in_meters.map_or(String::new(), |unc| unc.to_string()),
            self.coordinate_precision.map_or(String::new(), |prec| prec.to_string()),
            self.geodetic_datum.clone().unwrap_or_default(),
            self.access_rights.clone().unwrap_or_default(),
            self.license.clone().unwrap_or_default(),
            self.information_withheld.clone().unwrap_or_default(),
            self.modified.clone().unwrap_or_default(),
            self.captive.map_or(String::new(), |captive| captive.to_string()),
            self.event_time.clone().unwrap_or_default(),
            self.verbatim_event_date.clone().unwrap_or_default(),
            self.verbatim_locality.clone().unwrap_or_default(),
        ]
    }
}

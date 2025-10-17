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

    // Geographic Location fields
    /// The name of the continent in which the Location occurs
    pub continent: Option<String>,

    /// The standard code for the country in which the Location occurs
    #[serde(rename = "countryCode")]
    pub country_code: Option<String>,

    /// The full, unabbreviated name of the next smaller administrative region than country
    #[serde(rename = "stateProvince")]
    pub state_province: Option<String>,

    /// The full, unabbreviated name of the next smaller administrative region than stateProvince
    pub county: Option<String>,

    /// The full, unabbreviated name of the next smaller administrative region than county
    pub municipality: Option<String>,

    /// The specific description of the place
    pub locality: Option<String>,

    /// The name of the water body in which the Location occurs
    #[serde(rename = "waterBody")]
    pub water_body: Option<String>,

    /// The name of the island on or near which the Location occurs
    pub island: Option<String>,

    /// The name of the island group in which the Location occurs
    #[serde(rename = "islandGroup")]
    pub island_group: Option<String>,

    // Elevation and Depth
    /// The upper limit of the range of elevation (altitude, meters above sea level)
    pub elevation: Option<f64>,

    /// The vertical accuracy of the elevation
    #[serde(rename = "elevationAccuracy")]
    pub elevation_accuracy: Option<f64>,

    /// The lesser depth of a range of depth below the local surface, in meters
    pub depth: Option<f64>,

    /// The vertical accuracy of the depth measurement
    #[serde(rename = "depthAccuracy")]
    pub depth_accuracy: Option<f64>,

    /// The minimum distance in meters above the surface of the location
    #[serde(rename = "minimumDistanceAboveSurfaceInMeters")]
    pub minimum_distance_above_surface_in_meters: Option<f64>,

    /// The maximum distance in meters above the surface of the location
    #[serde(rename = "maximumDistanceAboveSurfaceInMeters")]
    pub maximum_distance_above_surface_in_meters: Option<f64>,

    /// A description of or reference to the habitat in which the Event occurred
    pub habitat: Option<String>,

    // Georeference Quality fields
    /// Notes or comments about the spatial description determination
    #[serde(rename = "georeferenceRemarks")]
    pub georeference_remarks: Option<String>,

    /// A list of maps, gazetteers, or other resources used to georeference
    #[serde(rename = "georeferenceSources")]
    pub georeference_sources: Option<String>,

    /// A categorical description of the verification status of the georeference
    #[serde(rename = "georeferenceVerificationStatus")]
    pub georeference_verification_status: Option<String>,

    /// The person who determined the georeference
    #[serde(rename = "georeferencedBy")]
    pub georeferenced_by: Option<String>,

    /// The ratio of the area of the point-radius to the area of the footprint
    #[serde(rename = "pointRadiusSpatialFit")]
    pub point_radius_spatial_fit: Option<f64>,

    /// The ratio of the area of the footprint to the area of the true spatial representation
    #[serde(rename = "footprintSpatialFit")]
    pub footprint_spatial_fit: Option<f64>,

    /// A Well-Known Text (WKT) representation of the shape that defines the Location
    #[serde(rename = "footprintWKT")]
    pub footprint_wkt: Option<String>,

    /// The spatial reference system (SRS) for the footprint
    #[serde(rename = "footprintSRS")]
    pub footprint_srs: Option<String>,

    /// The spatial reference system (SRS) for verbatim coordinates
    #[serde(rename = "verbatimSRS")]
    pub verbatim_srs: Option<String>,

    /// The coordinate format for the verbatim latitude and longitude
    #[serde(rename = "verbatimCoordinateSystem")]
    pub verbatim_coordinate_system: Option<String>,

    /// The vertical datum used as the reference upon which the elevation values are based
    #[serde(rename = "verticalDatum")]
    pub vertical_datum: Option<String>,

    /// The original description of the elevation
    #[serde(rename = "verbatimElevation")]
    pub verbatim_elevation: Option<String>,

    /// The original description of the depth
    #[serde(rename = "verbatimDepth")]
    pub verbatim_depth: Option<String>,

    /// The distance in meters from the supplied coordinates to the centroid
    #[serde(rename = "distanceFromCentroidInMeters")]
    pub distance_from_centroid_in_meters: Option<f64>,

    /// A flag (true/false) indicating that the location has coordinates
    #[serde(rename = "hasCoordinate")]
    pub has_coordinate: Option<bool>,

    /// A flag indicating whether there are known issues with the geospatial data
    #[serde(rename = "hasGeospatialIssues")]
    pub has_geospatial_issues: Option<bool>,

    /// A list of geographic names less specific than the information in the locality
    #[serde(rename = "higherGeography")]
    pub higher_geography: Option<String>,

    /// An identifier for the geographic region
    #[serde(rename = "higherGeographyID")]
    pub higher_geography_id: Option<String>,

    /// Information about the source of the Location information
    #[serde(rename = "locationAccordingTo")]
    pub location_according_to: Option<String>,

    /// An identifier for the Location
    #[serde(rename = "locationID")]
    pub location_id: Option<String>,

    /// Comments or notes about the Location
    #[serde(rename = "locationRemarks")]
    pub location_remarks: Option<String>,

    // Temporal fields
    /// The four-digit year in which the Event occurred
    pub year: Option<i32>,

    /// The integer month in which the Event occurred
    pub month: Option<i32>,

    /// The integer day of the month on which the Event occurred
    pub day: Option<i32>,

    /// The earliest integer day of the year on which the Event occurred
    #[serde(rename = "startDayOfYear")]
    pub start_day_of_year: Option<i32>,

    /// The latest integer day of the year on which the Event occurred
    #[serde(rename = "endDayOfYear")]
    pub end_day_of_year: Option<i32>,

    // Event fields
    /// An identifier for the Event
    #[serde(rename = "eventID")]
    pub event_id: Option<String>,

    /// An identifier for the parent Event
    #[serde(rename = "parentEventID")]
    pub parent_event_id: Option<String>,

    /// The nature of the Event
    #[serde(rename = "eventType")]
    pub event_type: Option<String>,

    /// Comments or notes about the Event
    #[serde(rename = "eventRemarks")]
    pub event_remarks: Option<String>,

    /// The amount of effort expended during an Event
    #[serde(rename = "samplingEffort")]
    pub sampling_effort: Option<String>,

    /// The names of, references to, or descriptions of the methods or protocols
    #[serde(rename = "samplingProtocol")]
    pub sampling_protocol: Option<String>,

    /// A numeric value for a measurement of the size of a sample
    #[serde(rename = "sampleSizeValue")]
    pub sample_size_value: Option<f64>,

    /// The unit of measurement of the size of a sample
    #[serde(rename = "sampleSizeUnit")]
    pub sample_size_unit: Option<String>,

    /// Description or reference to the methods used for field observations
    #[serde(rename = "fieldNotes")]
    pub field_notes: Option<String>,

    /// An identifier for the field number
    #[serde(rename = "fieldNumber")]
    pub field_number: Option<String>,

    // Taxonomic fields
    /// The full scientific name of the accepted taxon
    #[serde(rename = "acceptedScientificName")]
    pub accepted_scientific_name: Option<String>,

    /// The full name of the accepted taxon
    #[serde(rename = "acceptedNameUsage")]
    pub accepted_name_usage: Option<String>,

    /// An identifier for the accepted name usage
    #[serde(rename = "acceptedNameUsageID")]
    pub accepted_name_usage_id: Option<String>,

    /// A list of names of higher taxonomic ranks
    #[serde(rename = "higherClassification")]
    pub higher_classification: Option<String>,

    /// The name of the subfamily in which the taxon is classified
    pub subfamily: Option<String>,

    /// The name of the subgenus in which the taxon is classified
    pub subgenus: Option<String>,

    /// The tribe in which the taxon is classified
    pub tribe: Option<String>,

    /// The subtribe in which the taxon is classified
    pub subtribe: Option<String>,

    /// The superfamily in which the taxon is classified
    pub superfamily: Option<String>,

    /// The full scientific name at the rank of species
    pub species: Option<String>,

    /// The genus part of the binomial name
    #[serde(rename = "genericName")]
    pub generic_name: Option<String>,

    /// The infrageneric part of a binomial name at ranks above species
    #[serde(rename = "infragenericEpithet")]
    pub infrageneric_epithet: Option<String>,

    /// A cultivar, hybrid, or strain name
    #[serde(rename = "cultivarEpithet")]
    pub cultivar_epithet: Option<String>,

    /// The full name, with authorship and date information, of the accepted taxon
    #[serde(rename = "parentNameUsage")]
    pub parent_name_usage: Option<String>,

    /// An identifier for the parentNameUsage
    #[serde(rename = "parentNameUsageID")]
    pub parent_name_usage_id: Option<String>,

    /// The original name as first established
    #[serde(rename = "originalNameUsage")]
    pub original_name_usage: Option<String>,

    /// An identifier for the originalNameUsage
    #[serde(rename = "originalNameUsageID")]
    pub original_name_usage_id: Option<String>,

    /// The reference to the source in which the scientific name was published
    #[serde(rename = "namePublishedIn")]
    pub name_published_in: Option<String>,

    /// An identifier for the publication in which the scientificName was published
    #[serde(rename = "namePublishedInID")]
    pub name_published_in_id: Option<String>,

    /// The four-digit year in which the scientificName was published
    #[serde(rename = "namePublishedInYear")]
    pub name_published_in_year: Option<i32>,

    /// The taxonomic code under which the scientificName is constructed
    #[serde(rename = "nomenclaturalCode")]
    pub nomenclatural_code: Option<String>,

    /// The status related to the original publication of the name
    #[serde(rename = "nomenclaturalStatus")]
    pub nomenclatural_status: Option<String>,

    /// The taxon name, with authorship, that the scientificName is currently considered to be
    #[serde(rename = "nameAccordingTo")]
    pub name_according_to: Option<String>,

    /// An identifier for the source in which the specific taxon concept is defined
    #[serde(rename = "nameAccordingToID")]
    pub name_according_to_id: Option<String>,

    /// An identifier for the taxonomic concept
    #[serde(rename = "taxonConceptID")]
    pub taxon_concept_id: Option<String>,

    /// An identifier for the set of taxon information
    #[serde(rename = "scientificNameID")]
    pub scientific_name_id: Option<String>,

    /// Comments or notes about the taxon or name
    #[serde(rename = "taxonRemarks")]
    pub taxon_remarks: Option<String>,

    /// A flag indicating a known problem with the taxonomy
    #[serde(rename = "taxonomicIssue")]
    pub taxonomic_issue: Option<bool>,

    /// A flag indicating a known problem that is not taxonomic
    #[serde(rename = "nonTaxonomicIssue")]
    pub non_taxonomic_issue: Option<bool>,

    /// A list of identifiers of other taxa associated with the Occurrence
    #[serde(rename = "associatedTaxa")]
    pub associated_taxa: Option<String>,

    /// The scientific name that was applied prior to correction
    #[serde(rename = "verbatimIdentification")]
    pub verbatim_identification: Option<String>,

    /// The verbatim original taxon rank
    #[serde(rename = "verbatimTaxonRank")]
    pub verbatim_taxon_rank: Option<String>,

    /// The verbatim original scientific name
    #[serde(rename = "verbatimScientificName")]
    pub verbatim_scientific_name: Option<String>,

    /// The scientific name of the name-bearing type
    #[serde(rename = "typifiedName")]
    pub typified_name: Option<String>,

    // Identification fields
    /// A list of names of people who assigned the taxon to the subject
    #[serde(rename = "identifiedBy")]
    pub identified_by: Option<String>,

    /// A list of identifiers for people who assigned the taxon
    #[serde(rename = "identifiedByID")]
    pub identified_by_id: Option<String>,

    /// The date on which the subject was identified as representing the taxon
    #[serde(rename = "dateIdentified")]
    pub date_identified: Option<String>,

    /// An identifier for the Identification
    #[serde(rename = "identificationID")]
    pub identification_id: Option<String>,

    /// A brief phrase or keyword to express the determiner's doubts about the Identification
    #[serde(rename = "identificationQualifier")]
    pub identification_qualifier: Option<String>,

    /// A list of references used in the Identification
    #[serde(rename = "identificationReferences")]
    pub identification_references: Option<String>,

    /// Comments or notes about the Identification
    #[serde(rename = "identificationRemarks")]
    pub identification_remarks: Option<String>,

    /// A categorical indicator of the verification status of the Identification
    #[serde(rename = "identificationVerificationStatus")]
    pub identification_verification_status: Option<String>,

    /// A list of previous assignments of names to the subject
    #[serde(rename = "previousIdentifications")]
    pub previous_identifications: Option<String>,

    /// A nomenclatural type designation
    #[serde(rename = "typeStatus")]
    pub type_status: Option<String>,

    // Collection/Institution fields
    /// The name of the institution having custody of the object
    #[serde(rename = "institutionCode")]
    pub institution_code: Option<String>,

    /// An identifier for the institution having custody of the object
    #[serde(rename = "institutionID")]
    pub institution_id: Option<String>,

    /// The name of the collection or dataset from which the record was derived
    #[serde(rename = "collectionCode")]
    pub collection_code: Option<String>,

    /// An identifier for the collection or dataset
    #[serde(rename = "collectionID")]
    pub collection_id: Option<String>,

    /// The name of the institution that has ownership of the biological individual
    #[serde(rename = "ownerInstitutionCode")]
    pub owner_institution_code: Option<String>,

    /// An identifier for the occurrence within the collection
    #[serde(rename = "catalogNumber")]
    pub catalog_number: Option<String>,

    /// An identifier given to the occurrence at the time it was recorded
    #[serde(rename = "recordNumber")]
    pub record_number: Option<String>,

    /// A list of additional identifiers for the occurrence
    #[serde(rename = "otherCatalogNumbers")]
    pub other_catalog_numbers: Option<String>,

    /// A list of preparations and preservation methods for a specimen
    pub preparations: Option<String>,

    /// The current state of a specimen with respect to the collection
    pub disposition: Option<String>,

    // Organism fields
    /// An identifier for the Organism instance
    #[serde(rename = "organismID")]
    pub organism_id: Option<String>,

    /// A textual name or label assigned to an Organism instance
    #[serde(rename = "organismName")]
    pub organism_name: Option<String>,

    /// A number or enumeration value for the quantity of organisms
    #[serde(rename = "organismQuantity")]
    pub organism_quantity: Option<f64>,

    /// The type of quantification system used for the organismQuantity
    #[serde(rename = "organismQuantityType")]
    pub organism_quantity_type: Option<String>,

    /// The relative measurement of the quantity of the organism
    #[serde(rename = "relativeOrganismQuantity")]
    pub relative_organism_quantity: Option<f64>,

    /// Comments or notes about the Organism instance
    #[serde(rename = "organismRemarks")]
    pub organism_remarks: Option<String>,

    /// A description of the extent of the Organism instance
    #[serde(rename = "organismScope")]
    pub organism_scope: Option<String>,

    /// A list of identifiers of other organisms associated with the subject organism
    #[serde(rename = "associatedOrganisms")]
    pub associated_organisms: Option<String>,

    /// The number of individuals present at the time of the Occurrence
    #[serde(rename = "individualCount")]
    pub individual_count: Option<i32>,

    /// The age class or life stage of the biological individual
    #[serde(rename = "lifeStage")]
    pub life_stage: Option<String>,

    /// The sex of the biological individual
    pub sex: Option<String>,

    /// The reproductive condition of the biological individual
    #[serde(rename = "reproductiveCondition")]
    pub reproductive_condition: Option<String>,

    /// A description of the behavior shown by the subject at the time of occurrence
    pub behavior: Option<String>,

    /// An indication of whether the organism was in a caste or morph
    pub caste: Option<String>,

    /// An indication of whether the organism is alive or dead
    pub vitality: Option<String>,

    /// A statement about whether the organism has been introduced to a location
    #[serde(rename = "degreeOfEstablishment")]
    pub degree_of_establishment: Option<String>,

    /// The process by which an organism became established in a location
    pub pathway: Option<String>,

    /// A flag indicating whether the organism is invasive in the location
    #[serde(rename = "isInvasive")]
    pub is_invasive: Option<bool>,

    // Material Sample fields
    /// An identifier for the MaterialSample
    #[serde(rename = "materialSampleID")]
    pub material_sample_id: Option<String>,

    /// An identifier for the MaterialEntity
    #[serde(rename = "materialEntityID")]
    pub material_entity_id: Option<String>,

    /// Comments or notes about the MaterialEntity instance
    #[serde(rename = "materialEntityRemarks")]
    pub material_entity_remarks: Option<String>,

    /// A list of identifiers of other occurrences associated with this Occurrence
    #[serde(rename = "associatedOccurrences")]
    pub associated_occurrences: Option<String>,

    /// A list of identifiers of genetic sequence information associated with the Occurrence
    #[serde(rename = "associatedSequences")]
    pub associated_sequences: Option<String>,

    /// A list of identifiers or references to literature associated with the Occurrence
    #[serde(rename = "associatedReferences")]
    pub associated_references: Option<String>,

    /// A flag indicating whether genetic sequences are available
    #[serde(rename = "isSequenced")]
    pub is_sequenced: Option<bool>,

    // Record-level fields
    /// A statement about the occurrence of the Organism
    #[serde(rename = "occurrenceStatus")]
    pub occurrence_status: Option<String>,

    /// A bibliographic reference for the resource as a statement indicating how this record should be cited
    #[serde(rename = "bibliographicCitation")]
    pub bibliographic_citation: Option<String>,

    /// A published reference to the resource
    pub references: Option<String>,

    /// The language of the resource
    pub language: Option<String>,

    /// A legal document granting permission to use the resource
    #[serde(rename = "rightsHolder")]
    pub rights_holder: Option<String>,

    /// Actions taken to make the resource fit for use
    #[serde(rename = "dataGeneralizations")]
    pub data_generalizations: Option<String>,

    /// Additional information as key-value pairs
    #[serde(rename = "dynamicProperties")]
    pub dynamic_properties: Option<String>,

    /// A category or description for the type of record
    #[serde(rename = "type")]
    pub type_field: Option<String>,

    /// An identifier for the set of data
    #[serde(rename = "datasetID")]
    pub dataset_id: Option<String>,

    /// The name identifying the dataset from which the record was derived
    #[serde(rename = "datasetName")]
    pub dataset_name: Option<String>,

    /// A list of flags indicating issues with the record
    pub issue: Option<String>,

    /// A category or type of media associated with the occurrence
    #[serde(rename = "mediaType")]
    pub media_type: Option<String>,

    /// An identifier for the project within which the data was collected
    #[serde(rename = "projectId")]
    pub project_id: Option<String>,

    /// The method or protocol used for data collection or sharing
    pub protocol: Option<String>,

    // Geological Context fields
    /// An identifier for the geological context
    #[serde(rename = "geologicalContextID")]
    pub geological_context_id: Option<String>,

    /// The full name of the lithostratigraphic bed
    pub bed: Option<String>,

    /// The full name of the lithostratigraphic formation
    pub formation: Option<String>,

    /// The full name of the lithostratigraphic group
    pub group: Option<String>,

    /// The full name of the lithostratigraphic member
    pub member: Option<String>,

    /// The combination of all lithostratigraphic names
    #[serde(rename = "lithostratigraphicTerms")]
    pub lithostratigraphic_terms: Option<String>,

    /// The earliest possible geochronologic eon
    #[serde(rename = "earliestEonOrLowestEonothem")]
    pub earliest_eon_or_lowest_eonothem: Option<String>,

    /// The latest possible geochronologic eon
    #[serde(rename = "latestEonOrHighestEonothem")]
    pub latest_eon_or_highest_eonothem: Option<String>,

    /// The earliest possible geochronologic era
    #[serde(rename = "earliestEraOrLowestErathem")]
    pub earliest_era_or_lowest_erathem: Option<String>,

    /// The latest possible geochronologic era
    #[serde(rename = "latestEraOrHighestErathem")]
    pub latest_era_or_highest_erathem: Option<String>,

    /// The earliest possible geochronologic period
    #[serde(rename = "earliestPeriodOrLowestSystem")]
    pub earliest_period_or_lowest_system: Option<String>,

    /// The latest possible geochronologic period
    #[serde(rename = "latestPeriodOrHighestSystem")]
    pub latest_period_or_highest_system: Option<String>,

    /// The earliest possible geochronologic epoch
    #[serde(rename = "earliestEpochOrLowestSeries")]
    pub earliest_epoch_or_lowest_series: Option<String>,

    /// The latest possible geochronologic epoch
    #[serde(rename = "latestEpochOrHighestSeries")]
    pub latest_epoch_or_highest_series: Option<String>,

    /// The earliest possible geochronologic age
    #[serde(rename = "earliestAgeOrLowestStage")]
    pub earliest_age_or_lowest_stage: Option<String>,

    /// The latest possible geochronologic age
    #[serde(rename = "latestAgeOrHighestStage")]
    pub latest_age_or_highest_stage: Option<String>,

    /// The lowest biostratigraphic zone
    #[serde(rename = "lowestBiostratigraphicZone")]
    pub lowest_biostratigraphic_zone: Option<String>,

    /// The highest biostratigraphic zone
    #[serde(rename = "highestBiostratigraphicZone")]
    pub highest_biostratigraphic_zone: Option<String>,

    // GBIF-specific fields
    /// The GBIF identifier for the occurrence record
    #[serde(rename = "gbifID")]
    pub gbif_id: Option<i64>,

    /// The GBIF region
    #[serde(rename = "gbifRegion")]
    pub gbif_region: Option<String>,

    /// The GBIF key (identifier) for the taxon
    #[serde(rename = "taxonKey")]
    pub taxon_key: Option<i64>,

    /// The GBIF key for the accepted taxon
    #[serde(rename = "acceptedTaxonKey")]
    pub accepted_taxon_key: Option<i64>,

    /// The GBIF key for the kingdom
    #[serde(rename = "kingdomKey")]
    pub kingdom_key: Option<i64>,

    /// The GBIF key for the phylum
    #[serde(rename = "phylumKey")]
    pub phylum_key: Option<i64>,

    /// The GBIF key for the class
    #[serde(rename = "classKey")]
    pub class_key: Option<i64>,

    /// The GBIF key for the order
    #[serde(rename = "orderKey")]
    pub order_key: Option<i64>,

    /// The GBIF key for the family
    #[serde(rename = "familyKey")]
    pub family_key: Option<i64>,

    /// The GBIF key for the genus
    #[serde(rename = "genusKey")]
    pub genus_key: Option<i64>,

    /// The GBIF key for the subgenus
    #[serde(rename = "subgenusKey")]
    pub subgenus_key: Option<i64>,

    /// The GBIF key for the species
    #[serde(rename = "speciesKey")]
    pub species_key: Option<i64>,

    /// The GBIF key for the dataset
    #[serde(rename = "datasetKey")]
    pub dataset_key: Option<String>,

    /// The organization that publishes the dataset
    pub publisher: Option<String>,

    /// The country of the organization that publishes the dataset
    #[serde(rename = "publishingCountry")]
    pub publishing_country: Option<String>,

    /// The GBIF region of the publishing organization
    #[serde(rename = "publishedByGbifRegion")]
    pub published_by_gbif_region: Option<String>,

    /// The date the record was last crawled by GBIF
    #[serde(rename = "lastCrawled")]
    pub last_crawled: Option<String>,

    /// The date the record was last parsed by GBIF
    #[serde(rename = "lastParsed")]
    pub last_parsed: Option<String>,

    /// The date the record was last interpreted by GBIF
    #[serde(rename = "lastInterpreted")]
    pub last_interpreted: Option<String>,

    /// The IUCN Red List Category
    #[serde(rename = "iucnRedListCategory")]
    pub iucn_red_list_category: Option<String>,

    /// A flag indicating whether the record was repatriated
    pub repatriated: Option<bool>,

    /// Administrative level 0 geographic identifier
    #[serde(rename = "level0Gid")]
    pub level0_gid: Option<String>,

    /// Administrative level 0 name
    #[serde(rename = "level0Name")]
    pub level0_name: Option<String>,

    /// Administrative level 1 geographic identifier
    #[serde(rename = "level1Gid")]
    pub level1_gid: Option<String>,

    /// Administrative level 1 name
    #[serde(rename = "level1Name")]
    pub level1_name: Option<String>,

    /// Administrative level 2 geographic identifier
    #[serde(rename = "level2Gid")]
    pub level2_gid: Option<String>,

    /// Administrative level 2 name
    #[serde(rename = "level2Name")]
    pub level2_name: Option<String>,

    /// Administrative level 3 geographic identifier
    #[serde(rename = "level3Gid")]
    pub level3_gid: Option<String>,

    /// Administrative level 3 name
    #[serde(rename = "level3Name")]
    pub level3_name: Option<String>,

    /// A list of identifiers for the people, groups, or organizations responsible for recording the occurrence
    #[serde(rename = "recordedByID")]
    pub recorded_by_id: Option<String>,

    /// The original text label for coordinates or location
    #[serde(rename = "verbatimLabel")]
    pub verbatim_label: Option<String>,
}

impl Occurrence {
    /// All supported DarwinCore occurrence fields for reading
    pub const FIELD_NAMES: &'static [&'static str] = &[
        "occurrenceID", "basisOfRecord", "recordedBy", "eventDate",
        "decimalLatitude", "decimalLongitude", "scientificName", "taxonRank",
        "taxonomicStatus", "vernacularName", "kingdom", "phylum", "class",
        "order", "family", "genus", "specificEpithet", "infraspecificEpithet",
        "taxonID", "occurrenceRemarks", "establishmentMeans", "georeferencedDate",
        "georeferenceProtocol", "coordinateUncertaintyInMeters", "coordinatePrecision",
        "geodeticDatum", "accessRights", "license", "informationWithheld", "modified",
        "captive", "eventTime", "verbatimEventDate", "verbatimLocality", "continent",
        "countryCode", "stateProvince", "county", "municipality", "locality",
        "waterBody", "island", "islandGroup", "elevation", "elevationAccuracy",
        "depth", "depthAccuracy", "minimumDistanceAboveSurfaceInMeters",
        "maximumDistanceAboveSurfaceInMeters", "habitat", "georeferenceRemarks",
        "georeferenceSources", "georeferenceVerificationStatus", "georeferencedBy",
        "pointRadiusSpatialFit", "footprintSpatialFit", "footprintWKT", "footprintSRS",
        "verbatimSRS", "verbatimCoordinateSystem", "verticalDatum", "verbatimElevation",
        "verbatimDepth", "distanceFromCentroidInMeters", "hasCoordinate",
        "hasGeospatialIssues", "higherGeography", "higherGeographyID",
        "locationAccordingTo", "locationID", "locationRemarks", "year", "month",
        "day", "startDayOfYear", "endDayOfYear", "eventID", "parentEventID",
        "eventType", "eventRemarks", "samplingEffort", "samplingProtocol",
        "sampleSizeValue", "sampleSizeUnit", "fieldNotes", "fieldNumber",
        "acceptedScientificName", "acceptedNameUsage", "acceptedNameUsageID",
        "higherClassification", "subfamily", "subgenus", "tribe", "subtribe",
        "superfamily", "species", "genericName", "infragenericEpithet",
        "cultivarEpithet", "parentNameUsage", "parentNameUsageID", "originalNameUsage",
        "originalNameUsageID", "namePublishedIn", "namePublishedInID",
        "namePublishedInYear", "nomenclaturalCode", "nomenclaturalStatus",
        "nameAccordingTo", "nameAccordingToID", "taxonConceptID", "scientificNameID",
        "taxonRemarks", "taxonomicIssue", "nonTaxonomicIssue", "associatedTaxa",
        "verbatimIdentification", "verbatimTaxonRank", "verbatimScientificName",
        "typifiedName", "identifiedBy", "identifiedByID", "dateIdentified",
        "identificationID", "identificationQualifier", "identificationReferences",
        "identificationRemarks", "identificationVerificationStatus",
        "previousIdentifications", "typeStatus", "institutionCode", "institutionID",
        "collectionCode", "collectionID", "ownerInstitutionCode", "catalogNumber",
        "recordNumber", "otherCatalogNumbers", "preparations", "disposition",
        "organismID", "organismName", "organismQuantity", "organismQuantityType",
        "relativeOrganismQuantity", "organismRemarks", "organismScope",
        "associatedOrganisms", "individualCount", "lifeStage", "sex",
        "reproductiveCondition", "behavior", "caste", "vitality",
        "degreeOfEstablishment", "pathway", "isInvasive", "materialSampleID",
        "materialEntityID", "materialEntityRemarks", "associatedOccurrences",
        "associatedSequences", "associatedReferences", "isSequenced",
        "occurrenceStatus", "bibliographicCitation", "references", "language",
        "rightsHolder", "dataGeneralizations", "dynamicProperties", "type",
        "datasetID", "datasetName", "issue", "mediaType", "projectId", "protocol",
        "geologicalContextID", "bed", "formation", "group", "member",
        "lithostratigraphicTerms", "earliestEonOrLowestEonothem",
        "latestEonOrHighestEonothem", "earliestEraOrLowestErathem",
        "latestEraOrHighestErathem", "earliestPeriodOrLowestSystem",
        "latestPeriodOrHighestSystem", "earliestEpochOrLowestSeries",
        "latestEpochOrHighestSeries", "earliestAgeOrLowestStage",
        "latestAgeOrHighestStage", "lowestBiostratigraphicZone",
        "highestBiostratigraphicZone", "gbifID", "gbifRegion", "taxonKey",
        "acceptedTaxonKey", "kingdomKey", "phylumKey", "classKey", "orderKey",
        "familyKey", "genusKey", "subgenusKey", "speciesKey", "datasetKey",
        "publisher", "publishingCountry", "publishedByGbifRegion", "lastCrawled",
        "lastParsed", "lastInterpreted", "iucnRedListCategory", "repatriated",
        "level0Gid", "level0Name", "level1Gid", "level1Name", "level2Gid",
        "level2Name", "level3Gid", "level3Name", "recordedByID", "verbatimLabel",
    ];

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

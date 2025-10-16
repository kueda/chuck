use std::path::{Path, PathBuf};
use duckdb::Row;

use crate::error::{ChuckError, Result};

/// Represents a DuckDB database for Darwin Core Archive data
pub struct Database {
    path: PathBuf,
    conn: duckdb::Connection,
}

impl Database {
    /// Creates a new database from core files
    pub fn create_from_core_files(
        core_files: &[PathBuf],
        db_path: &Path,
    ) -> Result<Self> {
        if core_files.is_empty() {
            return Err(ChuckError::NoCoreFiles);
        }

        let conn = duckdb::Connection::open(db_path)?;

        // Try to create table from first file
        let first_csv = core_files[0]
            .to_str()
            .ok_or(ChuckError::PathEncoding)?;

        let create_result = conn.execute(
            &format!(
                "CREATE TABLE occurrences AS SELECT * FROM read_csv_auto('{}')",
                first_csv
            ),
            [],
        );

        // If table already exists, insert from first file instead
        match create_result {
            Ok(_) => {
                // Insert remaining files
                for core_file in &core_files[1..] {
                    let csv_path = core_file
                        .to_str()
                        .ok_or(ChuckError::PathEncoding)?;

                    conn.execute(
                        &format!(
                            "INSERT INTO occurrences SELECT * FROM read_csv_auto('{}')",
                            csv_path
                        ),
                        [],
                    )?;
                }
            },
            Err(e) => {
                let error_msg = e.to_string();
                if error_msg.contains("already exists") || error_msg.contains("Table with name") {
                    // We've previously created this db file, nothing to do
                } else {
                    return Err(e.into());
                }
            }
        }

        Ok(Self {
            path: db_path.to_path_buf(),
            conn,
        })
    }

    /// Opens an existing database
    pub fn open(db_path: &Path) -> Result<Self> {
        let conn = duckdb::Connection::open(db_path)?;
        Ok(Self {
            path: db_path.to_path_buf(),
            conn,
        })
    }

    /// Counts the number of observations in the database
    pub fn count_records(&self) -> Result<usize> {
        let count: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM occurrences",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Helper to get column index from a row given a name
    fn col_index(row: &Row, name: &str) -> std::result::Result<usize, duckdb::Error> {
        row.as_ref().column_index(name)
    }

    /// Helper to get a required string value, converting from i64 if necessary
    fn col_string(row: &Row, name: &str) -> std::result::Result<String, duckdb::Error> {
        let idx = Self::col_index(&row, name)?;
        // Try as string first
        if let Ok(s) = row.get::<_, Option<String>>(idx) {
            Ok(s.unwrap_or_default())
        } else if let Ok(i) = row.get::<_, Option<i64>>(idx) {
            // Fall back to converting i64 to string
            Ok(i.map(|v| v.to_string()).unwrap_or_default())
        } else {
            Ok(String::new())
        }
    }

    /// Gets an optional string value, returning None if the column doesn't exist.
    /// Used for optional Darwin Core fields that may not be present in all archives.
    fn col_opt_string(row: &Row, name: &str) -> Option<String> {
        match row.as_ref().column_index(name) {
            Ok(idx) => {
                // Check if this is a Date32 column
                let col_type = row.as_ref().column_type(idx);
                if col_type.to_string() == "Date32" {
                    if let Ok(days) = row.get::<_, Option<i32>>(idx) {
                        return days.map(|d| {
                            let epoch = chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
                            let date = epoch + chrono::Duration::days(d as i64);
                            date.format("%Y-%m-%d").to_string()
                        });
                    }
                }

                // Try different types that DuckDB might infer
                if let Ok(s) = row.get::<_, Option<String>>(idx) {
                    s
                } else if let Ok(i) = row.get::<_, Option<i64>>(idx) {
                    i.map(|v| v.to_string())
                } else if let Ok(d) = row.get::<_, Option<f64>>(idx) {
                    d.map(|v| v.to_string())
                } else {
                    None
                }
            }
            Err(_) => None, // Column doesn't exist, return None
        }
    }

    /// Gets an optional f64 value, returning None if the column doesn't exist
    fn col_opt_f64(row: &Row, name: &str) -> Option<f64> {
        row.as_ref().column_index(name)
            .ok()
            .and_then(|idx| row.get::<_, Option<f64>>(idx).ok())
            .flatten()
    }

    /// Gets an optional i32 value, returning None if the column doesn't exist
    fn col_opt_i32(row: &Row, name: &str) -> Option<i32> {
        row.as_ref().column_index(name)
            .ok()
            .and_then(|idx| row.get::<_, Option<i32>>(idx).ok())
            .flatten()
    }

    /// Gets an optional bool value, returning None if the column doesn't exist
    fn col_opt_bool(row: &Row, name: &str) -> Option<bool> {
        row.as_ref().column_index(name)
            .ok()
            .and_then(|idx| row.get::<_, Option<bool>>(idx).ok())
            .flatten()
    }

    /// Gets an optional i64 value, returning None if the column doesn't exist
    fn col_opt_i64(row: &Row, name: &str) -> Option<i64> {
        row.as_ref().column_index(name)
            .ok()
            .and_then(|idx| row.get::<_, Option<i64>>(idx).ok())
            .flatten()
    }

    /// Searches for occurrences, returning up to the specified limit starting at offset
    pub fn search(
        &self,
        limit: usize,
        offset: usize,
        search_params: crate::commands::archive::SearchParams,
    ) -> Result<crate::commands::archive::SearchResult> {
        // Build dynamic WHERE clause
        let mut where_clauses = Vec::new();
        let mut count_params: Vec<Box<dyn duckdb::ToSql>> = Vec::new();

        if let Some(ref name) = search_params.scientific_name {
            where_clauses.push("scientificName ILIKE ?");
            count_params.push(Box::new(format!("%{}%", name)));
        }

        let where_clause = if where_clauses.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", where_clauses.join(" AND "))
        };

        // Execute COUNT query
        let count_query = format!("SELECT COUNT(*) FROM occurrences{}", where_clause);
        let count_param_refs: Vec<&dyn duckdb::ToSql> = count_params.iter().map(|p| p.as_ref()).collect();
        let total: usize = self.conn.query_row(&count_query, count_param_refs.as_slice(), |row| row.get(0))?;

        // Build SELECT query
        let select_query = format!("SELECT * FROM occurrences{} LIMIT ? OFFSET ?", where_clause);

        // Rebuild params for SELECT query (reuse where params + add limit/offset)
        let mut select_params: Vec<Box<dyn duckdb::ToSql>> = Vec::new();
        if let Some(ref name) = search_params.scientific_name {
            select_params.push(Box::new(format!("%{}%", name)));
        }
        select_params.push(Box::new(limit));
        select_params.push(Box::new(offset));

        let mut stmt = self.conn.prepare(&select_query)?;

        // Convert params to references for query_map
        let select_param_refs: Vec<&dyn duckdb::ToSql> = select_params.iter().map(|p| p.as_ref()).collect();

        let rows = stmt.query_map(select_param_refs.as_slice(), |row| {
            // Map DuckDB row to Occurrence struct
            // Use safe helpers for optional fields to handle archives with varying schemas
            Ok(chuck_core::darwin_core::Occurrence {
                occurrence_id: Self::col_string(&row, "occurrenceID")?,
                basis_of_record: Self::col_string(&row, "basisOfRecord")?,
                recorded_by: Self::col_string(&row, "recordedBy")?,
                event_date: Self::col_opt_string(&row, "eventDate"),
                decimal_latitude: Self::col_opt_f64(&row, "decimalLatitude"),
                decimal_longitude: Self::col_opt_f64(&row, "decimalLongitude"),
                scientific_name: Self::col_opt_string(&row, "scientificName"),
                taxon_rank: Self::col_opt_string(&row, "taxonRank"),
                taxonomic_status: Self::col_opt_string(&row, "taxonomicStatus"),
                vernacular_name: Self::col_opt_string(&row, "vernacularName"),
                kingdom: Self::col_opt_string(&row, "kingdom"),
                phylum: Self::col_opt_string(&row, "phylum"),
                class: Self::col_opt_string(&row, "class"),
                order: Self::col_opt_string(&row, "order"),
                family: Self::col_opt_string(&row, "family"),
                genus: Self::col_opt_string(&row, "genus"),
                specific_epithet: Self::col_opt_string(&row, "specificEpithet"),
                infraspecific_epithet: Self::col_opt_string(&row, "infraspecificEpithet"),
                taxon_id: Self::col_opt_i32(&row, "taxonID"),
                occurrence_remarks: Self::col_opt_string(&row, "occurrenceRemarks"),
                establishment_means: Self::col_opt_string(&row, "establishmentMeans"),
                georeferenced_date: Self::col_opt_string(&row, "georeferencedDate"),
                georeference_protocol: Self::col_opt_string(&row, "georeferenceProtocol"),
                coordinate_uncertainty_in_meters: Self::col_opt_f64(&row, "coordinateUncertaintyInMeters"),
                coordinate_precision: Self::col_opt_f64(&row, "coordinatePrecision"),
                geodetic_datum: Self::col_opt_string(&row, "geodeticDatum"),
                access_rights: Self::col_opt_string(&row, "accessRights"),
                license: Self::col_opt_string(&row, "license"),
                information_withheld: Self::col_opt_string(&row, "informationWithheld"),
                modified: Self::col_opt_string(&row, "modified"),
                captive: Self::col_opt_bool(&row, "captive"),
                event_time: Self::col_opt_string(&row, "eventTime"),
                verbatim_event_date: Self::col_opt_string(&row, "verbatimEventDate"),
                verbatim_locality: Self::col_opt_string(&row, "verbatimLocality"),
                // Geographic Location fields
                continent: Self::col_opt_string(&row, "continent"),
                country_code: Self::col_opt_string(&row, "countryCode"),
                state_province: Self::col_opt_string(&row, "stateProvince"),
                county: Self::col_opt_string(&row, "county"),
                municipality: Self::col_opt_string(&row, "municipality"),
                locality: Self::col_opt_string(&row, "locality"),
                water_body: Self::col_opt_string(&row, "waterBody"),
                island: Self::col_opt_string(&row, "island"),
                island_group: Self::col_opt_string(&row, "islandGroup"),
                // Elevation and Depth
                elevation: Self::col_opt_f64(&row, "elevation"),
                elevation_accuracy: Self::col_opt_f64(&row, "elevationAccuracy"),
                depth: Self::col_opt_f64(&row, "depth"),
                depth_accuracy: Self::col_opt_f64(&row, "depthAccuracy"),
                minimum_distance_above_surface_in_meters: Self::col_opt_f64(&row, "minimumDistanceAboveSurfaceInMeters"),
                maximum_distance_above_surface_in_meters: Self::col_opt_f64(&row, "maximumDistanceAboveSurfaceInMeters"),
                habitat: Self::col_opt_string(&row, "habitat"),
                // Georeference Quality fields
                georeference_remarks: Self::col_opt_string(&row, "georeferenceRemarks"),
                georeference_sources: Self::col_opt_string(&row, "georeferenceSources"),
                georeference_verification_status: Self::col_opt_string(&row, "georeferenceVerificationStatus"),
                georeferenced_by: Self::col_opt_string(&row, "georeferencedBy"),
                point_radius_spatial_fit: Self::col_opt_f64(&row, "pointRadiusSpatialFit"),
                footprint_spatial_fit: Self::col_opt_f64(&row, "footprintSpatialFit"),
                footprint_wkt: Self::col_opt_string(&row, "footprintWKT"),
                footprint_srs: Self::col_opt_string(&row, "footprintSRS"),
                verbatim_srs: Self::col_opt_string(&row, "verbatimSRS"),
                verbatim_coordinate_system: Self::col_opt_string(&row, "verbatimCoordinateSystem"),
                vertical_datum: Self::col_opt_string(&row, "verticalDatum"),
                verbatim_elevation: Self::col_opt_string(&row, "verbatimElevation"),
                verbatim_depth: Self::col_opt_string(&row, "verbatimDepth"),
                distance_from_centroid_in_meters: Self::col_opt_f64(&row, "distanceFromCentroidInMeters"),
                has_coordinate: Self::col_opt_bool(&row, "hasCoordinate"),
                has_geospatial_issues: Self::col_opt_bool(&row, "hasGeospatialIssues"),
                higher_geography: Self::col_opt_string(&row, "higherGeography"),
                higher_geography_id: Self::col_opt_string(&row, "higherGeographyID"),
                location_according_to: Self::col_opt_string(&row, "locationAccordingTo"),
                location_id: Self::col_opt_string(&row, "locationID"),
                location_remarks: Self::col_opt_string(&row, "locationRemarks"),
                // Temporal fields
                year: Self::col_opt_i32(&row, "year"),
                month: Self::col_opt_i32(&row, "month"),
                day: Self::col_opt_i32(&row, "day"),
                start_day_of_year: Self::col_opt_i32(&row, "startDayOfYear"),
                end_day_of_year: Self::col_opt_i32(&row, "endDayOfYear"),
                // Event fields
                event_id: Self::col_opt_string(&row, "eventID"),
                parent_event_id: Self::col_opt_string(&row, "parentEventID"),
                event_type: Self::col_opt_string(&row, "eventType"),
                event_remarks: Self::col_opt_string(&row, "eventRemarks"),
                sampling_effort: Self::col_opt_string(&row, "samplingEffort"),
                sampling_protocol: Self::col_opt_string(&row, "samplingProtocol"),
                sample_size_value: Self::col_opt_f64(&row, "sampleSizeValue"),
                sample_size_unit: Self::col_opt_string(&row, "sampleSizeUnit"),
                field_notes: Self::col_opt_string(&row, "fieldNotes"),
                field_number: Self::col_opt_string(&row, "fieldNumber"),
                // Taxonomic fields
                accepted_scientific_name: Self::col_opt_string(&row, "acceptedScientificName"),
                accepted_name_usage: Self::col_opt_string(&row, "acceptedNameUsage"),
                accepted_name_usage_id: Self::col_opt_string(&row, "acceptedNameUsageID"),
                higher_classification: Self::col_opt_string(&row, "higherClassification"),
                subfamily: Self::col_opt_string(&row, "subfamily"),
                subgenus: Self::col_opt_string(&row, "subgenus"),
                tribe: Self::col_opt_string(&row, "tribe"),
                subtribe: Self::col_opt_string(&row, "subtribe"),
                superfamily: Self::col_opt_string(&row, "superfamily"),
                species: Self::col_opt_string(&row, "species"),
                generic_name: Self::col_opt_string(&row, "genericName"),
                infrageneric_epithet: Self::col_opt_string(&row, "infragenericEpithet"),
                cultivar_epithet: Self::col_opt_string(&row, "cultivarEpithet"),
                parent_name_usage: Self::col_opt_string(&row, "parentNameUsage"),
                parent_name_usage_id: Self::col_opt_string(&row, "parentNameUsageID"),
                original_name_usage: Self::col_opt_string(&row, "originalNameUsage"),
                original_name_usage_id: Self::col_opt_string(&row, "originalNameUsageID"),
                name_published_in: Self::col_opt_string(&row, "namePublishedIn"),
                name_published_in_id: Self::col_opt_string(&row, "namePublishedInID"),
                name_published_in_year: Self::col_opt_i32(&row, "namePublishedInYear"),
                nomenclatural_code: Self::col_opt_string(&row, "nomenclaturalCode"),
                nomenclatural_status: Self::col_opt_string(&row, "nomenclaturalStatus"),
                name_according_to: Self::col_opt_string(&row, "nameAccordingTo"),
                name_according_to_id: Self::col_opt_string(&row, "nameAccordingToID"),
                taxon_concept_id: Self::col_opt_string(&row, "taxonConceptID"),
                scientific_name_id: Self::col_opt_string(&row, "scientificNameID"),
                taxon_remarks: Self::col_opt_string(&row, "taxonRemarks"),
                taxonomic_issue: Self::col_opt_bool(&row, "taxonomicIssue"),
                non_taxonomic_issue: Self::col_opt_bool(&row, "nonTaxonomicIssue"),
                associated_taxa: Self::col_opt_string(&row, "associatedTaxa"),
                verbatim_identification: Self::col_opt_string(&row, "verbatimIdentification"),
                verbatim_taxon_rank: Self::col_opt_string(&row, "verbatimTaxonRank"),
                verbatim_scientific_name: Self::col_opt_string(&row, "verbatimScientificName"),
                typified_name: Self::col_opt_string(&row, "typifiedName"),
                // Identification fields
                identified_by: Self::col_opt_string(&row, "identifiedBy"),
                identified_by_id: Self::col_opt_string(&row, "identifiedByID"),
                date_identified: Self::col_opt_string(&row, "dateIdentified"),
                identification_id: Self::col_opt_string(&row, "identificationID"),
                identification_qualifier: Self::col_opt_string(&row, "identificationQualifier"),
                identification_references: Self::col_opt_string(&row, "identificationReferences"),
                identification_remarks: Self::col_opt_string(&row, "identificationRemarks"),
                identification_verification_status: Self::col_opt_string(&row, "identificationVerificationStatus"),
                previous_identifications: Self::col_opt_string(&row, "previousIdentifications"),
                type_status: Self::col_opt_string(&row, "typeStatus"),
                // Collection/Institution fields
                institution_code: Self::col_opt_string(&row, "institutionCode"),
                institution_id: Self::col_opt_string(&row, "institutionID"),
                collection_code: Self::col_opt_string(&row, "collectionCode"),
                collection_id: Self::col_opt_string(&row, "collectionID"),
                owner_institution_code: Self::col_opt_string(&row, "ownerInstitutionCode"),
                catalog_number: Self::col_opt_string(&row, "catalogNumber"),
                record_number: Self::col_opt_string(&row, "recordNumber"),
                other_catalog_numbers: Self::col_opt_string(&row, "otherCatalogNumbers"),
                preparations: Self::col_opt_string(&row, "preparations"),
                disposition: Self::col_opt_string(&row, "disposition"),
                // Organism fields
                organism_id: Self::col_opt_string(&row, "organismID"),
                organism_name: Self::col_opt_string(&row, "organismName"),
                organism_quantity: Self::col_opt_f64(&row, "organismQuantity"),
                organism_quantity_type: Self::col_opt_string(&row, "organismQuantityType"),
                relative_organism_quantity: Self::col_opt_f64(&row, "relativeOrganismQuantity"),
                organism_remarks: Self::col_opt_string(&row, "organismRemarks"),
                organism_scope: Self::col_opt_string(&row, "organismScope"),
                associated_organisms: Self::col_opt_string(&row, "associatedOrganisms"),
                individual_count: Self::col_opt_i32(&row, "individualCount"),
                life_stage: Self::col_opt_string(&row, "lifeStage"),
                sex: Self::col_opt_string(&row, "sex"),
                reproductive_condition: Self::col_opt_string(&row, "reproductiveCondition"),
                behavior: Self::col_opt_string(&row, "behavior"),
                caste: Self::col_opt_string(&row, "caste"),
                vitality: Self::col_opt_string(&row, "vitality"),
                degree_of_establishment: Self::col_opt_string(&row, "degreeOfEstablishment"),
                pathway: Self::col_opt_string(&row, "pathway"),
                is_invasive: Self::col_opt_bool(&row, "isInvasive"),
                // Material Sample fields
                material_sample_id: Self::col_opt_string(&row, "materialSampleID"),
                material_entity_id: Self::col_opt_string(&row, "materialEntityID"),
                material_entity_remarks: Self::col_opt_string(&row, "materialEntityRemarks"),
                associated_occurrences: Self::col_opt_string(&row, "associatedOccurrences"),
                associated_sequences: Self::col_opt_string(&row, "associatedSequences"),
                associated_references: Self::col_opt_string(&row, "associatedReferences"),
                is_sequenced: Self::col_opt_bool(&row, "isSequenced"),
                // Record-level fields
                occurrence_status: Self::col_opt_string(&row, "occurrenceStatus"),
                bibliographic_citation: Self::col_opt_string(&row, "bibliographicCitation"),
                references: Self::col_opt_string(&row, "references"),
                language: Self::col_opt_string(&row, "language"),
                rights_holder: Self::col_opt_string(&row, "rightsHolder"),
                data_generalizations: Self::col_opt_string(&row, "dataGeneralizations"),
                dynamic_properties: Self::col_opt_string(&row, "dynamicProperties"),
                type_field: Self::col_opt_string(&row, "type"),
                dataset_id: Self::col_opt_string(&row, "datasetID"),
                dataset_name: Self::col_opt_string(&row, "datasetName"),
                issue: Self::col_opt_string(&row, "issue"),
                media_type: Self::col_opt_string(&row, "mediaType"),
                project_id: Self::col_opt_string(&row, "projectId"),
                protocol: Self::col_opt_string(&row, "protocol"),
                // Geological Context fields
                geological_context_id: Self::col_opt_string(&row, "geologicalContextID"),
                bed: Self::col_opt_string(&row, "bed"),
                formation: Self::col_opt_string(&row, "formation"),
                group: Self::col_opt_string(&row, "group"),
                member: Self::col_opt_string(&row, "member"),
                lithostratigraphic_terms: Self::col_opt_string(&row, "lithostratigraphicTerms"),
                earliest_eon_or_lowest_eonothem: Self::col_opt_string(&row, "earliestEonOrLowestEonothem"),
                latest_eon_or_highest_eonothem: Self::col_opt_string(&row, "latestEonOrHighestEonothem"),
                earliest_era_or_lowest_erathem: Self::col_opt_string(&row, "earliestEraOrLowestErathem"),
                latest_era_or_highest_erathem: Self::col_opt_string(&row, "latestEraOrHighestErathem"),
                earliest_period_or_lowest_system: Self::col_opt_string(&row, "earliestPeriodOrLowestSystem"),
                latest_period_or_highest_system: Self::col_opt_string(&row, "latestPeriodOrHighestSystem"),
                earliest_epoch_or_lowest_series: Self::col_opt_string(&row, "earliestEpochOrLowestSeries"),
                latest_epoch_or_highest_series: Self::col_opt_string(&row, "latestEpochOrHighestSeries"),
                earliest_age_or_lowest_stage: Self::col_opt_string(&row, "earliestAgeOrLowestStage"),
                latest_age_or_highest_stage: Self::col_opt_string(&row, "latestAgeOrHighestStage"),
                lowest_biostratigraphic_zone: Self::col_opt_string(&row, "lowestBiostratigraphicZone"),
                highest_biostratigraphic_zone: Self::col_opt_string(&row, "highestBiostratigraphicZone"),
                // GBIF-specific fields
                gbif_id: Self::col_opt_i64(&row, "gbifID"),
                gbif_region: Self::col_opt_string(&row, "gbifRegion"),
                taxon_key: Self::col_opt_i64(&row, "taxonKey"),
                accepted_taxon_key: Self::col_opt_i64(&row, "acceptedTaxonKey"),
                kingdom_key: Self::col_opt_i64(&row, "kingdomKey"),
                phylum_key: Self::col_opt_i64(&row, "phylumKey"),
                class_key: Self::col_opt_i64(&row, "classKey"),
                order_key: Self::col_opt_i64(&row, "orderKey"),
                family_key: Self::col_opt_i64(&row, "familyKey"),
                genus_key: Self::col_opt_i64(&row, "genusKey"),
                subgenus_key: Self::col_opt_i64(&row, "subgenusKey"),
                species_key: Self::col_opt_i64(&row, "speciesKey"),
                dataset_key: Self::col_opt_string(&row, "datasetKey"),
                publisher: Self::col_opt_string(&row, "publisher"),
                publishing_country: Self::col_opt_string(&row, "publishingCountry"),
                published_by_gbif_region: Self::col_opt_string(&row, "publishedByGbifRegion"),
                last_crawled: Self::col_opt_string(&row, "lastCrawled"),
                last_parsed: Self::col_opt_string(&row, "lastParsed"),
                last_interpreted: Self::col_opt_string(&row, "lastInterpreted"),
                iucn_red_list_category: Self::col_opt_string(&row, "iucnRedListCategory"),
                repatriated: Self::col_opt_bool(&row, "repatriated"),
                level0_gid: Self::col_opt_string(&row, "level0Gid"),
                level0_name: Self::col_opt_string(&row, "level0Name"),
                level1_gid: Self::col_opt_string(&row, "level1Gid"),
                level1_name: Self::col_opt_string(&row, "level1Name"),
                level2_gid: Self::col_opt_string(&row, "level2Gid"),
                level2_name: Self::col_opt_string(&row, "level2Name"),
                level3_gid: Self::col_opt_string(&row, "level3Gid"),
                level3_name: Self::col_opt_string(&row, "level3Name"),
                recorded_by_id: Self::col_opt_string(&row, "recordedByID"),
                verbatim_label: Self::col_opt_string(&row, "verbatimLabel"),
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }

        Ok(crate::commands::archive::SearchResult {
            total,
            results,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    struct TestFixture {
        temp_dir: PathBuf,
        csv_paths: Vec<PathBuf>,
        db_path: PathBuf,
    }

    impl TestFixture {
        fn new(test_name: &str, csv_data: Vec<&[u8]>) -> Self {
            let temp_dir = std::env::temp_dir()
                .join(format!("chuck_test_db_{}", test_name));

            // Clean up from any previous test runs
            std::fs::remove_dir_all(&temp_dir).ok();
            std::fs::create_dir_all(&temp_dir).unwrap();

            // Create CSV files
            let mut csv_paths = Vec::new();
            for (i, data) in csv_data.iter().enumerate() {
                let csv_path = temp_dir.join(format!("test{}.csv", i));
                let mut file = std::fs::File::create(&csv_path).unwrap();
                file.write_all(data).unwrap();
                csv_paths.push(csv_path);
            }

            let db_path = temp_dir.join("test.db");

            Self {
                temp_dir,
                csv_paths,
                db_path,
            }
        }
    }

    impl Drop for TestFixture {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.temp_dir).ok();
        }
    }

    #[test]
    fn test_create_with_existing_table() {
        let fixture = TestFixture::new(
            "existing_table",
            vec![b"id,name\n1,Alice\n2,Bob\n"]
        );

        // First call should succeed
        let result1 = Database::create_from_core_files(
            &fixture.csv_paths,
            &fixture.db_path
        );
        assert!(result1.is_ok());
        let db1 = result1.unwrap();
        assert_eq!(db1.count_records().unwrap(), 2);

        // Second call should recognize existing table and not alter it
        let result2 = Database::create_from_core_files(
            &fixture.csv_paths,
            &fixture.db_path
        );

        assert!(result2.is_ok());
        let db2 = result2.unwrap();
        assert_eq!(db2.count_records().unwrap(), 2);

        // Cleanup happens automatically via Drop
    }

    #[test]
    fn test_create_with_multiple_core_files() {
        let fixture = TestFixture::new(
            "multiple_cores",
            vec![
                b"id,name\n1,Collins\n2,Gardiner\n",
                b"3,Lizzy\n4,Jane\n"
            ]
        );

        let result = Database::create_from_core_files(
            &fixture.csv_paths,
            &fixture.db_path
        );
        assert!(result.is_ok());
        let db = result.unwrap();
        assert_eq!(db.count_records().unwrap(), 4);

        // Cleanup happens automatically via Drop
    }

    #[test]
    fn test_search_returns_occurrence_records() {
        // Create a CSV with Darwin Core occurrence fields
        let csv_data = br#"occurrenceID,basisOfRecord,recordedBy,eventDate,decimalLatitude,decimalLongitude,scientificName,taxonRank,taxonomicStatus,vernacularName,kingdom,phylum,class,order,family,genus,specificEpithet,infraspecificEpithet,taxonID,occurrenceRemarks,establishmentMeans,georeferencedDate,georeferenceProtocol,coordinateUncertaintyInMeters,coordinatePrecision,geodeticDatum,accessRights,license,informationWithheld,modified,captive,eventTime,verbatimEventDate,verbatimLocality
123456,HumanObservation,John Doe,2024-01-15,34.0522,-118.2437,Quercus agrifolia,species,accepted,Coast Live Oak,Plantae,Tracheophyta,Magnoliopsida,Fagales,Fagaceae,Quercus,agrifolia,,12345,Observed in park,native,2024-01-15,GPS,10.0,0.0001,WGS84,public,CC-BY,,,false,14:30:00,January 15 2024,Golden Gate Park
789012,HumanObservation,Jane Smith,2024-01-16,37.7749,-122.4194,Pinus radiata,species,accepted,Monterey Pine,Plantae,Tracheophyta,Pinopsida,Pinales,Pinaceae,Pinus,radiata,,67890,Tall specimen,native,2024-01-16,GPS,5.0,0.0001,WGS84,public,CC-BY-NC,,,false,09:15:00,January 16 2024,Presidio
345678,HumanObservation,Bob Jones,2024-01-17,36.7783,-119.4179,Sequoiadendron giganteum,species,accepted,Giant Sequoia,Plantae,Tracheophyta,Pinopsida,Pinales,Cupressaceae,Sequoiadendron,giganteum,,11111,Ancient tree,native,2024-01-17,GPS,20.0,0.0001,WGS84,public,CC0,,,false,11:00:00,January 17 2024,Sequoia National Park
"#;

        let fixture = TestFixture::new(
            "search_occurrences",
            vec![csv_data]
        );

        let db = Database::create_from_core_files(
            &fixture.csv_paths,
            &fixture.db_path
        ).unwrap();

        use crate::commands::archive::SearchParams;

        // Test searching for all records
        let search_result = db.search(10, 0, SearchParams::default()).unwrap();
        assert_eq!(search_result.total, 3);
        assert_eq!(search_result.results.len(), 3);

        // Verify first occurrence fields
        let first = &search_result.results[0];
        assert_eq!(first.occurrence_id, "123456");
        assert_eq!(first.basis_of_record, "HumanObservation");
        assert_eq!(first.recorded_by, "John Doe");
        assert_eq!(first.event_date, Some("2024-01-15".to_string()));
        assert_eq!(first.decimal_latitude, Some(34.0522));
        assert_eq!(first.decimal_longitude, Some(-118.2437));
        assert_eq!(first.scientific_name, Some("Quercus agrifolia".to_string()));
        assert_eq!(first.taxon_rank, Some("species".to_string()));
        assert_eq!(first.kingdom, Some("Plantae".to_string()));
        assert_eq!(first.family, Some("Fagaceae".to_string()));

        // Test limit parameter
        let limited = db.search(2, 0, SearchParams::default()).unwrap();
        assert_eq!(limited.total, 3);
        assert_eq!(limited.results.len(), 2);

        // Test offset parameter
        let offset_result = db.search(2, 1, SearchParams::default()).unwrap();
        assert_eq!(offset_result.total, 3);
        assert_eq!(offset_result.results.len(), 2);
        assert_eq!(offset_result.results[0].occurrence_id, "789012");

        // Test limit larger than available records
        let all = db.search(100, 0, SearchParams::default()).unwrap();
        assert_eq!(all.total, 3);
        assert_eq!(all.results.len(), 3);
    }

    #[test]
    fn test_search_by_scientific_name() {
        use crate::commands::archive::SearchParams;

        // Create test data with various scientific names
        let csv_data = br#"occurrenceID,basisOfRecord,recordedBy,eventDate,decimalLatitude,decimalLongitude,scientificName,taxonRank,taxonomicStatus,vernacularName,kingdom,phylum,class,order,family,genus,specificEpithet,infraspecificEpithet,taxonID,occurrenceRemarks,establishmentMeans,georeferencedDate,georeferenceProtocol,coordinateUncertaintyInMeters,coordinatePrecision,geodeticDatum,accessRights,license,informationWithheld,modified,captive,eventTime,verbatimEventDate,verbatimLocality
1,obs,John,2024-01-01,0,0,Foobar,species,accepted,,,,,,,,,,,,,,,,,,,,,,,,,
2,obs,Jane,2024-01-01,0,0,foo,species,accepted,,,,,,,,,,,,,,,,,,,,,,,,,
3,obs,Bob,2024-01-01,0,0,Foo,species,accepted,,,,,,,,,,,,,,,,,,,,,,,,,
4,obs,Alice,2024-01-01,0,0,Barfoo,species,accepted,,,,,,,,,,,,,,,,,,,,,,,,,
5,obs,Charlie,2024-01-01,0,0,Bar,species,accepted,,,,,,,,,,,,,,,,,,,,,,,,,
"#;

        let fixture = TestFixture::new("search_scientific_name", vec![csv_data]);

        let db = Database::create_from_core_files(&fixture.csv_paths, &fixture.db_path).unwrap();

        // Search for "foo" (case-insensitive partial match)
        let search_result = db.search(10, 0, SearchParams {
            scientific_name: Some("foo".to_string()),
        }).unwrap();

        // Should return 4 results: "Foobar", "foo", "Foo", "Barfoo"
        assert_eq!(search_result.total, 4, "Expected total count of 4");
        assert_eq!(search_result.results.len(), 4, "Expected 4 results containing 'foo'");

        // Verify the names contain "foo" (case-insensitive)
        for result in &search_result.results {
            let name = result.scientific_name.as_ref().unwrap().to_lowercase();
            assert!(
                name.contains("foo"),
                "Expected '{}' to contain 'foo'",
                result.scientific_name.as_ref().unwrap()
            );
        }

        // Should NOT return "Bar"
        for result in &search_result.results {
            assert_ne!(
                result.scientific_name.as_ref().unwrap(),
                "Bar",
                "Should not return 'Bar'"
            );
        }
    }
}


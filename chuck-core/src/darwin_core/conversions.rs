use inaturalist::models::{Observation, ShowTaxon};
use std::collections::HashMap;
use super::{Occurrence, Multimedia, Audiovisual, Identification};

// GBIF-valid life stages
const GBIF_LIFE_STAGES: &[&str] = &[
    "adult", "agamont", "ammocoete", "bipinnaria", "blastomere", "calf", "caterpillar",
    "chick", "eft", "egg", "elver", "embryo", "fawn", "foal", "fry", "gamete",
    "gametophyte", "gamont", "glochidium", "grub", "hatchling", "imago", "infant",
    "juvenile", "kit", "kitten", "larva", "larvae", "leptocephalus", "maggot",
    "nauplius", "nymph", "ovule", "ovum", "planula", "polewig", "pollen", "polliwig",
    "polliwog", "pollywog", "polwig", "protonema", "pup", "pupa", "puppe", "seed",
    "seedling", "sperm", "spore", "sporophyte", "tadpole", "trochophore", "veliger",
    "whelp", "wriggler", "zoea", "zygote",
];

// Valid sex values from iNaturalist controlled terms
const VALID_SEXES: &[&str] = &["female", "male", "cannot be determined"];

/// Extract life stage from annotations, following the same logic as
/// https://github.com/inaturalist/inaturalist/blob/main/lib/darwin_core/occurrence.rb#L490
fn extract_life_stage(obs: &Observation) -> Option<String> {
    extract_annotation_value(obs, "Life Stage", |value_lower| {
        GBIF_LIFE_STAGES.contains(&value_lower)
    })
}

/// Extract sex from annotations, following the same logic as
/// https://github.com/inaturalist/inaturalist/blob/main/lib/darwin_core/occurrence.rb
fn extract_sex(obs: &Observation) -> Option<String> {
    let value = extract_annotation_value(obs, "Sex", |value_lower| {
        VALID_SEXES.contains(&value_lower)
    })?;

    // Map "cannot be determined" to "undetermined" as per Ruby code
    if value == "cannot be determined" {
        Some("undetermined".to_string())
    } else {
        Some(value)
    }
}

/// Extract reproductive condition from annotations
fn extract_reproductive_condition(obs: &Observation) -> Option<String> {
    // No validation list - accept any value from Flowers and Fruits annotation
    extract_annotation_value(obs, "Flowers and Fruits", |_| true)
}

/// Helper function to extract annotation values with a validation function
fn extract_annotation_value<F>(
    obs: &Observation,
    attribute_name: &str,
    validator: F,
) -> Option<String>
where
    F: Fn(&str) -> bool,
{
    let annotations = obs.annotations.as_ref()?;
    let prefix = format!("{attribute_name}=");

    // Find annotations with positive vote scores
    let matching_annotations: Vec<_> = annotations
        .iter()
        .filter(|a| {
            if let Some(ref concat_val) = a.concatenated_attr_val {
                concat_val.starts_with(&prefix) && a.vote_score.unwrap_or(0) >= 0
            } else {
                false
            }
        })
        .collect();

    // Get the annotation with the highest vote score
    let winning_annotation = matching_annotations
        .iter()
        .max_by_key(|a| a.vote_score.unwrap_or(0))?;

    // Extract the value part
    let concat_val = winning_annotation.concatenated_attr_val.as_ref()?;
    let value = concat_val.strip_prefix(&prefix)?;
    let value_lower = value.to_lowercase();

    // Only return if it passes validation
    if validator(&value_lower) {
        Some(value_lower)
    } else {
        None
    }
}

impl From<(&Observation, &HashMap<i32, ShowTaxon>)> for Occurrence {
    fn from((obs, taxa_hash): (&Observation, &HashMap<i32, ShowTaxon>)) -> Self {
        // Extract coordinates if available

        let geojson = obs.private_geojson.as_ref().or(obs.geojson.as_ref());
        let no_coords = (None, None);
        let (decimal_latitude, decimal_longitude) = if let Some(geojson) = geojson {
            if let Some(coordinates) = &geojson.coordinates {
                if coordinates.len() >= 2 {
                    (Some(coordinates[1]), Some(coordinates[0]))
                } else {
                    no_coords
                }
            } else {
                no_coords
            }
        } else {
            no_coords
        };

        // Extract scientific name and rank from taxon
        let (scientific_name, taxon_rank, vernacular_name, kingdom, phylum, class, order,
            superfamily, family, subfamily, tribe, subtribe, genus, subgenus, species) =
            if let Some(taxon) = &obs.taxon {
                // Get taxonomic hierarchy from ancestor_ids using taxa_hash
                let (kingdom, phylum, class, order, superfamily, family, subfamily, tribe,
                    subtribe, genus, subgenus, species) = if let Some(ancestor_ids) = &taxon.ancestor_ids {
                    let mut k = None;
                    let mut p = None;
                    let mut c = None;
                    let mut o = None;
                    let mut sf = None;
                    let mut f = None;
                    let mut sbf = None;
                    let mut t = None;
                    let mut st = None;
                    let mut g = None;
                    let mut sg = None;
                    let mut sp = None;

                    // Look up each ancestor in the taxa hash and extract names by rank
                    for ancestor_id in ancestor_ids {
                        if let Some(ancestor_taxon) = taxa_hash.get(ancestor_id) {
                            if let Some(rank) = &ancestor_taxon.rank {
                                match rank.as_str() {
                                    "kingdom" => k = ancestor_taxon.name.clone(),
                                    "phylum" => p = ancestor_taxon.name.clone(),
                                    "class" => c = ancestor_taxon.name.clone(),
                                    "order" => o = ancestor_taxon.name.clone(),
                                    "superfamily" => sf = ancestor_taxon.name.clone(),
                                    "family" => f = ancestor_taxon.name.clone(),
                                    "subfamily" => sbf = ancestor_taxon.name.clone(),
                                    "tribe" => t = ancestor_taxon.name.clone(),
                                    "subtribe" => st = ancestor_taxon.name.clone(),
                                    "genus" => g = ancestor_taxon.name.clone(),
                                    "subgenus" => sg = ancestor_taxon.name.clone(),
                                    "species" => sp = ancestor_taxon.name.clone(),
                                    _ => {} // Ignore other ranks
                                }
                            }
                        }
                    }

                    (k, p, c, o, sf, f, sbf, t, st, g, sg, sp)
                } else {
                    (None, None, None, None, None, None, None, None, None, None, None, None)
                };

                (
                    taxon.name.clone(),
                    taxon.rank.clone(),
                    taxon.preferred_common_name.clone(),
                    kingdom,
                    phylum,
                    class,
                    order,
                    superfamily,
                    family,
                    subfamily,
                    tribe,
                    subtribe,
                    genus,
                    subgenus,
                    species,
                )
            } else {
                (None, None, None, None, None, None, None, None, None, None, None, None, None, None, None)
            };

        // Determine establishment means based on captive flag
        let establishment_means = if obs.captive.unwrap_or(false) {
            Some("managed".to_string())
        } else {
            Some("native".to_string()) // Default assumption for wild observations
        };

        let pvt_coords_available = obs.private_geojson.as_ref().is_some();

        // The best available accuracy field depends on whether the
        // coordinates are obscured, i.e. even if they are obscured,
        // positional_accuracy gets included, but it doesn't describe the
        // accuracy of the obscured coordinates
        let acc = if pvt_coords_available {
            obs.positional_accuracy
        } else {
            obs.public_positional_accuracy
        };
        let coordinate_uncertainty_in_meters = acc.map(|acc| acc as f64);

        // Extract user login for recordedBy
        let recorded_by = obs.user.as_ref()
            .and_then(|user| user.login.clone())
            .unwrap_or_default();

        // Format event date
        let event_date = obs.observed_on.clone();

        // Extract year, month, day from event_date if available
        let (year, month, day) = if let Some(ref date_str) = event_date {
            // Parse ISO 8601 date format: YYYY-MM-DD
            let parts: Vec<&str> = date_str.split('-').collect();
            if parts.len() == 3 {
                let y = parts[0].parse::<i32>().ok();
                let m = parts[1].parse::<i32>().ok();
                let d = parts[2].parse::<i32>().ok();
                (y, m, d)
            } else {
                (None, None, None)
            }
        } else {
            (None, None, None)
        };

        // Extract occurrence remarks from description
        let occurrence_remarks = obs.description.clone();

        // Extract annotation values
        let life_stage = extract_life_stage(obs);
        let sex = extract_sex(obs);
        let reproductive_condition = extract_reproductive_condition(obs);

        let information_withheld = match obs.geoprivacy.as_deref() {
            Some("private") => {
                if pvt_coords_available {
                    Some("Coordinates hidden by the observer but included \
                        here with the observer's permission".to_string())
                } else {
                    Some("Coordinates hidden by the observer".to_string())
                }
            },
            Some("obscured") => {
                if pvt_coords_available {
                    Some("Coordinates obscured by the observer but included \
                        here with the observer's permission".to_string())
                } else {
                    Some("Coordinates obscured by the observer".to_string())
                }
            }
            None => {
                match obs.taxon_geoprivacy.as_deref() {
                    Some("private") => {
                        if pvt_coords_available {
                            Some("Coordinates hidden due to iNaturalist \
                                taxon geoprivacy but included here with \
                                the observer's permission".to_string())
                        } else {
                            Some("Coordinates hidden due to iNaturalist \
                                taxon geoprivacy".to_string())
                        }
                    },
                    Some("obscured") => {
                        if pvt_coords_available {
                            Some("Coordinates obscured due to iNaturalist \
                                taxon geoprivacy but included here with the \
                                observer's permission".to_string())
                        } else {
                            Some("Coordinates obscured due to iNaturalist \
                                taxon geoprivacy".to_string())
                        }
                    },
                    None => None,
                    _ => None,
                }
            },
            _ => None,
        };

        // Extract license information
        let license = obs.license_code.clone();

        Occurrence {
            id: None,
            occurrence_id: obs.id.map(|id| format!("https://www.inaturalist.org/observations/{id}")).unwrap_or_default(),
            basis_of_record: "HumanObservation".to_string(),
            recorded_by,
            event_date,
            decimal_latitude,
            decimal_longitude,
            scientific_name,
            taxon_rank,
            taxonomic_status: Some("accepted".to_string()), // Default for iNaturalist taxa
            vernacular_name,
            kingdom,
            phylum,
            class,
            order,
            family,
            genus,
            specific_epithet: None, // Would need name parsing
            infraspecific_epithet: None, // Would need name parsing
            taxon_id: obs.taxon.as_ref()
                .and_then(|t|
                    t.id.map(|id| format!("https://www.inaturalist.org/taxa/{id}"))
                ),
            occurrence_remarks,
            establishment_means,
            georeferenced_date: None, // iNaturalist doesn't provide this specifically
            georeference_protocol: None,
            coordinate_uncertainty_in_meters,
            coordinate_precision: None, // Coordinate decimal places provided as-is
            geodetic_datum: Some("WGS84".to_string()),
            access_rights: None, // Not relevant for iNat observations beyond the license
            license,
            information_withheld,
            modified: obs.updated_at.clone(), // Use the observation's updated timestamp
            captive: obs.captive, // Use the observation's captive flag
            event_time: obs.time_observed_at.as_ref().and_then(|datetime| {
                // Extract time portion from ISO 8601 datetime string
                datetime.split('T').nth(1).map(|time_part| {
                    // Remove Z suffix if present and return just the time
                    time_part.trim_end_matches('Z').to_string()
                })
            }),
            verbatim_event_date: obs.observed_on_string.clone(),
            verbatim_locality: obs.private_place_guess.clone().or(obs.place_guess.clone()),
            continent: None,
            country_code: None,
            state_province: None,
            county: None,
            municipality: None,
            locality: None,
            water_body: None,
            island: None,
            island_group: None,
            elevation: None,
            elevation_accuracy: None,
            depth: None,
            depth_accuracy: None,
            minimum_distance_above_surface_in_meters: None,
            maximum_distance_above_surface_in_meters: None,
            habitat: None,
            georeference_remarks: None,
            georeference_sources: None,
            georeference_verification_status: None,
            georeferenced_by: None,
            point_radius_spatial_fit: None,
            footprint_spatial_fit: None,
            footprint_wkt: None,
            footprint_srs: None,
            verbatim_srs: None,
            verbatim_coordinate_system: None,
            vertical_datum: None,
            verbatim_elevation: None,
            verbatim_depth: None,
            distance_from_centroid_in_meters: None,
            has_coordinate: Some(decimal_latitude.is_some() && decimal_longitude.is_some()),
            has_geospatial_issues: None,
            higher_geography: None,
            higher_geography_id: None,
            location_according_to: None,
            location_id: None,
            location_remarks: obs.place_guess.clone(),
            year,
            month,
            day,
            start_day_of_year: None,
            end_day_of_year: None,
            event_id: None,
            parent_event_id: None,
            event_type: Some("Observation".to_string()),
            event_remarks: obs.description.clone(),
            sampling_effort: None,
            sampling_protocol: None,
            sample_size_value: None,
            sample_size_unit: None,
            field_notes: None,
            field_number: None,
            accepted_scientific_name: None,
            accepted_name_usage: None,
            accepted_name_usage_id: None,
            higher_classification: None,
            subfamily,
            subgenus,
            tribe,
            subtribe,
            superfamily,
            species,
            generic_name: None,
            infrageneric_epithet: None,
            cultivar_epithet: None,
            parent_name_usage: None,
            parent_name_usage_id: None,
            original_name_usage: None,
            original_name_usage_id: None,
            name_published_in: None,
            name_published_in_id: None,
            name_published_in_year: None,
            nomenclatural_code: None,
            nomenclatural_status: None,
            name_according_to: None,
            name_according_to_id: None,
            taxon_concept_id: None,
            scientific_name_id: None,
            taxon_remarks: None,
            taxonomic_issue: None,
            non_taxonomic_issue: None,
            associated_taxa: None,
            verbatim_identification: None,
            verbatim_taxon_rank: None,
            verbatim_scientific_name: None,
            typified_name: None,
            identified_by: None,
            identified_by_id: None,
            date_identified: None,
            identification_id: None,
            identification_qualifier: None,
            identification_references: None,
            identification_remarks: None,
            identification_verification_status: None,
            previous_identifications: None,
            type_status: None,
            institution_code: Some(String::from("iNaturalist")),
            institution_id: None,
            collection_code: Some(String::from("Observations")),
            collection_id: None,
            owner_institution_code: None,
            catalog_number: obs.id.map(|id| format!("{id}")),
            record_number: None,
            other_catalog_numbers: None,
            preparations: None,
            disposition: None,
            organism_id: None,
            organism_name: None,
            organism_quantity: None,
            organism_quantity_type: None,
            relative_organism_quantity: None,
            organism_remarks: None,
            organism_scope: None,
            associated_organisms: None,
            individual_count: None,
            life_stage,
            sex,
            reproductive_condition,
            behavior: None,
            caste: None,
            vitality: None,
            degree_of_establishment: None,
            pathway: None,
            is_invasive: None,
            material_sample_id: None,
            material_entity_id: None,
            material_entity_remarks: None,
            associated_occurrences: None,
            associated_sequences: None,
            associated_references: None,
            is_sequenced: None,
            occurrence_status: None,
            bibliographic_citation: None,
            references: None,
            language: None,
            rights_holder: None,
            data_generalizations: None,
            dynamic_properties: None,
            type_field: None,
            dataset_id: None,
            dataset_name: None,
            issue: None,
            media_type: None,
            project_id: None,
            protocol: None,
            geological_context_id: None,
            bed: None,
            formation: None,
            group: None,
            member: None,
            lithostratigraphic_terms: None,
            earliest_eon_or_lowest_eonothem: None,
            latest_eon_or_highest_eonothem: None,
            earliest_era_or_lowest_erathem: None,
            latest_era_or_highest_erathem: None,
            earliest_period_or_lowest_system: None,
            latest_period_or_highest_system: None,
            earliest_epoch_or_lowest_series: None,
            latest_epoch_or_highest_series: None,
            earliest_age_or_lowest_stage: None,
            latest_age_or_highest_stage: None,
            lowest_biostratigraphic_zone: None,
            highest_biostratigraphic_zone: None,
            gbif_id: None,
            gbif_region: None,
            taxon_key: None,
            accepted_taxon_key: None,
            kingdom_key: None,
            phylum_key: None,
            class_key: None,
            order_key: None,
            family_key: None,
            genus_key: None,
            subgenus_key: None,
            species_key: None,
            dataset_key: None,
            publisher: None,
            publishing_country: None,
            published_by_gbif_region: None,
            last_crawled: None,
            last_parsed: None,
            last_interpreted: None,
            iucn_red_list_category: None,
            repatriated: None,
            level0_gid: None,
            level0_name: None,
            level1_gid: None,
            level1_name: None,
            level2_gid: None,
            level2_name: None,
            level3_gid: None,
            level3_name: None,
            recorded_by_id: None,
            verbatim_label: None,
        }
    }
}

// Backward-compatible implementation without taxonomic hierarchy
impl From<&Observation> for Occurrence {
    fn from(obs: &Observation) -> Self {
        let empty_taxa_hash = HashMap::new();
        Self::from((obs, &empty_taxa_hash))
    }
}

// Map iNaturalist photo with context to a DarwinCore multimedia record
impl From<(&inaturalist::models::Photo, &str, Option<&inaturalist::models::User>, &HashMap<i32, String>)> for Multimedia {
    fn from((photo, occurrence_id, user, photo_mapping): (&inaturalist::models::Photo, &str, Option<&inaturalist::models::User>, &HashMap<i32, String>)) -> Self {
        // Use local file path if available, otherwise use HTTP URL
        let identifier = if let Some(id) = photo.id {
            photo_mapping.get(&id).cloned().or_else(|| {
                // Fallback to HTTP URL if not in mapping
                photo.url.as_ref().map(|url| {
                    url.replace("square", "original")
                        .replace("small", "original")
                        .replace("medium", "original")
                        .replace("large", "original")
                })
            })
        } else {
            None
        };

        // Extract creator from user
        let creator = user.and_then(|u| u.login.clone());

        // Use photo license
        let license = photo.license_code.clone();

        // Use user as rights holder
        let rights_holder = user.and_then(|u| u.login.clone());

        Self {
            coreid: None,
            occurrence_id: format!("https://www.inaturalist.org/observations/{occurrence_id}"),
            r#type: Some("StillImage".to_string()),
            format: Some("image/jpeg".to_string()), // iNaturalist photos are typically JPEG
            identifier,
            references: format!(
                "http://www.inaturalist.org/photos/{}",
                photo.id.unwrap_or_default()
            ).into(), // The original photo URL as reference
            title: None, // iNaturalist photos don't typically have titles
            description: None, // Could potentially use observation description
            created: None, // Photos don't have creation date in iNaturalist API
            creator,
            contributor: None,
            publisher: Some("iNaturalist".to_string()),
            audience: None,
            source: None,
            license,
            rights_holder,
            dataset_id: None,
        }
    }
}

// Map iNaturalist photo with observation context to a DarwinCore audiovisual record
impl From<(&inaturalist::models::Photo, &str, &Observation, &HashMap<i32, String>)> for Audiovisual {
    fn from((photo, occurrence_id, observation, photo_mapping): (&inaturalist::models::Photo, &str, &Observation, &HashMap<i32, String>)) -> Self {
        // Use local file path if available, otherwise use HTTP URL
        let access_uri = if let Some(id) = photo.id {
            photo_mapping.get(&id).cloned().or_else(|| {
                // Fallback to HTTP URL if not in mapping
                photo.url.as_ref().map(|url| {
                    url.replace("square", "original")
                        .replace("small", "original")
                        .replace("medium", "original")
                        .replace("large", "original")
                })
            })
        } else {
            None
        };

        // Create identifier URL from photo ID
        let identifier = photo.id.map(|id| format!("https://www.inaturalist.org/photos/{id}"));

        // Extract coordinates if available
        let (decimal_latitude, decimal_longitude) = if let Some(geojson) = &observation.geojson {
            if let Some(coordinates) = &geojson.coordinates {
                if coordinates.len() >= 2 {
                    (Some(coordinates[1]), Some(coordinates[0]))
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        // Extract taxonomic information from observation
        let (scientific_name, common_name) = if let Some(taxon) = &observation.taxon {
            (taxon.name.clone(), taxon.preferred_common_name.clone())
        } else {
            (None, None)
        };

        // Extract user information
        let owner = observation.user.as_ref().and_then(|user| user.login.clone());

        // TODO: Extract pixel dimensions from original_dimensions if the inaturalist crate
        // supported reading them from the API (they're available in the API response)
        let (pixel_x_dimension, pixel_y_dimension) = (None, None);

        Self {
            coreid: None,
            occurrence_id: format!("https://www.inaturalist.org/observations/{occurrence_id}"),
            identifier,
            r#type: Some("StillImage".to_string()),
            title: None,
            modified: None,
            metadata_language_literal: Some("en".to_string()),
            available: Some("online".to_string()),
            rights: photo.license_code.clone(),
            owner,
            usage_terms: photo.license_code.clone(),
            credit: photo.attribution.clone(),
            attribution_link_url: photo.id.map(|id| format!("https://www.inaturalist.org/photos/{id}")),
            source: Some("iNaturalist".to_string()),
            description: None, // iNat photos don't have their own descriptions
            caption: None,
            comments: None,
            scientific_name,
            common_name,
            life_stage: None, // Could potentially be mapped from observation annotations
            part_of_organism: None,
            location_shown: None, // Could be derived from place information
            location_created: None,
            continent: None,
            country: None,
            country_code: None,
            state_province: None,
            locality: None,
            decimal_latitude,
            decimal_longitude,
            access_uri,
            format: Some("image/jpeg".to_string()), // iNaturalist photos are typically JPEG
            extent: None,
            pixel_x_dimension,
            pixel_y_dimension,
            created: None, // Photo creation date not available in iNaturalist API
            date_time_original: None, // Can't be derived from API response
            temporal_coverage: None, // Can't be derived from API response
        }
    }
}

/// Convert iNaturalist identification category to verification status URI
fn ident_category_to_verification_status_uri(category: &inaturalist::models::identification::Category) -> String {
    match category {
        inaturalist::models::identification::Category::Leading => "https://www.inaturalist.org/terminology/leading".to_string(),
        inaturalist::models::identification::Category::Supporting => "https://www.inaturalist.org/terminology/supporting".to_string(),
        inaturalist::models::identification::Category::Maverick => "https://www.inaturalist.org/terminology/maverick".to_string(),
        inaturalist::models::identification::Category::Improving => "https://www.inaturalist.org/terminology/improving".to_string(),
    }
}

// Map iNaturalist identification to DarwinCore identification record
impl From<(&inaturalist::models::Identification, &str, &HashMap<i32, ShowTaxon>)> for Identification {
    fn from((identification, occurrence_id, taxa_hash): (&inaturalist::models::Identification, &str, &HashMap<i32, ShowTaxon>)) -> Self {
        // Combine user login and name for identifiedBy
        let identified_by = if let Some(ref user) = identification.user {
            let parts: Vec<&str> = [
                user.login.as_deref().unwrap_or(""),
                user.name.as_deref().unwrap_or(""),
            ]
            .iter()
            .filter(|s| !s.is_empty())
            .copied()
            .collect();

            Some(parts.join("|"))
        } else {
            None
        };

        // Combine user ORCID and iNat URL for identifiedByID
        let identified_by_id = if let Some(ref user) = identification.user {
            let mut parts = Vec::new();

            if let Some(ref orcid) = user.orcid {
                if !orcid.is_empty() {
                    parts.push(orcid.clone());
                }
            }

            if let Some(user_id) = user.id {
                parts.push(format!("https://www.inaturalist.org/users/{user_id}"));
            }

            Some(parts.join("|"))
        } else {
            None
        };

        // Extract taxonomic hierarchy from ancestor_ids using taxa_hash
        let (
            higher_classification,
            kingdom,
            phylum,
            class,
            order,
            superfamily,
            family,
            subfamily,
            tribe,
            subtribe,
            genus,
            subgenus,
            infrageneric_epithet,
            specific_epithet,
            infraspecific_epithet
        ) =
            if let Some(taxon) = &identification.taxon {
                let mut taxon_ids = vec!(taxon.id.unwrap());
                if let Some(ancestor_ids) = &taxon.ancestor_ids {
                    taxon_ids.extend(ancestor_ids.iter());
                }
                // Collect all ancestor names for higherClassification
                let all_ancestor_names: Vec<String> = taxon_ids
                    .iter()
                    .filter_map(|ancestor_id| {
                        taxa_hash.get(ancestor_id)
                            .and_then(|ancestor_taxon| ancestor_taxon.name.clone())
                    })
                    .collect();

                let higher_classification = if all_ancestor_names.is_empty() {
                    None
                } else {
                    Some(all_ancestor_names.join(" | "))
                };

                // Extract specific ranks for dedicated fields
                let mut k = None;
                let mut p = None;
                let mut c = None;
                let mut o = None;
                let mut sf = None;
                let mut f = None;
                let mut sbf = None;
                let mut t = None;
                let mut st = None;
                let mut g = None;
                let mut sg = None;
                let mut section = None;
                let mut species_name = None;
                let mut infraspecies_name = None;

                for taxon_id in taxon_ids {
                    if let Some(ancestor_taxon) = taxa_hash.get(&taxon_id) {
                        if let Some(rank) = &ancestor_taxon.rank {
                            match rank.as_str() {
                                "kingdom" => k = ancestor_taxon.name.clone(),
                                "phylum" => p = ancestor_taxon.name.clone(),
                                "class" => c = ancestor_taxon.name.clone(),
                                "order" => o = ancestor_taxon.name.clone(),
                                "superfamily" => sf = ancestor_taxon.name.clone(),
                                "family" => f = ancestor_taxon.name.clone(),
                                "subfamily" => sbf = ancestor_taxon.name.clone(),
                                "tribe" => t = ancestor_taxon.name.clone(),
                                "subtribe" => st = ancestor_taxon.name.clone(),
                                "genus" => g = ancestor_taxon.name.clone(),
                                "subgenus" => sg = ancestor_taxon.name.clone(),
                                "section" => section = ancestor_taxon.name.clone(),
                                "species" => species_name = ancestor_taxon.name.clone(),
                                _ => {} // Handle other ranks
                            }
                        }
                        // Check for infraspecies by rank_level
                        if let Some(rank_level) = ancestor_taxon.rank_level {
                            if rank_level == 5.0 {
                                infraspecies_name = ancestor_taxon.name.clone();
                            }
                        }
                    }
                }

                // Extract specific epithet from species name
                let specific_epithet = species_name.as_ref().and_then(|name| {
                    let parts: Vec<&str> = name.split_whitespace().collect();
                    if parts.len() >= 2 {
                        Some(parts[1].to_string())
                    } else {
                        None
                    }
                });

                // Extract infraspecific epithet from infraspecies name
                let infraspecific_epithet = infraspecies_name.as_ref().and_then(|name| {
                    let parts: Vec<&str> = name.split_whitespace().collect();
                    if parts.len() >= 3 {
                        Some(parts[2].to_string())
                    } else {
                        None
                    }
                });

                (higher_classification, k, p, c, o, sf, f, sbf, t, st, g, sg, section, specific_epithet, infraspecific_epithet)
            } else {
                (None, None, None, None, None, None, None, None, None, None, None, None, None, None, None)
            };

        // Determine taxonomic status
        let taxonomic_status = if let Some(ref taxon) = identification.taxon {
            if taxon.is_active.unwrap_or(true) {
                Some("active".to_string())
            } else {
                Some("inactive".to_string())
            }
        } else {
            None
        };

        Self {
            coreid: None,
            occurrence_id: format!("https://www.inaturalist.org/observations/{occurrence_id}"),
            identification_id: identification.id.map(|id| id.to_string()),
            identified_by,
            identified_by_id,
            date_identified: identification.created_at.clone(),
            identification_remarks: identification.body.clone(),
            taxon_id: identification.taxon.as_ref()
                .and_then(|t|
                    t.id.map(|id| format!("https://www.inaturalist.org/taxa/{id}"))
                ),
            scientific_name: identification.taxon.as_ref().and_then(|t| t.name.clone()),
            taxon_rank: identification.taxon.as_ref().and_then(|t| t.rank.clone()),
            vernacular_name: identification.taxon.as_ref().and_then(|t| t.preferred_common_name.clone()),
            taxonomic_status,
            higher_classification,
            kingdom,
            phylum,
            class,
            order,
            superfamily,
            family,
            subfamily,
            tribe,
            subtribe,
            genus,
            subgenus,
            infrageneric_epithet,
            specific_epithet,
            infraspecific_epithet,
            identification_verification_status: identification.category.as_ref().map(ident_category_to_verification_status_uri),
            identification_current: identification.current,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use inaturalist::models::{Observation, ObservationTaxon};

    static LAT: f64 = 1.0;
    static LNG: f64 = 2.0;
    static PRIVATE_LAT: f64 = -1.0;
    static PRIVATE_LNG: f64 = -2.0;
    static ACC: i32 = 10;
    static PUBLIC_ACC: i32 = 100;

    #[test]
    fn test_decimal_latitude_from_private_geojson() {
        let mut obs = Observation::default();
        obs.private_geojson = Some(Box::new(inaturalist::models::PointGeoJson {
            r#type: Some("Point".to_string()),
            coordinates: Some(vec![PRIVATE_LNG, PRIVATE_LAT]), // [longitude, latitude]
        }));
        obs.geojson = Some(Box::new(inaturalist::models::PointGeoJson {
            r#type: Some("Point".to_string()),
            coordinates: Some(vec![LNG, LAT]), // Different coords
        }));

        let occurrence = Occurrence::from(&obs);
        assert_eq!(occurrence.decimal_latitude, Some(PRIVATE_LAT));
        assert_eq!(occurrence.decimal_longitude, Some(PRIVATE_LNG));
    }

    #[test]
    fn test_decimal_latitude_from_public_geojson_when_no_private() {
        let mut obs = Observation::default();
        obs.private_geojson = None;
        obs.geojson = Some(Box::new(inaturalist::models::PointGeoJson {
            r#type: Some("Point".to_string()),
            coordinates: Some(vec![LNG, LAT]),
        }));

        let occurrence = Occurrence::from(&obs);
        assert_eq!(occurrence.decimal_latitude, Some(LAT));
        assert_eq!(occurrence.decimal_longitude, Some(LNG));
    }

    #[test]
    fn test_decimal_latitude_none_when_no_coordinates() {
        let mut obs = Observation::default();
        obs.private_geojson = None;
        obs.geojson = None;

        let occurrence = Occurrence::from(&obs);
        assert_eq!(occurrence.decimal_latitude, None);
        assert_eq!(occurrence.decimal_longitude, None);
    }

    #[test]
    fn test_coordinate_uncertainty_with_private_coords() {
        let mut obs = Observation::default();
        obs.private_geojson = Some(Box::new(inaturalist::models::PointGeoJson {
            r#type: Some("Point".to_string()),
            coordinates: Some(vec![PRIVATE_LNG, PRIVATE_LAT]), // [longitude, latitude]
        }));
        obs.geojson = Some(Box::new(inaturalist::models::PointGeoJson {
            r#type: Some("Point".to_string()),
            coordinates: Some(vec![LNG, LAT]), // Different coords
        }));
        obs.positional_accuracy = Some(ACC);
        obs.public_positional_accuracy = Some(PUBLIC_ACC);

        let occurrence = Occurrence::from(&obs);
        assert_eq!(occurrence.coordinate_uncertainty_in_meters, Some(ACC as f64));
    }

    #[test]
    fn test_coordinate_uncertainty_without_private_coords() {
        let mut obs = Observation::default();
        obs.geojson = Some(Box::new(inaturalist::models::PointGeoJson {
            r#type: Some("Point".to_string()),
            coordinates: Some(vec![LNG, LAT]), // Different coords
        }));
        obs.positional_accuracy = Some(ACC);
        obs.public_positional_accuracy = Some(PUBLIC_ACC);

        let occurrence = Occurrence::from(&obs);
        assert_eq!(occurrence.coordinate_uncertainty_in_meters, Some(PUBLIC_ACC as f64));
    }

    #[test]
    fn test_taxonomic_hierarchy_population() {
        use inaturalist::models::ObservationTaxon;

        let mut obs = Observation::default();
        let mut taxon = ObservationTaxon::default();
        taxon.ancestor_ids = Some(vec![1, 2, 3, 4, 5, 6, 7]);
        obs.taxon = Some(Box::new(taxon));

        // Create taxa_hash with different taxonomic ranks
        let mut taxa_hash = HashMap::new();

        // Kingdom
        let mut kingdom_taxon = ShowTaxon::default();
        kingdom_taxon.id = Some(1);
        kingdom_taxon.name = Some("Animalia".to_string());
        kingdom_taxon.rank = Some("kingdom".to_string());
        taxa_hash.insert(1, kingdom_taxon);

        // Phylum
        let mut phylum_taxon = ShowTaxon::default();
        phylum_taxon.id = Some(2);
        phylum_taxon.name = Some("Chordata".to_string());
        phylum_taxon.rank = Some("phylum".to_string());
        taxa_hash.insert(2, phylum_taxon);

        // Class
        let mut class_taxon = ShowTaxon::default();
        class_taxon.id = Some(3);
        class_taxon.name = Some("Mammalia".to_string());
        class_taxon.rank = Some("class".to_string());
        taxa_hash.insert(3, class_taxon);

        // Order
        let mut order_taxon = ShowTaxon::default();
        order_taxon.id = Some(4);
        order_taxon.name = Some("Carnivora".to_string());
        order_taxon.rank = Some("order".to_string());
        taxa_hash.insert(4, order_taxon);

        // Family
        let mut family_taxon = ShowTaxon::default();
        family_taxon.id = Some(5);
        family_taxon.name = Some("Felidae".to_string());
        family_taxon.rank = Some("family".to_string());
        taxa_hash.insert(5, family_taxon);

        // Genus
        let mut genus_taxon = ShowTaxon::default();
        genus_taxon.id = Some(6);
        genus_taxon.name = Some("Panthera".to_string());
        genus_taxon.rank = Some("genus".to_string());
        taxa_hash.insert(6, genus_taxon);

        // Species (should not be mapped to any field)
        let mut species_taxon = ShowTaxon::default();
        species_taxon.id = Some(7);
        species_taxon.name = Some("Panthera leo".to_string());
        species_taxon.rank = Some("species".to_string());
        taxa_hash.insert(7, species_taxon);

        let occurrence = Occurrence::from((&obs, &taxa_hash));

        assert_eq!(occurrence.kingdom, Some("Animalia".to_string()));
        assert_eq!(occurrence.phylum, Some("Chordata".to_string()));
        assert_eq!(occurrence.class, Some("Mammalia".to_string()));
        assert_eq!(occurrence.order, Some("Carnivora".to_string()));
        assert_eq!(occurrence.family, Some("Felidae".to_string()));
        assert_eq!(occurrence.genus, Some("Panthera".to_string()));
    }

    #[test]
    fn test_taxonomic_hierarchy_with_empty_taxa_hash() {
        use inaturalist::models::ObservationTaxon;

        let mut obs = Observation::default();
        let mut taxon = ObservationTaxon::default();
        taxon.ancestor_ids = Some(vec![1, 2, 3]);
        obs.taxon = Some(Box::new(taxon));

        let taxa_hash = HashMap::new(); // Empty hash

        let occurrence = Occurrence::from((&obs, &taxa_hash));

        assert_eq!(occurrence.kingdom, None);
        assert_eq!(occurrence.phylum, None);
        assert_eq!(occurrence.class, None);
        assert_eq!(occurrence.order, None);
        assert_eq!(occurrence.family, None);
        assert_eq!(occurrence.genus, None);
    }

    #[test]
    fn test_taxonomic_hierarchy_partial_data() {
        use inaturalist::models::ObservationTaxon;

        let mut obs = Observation::default();
        let mut taxon = ObservationTaxon::default();
        taxon.ancestor_ids = Some(vec![1, 2, 3]);
        obs.taxon = Some(Box::new(taxon));

        let mut taxa_hash = HashMap::new();

        // Only kingdom and genus (missing intermediate ranks)
        let mut kingdom_taxon = ShowTaxon::default();
        kingdom_taxon.id = Some(1);
        kingdom_taxon.name = Some("Plantae".to_string());
        kingdom_taxon.rank = Some("kingdom".to_string());
        taxa_hash.insert(1, kingdom_taxon);

        let mut genus_taxon = ShowTaxon::default();
        genus_taxon.id = Some(3);
        genus_taxon.name = Some("Rosa".to_string());
        genus_taxon.rank = Some("genus".to_string());
        taxa_hash.insert(3, genus_taxon);

        let occurrence = Occurrence::from((&obs, &taxa_hash));

        assert_eq!(occurrence.kingdom, Some("Plantae".to_string()));
        assert_eq!(occurrence.phylum, None); // Missing
        assert_eq!(occurrence.class, None);  // Missing
        assert_eq!(occurrence.order, None);  // Missing
        assert_eq!(occurrence.family, None); // Missing
        assert_eq!(occurrence.genus, Some("Rosa".to_string()));
    }

    #[test]
    fn test_year_month_day_extraction_from_event_date() {
        let mut obs = Observation::default();
        obs.observed_on = Some("2020-05-02".to_string());

        let occurrence = Occurrence::from(&obs);

        assert_eq!(occurrence.year, Some(2020));
        assert_eq!(occurrence.month, Some(5));
        assert_eq!(occurrence.day, Some(2));
    }

    #[test]
    fn test_additional_taxonomic_ranks() {
        use inaturalist::models::ObservationTaxon;

        let mut obs = Observation::default();
        let mut taxon = ObservationTaxon::default();
        taxon.ancestor_ids = Some(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
        obs.taxon = Some(Box::new(taxon));

        let mut taxa_hash = HashMap::new();

        // Kingdom
        let mut kingdom_taxon = ShowTaxon::default();
        kingdom_taxon.id = Some(1);
        kingdom_taxon.name = Some("Animalia".to_string());
        kingdom_taxon.rank = Some("kingdom".to_string());
        taxa_hash.insert(1, kingdom_taxon);

        // Phylum
        let mut phylum_taxon = ShowTaxon::default();
        phylum_taxon.id = Some(2);
        phylum_taxon.name = Some("Arthropoda".to_string());
        phylum_taxon.rank = Some("phylum".to_string());
        taxa_hash.insert(2, phylum_taxon);

        // Class
        let mut class_taxon = ShowTaxon::default();
        class_taxon.id = Some(3);
        class_taxon.name = Some("Insecta".to_string());
        class_taxon.rank = Some("class".to_string());
        taxa_hash.insert(3, class_taxon);

        // Order
        let mut order_taxon = ShowTaxon::default();
        order_taxon.id = Some(4);
        order_taxon.name = Some("Lepidoptera".to_string());
        order_taxon.rank = Some("order".to_string());
        taxa_hash.insert(4, order_taxon);

        // Superfamily
        let mut superfamily_taxon = ShowTaxon::default();
        superfamily_taxon.id = Some(5);
        superfamily_taxon.name = Some("Papilionoidea".to_string());
        superfamily_taxon.rank = Some("superfamily".to_string());
        taxa_hash.insert(5, superfamily_taxon);

        // Family
        let mut family_taxon = ShowTaxon::default();
        family_taxon.id = Some(6);
        family_taxon.name = Some("Nymphalidae".to_string());
        family_taxon.rank = Some("family".to_string());
        taxa_hash.insert(6, family_taxon);

        // Subfamily
        let mut subfamily_taxon = ShowTaxon::default();
        subfamily_taxon.id = Some(7);
        subfamily_taxon.name = Some("Danainae".to_string());
        subfamily_taxon.rank = Some("subfamily".to_string());
        taxa_hash.insert(7, subfamily_taxon);

        // Tribe
        let mut tribe_taxon = ShowTaxon::default();
        tribe_taxon.id = Some(8);
        tribe_taxon.name = Some("Danaini".to_string());
        tribe_taxon.rank = Some("tribe".to_string());
        taxa_hash.insert(8, tribe_taxon);

        // Subtribe
        let mut subtribe_taxon = ShowTaxon::default();
        subtribe_taxon.id = Some(9);
        subtribe_taxon.name = Some("Danaina".to_string());
        subtribe_taxon.rank = Some("subtribe".to_string());
        taxa_hash.insert(9, subtribe_taxon);

        // Genus
        let mut genus_taxon = ShowTaxon::default();
        genus_taxon.id = Some(10);
        genus_taxon.name = Some("Danaus".to_string());
        genus_taxon.rank = Some("genus".to_string());
        taxa_hash.insert(10, genus_taxon);

        // Subgenus
        let mut subgenus_taxon = ShowTaxon::default();
        subgenus_taxon.id = Some(11);
        subgenus_taxon.name = Some("Danaus".to_string());
        subgenus_taxon.rank = Some("subgenus".to_string());
        taxa_hash.insert(11, subgenus_taxon);

        // Species
        let mut species_taxon = ShowTaxon::default();
        species_taxon.id = Some(12);
        species_taxon.name = Some("Danaus plexippus".to_string());
        species_taxon.rank = Some("species".to_string());
        taxa_hash.insert(12, species_taxon);

        let occurrence = Occurrence::from((&obs, &taxa_hash));

        assert_eq!(occurrence.kingdom, Some("Animalia".to_string()));
        assert_eq!(occurrence.phylum, Some("Arthropoda".to_string()));
        assert_eq!(occurrence.class, Some("Insecta".to_string()));
        assert_eq!(occurrence.order, Some("Lepidoptera".to_string()));
        assert_eq!(occurrence.superfamily, Some("Papilionoidea".to_string()));
        assert_eq!(occurrence.family, Some("Nymphalidae".to_string()));
        assert_eq!(occurrence.subfamily, Some("Danainae".to_string()));
        assert_eq!(occurrence.tribe, Some("Danaini".to_string()));
        assert_eq!(occurrence.subtribe, Some("Danaina".to_string()));
        assert_eq!(occurrence.genus, Some("Danaus".to_string()));
        assert_eq!(occurrence.subgenus, Some("Danaus".to_string()));
        assert_eq!(occurrence.species, Some("Danaus plexippus".to_string()));
    }

    #[test]
    fn test_life_stage_extraction_from_annotations() {
        use inaturalist::models::Annotation;

        let mut obs = Observation::default();

        // Create Life Stage annotation with "Adult" value
        let mut annotation = Annotation::default();
        annotation.concatenated_attr_val = Some("Life Stage=Adult".to_string());
        annotation.vote_score = Some(2);

        obs.annotations = Some(vec![annotation]);

        let occurrence = Occurrence::from(&obs);

        assert_eq!(occurrence.life_stage, Some("adult".to_string()));
    }

    #[test]
    fn test_life_stage_ignores_invalid_values() {
        use inaturalist::models::Annotation;

        let mut obs = Observation::default();

        // Create Life Stage annotation with invalid value
        let mut annotation = Annotation::default();
        annotation.concatenated_attr_val = Some("Life Stage=InvalidValue".to_string());
        annotation.vote_score = Some(2);

        obs.annotations = Some(vec![annotation]);

        let occurrence = Occurrence::from(&obs);

        assert_eq!(occurrence.life_stage, None);
    }

    #[test]
    fn test_life_stage_selects_highest_vote_score() {
        use inaturalist::models::Annotation;

        let mut obs = Observation::default();

        // Create multiple Life Stage annotations with different vote scores
        let mut annotation1 = Annotation::default();
        annotation1.concatenated_attr_val = Some("Life Stage=Juvenile".to_string());
        annotation1.vote_score = Some(1);

        let mut annotation2 = Annotation::default();
        annotation2.concatenated_attr_val = Some("Life Stage=Adult".to_string());
        annotation2.vote_score = Some(5);

        let mut annotation3 = Annotation::default();
        annotation3.concatenated_attr_val = Some("Life Stage=Larva".to_string());
        annotation3.vote_score = Some(2);

        obs.annotations = Some(vec![annotation1, annotation2, annotation3]);

        let occurrence = Occurrence::from(&obs);

        assert_eq!(occurrence.life_stage, Some("adult".to_string()));
    }

    #[test]
    fn test_life_stage_ignores_negative_vote_scores() {
        use inaturalist::models::Annotation;

        let mut obs = Observation::default();

        // Create Life Stage annotation with negative vote score
        let mut annotation = Annotation::default();
        annotation.concatenated_attr_val = Some("Life Stage=Adult".to_string());
        annotation.vote_score = Some(-1);

        obs.annotations = Some(vec![annotation]);

        let occurrence = Occurrence::from(&obs);

        assert_eq!(occurrence.life_stage, None);
    }

    #[test]
    fn test_sex_extraction_from_annotations() {
        use inaturalist::models::Annotation;

        let mut obs = Observation::default();

        // Create Sex annotation with "Female" value
        let mut annotation = Annotation::default();
        annotation.concatenated_attr_val = Some("Sex=Female".to_string());
        annotation.vote_score = Some(2);

        obs.annotations = Some(vec![annotation]);

        let occurrence = Occurrence::from(&obs);

        assert_eq!(occurrence.sex, Some("female".to_string()));
    }

    #[test]
    fn test_sex_maps_cannot_be_determined_to_undetermined() {
        use inaturalist::models::Annotation;

        let mut obs = Observation::default();

        // Create Sex annotation with "Cannot Be Determined" value
        let mut annotation = Annotation::default();
        annotation.concatenated_attr_val = Some("Sex=Cannot Be Determined".to_string());
        annotation.vote_score = Some(2);

        obs.annotations = Some(vec![annotation]);

        let occurrence = Occurrence::from(&obs);

        assert_eq!(occurrence.sex, Some("undetermined".to_string()));
    }

    #[test]
    fn test_sex_ignores_invalid_values() {
        use inaturalist::models::Annotation;

        let mut obs = Observation::default();

        // Create Sex annotation with invalid value
        let mut annotation = Annotation::default();
        annotation.concatenated_attr_val = Some("Sex=Unknown".to_string());
        annotation.vote_score = Some(2);

        obs.annotations = Some(vec![annotation]);

        let occurrence = Occurrence::from(&obs);

        assert_eq!(occurrence.sex, None);
    }

    #[test]
    fn test_sex_selects_highest_vote_score() {
        use inaturalist::models::Annotation;

        let mut obs = Observation::default();

        // Create multiple Sex annotations with different vote scores
        let mut annotation1 = Annotation::default();
        annotation1.concatenated_attr_val = Some("Sex=Female".to_string());
        annotation1.vote_score = Some(1);

        let mut annotation2 = Annotation::default();
        annotation2.concatenated_attr_val = Some("Sex=Male".to_string());
        annotation2.vote_score = Some(5);

        obs.annotations = Some(vec![annotation1, annotation2]);

        let occurrence = Occurrence::from(&obs);

        assert_eq!(occurrence.sex, Some("male".to_string()));
    }

    #[test]
    fn test_reproductive_condition_extraction_from_annotations() {
        use inaturalist::models::Annotation;

        let mut obs = Observation::default();

        // Create Flowers and Fruits annotation with "Flowering" value
        let mut annotation = Annotation::default();
        annotation.concatenated_attr_val = Some("Flowers and Fruits=Flowering".to_string());
        annotation.vote_score = Some(2);

        obs.annotations = Some(vec![annotation]);

        let occurrence = Occurrence::from(&obs);

        assert_eq!(occurrence.reproductive_condition, Some("flowering".to_string()));
    }

    #[test]
    fn test_reproductive_condition_accepts_any_value() {
        use inaturalist::models::Annotation;

        let mut obs = Observation::default();

        // Create Flowers and Fruits annotation with any value
        let mut annotation = Annotation::default();
        annotation.concatenated_attr_val = Some("Flowers and Fruits=Fruiting".to_string());
        annotation.vote_score = Some(2);

        obs.annotations = Some(vec![annotation]);

        let occurrence = Occurrence::from(&obs);

        assert_eq!(occurrence.reproductive_condition, Some("fruiting".to_string()));
    }

    #[test]
    fn test_reproductive_condition_selects_highest_vote_score() {
        use inaturalist::models::Annotation;

        let mut obs = Observation::default();

        // Create multiple Flowers and Fruits annotations with different vote scores
        let mut annotation1 = Annotation::default();
        annotation1.concatenated_attr_val = Some("Flowers and Fruits=Flowering".to_string());
        annotation1.vote_score = Some(1);

        let mut annotation2 = Annotation::default();
        annotation2.concatenated_attr_val = Some("Flowers and Fruits=Fruiting".to_string());
        annotation2.vote_score = Some(5);

        obs.annotations = Some(vec![annotation1, annotation2]);

        let occurrence = Occurrence::from(&obs);

        assert_eq!(occurrence.reproductive_condition, Some("fruiting".to_string()));
    }

    #[test]
    fn test_occ_occurrence_id_to_uri() {
        let mut obs = Observation::default();
        obs.id = Some(123);
        let occ = Occurrence::from(&obs);
        assert!(obs.id.is_some());
        assert_eq!(
            occ.occurrence_id,
            format!("https://www.inaturalist.org/observations/{}", obs.id.unwrap())
        );
    }

    #[test]
    fn test_occ_taxon_id_to_uri() {
        let mut obs = Observation::default();
        let mut taxon = ObservationTaxon::default();
        let taxon_id = 456;
        taxon.id = Some(taxon_id);
        obs.taxon = Some(Box::new(taxon));
        let occ = Occurrence::from(&obs);
        assert!(obs.taxon.is_some());
        assert!(obs.taxon.unwrap().id.is_some());
        assert_eq!(
            occ.taxon_id,
            Some(format!("https://www.inaturalist.org/taxa/{taxon_id}"))
        );
    }
}

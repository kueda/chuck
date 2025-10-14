use inaturalist::models::{Observation, ShowTaxon};
use std::collections::HashMap;
use super::{Occurrence, Multimedia, Audiovisual, Identification};

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
        let (scientific_name, taxon_rank, vernacular_name, kingdom, phylum, class, order, family, genus) =
            if let Some(taxon) = &obs.taxon {
                // Get taxonomic hierarchy from ancestor_ids using taxa_hash
                let (kingdom, phylum, class, order, family, genus) = if let Some(ancestor_ids) = &taxon.ancestor_ids {
                    let mut k = None;
                    let mut p = None;
                    let mut c = None;
                    let mut o = None;
                    let mut f = None;
                    let mut g = None;

                    // Look up each ancestor in the taxa hash and extract names by rank
                    for ancestor_id in ancestor_ids {
                        if let Some(ancestor_taxon) = taxa_hash.get(ancestor_id) {
                            if let Some(rank) = &ancestor_taxon.rank {
                                match rank.as_str() {
                                    "kingdom" => k = ancestor_taxon.name.clone(),
                                    "phylum" => p = ancestor_taxon.name.clone(),
                                    "class" => c = ancestor_taxon.name.clone(),
                                    "order" => o = ancestor_taxon.name.clone(),
                                    "family" => f = ancestor_taxon.name.clone(),
                                    "genus" => g = ancestor_taxon.name.clone(),
                                    _ => {} // Ignore other ranks
                                }
                            }
                        }
                    }

                    (k, p, c, o, f, g)
                } else {
                    (None, None, None, None, None, None)
                };

                (
                    taxon.name.clone(),
                    taxon.rank.clone(),
                    taxon.preferred_common_name.clone(),
                    kingdom,
                    phylum,
                    class,
                    order,
                    family,
                    genus,
                )
            } else {
                (None, None, None, None, None, None, None, None, None)
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
        let coordinate_uncertainty_in_meters = acc.map_or(None, |acc| Some(acc as f64));

        // Extract user login for recordedBy
        let recorded_by = obs.user.as_ref()
            .and_then(|user| user.login.clone())
            .unwrap_or_default();

        // Format event date
        let event_date = obs.observed_on.clone();

        // Extract occurrence remarks from description
        let occurrence_remarks = obs.description.clone();

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
            occurrence_id: obs.id.map(|id| format!("{}", id)).unwrap_or_default(),
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
            taxon_id: obs.taxon.as_ref().and_then(|t| t.id),
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
            occurrence_id: occurrence_id.to_string(),
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
        let identifier = photo.id.map(|id| format!("https://www.inaturalist.org/photos/{}", id));

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
            occurrence_id: occurrence_id.to_string(),
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
            attribution_link_url: photo.id.map(|id| format!("https://www.inaturalist.org/photos/{}", id)),
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
                parts.push(format!("https://www.inaturalist.org/users/{}", user_id));
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
            occurrence_id: occurrence_id.to_string(),
            identification_id: identification.id.map(|id| id.to_string()),
            identified_by,
            identified_by_id,
            date_identified: identification.created_at.clone(),
            identification_remarks: identification.body.clone(),
            taxon_id: identification.taxon.as_ref().and_then(|t| t.id.map(|id| id.to_string())),
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
    use inaturalist::models::Observation;

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
}

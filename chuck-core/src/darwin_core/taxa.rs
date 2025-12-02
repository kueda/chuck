use inaturalist::models::{Observation, ShowTaxon};
use inaturalist::apis::taxa_api;
use crate::api::{client::get_config, rate_limiter::get_rate_limiter};
use std::collections::{HashMap, HashSet};
use tokio::time::{sleep, Duration};

/// Collect all unique taxon IDs from observations and their identifications
pub fn collect_taxon_ids(observations: &[Observation]) -> Vec<i32> {
    let mut all_taxon_ids = HashSet::new();

    for obs in observations {
        // Collect from observation taxon
        if let Some(taxon) = &obs.taxon {
            if let Some(ancestor_ids) = &taxon.ancestor_ids {
                all_taxon_ids.extend(ancestor_ids.iter());
            }
        }
        // Collect from identification taxa
        if let Some(identifications) = &obs.identifications {
            for identification in identifications {
                if let Some(taxon) = &identification.taxon {
                    if let Some(ancestor_ids) = &taxon.ancestor_ids {
                        all_taxon_ids.extend(ancestor_ids.iter());
                    }
                }
            }
        }
    }

    all_taxon_ids.into_iter().collect()
}

/// Fetch taxa in chunks with retry logic and progress callback
pub async fn fetch_taxa_for_observations<F>(
    taxon_ids: &[i32],
    progress_callback: Option<F>,
    config: Option<&inaturalist::apis::configuration::Configuration>,
) -> Result<HashMap<i32, ShowTaxon>, Box<dyn std::error::Error>>
where
    F: Fn(usize, usize) + Send + Sync + Clone + 'static
{
    let mut taxa_hash = HashMap::new();

    // Use provided config or get global config
    let config_instance = if let Some(cfg) = config {
        cfg.clone()
    } else {
        get_config().await.read().await.clone()
    };
    let config = tokio::sync::RwLock::new(config_instance);

    let rate_limiter = get_rate_limiter().await;
    let total_chunks = (taxon_ids.len() + 499) / 500;
    let mut chunks_processed = 0;

    for chunk in taxon_ids.chunks(500) {
        // Rate limiting: coordinate with other API requests
        if !taxa_hash.is_empty() {
            rate_limiter.wait_for_next_request().await;
        }

        let params = taxa_api::TaxaGetParams {
            q: None,
            is_active: None,
            id: Some(chunk.to_vec()),
            parent_id: None,
            rank: None,
            rank_level: None,
            id_above: None,
            id_below: None,
            per_page: Some("500".to_string()),
            locale: None,
            preferred_place_id: None,
            only_id: None,
            all_names: None,
            order: None,
            order_by: None,
        };

        // Retry with exponential backoff (3 attempts total)
        let mut attempt = 0;
        let response = loop {
            attempt += 1;
            let config_read = config.read().await;
            match taxa_api::taxa_get(&*config_read, params.clone()).await {
                Ok(response) => break response,
                Err(e) => {
                    if attempt >= 3 {
                        return Err(format!(
                            "Failed to fetch taxa after 3 attempts: {}", e
                        ).into());
                    }
                    let backoff_ms = 1000 * (2_u64.pow(attempt - 1));
                    eprintln!(
                        "Taxa API request failed (attempt {}), retrying in {}ms: {}",
                        attempt, backoff_ms, e
                    );
                    sleep(Duration::from_millis(backoff_ms)).await;
                }
            }
        };

        for taxon in response.results {
            if let Some(id) = taxon.id {
                taxa_hash.insert(id, taxon);
            }
        }

        chunks_processed += 1;
        if let Some(ref callback) = progress_callback {
            callback(chunks_processed, total_chunks);
        }
    }

    Ok(taxa_hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use inaturalist::models::{Identification, ObservationTaxon};

    fn create_taxon_with_ancestors(id: i32, ancestor_ids: Vec<i32>) -> Box<ObservationTaxon> {
        Box::new(ObservationTaxon {
            id: Some(id),
            ancestor_ids: Some(ancestor_ids),
            ..Default::default()
        })
    }

    #[test]
    fn test_collect_taxon_ids_empty() {
        let observations: Vec<Observation> = vec![];
        let taxon_ids = collect_taxon_ids(&observations);
        assert_eq!(taxon_ids.len(), 0);
    }

    #[test]
    fn test_collect_taxon_ids_from_observation_taxon() {
        let obs = Observation {
            id: Some(1),
            taxon: Some(create_taxon_with_ancestors(100, vec![1, 2, 3, 100])),
            ..Default::default()
        };

        let taxon_ids = collect_taxon_ids(&[obs]);
        assert_eq!(taxon_ids.len(), 4);
        assert!(taxon_ids.contains(&1));
        assert!(taxon_ids.contains(&2));
        assert!(taxon_ids.contains(&3));
        assert!(taxon_ids.contains(&100));
    }

    #[test]
    fn test_collect_taxon_ids_from_identifications() {
        let obs = Observation {
            id: Some(1),
            taxon: None,
            identifications: Some(vec![
                Identification {
                    taxon: Some(create_taxon_with_ancestors(200, vec![4, 5, 200])),
                    ..Default::default()
                },
            ]),
            ..Default::default()
        };

        let taxon_ids = collect_taxon_ids(&[obs]);
        assert_eq!(taxon_ids.len(), 3);
        assert!(taxon_ids.contains(&4));
        assert!(taxon_ids.contains(&5));
        assert!(taxon_ids.contains(&200));
    }

    #[test]
    fn test_collect_taxon_ids_deduplication() {
        let obs1 = Observation {
            id: Some(1),
            taxon: Some(create_taxon_with_ancestors(100, vec![1, 2, 3, 100])),
            ..Default::default()
        };

        let obs2 = Observation {
            id: Some(2),
            taxon: Some(create_taxon_with_ancestors(200, vec![1, 2, 200])),
            ..Default::default()
        };

        let taxon_ids = collect_taxon_ids(&[obs1, obs2]);
        // Should have 1, 2, 3, 100, 200 (deduplicated)
        assert_eq!(taxon_ids.len(), 5);
        assert!(taxon_ids.contains(&1));
        assert!(taxon_ids.contains(&2));
        assert!(taxon_ids.contains(&3));
        assert!(taxon_ids.contains(&100));
        assert!(taxon_ids.contains(&200));
    }

    #[test]
    fn test_collect_taxon_ids_from_both_sources() {
        let obs = Observation {
            id: Some(1),
            taxon: Some(create_taxon_with_ancestors(100, vec![1, 2, 100])),
            identifications: Some(vec![
                Identification {
                    taxon: Some(create_taxon_with_ancestors(200, vec![1, 3, 200])),
                    ..Default::default()
                },
            ]),
            ..Default::default()
        };

        let taxon_ids = collect_taxon_ids(&[obs]);
        // Should have 1, 2, 3, 100, 200 (deduplicated across both sources)
        assert_eq!(taxon_ids.len(), 5);
        assert!(taxon_ids.contains(&1));
        assert!(taxon_ids.contains(&2));
        assert!(taxon_ids.contains(&3));
        assert!(taxon_ids.contains(&100));
        assert!(taxon_ids.contains(&200));
    }
}

use inaturalist::apis::observations_api;

pub const PER_PAGE: u32 = 200;

pub static DEFAULT_GET_PARAMS: observations_api::ObservationsGetParams = observations_api::ObservationsGetParams {
    acc: None,
    acc_above: None,
    acc_below: None,
    acc_below_or_unknown: None,
    annotation_user_id: None,
    apply_project_rules_for: None,
    captive: None,
    created_d1: None,
    created_d2: None,
    created_day: None,
    created_month: None,
    created_on: None,
    created_year: None,
    cs: None,
    csa: None,
    csi: None,
    d1: None,
    d2: None,
    day: None,
    endemic: None,
    expected_nearby: None,
    geo: None,
    geoprivacy: None,
    hour: None,
    hrank: None,
    iconic_taxa: None,
    id: None,
    id_above: None,
    id_below: None,
    id_please: None,
    ident_user_id: None,
    identifications: None,
    identified: None,
    introduced: None,
    lat: None,
    license: None,
    licensed: None,
    list_id: None,
    lng: None,
    locale: None,
    lrank: None,
    mappable: None,
    month: None,
    native: None,
    nelat: None,
    nelng: None,
    not_id: None,
    not_in_project: None,
    not_matching_project_rules_for: None,
    obscuration: None,
    observation_accuracy_experiment_id: None,
    observed_on: None,
    ofv_datatype: None,
    only_id: None,
    order: None,
    order_by: None,
    out_of_range: None,
    page: None,
    pcid: None,
    per_page: None,
    photo_license: None,
    photo_licensed: None,
    photos: None,
    place_id: None,
    popular: None,
    preferred_place_id: None,
    project_id: None,
    q: None,
    quality_grade: None,
    radius: None,
    rank: None,
    reviewed: None,
    search_on: None,
    site_id: None,
    sound_license: None,
    sounds: None,
    swlat: None,
    swlng: None,
    taxon_geoprivacy: None,
    taxon_id: None,
    taxon_is_active: None,
    taxon_name: None,
    term_id: None,
    term_id_or_unknown: None,
    term_value_id: None,
    threatened: None,
    ttl: None,
    unobserved_by_user_id: None,
    updated_since: None,
    user_id: None,
    user_login: None,
    verifiable: None,
    viewer_id: None,
    without_taxon_id: None,
    without_term_id: None,
    without_term_value_id: None,
    year: None,
};

// TODO: accept more options, and maybe make a separate method that just converts a query string to API params
pub fn build_params(
    taxon: Option<String>,
    place_id: Option<i32>,
    user: Option<String>,
    d1: Option<String>,
    d2: Option<String>,
    created_d1: Option<String>,
    created_d2: Option<String>,
) -> observations_api::ObservationsGetParams {
    let mut params = observations_api::ObservationsGetParams {
        per_page: Some(PER_PAGE.to_string()),
        ..DEFAULT_GET_PARAMS.clone()
    };

    if let Some(taxon) = taxon {
        if let Ok(taxon_id) = taxon.parse::<i32>() {
            params.taxon_id = Some(vec![taxon_id.to_string()]);
        } else {
            params.taxon_name = Some(vec![taxon]);
        }
    }

    if let Some(place_id) = place_id {
        params.place_id = Some(vec![place_id]);
    }

    if let Some(user) = user {
        params.user_id = Some(vec![user.to_string()]);
    }

    params.d1 = d1;
    params.d2 = d2;
    params.created_d1 = created_d1;
    params.created_d2 = created_d2;

    params
}

/// Extract human-readable criteria from ObservationsGetParams
/// Note: We manually check each field because ObservationsGetParams doesn't implement
/// reflection by default. We could use serde to serialize to a map and iterate
/// dynamically, but that would have runtime performance costs.
pub fn extract_criteria(params: &observations_api::ObservationsGetParams) -> Vec<String> {
    let mut criteria = Vec::new();

    if let Some(ref values) = params.taxon_id {
        if !values.is_empty() {
            criteria.push(format!("taxon_id: {}", values.join(", ")));
        }
    }
    if let Some(ref values) = params.taxon_name {
        if !values.is_empty() {
            criteria.push(format!("taxon_name: {}", values.join(", ")));
        }
    }
    if let Some(ref values) = params.user_id {
        if !values.is_empty() {
            criteria.push(format!("user_id: {}", values.join(", ")));
        }
    }
    if let Some(ref values) = params.user_login {
        if !values.is_empty() {
            criteria.push(format!("user_login: {}", values.join(", ")));
        }
    }
    if let Some(ref values) = params.place_id {
        if !values.is_empty() {
            criteria.push(format!("place_id: {}", values.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", ")));
        }
    }
    if let Some(ref value) = params.lat {
        criteria.push(format!("lat: {value}"));
    }
    if let Some(ref value) = params.lng {
        criteria.push(format!("lng: {value}"));
    }
    if let Some(ref value) = params.radius {
        criteria.push(format!("radius: {value}"));
    }
    if let Some(ref value) = params.d1 {
        criteria.push(format!("observed_after: {value}"));
    }
    if let Some(ref value) = params.d2 {
        criteria.push(format!("observed_before: {value}"));
    }
    if let Some(ref value) = params.quality_grade {
        criteria.push(format!("quality_grade: {value}"));
    }
    if let Some(ref value) = params.photos {
        criteria.push(format!("photos: {value}"));
    }
    if let Some(ref value) = params.sounds {
        criteria.push(format!("sounds: {value}"));
    }
    if let Some(ref value) = params.captive {
        criteria.push(format!("captive: {value}"));
    }

    criteria
}

#[cfg(test)]
mod tests {
    use super::*;

    mod build_params {
        use super::*;

        #[test]
        fn test_without_taxon() {
            assert_eq!(build_params(None, None, None, None, None, None, None).taxon_id, None);
        }

        #[test]
        fn test_taxon_as_id() {
            assert_eq!(
                build_params(Some("123".to_string()), None, None, None, None, None, None).taxon_id,
                Some(vec!["123".to_string()])
            );
        }

        #[test]
        fn test_taxon_as_name() {
            assert_eq!(
                build_params(Some("Centromadia".to_string()), None, None, None, None, None, None).taxon_name,
                Some(vec!["Centromadia".to_string()])
            );
        }

        #[test]
        fn test_with_place_id() {
            assert_eq!(build_params(None, Some(123), None, None, None, None, None).place_id, Some(vec![123]));
        }

        #[test]
        fn test_without_place_id() {
            assert_eq!(build_params(None, None, None, None, None, None, None).place_id, None);
        }

        #[test]
        fn test_with_user_id() {
            assert_eq!(
                build_params(None, None, Some("123".to_string()), None, None, None, None).user_id,
                Some(vec!["123".to_string()])
            );
        }

        #[test]
        fn test_without_user_id() {
            assert_eq!(build_params(None, None, None, None, None, None, None).user_id, None);
        }

        #[test]
        fn test_with_d1() {
            assert_eq!(
                build_params(None, None, None, Some("2020-01-01".to_string()), None, None, None).d1,
                Some("2020-01-01".to_string())
            );
        }

        #[test]
        fn test_with_d2() {
            assert_eq!(
                build_params(None, None, None, None, Some("2020-01-01".to_string()), None, None).d2,
                Some("2020-01-01".to_string())
            );
        }

        #[test]
        fn test_with_created_d1() {
            assert_eq!(
                build_params(None, None, None, None, None, Some("2020-01-01".to_string()), None).created_d1,
                Some("2020-01-01".to_string())
            );
        }

        #[test]
        fn test_with_created_d2() {
            assert_eq!(
                build_params(None, None, None, None, None, None, Some("2020-01-01".to_string())).created_d2,
                Some("2020-01-01".to_string())
            );
        }
    }
}

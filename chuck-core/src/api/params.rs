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

/// Parse an iNat observations query string into ObservationsGetParams.
/// Unknown keys and `any` values are silently ignored.
/// `per_page` is set to PER_PAGE; pagination params are always skipped.
pub fn parse_url_params(query: &str) -> observations_api::ObservationsGetParams {
    let query = query.trim_start_matches('?');
    let mut params = observations_api::ObservationsGetParams {
        per_page: Some(PER_PAGE.to_string()),
        ..DEFAULT_GET_PARAMS.clone()
    };

    // Collect all values grouped by key, handling repeated keys and
    // comma-separated values. Drop "any" and empty values.
    let mut map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for (raw_key, raw_val) in url::form_urlencoded::parse(query.as_bytes()) {
        let key = raw_key.to_string();
        let val = raw_val.to_string();
        if val == "any" || val.is_empty() {
            continue;
        }
        // Support comma-separated multi-values (e.g. taxon_id=1,2)
        for part in val.split(',') {
            let part = part.trim().to_string();
            if part.is_empty() || part == "any" {
                continue;
            }
            map.entry(key.clone()).or_default().push(part);
        }
    }

    let string_val = |key: &str| -> Option<String> {
        map.get(key).and_then(|v| v.first().cloned())
    };
    let vec_string = |key: &str| -> Option<Vec<String>> {
        map.get(key).filter(|v| !v.is_empty()).cloned()
    };
    let vec_i32 = |key: &str| -> Option<Vec<i32>> {
        map.get(key)
            .map(|vals| {
                vals.iter()
                    .filter_map(|v| v.parse::<i32>().ok())
                    .collect::<Vec<_>>()
            })
            .filter(|v| !v.is_empty())
    };
    let i32_val = |key: &str| -> Option<i32> {
        map.get(key)
            .and_then(|v| v.first())
            .and_then(|s| s.parse::<i32>().ok())
    };
    let f64_val = |key: &str| -> Option<f64> {
        map.get(key)
            .and_then(|v| v.first())
            .and_then(|s| s.parse::<f64>().ok())
    };
    let bool_val = |key: &str| -> Option<bool> {
        map.get(key)
            .and_then(|v| v.first())
            .and_then(|s| match s.as_str() {
                "true" | "1" => Some(true),
                "false" | "0" => Some(false),
                _ => None,
            })
    };

    // --- Vec<String> params ---
    params.taxon_id = vec_string("taxon_id");
    params.without_taxon_id = vec_string("without_taxon_id");
    params.taxon_name = vec_string("taxon_name");
    params.user_id = vec_string("user_id");
    params.user_login = vec_string("user_login");
    params.annotation_user_id = vec_string("annotation_user_id");
    params.project_id = vec_string("project_id");
    params.rank = vec_string("rank");
    params.site_id = vec_string("site_id");
    params.license = vec_string("license");
    params.photo_license = vec_string("photo_license");
    params.sound_license = vec_string("sound_license");
    params.ofv_datatype = vec_string("ofv_datatype");
    params.iconic_taxa = vec_string("iconic_taxa");
    params.geoprivacy = vec_string("geoprivacy");
    params.taxon_geoprivacy = vec_string("taxon_geoprivacy");
    params.obscuration = vec_string("obscuration");
    params.csi = vec_string("csi");
    params.hour = vec_string("hour");
    params.day = vec_string("day");
    params.month = vec_string("month");
    params.year = vec_string("year");
    params.created_day = vec_string("created_day");
    params.created_month = vec_string("created_month");
    params.created_year = vec_string("created_year");
    params.id = vec_string("id");
    params.not_id = vec_string("not_id");

    // --- Vec<i32> params ---
    params.place_id = vec_i32("place_id");
    params.term_id = vec_i32("term_id");
    params.term_value_id = vec_i32("term_value_id");
    params.without_term_value_id = vec_i32("without_term_value_id");
    params.term_id_or_unknown = vec_i32("term_id_or_unknown");
    params.observation_accuracy_experiment_id = vec_i32("observation_accuracy_experiment_id");

    // --- Option<i32> params ---
    params.ident_user_id = i32_val("ident_user_id");
    params.without_term_id = i32_val("without_term_id");
    params.list_id = i32_val("list_id");
    params.unobserved_by_user_id = i32_val("unobserved_by_user_id");
    params.preferred_place_id = i32_val("preferred_place_id");

    // --- String params ---
    params.d1 = string_val("d1");
    params.d2 = string_val("d2");
    params.created_d1 = string_val("created_d1");
    params.created_d2 = string_val("created_d2");
    params.created_on = string_val("created_on");
    params.observed_on = string_val("observed_on");
    params.acc_above = string_val("acc_above");
    params.acc_below = string_val("acc_below");
    params.acc_below_or_unknown = string_val("acc_below_or_unknown");
    params.apply_project_rules_for = string_val("apply_project_rules_for");
    params.cs = string_val("cs");
    params.csa = string_val("csa");
    params.hrank = string_val("hrank");
    params.lrank = string_val("lrank");
    params.id_above = string_val("id_above");
    params.id_below = string_val("id_below");
    params.identifications = string_val("identifications");
    params.radius = string_val("radius");
    params.not_in_project = string_val("not_in_project");
    params.not_matching_project_rules_for = string_val("not_matching_project_rules_for");
    params.q = string_val("q");
    params.quality_grade = string_val("quality_grade");
    params.updated_since = string_val("updated_since");
    params.viewer_id = string_val("viewer_id");

    // --- Option<f64> params ---
    params.lat = f64_val("lat");
    params.lng = f64_val("lng");
    params.nelat = f64_val("nelat");
    params.nelng = f64_val("nelng");
    params.swlat = f64_val("swlat");
    params.swlng = f64_val("swlng");

    // --- bool params ---
    params.acc = bool_val("acc");
    params.captive = bool_val("captive");
    params.endemic = bool_val("endemic");
    params.geo = bool_val("geo");
    params.id_please = bool_val("id_please");
    params.identified = bool_val("identified");
    params.introduced = bool_val("introduced");
    params.mappable = bool_val("mappable");
    params.native = bool_val("native");
    params.out_of_range = bool_val("out_of_range");
    params.pcid = bool_val("pcid");
    params.photos = bool_val("photos");
    params.popular = bool_val("popular");
    params.sounds = bool_val("sounds");
    params.taxon_is_active = bool_val("taxon_is_active");
    params.threatened = bool_val("threatened");
    params.verifiable = bool_val("verifiable");
    params.licensed = bool_val("licensed");
    params.photo_licensed = bool_val("photo_licensed");
    params.expected_nearby = bool_val("expected_nearby");
    params.reviewed = bool_val("reviewed");

    params
}

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

    mod parse_url_params {
        use super::*;

        #[test]
        fn test_basic_taxon_id() {
            let p = parse_url_params("taxon_id=47790");
            assert_eq!(p.taxon_id, Some(vec!["47790".to_string()]));
        }

        #[test]
        fn test_any_value_is_dropped() {
            let p = parse_url_params("place_id=any");
            assert_eq!(p.place_id, None);
        }

        #[test]
        fn test_unknown_param_is_dropped() {
            let p = parse_url_params("view=species");
            assert_eq!(p.taxon_id, None);
            assert_eq!(p.place_id, None);
            assert_eq!(p.user_id, None);
        }
    }

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

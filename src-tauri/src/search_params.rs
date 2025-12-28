use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tauri::http::Uri;
use url::Url;

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct SearchParams {
    pub sort_by: Option<String>,
    pub sort_direction: Option<String>,

    // Bounding box parameters (all four must be present to filter by bbox)
    pub nelat: Option<String>,
    pub nelng: Option<String>,
    pub swlat: Option<String>,
    pub swlng: Option<String>,

    // In theory this will flatten the HashMap during serialization and during
    // deserialization, unflatten everything that remains after deserializing
    // the named params above into filters
    #[serde(flatten)]
    pub filters: HashMap<String, String>,
}

impl SearchParams {
    pub fn from_uri(uri: &Uri) -> Self {
        let url = Url::parse(&uri.to_string()).unwrap();
        let query_hash: HashMap<String, String> = url.query_pairs().into_owned().collect();

        let mut filters = HashMap::new();
        let mut sort_by = None;
        let mut sort_direction = None;
        let mut nelat = None;
        let mut nelng = None;
        let mut swlat = None;
        let mut swlng = None;

        for (key, value) in query_hash {
            match key.as_str() {
                "sort_by" => sort_by = Some(value),
                "sort_direction" => sort_direction = Some(value),
                "nelat" => nelat = Some(value),
                "nelng" => nelng = Some(value),
                "swlat" => swlat = Some(value),
                "swlng" => swlng = Some(value),
                _ => {
                    filters.insert(key, value);
                }
            }
        }

        SearchParams {
            filters,
            sort_by,
            sort_direction,
            nelat,
            nelng,
            swlat,
            swlng,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params_from_url(url: String) -> SearchParams {
        let uri: Uri = url.parse().unwrap();
        SearchParams::from_uri(&uri)
    }

    #[test]
    fn test_search_params_from_uri_with_filter() {
        let params = params_from_url("http://local/?scientificName=foo".to_string());
        assert_eq!(params.filters.get("scientificName"), Some(&"foo".to_string()));
    }

    #[test]
    fn test_search_params_from_uri_with_multiple_filters() {
        let params = params_from_url("http://local/?scientificName=foo&genus=bar".to_string());
        assert_eq!(params.filters.get("scientificName"), Some(&"foo".to_string()));
        assert_eq!(params.filters.get("genus"), Some(&"bar".to_string()));
    }

    #[test]
    fn test_search_params_from_uri_with_sort_by() {
        assert_eq!(
            params_from_url("http://local/?sort_by=foo".to_string()).sort_by,
            Some("foo".to_string())
        );
    }

    #[test]
    fn test_search_params_from_uri_with_sort_direction() {
        assert_eq!(
            params_from_url("http://local/?sort_direction=foo".to_string()).sort_direction,
            Some("foo".to_string())
        );
    }

    #[test]
    fn test_search_params_from_uri_with_filters_and_sorting() {
        let params = params_from_url("http://local/?scientificName=foo&sort_by=genus&sort_direction=DESC".to_string());
        assert_eq!(params.filters.get("scientificName"), Some(&"foo".to_string()));
        assert_eq!(params.sort_by, Some("genus".to_string()));
        assert_eq!(params.sort_direction, Some("DESC".to_string()));
    }

    #[test]
    fn test_search_params_from_uri_with_bbox() {
        let params = params_from_url("http://local/?nelat=40&nelng=-120&swlat=35&swlng=-125".to_string());
        assert_eq!(params.nelat, Some("40".to_string()));
        assert_eq!(params.nelng, Some("-120".to_string()));
        assert_eq!(params.swlat, Some("35".to_string()));
        assert_eq!(params.swlng, Some("-125".to_string()));
    }

    #[test]
    fn test_search_params_bbox_not_in_filters() {
        let params = params_from_url("http://local/?nelat=40&nelng=-120&swlat=35&swlng=-125".to_string());
        assert_eq!(params.filters.get("nelat"), None);
        assert_eq!(params.filters.get("nelng"), None);
        assert_eq!(params.filters.get("swlat"), None);
        assert_eq!(params.filters.get("swlng"), None);
    }

    #[test]
    fn test_search_params_from_uri_with_bbox_and_filters() {
        let params = params_from_url("http://local/?scientificName=foo&nelat=40&nelng=-120&swlat=35&swlng=-125".to_string());
        assert_eq!(params.filters.get("scientificName"), Some(&"foo".to_string()));
        assert_eq!(params.nelat, Some("40".to_string()));
        assert_eq!(params.nelng, Some("-120".to_string()));
        assert_eq!(params.swlat, Some("35".to_string()));
        assert_eq!(params.swlng, Some("-125".to_string()));
        // Verify bbox params are not in filters
        assert_eq!(params.filters.get("nelat"), None);
    }
}

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tauri::http::Uri;
use url::Url;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct SearchParams {
    pub order_by: Option<String>,
    pub order: Option<String>,

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
        let mut order_by = None;
        let mut order = None;

        for (key, value) in query_hash {
            match key.as_str() {
                "order_by" => order_by = Some(value),
                "order" => order = Some(value),
                _ => {
                    filters.insert(key, value);
                }
            }
        }

        SearchParams {
            filters,
            order_by,
            order,
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
    fn test_search_params_from_uri_with_order_by() {
        assert_eq!(
            params_from_url("http://local/?order_by=foo".to_string()).order_by,
            Some("foo".to_string())
        );
    }

    #[test]
    fn test_search_params_from_uri_with_order() {
        assert_eq!(
            params_from_url("http://local/?order=foo".to_string()).order,
            Some("foo".to_string())
        );
    }

    #[test]
    fn test_search_params_from_uri_with_filters_and_sorting() {
        let params = params_from_url("http://local/?scientificName=foo&order_by=genus&order=DESC".to_string());
        assert_eq!(params.filters.get("scientificName"), Some(&"foo".to_string()));
        assert_eq!(params.order_by, Some("genus".to_string()));
        assert_eq!(params.order, Some("DESC".to_string()));
    }
}

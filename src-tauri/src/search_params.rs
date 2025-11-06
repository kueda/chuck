use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tauri::http::Uri;
use url::Url;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct SearchParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scientific_name: Option<String>,
    pub order_by: Option<String>,
    pub order: Option<String>,
}

impl SearchParams {
    pub fn from_uri(uri: &Uri) -> Self {
        let url = Url::parse(&uri.to_string()).unwrap();
        let query_hash: HashMap<String, String> = url.query_pairs().into_owned().collect();
        SearchParams {
            scientific_name: query_hash.get("scientific_name").cloned(),
            order_by: query_hash.get("order_by").cloned(),
            order: query_hash.get("order").cloned(),
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
    fn test_search_params_from_uri_with_scientific_name() {
        assert_eq!(
            params_from_url("http://local/?scientific_name=foo".to_string()).scientific_name,
            Some("foo".to_string())
        );
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
}

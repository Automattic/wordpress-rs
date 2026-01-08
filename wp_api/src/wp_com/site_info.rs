use crate::url_query::{AppendUrlQueryPairs, QueryPairs};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, uniffi::Record)]
pub struct SiteInfoParameters {
    pub url: String,
}

impl AppendUrlQueryPairs for SiteInfoParameters {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut.append_pair("url", &self.url);
    }
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct SiteInfoResponse {
    pub url_after_redirects: String,
    pub exists: bool,
    pub is_word_press: bool,
    pub has_jetpack: bool,
    pub is_jetpack_active: bool,
    pub is_jetpack_connected: bool,
    pub is_word_press_dot_com: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use url::Url;

    #[test]
    fn test_site_info_parameters_append_query_pairs() {
        let mut url = Url::parse("https://public-api.wordpress.com/rest/v1.1/connect/site-info")
            .expect("Failed to parse url");

        let params = SiteInfoParameters {
            url: "https://example.com".to_string(),
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/connect/site-info?url=https%3A%2F%2Fexample.com"
        );
    }

    #[test]
    fn test_site_info_response_wordpress_com_site() {
        let json_str = r#"{
            "urlAfterRedirects": "https://myblog.wordpress.com",
            "exists": true,
            "isWordPress": true,
            "hasJetpack": true,
            "isJetpackActive": true,
            "isJetpackConnected": false,
            "isWordPressDotCom": true
        }"#;
        let response: SiteInfoResponse = serde_json::from_str(json_str).unwrap();

        assert_eq!(response.url_after_redirects, "https://myblog.wordpress.com");
        assert!(response.exists);
        assert!(response.is_word_press);
        assert!(response.has_jetpack);
        assert!(response.is_jetpack_active);
        assert!(!response.is_jetpack_connected);
        assert!(response.is_word_press_dot_com);
    }

    #[test]
    fn test_site_info_response_self_hosted_site() {
        let json_str = r#"{
            "urlAfterRedirects": "https://example.com",
            "exists": true,
            "isWordPress": true,
            "hasJetpack": true,
            "isJetpackActive": true,
            "isJetpackConnected": false,
            "isWordPressDotCom": false
        }"#;
        let response: SiteInfoResponse = serde_json::from_str(json_str).unwrap();

        assert_eq!(response.url_after_redirects, "https://example.com");
        assert!(!response.is_word_press_dot_com);
        assert!(response.is_word_press);
    }

    #[test]
    fn test_site_info_response_non_wordpress_site() {
        let json_str = r#"{
            "urlAfterRedirects": "https://example.com",
            "exists": true,
            "isWordPress": false,
            "hasJetpack": false,
            "isJetpackActive": false,
            "isJetpackConnected": false,
            "isWordPressDotCom": false
        }"#;
        let response: SiteInfoResponse = serde_json::from_str(json_str).unwrap();

        assert_eq!(response.url_after_redirects, "https://example.com");
        assert!(response.exists);
        assert!(!response.is_word_press);
        assert!(!response.has_jetpack);
        assert!(!response.is_jetpack_active);
        assert!(!response.is_jetpack_connected);
        assert!(!response.is_word_press_dot_com);
    }
}

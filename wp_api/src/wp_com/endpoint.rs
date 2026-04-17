use crate::{
    parsed_url::ParsedUrl,
    request::endpoint::{ApiUrlResolver, AsNamespace, WpNamespace},
    wp_com::WpComBaseUrl,
};
use std::sync::Arc;
use strum::IntoEnumIterator;

pub mod extensions;
pub mod followers_endpoint;
pub mod jetpack_connection_endpoint;
pub mod languages_endpoint;
pub mod me_endpoint;
pub mod oauth2;
pub mod sites_endpoint;
pub mod stats_city_views_endpoint;
pub mod stats_clicks_endpoint;
pub mod stats_country_views_endpoint;
pub mod stats_devices_browser_endpoint;
pub mod stats_devices_platform_endpoint;
pub mod stats_devices_screensize_endpoint;
pub mod stats_emails_summary_endpoint;
pub mod stats_file_downloads_endpoint;
pub mod stats_insights_endpoint;
pub mod stats_referrers_endpoint;
pub mod stats_region_views_endpoint;
pub mod stats_search_terms_endpoint;
pub mod stats_subscribers_endpoint;
pub mod stats_summary_endpoint;
pub mod stats_tags_endpoint;
pub mod stats_top_authors_endpoint;
pub mod stats_top_posts_endpoint;
pub mod stats_utm_endpoint;
pub mod stats_video_plays_endpoint;
pub mod stats_visits_endpoint;
pub mod subscribers_endpoint;
pub mod support_bots_endpoint;
pub mod support_eligibility_endpoint;
pub mod support_tickets_endpoint;

#[derive(uniffi::Object)]
pub struct WpComDotOrgApiUrlResolver {
    pub base_url: ParsedUrl,
    pub site_id: String,
}

#[uniffi::export]
impl WpComDotOrgApiUrlResolver {
    #[uniffi::constructor]
    pub fn new(site_id: String, base_url: WpComBaseUrl) -> Self {
        Self {
            base_url: base_url.parsed_url(),
            site_id,
        }
    }
}

#[uniffi::export]
impl ApiUrlResolver for WpComDotOrgApiUrlResolver {
    fn resolve(&self, namespace: String, endpoint_segments: Vec<String>) -> Arc<ParsedUrl> {
        {
            if !WpNamespace::iter().any(|n| n.namespace_value() == namespace) {
                panic!(
                    "`WpComDotOrgApiUrlResolver` doesn't support the namespace `{}`. The supported namespaces are: {:?}",
                    namespace,
                    WpNamespace::iter()
                );
            }
        }

        // The API root endpoint needs special handling for WordPress.com
        if namespace == WpNamespace::None.namespace_value() && endpoint_segments.is_empty() {
            let url_string = format!(
                "https://public-api.wordpress.com/wp-json/?rest_route=/sites/{}/",
                self.site_id
            );
            let parsed_url =
                ParsedUrl::parse(&url_string).expect("WordPress.com API root URL is valid");
            return Arc::new(parsed_url);
        }

        Arc::new(
            self.base_url
                .by_extending_and_splitting_by_forward_slash(
                    vec![namespace, "sites".to_string(), self.site_id.to_string()]
                        .into_iter()
                        .chain(endpoint_segments),
                )
                .into(),
        )
    }

    fn route_path(&self, namespace: String, endpoint_path: String) -> String {
        format!(
            "{}/sites/{}/{}",
            namespace.trim_end_matches('/'),
            self.site_id,
            endpoint_path.trim_start_matches('/')
        )
    }
}

#[derive(Debug)]
pub(crate) struct WpComApiClientInternalUrlResolver {
    pub base_url: ParsedUrl,
}

impl WpComApiClientInternalUrlResolver {
    fn new() -> Self {
        Self {
            base_url: WpComBaseUrl::Production.parsed_url(),
        }
    }
}

impl Default for WpComApiClientInternalUrlResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiUrlResolver for WpComApiClientInternalUrlResolver {
    fn resolve(&self, namespace: String, endpoint_segments: Vec<String>) -> Arc<ParsedUrl> {
        {
            if WpNamespace::iter().any(|n| n.namespace_value() == namespace) {
                panic!(
                    "`WpComApiClient` doesn't support the namespace `{namespace}`. Try using `WpApiClient` instead.",
                );
            }
        }
        Arc::new(
            self.base_url
                .by_extending_and_splitting_by_forward_slash(
                    vec![namespace].into_iter().chain(endpoint_segments),
                )
                .into(),
        )
    }

    fn route_path(&self, namespace: String, endpoint_path: String) -> String {
        format!(
            "{}/{}",
            namespace.trim_end_matches('/'),
            endpoint_path.trim_start_matches('/')
        )
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{request::endpoint::ApiEndpointUrl, wp_com::WpComNamespace};
    use rstest::*;
    use std::sync::Arc;

    const WP_COM_BASE_URL: &str = "https://public-api.wordpress.com";

    #[fixture]
    pub fn fixture_wp_com_api_url_resolver() -> Arc<dyn ApiUrlResolver> {
        Arc::new(WpComApiClientInternalUrlResolver::default())
    }

    pub fn validate_wp_com_rest_v1_1_endpoint(endpoint_url: ApiEndpointUrl, path: &str) {
        validate_endpoint(WpComNamespace::RestV1_1, endpoint_url, path);
    }

    fn validate_endpoint(namespace: WpComNamespace, endpoint_url: ApiEndpointUrl, path: &str) {
        assert_eq!(
            endpoint_url.as_str(),
            format!("{}{}{}", WP_COM_BASE_URL, namespace.namespace_value(), path)
        );
    }

    #[rstest]
    #[case("/wp/v2", vec!["posts".to_string()], "https://public-api.wordpress.com/wp/v2/sites/example.wordpress.com/posts")]
    fn wp_com_dot_org_api_url_resolver(
        #[case] namespace: &str,
        #[case] endpoint_segments: Vec<String>,
        #[case] expected_url: &str,
    ) {
        let resolver = WpComDotOrgApiUrlResolver::new(
            "example.wordpress.com".to_string(),
            WpComBaseUrl::Production,
        );
        assert_eq!(
            resolver
                .resolve(namespace.to_string(), endpoint_segments)
                .url(),
            expected_url
        );
    }
}

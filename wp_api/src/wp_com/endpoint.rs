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
pub mod oauth2;
pub mod subscribers_endpoint;
pub mod support_bots_endpoint;

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

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

use crate::{
    parsed_url::ParsedUrl,
    request::endpoint::{ApiUrlResolver, AsNamespace, WpNamespace},
};
use std::sync::Arc;
use strum::IntoEnumIterator;
use url::Url;

pub mod jetpack_connection_endpoint;
pub mod oauth2;
pub mod subscribers;
pub mod support_bots_endpoint;
pub mod support_eligibility_endpoint;
pub mod support_tickets_endpoint;

#[derive(uniffi::Object)]
pub struct WpComDotOrgApiUrlResolver {
    pub base_url: ParsedUrl,
    pub site_url: String,
}

#[uniffi::export]
impl WpComDotOrgApiUrlResolver {
    #[uniffi::constructor]
    pub fn new(site_url: String) -> Self {
        Self {
            base_url: wp_com_base_url(),
            site_url,
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
                    vec![namespace, "sites".to_string(), self.site_url.to_string()]
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
            base_url: wp_com_base_url(),
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
                    "`WpComApiClient` doesn't support the namespace `{}`. Try using `WpApiClient` instead.",
                    namespace,
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

fn wp_com_base_url() -> ParsedUrl {
    Url::parse("https://public-api.wordpress.com")
        .expect("This is a valid URL")
        .into()
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
        let resolver = WpComDotOrgApiUrlResolver::new("example.wordpress.com".to_string());
        assert_eq!(
            resolver
                .resolve(namespace.to_string(), endpoint_segments)
                .url(),
            expected_url
        );
    }
}

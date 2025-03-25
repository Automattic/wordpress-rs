use url::Url;

pub mod application_passwords_endpoint;
pub mod categories_endpoint;
pub mod comments_endpoint;
pub mod media_endpoint;
pub mod plugins_endpoint;
pub mod post_types_endpoint;
pub mod posts_endpoint;
pub mod search_endpoint;
pub mod site_settings_endpoint;
pub mod tags_endpoint;
pub mod taxonomies_endpoint;
pub mod templates_endpoint;
pub mod themes_endpoint;
pub mod users_endpoint;
pub mod wp_site_health_tests_endpoint;

pub const WP_JSON_PATH_SEGMENTS: [&str; 1] = ["wp-json"];

uniffi::custom_newtype!(WpEndpointUrl, String);
#[derive(Debug, Clone)]
pub struct WpEndpointUrl(pub String);

impl From<Url> for WpEndpointUrl {
    fn from(url: Url) -> Self {
        Self(url.to_string())
    }
}

impl From<WpEndpointUrl> for String {
    fn from(url: WpEndpointUrl) -> Self {
        url.0
    }
}

#[derive(Debug)]
pub struct ApiEndpointUrl {
    url: Url,
}

impl ApiEndpointUrl {
    pub fn new(url: Url) -> Self {
        Self { url }
    }

    pub fn as_str(&self) -> &str {
        self.url.as_str()
    }
}

impl From<Url> for ApiEndpointUrl {
    fn from(url: Url) -> Self {
        Self::new(url)
    }
}

impl From<ApiEndpointUrl> for WpEndpointUrl {
    fn from(url: ApiEndpointUrl) -> Self {
        Self(url.as_str().to_string())
    }
}

pub trait DerivedRequest {
    // This can be used to add additional parameters to a request if it has no params type.
    //
    // For example, `/posts` request has `Trash` & `Delete` variants. These variants don't have a
    // params type such as `PostTrashParams` or `PostDeleteParams`. However, they still need to
    // pass some static parameters, `force=false` and `force=true` respectively.
    //
    // In most cases overriding this shouldn't be necessary and `[AppendUrlQueryPairs]` trait for
    // the request's params type should be used instead.
    fn additional_query_pairs(&self) -> Vec<(&str, String)> {
        Vec::new()
    }

    fn namespace() -> impl AsNamespace;
}

pub trait AsNamespace {
    fn as_str(&self) -> &str;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum WpNamespace {
    WpSiteHealthV1,
    WpV2,
}

impl AsNamespace for WpNamespace {
    fn as_str(&self) -> &str {
        match self {
            Self::WpSiteHealthV1 => "/wp-site-health/v1",
            Self::WpV2 => "/wp/v2",
        }
    }
}

mod macros {
    macro_rules! default_sparse_field_implementation_from_field_name {
        ($ident:ident) => {
            paste::paste! {
                impl SparseField for $ident {
                    fn as_str(&self) -> &str {
                        self.as_field_name()
                    }
                }
            }
        };
    }

    pub(crate) use default_sparse_field_implementation_from_field_name;
}

#[cfg(test)]
mod tests {
    use crate::ParsedUrl;

    use super::*;
    use rstest::*;
    use std::sync::Arc;

    #[fixture]
    pub fn fixture_api_base_url() -> Arc<ParsedUrl> {
        ParsedUrl::try_from("https://example.com/wp-json")
            .unwrap()
            .into()
    }

    pub fn validate_wp_v2_endpoint(endpoint_url: ApiEndpointUrl, path: &str) {
        validate_endpoint(WpNamespace::WpV2, endpoint_url, path);
    }

    pub fn validate_wp_site_health_endpoint(endpoint_url: ApiEndpointUrl, path: &str) {
        validate_endpoint(WpNamespace::WpSiteHealthV1, endpoint_url, path);
    }

    fn validate_endpoint(namespace: WpNamespace, endpoint_url: ApiEndpointUrl, path: &str) {
        assert_eq!(
            endpoint_url.as_str(),
            format!(
                "{}{}{}",
                fixture_api_base_url().as_str(),
                namespace.as_str(),
                path
            )
        );
    }
}

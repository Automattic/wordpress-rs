use crate::parsed_url::ParsedUrl;
use std::sync::Arc;
use strum_macros::EnumIter;
use url::Url;

pub mod api_root_endpoint;
pub mod application_passwords_endpoint;
pub mod block_types_endpoint;
pub mod comments_endpoint;
pub mod media_endpoint;
pub mod menu_locations_endpoint;
pub mod nav_menu_item_autosaves_endpoint;
pub mod nav_menu_items_endpoint;
pub mod nav_menus_endpoint;
pub mod navigation_autosaves_endpoint;
pub mod navigation_revisions_endpoint;
pub mod navigations_endpoint;
pub mod plugins_endpoint;
pub mod post_autosaves_endpoint;
pub mod post_revisions_endpoint;
pub mod post_statuses_endpoint;
pub mod post_types_endpoint;
pub mod posts_endpoint;
pub mod search_endpoint;
pub mod site_settings_endpoint;
pub mod taxonomies_endpoint;
pub mod template_autosaves_endpoint;
pub mod template_part_autosaves_endpoint;
pub mod template_part_revisions_endpoint;
pub mod template_parts_endpoint;
pub mod template_revisions_endpoint;
pub mod templates_endpoint;
pub mod terms_endpoint;
pub mod themes_endpoint;
pub mod users_endpoint;
pub mod widget_types_endpoint;
pub mod widgets_endpoint;
pub mod wp_block_editor_endpoint;
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

    fn namespace(&self) -> impl AsNamespace;
}

pub trait AsNamespace: Send + Sync {
    fn namespace_value(&self) -> &'static str;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, EnumIter)]
pub enum WpNamespace {
    None,
    WpBlockEditorV1,
    WpSiteHealthV1,
    WpV2,
}

impl AsNamespace for WpNamespace {
    fn namespace_value(&self) -> &'static str {
        match self {
            Self::None => "",
            Self::WpBlockEditorV1 => "/wp-block-editor/v1",
            Self::WpSiteHealthV1 => "/wp-site-health/v1",
            Self::WpV2 => "/wp/v2",
        }
    }
}

#[uniffi::export(with_foreign)]
pub trait ApiUrlResolver: Send + Sync {
    fn resolve(&self, namespace: String, endpoint_segments: Vec<String>) -> Arc<ParsedUrl>;

    /// Returns the route key for an endpoint, matching the keys used in
    /// `WpApiDetails.routes`. Implementations must produce the same path
    /// structure that `resolve` would produce after the base URL.
    fn route_path(&self, namespace: String, endpoint_path: String) -> String;
}

#[derive(Debug, uniffi::Object)]
pub struct WpOrgSiteApiUrlResolver {
    pub api_root_url: Arc<ParsedUrl>,
}

#[uniffi::export]
impl WpOrgSiteApiUrlResolver {
    #[uniffi::constructor]
    pub fn new(api_root_url: Arc<ParsedUrl>) -> Self {
        Self { api_root_url }
    }

    fn api_root_url(&self) -> Arc<ParsedUrl> {
        self.api_root_url.clone()
    }
}

#[uniffi::export]
impl ApiUrlResolver for WpOrgSiteApiUrlResolver {
    fn resolve(&self, namespace: String, endpoint_segments: Vec<String>) -> Arc<ParsedUrl> {
        Arc::new(
            self.api_root_url
                .by_extending_and_splitting_by_forward_slash(
                    [namespace].into_iter().chain(endpoint_segments),
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
mod tests {
    use super::*;
    use crate::parsed_url::ParsedUrl;
    use rstest::*;
    use std::sync::Arc;

    const EXAMPLE_WP_JSON_URL: &str = "https://example.com/wp-json";

    #[fixture]
    pub fn fixture_wp_org_site_api_url_resolver() -> Arc<dyn ApiUrlResolver> {
        Arc::new(WpOrgSiteApiUrlResolver::new(
            ParsedUrl::try_from(EXAMPLE_WP_JSON_URL)
                .expect("Example wp json url is valid")
                .into(),
        ))
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
                EXAMPLE_WP_JSON_URL,
                namespace.namespace_value(),
                path
            )
        );
    }
}

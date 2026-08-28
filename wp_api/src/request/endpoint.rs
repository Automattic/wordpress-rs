use crate::parsed_url::ParsedUrl;
use crate::resolved_url::ResolvedUrl;
use std::sync::Arc;
use strum_macros::EnumIter;
use url::Url;

pub mod api_root_endpoint;
pub mod application_passwords_endpoint;
pub mod block_autosaves_endpoint;
pub mod block_directory_endpoint;
pub mod block_pattern_categories_endpoint;
pub mod block_patterns_endpoint;
pub mod block_renderer_endpoint;
pub mod block_revisions_endpoint;
pub mod block_types_endpoint;
pub mod blocks_endpoint;
pub mod comments_endpoint;
pub mod global_styles_endpoint;
pub mod global_styles_revisions_endpoint;
pub mod media_endpoint;
pub mod menu_locations_endpoint;
pub mod nav_menu_item_autosaves_endpoint;
pub mod nav_menu_items_endpoint;
pub mod nav_menus_endpoint;
pub mod navigation_autosaves_endpoint;
pub mod navigation_revisions_endpoint;
pub mod navigations_endpoint;
pub mod pattern_directory_endpoint;
pub mod plugins_endpoint;
pub mod post_autosaves_endpoint;
pub mod post_revisions_endpoint;
pub mod post_statuses_endpoint;
pub mod post_types_endpoint;
pub mod posts_endpoint;
pub mod search_endpoint;
pub mod sidebars_endpoint;
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

#[cfg(test)]
mod index_self_href_url_tests;
#[cfg(test)]
mod plain_permalinks_url_tests;

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
    /// Resolves an endpoint to a [`ResolvedUrl`], which carries both the request
    /// URL to fetch (`url()`) and the canonical, origin-less route key that
    /// preloading editors match on (`canonical_route_key()`).
    fn resolve(&self, namespace: String, endpoint_segments: Vec<String>) -> Arc<ResolvedUrl>;

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
    fn resolve(&self, namespace: String, endpoint_segments: Vec<String>) -> Arc<ResolvedUrl> {
        let route_path = self.route_path(namespace.clone(), endpoint_segments.join("/"));
        let url = self
            .api_root_url
            .by_extending_rest_api_path([namespace].into_iter().chain(endpoint_segments));
        ResolvedUrl::new(
            Arc::new(ParsedUrl::new(url)),
            self.api_root_url.clone(),
            route_path,
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

    #[rstest]
    #[case::pretty_permalinks(
        "https://example.com/wp-json",
        "/wp/v2",
        vec!["users".to_string(), "me".to_string()],
        "https://example.com/wp-json/wp/v2/users/me"
    )]
    #[case::plain_permalinks_rest_route_form(
        "https://example.com/index.php?rest_route=/",
        "/wp/v2",
        vec!["users".to_string(), "me".to_string()],
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fusers%2Fme"
    )]
    fn wp_org_resolver_handles_both_api_root_forms(
        #[case] api_root: &str,
        #[case] namespace: &str,
        #[case] endpoint_segments: Vec<String>,
        #[case] expected: &str,
    ) {
        let resolver =
            WpOrgSiteApiUrlResolver::new(ParsedUrl::parse(api_root).expect("valid url").into());
        assert_eq!(
            resolver
                .resolve(namespace.to_string(), endpoint_segments)
                .url(),
            expected
        );
    }

    /// The consumer flow the query-append primitive unblocks:
    /// `resolver.resolve(ns, segments).by_appending_query_pairs([...])`, correct
    /// on both API-root forms.
    #[rstest]
    #[case::pretty_permalinks(
        "https://example.com/wp-json",
        "https://example.com/wp-json/wp/v2/themes?context=edit&exclude=core%2Cgutenberg"
    )]
    #[case::plain_permalinks_rest_route_form(
        "https://example.com/index.php?rest_route=/",
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fthemes&context=edit&exclude=core%2Cgutenberg"
    )]
    #[case::plain_permalinks_rest_route_no_trailing_slash(
        "https://example.com/index.php?rest_route=",
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fthemes&context=edit&exclude=core%2Cgutenberg"
    )]
    fn resolve_then_append_query_pairs(#[case] api_root: &str, #[case] expected: &str) {
        use crate::parsed_url::QueryPair;

        let resolver =
            WpOrgSiteApiUrlResolver::new(ParsedUrl::parse(api_root).expect("valid url").into());
        let resolved = resolver.resolve(
            WpNamespace::WpV2.namespace_value().to_string(),
            vec!["themes".to_string()],
        );
        let result = resolved.by_appending_query_pairs(vec![
            QueryPair {
                name: "context".to_string(),
                value: "edit".to_string(),
            },
            QueryPair {
                name: "exclude".to_string(),
                value: "core,gutenberg".to_string(),
            },
        ]);
        assert_eq!(result.url(), expected);
        // The canonical route key is origin-less and identical on every API-root
        // form — the request URLs above differ, but the key does not.
        assert_eq!(
            result.canonical_route_key(),
            "/wp/v2/themes?context=edit&exclude=core%2Cgutenberg"
        );
    }

    /// End-to-end through `resolve()`: for the same endpoint, a pretty
    /// (`…/wp-json/…`) and a plain (`…?rest_route=…`) API root build different
    /// request URLs but the **same** `canonical_route_key()` — the invariant the
    /// preload middleware relies on. Covers a query-less key (no `?`), multiple
    /// params, an encoded value, and a route with embedded slashes.
    #[rstest]
    #[case::two_pairs(
        "themes",
        vec![("context", "edit"), ("status", "active")],
        "https://example.com/wp-json/wp/v2/themes?context=edit&status=active",
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fthemes&context=edit&status=active",
        "/wp/v2/themes?context=edit&status=active"
    )]
    #[case::no_query(
        "themes",
        vec![],
        "https://example.com/wp-json/wp/v2/themes",
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fthemes",
        "/wp/v2/themes"
    )]
    #[case::encoded_value(
        "themes",
        vec![("exclude", "core,gutenberg")],
        "https://example.com/wp-json/wp/v2/themes?exclude=core%2Cgutenberg",
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fthemes&exclude=core%2Cgutenberg",
        "/wp/v2/themes?exclude=core%2Cgutenberg"
    )]
    #[case::embedded_slash_route(
        "block-renderer/core/paragraph",
        vec![("context", "edit")],
        "https://example.com/wp-json/wp/v2/block-renderer/core/paragraph?context=edit",
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fblock-renderer%2Fcore%2Fparagraph&context=edit",
        "/wp/v2/block-renderer/core/paragraph?context=edit"
    )]
    fn resolved_url_request_and_key(
        #[case] segment: &str,
        #[case] pairs: Vec<(&str, &str)>,
        #[case] expected_pretty_url: &str,
        #[case] expected_plain_url: &str,
        #[case] expected_key: &str,
    ) {
        use crate::parsed_url::QueryPair;

        for (root, expected_url) in [
            ("https://example.com/wp-json", expected_pretty_url),
            (
                "https://example.com/index.php?rest_route=/",
                expected_plain_url,
            ),
        ] {
            let resolver =
                WpOrgSiteApiUrlResolver::new(ParsedUrl::parse(root).expect("valid url").into());
            let resolved = resolver
                .resolve(
                    WpNamespace::WpV2.namespace_value().to_string(),
                    vec![segment.to_string()],
                )
                .by_appending_query_pairs(
                    pairs
                        .iter()
                        .map(|(name, value)| QueryPair {
                            name: name.to_string(),
                            value: value.to_string(),
                        })
                        .collect(),
                );
            assert_eq!(resolved.url(), expected_url);
            // Identical for BOTH roots.
            assert_eq!(resolved.canonical_route_key(), expected_key);
        }
    }
}

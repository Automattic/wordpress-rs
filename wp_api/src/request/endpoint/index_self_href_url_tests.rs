//! Full-URL oracle: for each non-parameterized route that a
//! `request::endpoint` builder targets and that WordPress publishes in the REST
//! index, assert our resolver builds *exactly* the URL the server publishes in
//! that route's `_links.self.href`.
//!
//! Each expected URL is read at run time from the committed real-site index
//! fixture `test-data/api-details/test-case-03.json` (a self-hosted `wp-json`
//! site) — WordPress's own `rest_url()` output, looked up by route with
//! [`published_self_href`]. Nothing here is hand-transcribed: the only literal
//! each test carries is the route key it looks up, and a mistyped key fails
//! loudly because the fixture won't contain it. The fixture — i.e. the server —
//! is the source of truth, not a golden copied into this file.
//!
//! The REST index only emits a concrete `self` href for routes without path
//! parameters, so this checks 38 routes across 29 endpoint modules. Two
//! published routes are deliberately left out:
//! - `/` (the API root): the index publishes `…/wp-json/` while
//!   `ApiRootRequestEndpoint` builds `…/wp-json`, so an exact comparison would
//!   fail on the trailing slash alone.
//! - `/jetpack/v4/connection*`: built by the Jetpack client, not a
//!   `request::endpoint` module.
//!
//! This validates the route *path* on the `wp-json` form only. The ID-bearing
//! routes, which the index never publishes a URL for, have no server-sourced
//! check here; one representative (`/wp/v2/users/<id>`) is fetched end-to-end
//! over `?rest_route=` by the plain-permalinks integration test.
//!
//! The query string our representative call appends (`?context=edit`, etc.) is
//! stripped before comparison, since the advertised `self` href is the bare
//! route URL.

use super::block_directory_endpoint::BlockDirectoryRequestEndpoint;
use super::block_pattern_categories_endpoint::BlockPatternCategoriesRequestEndpoint;
use super::block_patterns_endpoint::BlockPatternsRequestEndpoint;
use super::block_types_endpoint::BlockTypesRequestEndpoint;
use super::blocks_endpoint::BlocksRequestEndpoint;
use super::comments_endpoint::CommentsRequestEndpoint;
use super::media_endpoint::MediaRequestEndpoint;
use super::menu_locations_endpoint::MenuLocationsRequestEndpoint;
use super::nav_menu_items_endpoint::NavMenuItemsRequestEndpoint;
use super::nav_menus_endpoint::NavMenusRequestEndpoint;
use super::navigations_endpoint::NavigationsRequestEndpoint;
use super::pattern_directory_endpoint::PatternDirectoryRequestEndpoint;
use super::plugins_endpoint::PluginsRequestEndpoint;
use super::post_statuses_endpoint::PostStatusesRequestEndpoint;
use super::post_types_endpoint::PostTypesRequestEndpoint;
use super::posts_endpoint::PostEndpointType;
use super::posts_endpoint::PostsRequestEndpoint;
use super::search_endpoint::SearchRequestEndpoint;
use super::sidebars_endpoint::SidebarsRequestEndpoint;
use super::site_settings_endpoint::SiteSettingsRequestEndpoint;
use super::taxonomies_endpoint::TaxonomiesRequestEndpoint;
use super::template_parts_endpoint::TemplatePartsRequestEndpoint;
use super::templates_endpoint::TemplatesRequestEndpoint;
use super::terms_endpoint::TermEndpointType;
use super::terms_endpoint::TermsRequestEndpoint;
use super::themes_endpoint::ThemesRequestEndpoint;
use super::users_endpoint::UsersRequestEndpoint;
use super::widget_types_endpoint::WidgetTypesRequestEndpoint;
use super::widgets_endpoint::WidgetsRequestEndpoint;
use super::wp_block_editor_endpoint::WpBlockEditorRequestEndpoint;
use super::wp_site_health_tests_endpoint::WpSiteHealthTestsRequestEndpoint;
use super::{ApiUrlResolver, WpOrgSiteApiUrlResolver};
use crate::block_directory::BlockDirectorySearchParams;
use crate::blocks::BlockListParams;
use crate::comments::CommentListParams;
use crate::media::MediaListParams;
use crate::nav_menu_items::NavMenuItemListParams;
use crate::nav_menus::NavMenuListParams;
use crate::navigations::NavigationListParams;
use crate::parsed_url::ParsedUrl;
use crate::pattern_directory::PatternDirectoryCategoryId;
use crate::pattern_directory::PatternDirectoryListParams;
use crate::posts::PostListParams;
use crate::search_results::SearchListParams;
use crate::taxonomies::TaxonomyListParams;
use crate::template_parts::TemplatePartListParams;
use crate::templates::TemplateListParams;
use crate::terms::TermListParams;
use crate::themes::ThemeListParams;
use crate::widgets::WidgetListParams;
use crate::wp_block_editor::WpBlockEditorSettingsContext;
use crate::wp_block_editor::WpBlockEditorSettingsParams;
use crate::wp_site_health_tests::SparseWpSiteHealthTestField;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock};

const API_ROOT: &str = "https://jetpack.wpmt.co/wp-json";

/// The committed real-site REST index fixture, parsed once for the whole module
/// (it is ~770 KB, so re-reading and re-parsing it per assertion would be
/// wasteful).
static REST_INDEX: LazyLock<Value> = LazyLock::new(|| {
    let mut path = PathBuf::from(env!("CARGO_WORKSPACE_DIR"));
    path.push("test-data");
    path.push("api-details");
    path.push("test-case-03.json");
    let json = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read REST index fixture {}: {e}", path.display()));
    serde_json::from_str(&json).expect("REST index fixture is valid JSON")
});

fn resolver() -> Arc<dyn ApiUrlResolver> {
    Arc::new(WpOrgSiteApiUrlResolver::new(
        ParsedUrl::parse(API_ROOT).expect("valid url").into(),
    ))
}

fn strip_query(url: &str) -> &str {
    url.split_once('?').map_or(url, |(base, _)| base)
}

/// The exact URL WordPress published for `route` in the committed REST index
/// fixture (`test-data/api-details/test-case-03.json`) — its own `rest_url()`
/// output, read from that route's `_links.self[0].href`.
///
/// Panics if the fixture has no such route, so a mistyped route key fails the
/// test loudly instead of silently asserting against a value invented here.
fn published_self_href(route: &str) -> String {
    REST_INDEX["routes"][route]["_links"]["self"][0]["href"]
        .as_str()
        .unwrap_or_else(|| panic!("fixture publishes no `_links.self[0].href` for route `{route}`"))
        .to_string()
}

#[test]
fn block_directory() {
    let endpoint = BlockDirectoryRequestEndpoint::new(resolver());
    let built = endpoint
        .search(&BlockDirectorySearchParams::new("coblocks".to_string()))
        .as_str()
        .to_string();
    assert_eq!(
        strip_query(&built),
        published_self_href("/wp/v2/block-directory/search")
    );
}

#[test]
fn block_pattern_categories() {
    let endpoint = BlockPatternCategoriesRequestEndpoint::new(resolver());
    let built = endpoint.list_with_edit_context().as_str().to_string();
    assert_eq!(
        strip_query(&built),
        published_self_href("/wp/v2/block-patterns/categories")
    );
}

#[test]
fn block_patterns() {
    let endpoint = BlockPatternsRequestEndpoint::new(resolver());
    let built = endpoint.list_with_edit_context().as_str().to_string();
    assert_eq!(
        strip_query(&built),
        published_self_href("/wp/v2/block-patterns/patterns")
    );
}

#[test]
fn block_types() {
    let endpoint = BlockTypesRequestEndpoint::new(resolver());
    let built = endpoint.list_with_edit_context().as_str().to_string();
    assert_eq!(
        strip_query(&built),
        published_self_href("/wp/v2/block-types")
    );
}

#[test]
fn blocks() {
    let endpoint = BlocksRequestEndpoint::new(resolver());
    let built = endpoint
        .list_with_edit_context(&BlockListParams::default())
        .as_str()
        .to_string();
    assert_eq!(strip_query(&built), published_self_href("/wp/v2/blocks"));
}

#[test]
fn comments() {
    let endpoint = CommentsRequestEndpoint::new(resolver());
    let built = endpoint
        .list_with_edit_context(&CommentListParams::default())
        .as_str()
        .to_string();
    assert_eq!(strip_query(&built), published_self_href("/wp/v2/comments"));
}

#[test]
fn media() {
    let endpoint = MediaRequestEndpoint::new(resolver());
    let built = endpoint
        .list_with_edit_context(&MediaListParams::default())
        .as_str()
        .to_string();
    assert_eq!(strip_query(&built), published_self_href("/wp/v2/media"));
}

#[test]
fn menu_locations() {
    let endpoint = MenuLocationsRequestEndpoint::new(resolver());
    let built = endpoint.list_with_edit_context().as_str().to_string();
    assert_eq!(
        strip_query(&built),
        published_self_href("/wp/v2/menu-locations")
    );
}

#[test]
fn navigations() {
    let endpoint = NavigationsRequestEndpoint::new(resolver());
    let built = endpoint
        .list_with_edit_context(&NavigationListParams::default())
        .as_str()
        .to_string();
    assert_eq!(
        strip_query(&built),
        published_self_href("/wp/v2/navigation")
    );
}

#[test]
fn pattern_directory() {
    let endpoint = PatternDirectoryRequestEndpoint::new(resolver());
    let params = PatternDirectoryListParams {
        per_page: Some(10),
        category: Some(PatternDirectoryCategoryId(5)),
        ..Default::default()
    };
    let built = endpoint
        .list_with_view_context(&params)
        .as_str()
        .to_string();
    assert_eq!(
        strip_query(&built),
        published_self_href("/wp/v2/pattern-directory/patterns")
    );
}

#[test]
fn post_statuses() {
    let endpoint = PostStatusesRequestEndpoint::new(resolver());
    let built = endpoint.list_with_edit_context().as_str().to_string();
    assert_eq!(strip_query(&built), published_self_href("/wp/v2/statuses"));
}

#[test]
fn post_types() {
    let endpoint = PostTypesRequestEndpoint::new(resolver());
    let built = endpoint.list_with_edit_context().as_str().to_string();
    assert_eq!(strip_query(&built), published_self_href("/wp/v2/types"));
}

#[test]
fn posts() {
    let endpoint = PostsRequestEndpoint::new(resolver());
    let built = endpoint
        .list_with_edit_context(&PostEndpointType::Posts, &PostListParams::default())
        .as_str()
        .to_string();
    assert_eq!(strip_query(&built), published_self_href("/wp/v2/posts"));
}

#[test]
fn search() {
    let endpoint = SearchRequestEndpoint::new(resolver());
    let built = endpoint
        .list_with_embed_context(&SearchListParams::default())
        .as_str()
        .to_string();
    assert_eq!(strip_query(&built), published_self_href("/wp/v2/search"));
}

#[test]
fn sidebars() {
    let endpoint = SidebarsRequestEndpoint::new(resolver());
    let built = endpoint.list_with_edit_context().as_str().to_string();
    assert_eq!(strip_query(&built), published_self_href("/wp/v2/sidebars"));
}

#[test]
fn site_settings() {
    let endpoint = SiteSettingsRequestEndpoint::new(resolver());
    let built = endpoint.retrieve_with_edit_context().as_str().to_string();
    assert_eq!(strip_query(&built), published_self_href("/wp/v2/settings"));
}

#[test]
fn taxonomies() {
    let endpoint = TaxonomiesRequestEndpoint::new(resolver());
    let built = endpoint
        .list_with_edit_context(&TaxonomyListParams::default())
        .as_str()
        .to_string();
    assert_eq!(
        strip_query(&built),
        published_self_href("/wp/v2/taxonomies")
    );
}

#[test]
fn users() {
    let endpoint = UsersRequestEndpoint::new(resolver());
    let built = endpoint
        .list_with_edit_context(&crate::UserListParams::default())
        .as_str()
        .to_string();
    assert_eq!(strip_query(&built), published_self_href("/wp/v2/users"));
}

#[test]
fn widget_types() {
    let endpoint = WidgetTypesRequestEndpoint::new(resolver());
    let built = endpoint.list_with_edit_context().as_str().to_string();
    assert_eq!(
        strip_query(&built),
        published_self_href("/wp/v2/widget-types")
    );
}

#[test]
fn widgets() {
    let endpoint = WidgetsRequestEndpoint::new(resolver());
    let built = endpoint
        .list_with_edit_context(&WidgetListParams::default())
        .as_str()
        .to_string();
    assert_eq!(strip_query(&built), published_self_href("/wp/v2/widgets"));
}

#[test]
fn wp_block_editor() {
    let endpoint = WpBlockEditorRequestEndpoint::new(resolver());
    let params = WpBlockEditorSettingsParams {
        context: Some(WpBlockEditorSettingsContext::WidgetsEditor),
    };
    let built = endpoint.retrieve_settings(&params).as_str().to_string();
    assert_eq!(
        strip_query(&built),
        published_self_href("/wp-block-editor/v1/settings")
    );
}

#[test]
fn wp_site_health_tests() {
    let endpoint = WpSiteHealthTestsRequestEndpoint::new(resolver());
    let built = endpoint
        .filter_background_updates(&[
            SparseWpSiteHealthTestField::Actions,
            SparseWpSiteHealthTestField::Badge,
        ])
        .as_str()
        .to_string();
    assert_eq!(
        strip_query(&built),
        published_self_href("/wp-site-health/v1/tests/background-updates")
    );
}

// --- Collection routes the index also publishes a `self` href for. Their
// `plain_permalinks_url_tests` entries exercise the parameterized (retrieve)
// form, so the server-sourced oracle here locks the shared route-path prefix. ---

#[test]
fn themes() {
    let endpoint = ThemesRequestEndpoint::new(resolver());
    let built = endpoint
        .list_with_edit_context(&ThemeListParams::default())
        .as_str()
        .to_string();
    assert_eq!(strip_query(&built), published_self_href("/wp/v2/themes"));
}

#[test]
fn plugins() {
    let endpoint = PluginsRequestEndpoint::new(resolver());
    let built = endpoint
        .list_with_edit_context(&crate::PluginListParams::default())
        .as_str()
        .to_string();
    assert_eq!(strip_query(&built), published_self_href("/wp/v2/plugins"));
}

#[test]
fn templates() {
    let endpoint = TemplatesRequestEndpoint::new(resolver());
    let built = endpoint
        .list_with_edit_context(&TemplateListParams::default())
        .as_str()
        .to_string();
    assert_eq!(strip_query(&built), published_self_href("/wp/v2/templates"));
}

#[test]
fn template_parts() {
    let endpoint = TemplatePartsRequestEndpoint::new(resolver());
    let built = endpoint
        .list_with_edit_context(&TemplatePartListParams::default())
        .as_str()
        .to_string();
    assert_eq!(
        strip_query(&built),
        published_self_href("/wp/v2/template-parts")
    );
}

#[test]
fn nav_menus() {
    let endpoint = NavMenusRequestEndpoint::new(resolver());
    let built = endpoint
        .list_with_edit_context(&NavMenuListParams::default())
        .as_str()
        .to_string();
    assert_eq!(strip_query(&built), published_self_href("/wp/v2/menus"));
}

#[test]
fn nav_menu_items() {
    let endpoint = NavMenuItemsRequestEndpoint::new(resolver());
    let built = endpoint
        .list_with_edit_context(&NavMenuItemListParams::default())
        .as_str()
        .to_string();
    assert_eq!(
        strip_query(&built),
        published_self_href("/wp/v2/menu-items")
    );
}

#[test]
fn categories() {
    let endpoint = TermsRequestEndpoint::new(resolver());
    let built = endpoint
        .list_with_edit_context(&TermEndpointType::Categories, &TermListParams::default())
        .as_str()
        .to_string();
    assert_eq!(
        strip_query(&built),
        published_self_href("/wp/v2/categories")
    );
}

#[test]
fn tags() {
    let endpoint = TermsRequestEndpoint::new(resolver());
    let built = endpoint
        .list_with_edit_context(&TermEndpointType::Tags, &TermListParams::default())
        .as_str()
        .to_string();
    assert_eq!(strip_query(&built), published_self_href("/wp/v2/tags"));
}

// --- Other non-parameterized routes the index publishes a `self` href for, on
// modules already covered above. ---

#[test]
fn users_me() {
    let endpoint = UsersRequestEndpoint::new(resolver());
    let built = endpoint
        .retrieve_me_with_edit_context()
        .as_str()
        .to_string();
    assert_eq!(strip_query(&built), published_self_href("/wp/v2/users/me"));
}

#[test]
fn pages() {
    let endpoint = PostsRequestEndpoint::new(resolver());
    let built = endpoint
        .list_with_edit_context(&PostEndpointType::Pages, &PostListParams::default())
        .as_str()
        .to_string();
    assert_eq!(strip_query(&built), published_self_href("/wp/v2/pages"));
}

#[test]
fn wp_site_health_tests_remaining_routes() {
    let endpoint = WpSiteHealthTestsRequestEndpoint::new(resolver());
    for (built, route) in [
        (
            endpoint.loopback_requests(),
            "/wp-site-health/v1/tests/loopback-requests",
        ),
        (
            endpoint.https_status(),
            "/wp-site-health/v1/tests/https-status",
        ),
        (
            endpoint.dotorg_communication(),
            "/wp-site-health/v1/tests/dotorg-communication",
        ),
        (
            endpoint.authorization_header(),
            "/wp-site-health/v1/tests/authorization-header",
        ),
        (endpoint.page_cache(), "/wp-site-health/v1/tests/page-cache"),
        (
            endpoint.directory_sizes(),
            "/wp-site-health/v1/directory-sizes",
        ),
    ] {
        assert_eq!(
            strip_query(built.as_str()),
            published_self_href(route),
            "route `{route}`"
        );
    }
}

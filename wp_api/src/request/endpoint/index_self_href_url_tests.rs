//! Full-URL oracle: for every self-hosted endpoint that WordPress advertises as
//! a non-parameterized route, assert our resolver builds *exactly* the URL the
//! server publishes in the REST index's `_links.self.href`.
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
//! parameters, so this covers the 30 collection/singleton endpoints; the
//! ID-bearing ones are covered end-to-end by the plain-permalinks integration
//! tests instead.
//!
//! The query string our representative call appends (`?context=edit`, etc.) is
//! stripped before comparison, since the advertised `self` href is the bare
//! route URL.

use super::{ApiUrlResolver, WpOrgSiteApiUrlResolver};
use crate::parsed_url::ParsedUrl;
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
    url.split('?').next().unwrap_or(url)
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
    let endpoint = super::block_directory_endpoint::BlockDirectoryRequestEndpoint::new(resolver());
    let built = endpoint
        .search(&crate::block_directory::BlockDirectorySearchParams::new(
            "coblocks".to_string(),
        ))
        .as_str()
        .to_string();
    assert_eq!(
        strip_query(&built),
        published_self_href("/wp/v2/block-directory/search")
    );
}

#[test]
fn block_pattern_categories() {
    let endpoint =
        super::block_pattern_categories_endpoint::BlockPatternCategoriesRequestEndpoint::new(
            resolver(),
        );
    let built = endpoint.list_with_edit_context().as_str().to_string();
    assert_eq!(
        strip_query(&built),
        published_self_href("/wp/v2/block-patterns/categories")
    );
}

#[test]
fn block_patterns() {
    let endpoint = super::block_patterns_endpoint::BlockPatternsRequestEndpoint::new(resolver());
    let built = endpoint.list_with_edit_context().as_str().to_string();
    assert_eq!(
        strip_query(&built),
        published_self_href("/wp/v2/block-patterns/patterns")
    );
}

#[test]
fn block_types() {
    let endpoint = super::block_types_endpoint::BlockTypesRequestEndpoint::new(resolver());
    let built = endpoint.list_with_edit_context().as_str().to_string();
    assert_eq!(
        strip_query(&built),
        published_self_href("/wp/v2/block-types")
    );
}

#[test]
fn blocks() {
    let endpoint = super::blocks_endpoint::BlocksRequestEndpoint::new(resolver());
    let built = endpoint
        .list_with_edit_context(&crate::blocks::BlockListParams::default())
        .as_str()
        .to_string();
    assert_eq!(strip_query(&built), published_self_href("/wp/v2/blocks"));
}

#[test]
fn comments() {
    let endpoint = super::comments_endpoint::CommentsRequestEndpoint::new(resolver());
    let built = endpoint
        .list_with_edit_context(&crate::comments::CommentListParams::default())
        .as_str()
        .to_string();
    assert_eq!(strip_query(&built), published_self_href("/wp/v2/comments"));
}

#[test]
fn media() {
    let endpoint = super::media_endpoint::MediaRequestEndpoint::new(resolver());
    let built = endpoint
        .list_with_edit_context(&crate::media::MediaListParams::default())
        .as_str()
        .to_string();
    assert_eq!(strip_query(&built), published_self_href("/wp/v2/media"));
}

#[test]
fn menu_locations() {
    let endpoint = super::menu_locations_endpoint::MenuLocationsRequestEndpoint::new(resolver());
    let built = endpoint.list_with_edit_context().as_str().to_string();
    assert_eq!(
        strip_query(&built),
        published_self_href("/wp/v2/menu-locations")
    );
}

#[test]
fn navigations() {
    let endpoint = super::navigations_endpoint::NavigationsRequestEndpoint::new(resolver());
    let built = endpoint
        .list_with_edit_context(&crate::navigations::NavigationListParams::default())
        .as_str()
        .to_string();
    assert_eq!(
        strip_query(&built),
        published_self_href("/wp/v2/navigation")
    );
}

#[test]
fn pattern_directory() {
    let endpoint =
        super::pattern_directory_endpoint::PatternDirectoryRequestEndpoint::new(resolver());
    let params = crate::pattern_directory::PatternDirectoryListParams {
        per_page: Some(10),
        category: Some(crate::pattern_directory::PatternDirectoryCategoryId(5)),
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
    let endpoint = super::post_statuses_endpoint::PostStatusesRequestEndpoint::new(resolver());
    let built = endpoint.list_with_edit_context().as_str().to_string();
    assert_eq!(strip_query(&built), published_self_href("/wp/v2/statuses"));
}

#[test]
fn post_types() {
    let endpoint = super::post_types_endpoint::PostTypesRequestEndpoint::new(resolver());
    let built = endpoint.list_with_edit_context().as_str().to_string();
    assert_eq!(strip_query(&built), published_self_href("/wp/v2/types"));
}

#[test]
fn posts() {
    let endpoint = super::posts_endpoint::PostsRequestEndpoint::new(resolver());
    let built = endpoint
        .list_with_edit_context(
            &crate::request::endpoint::posts_endpoint::PostEndpointType::Posts,
            &crate::posts::PostListParams::default(),
        )
        .as_str()
        .to_string();
    assert_eq!(strip_query(&built), published_self_href("/wp/v2/posts"));
}

#[test]
fn search() {
    let endpoint = super::search_endpoint::SearchRequestEndpoint::new(resolver());
    let built = endpoint
        .list_with_embed_context(&crate::search_results::SearchListParams::default())
        .as_str()
        .to_string();
    assert_eq!(strip_query(&built), published_self_href("/wp/v2/search"));
}

#[test]
fn sidebars() {
    let endpoint = super::sidebars_endpoint::SidebarsRequestEndpoint::new(resolver());
    let built = endpoint.list_with_edit_context().as_str().to_string();
    assert_eq!(strip_query(&built), published_self_href("/wp/v2/sidebars"));
}

#[test]
fn site_settings() {
    let endpoint = super::site_settings_endpoint::SiteSettingsRequestEndpoint::new(resolver());
    let built = endpoint.retrieve_with_edit_context().as_str().to_string();
    assert_eq!(strip_query(&built), published_self_href("/wp/v2/settings"));
}

#[test]
fn taxonomies() {
    let endpoint = super::taxonomies_endpoint::TaxonomiesRequestEndpoint::new(resolver());
    let built = endpoint
        .list_with_edit_context(&crate::taxonomies::TaxonomyListParams::default())
        .as_str()
        .to_string();
    assert_eq!(
        strip_query(&built),
        published_self_href("/wp/v2/taxonomies")
    );
}

#[test]
fn users() {
    let endpoint = super::users_endpoint::UsersRequestEndpoint::new(resolver());
    let built = endpoint
        .list_with_edit_context(&crate::UserListParams::default())
        .as_str()
        .to_string();
    assert_eq!(strip_query(&built), published_self_href("/wp/v2/users"));
}

#[test]
fn widget_types() {
    let endpoint = super::widget_types_endpoint::WidgetTypesRequestEndpoint::new(resolver());
    let built = endpoint.list_with_edit_context().as_str().to_string();
    assert_eq!(
        strip_query(&built),
        published_self_href("/wp/v2/widget-types")
    );
}

#[test]
fn widgets() {
    let endpoint = super::widgets_endpoint::WidgetsRequestEndpoint::new(resolver());
    let built = endpoint
        .list_with_edit_context(&crate::widgets::WidgetListParams::default())
        .as_str()
        .to_string();
    assert_eq!(strip_query(&built), published_self_href("/wp/v2/widgets"));
}

#[test]
fn wp_block_editor() {
    let endpoint = super::wp_block_editor_endpoint::WpBlockEditorRequestEndpoint::new(resolver());
    let params = crate::wp_block_editor::WpBlockEditorSettingsParams {
        context: Some(crate::wp_block_editor::WpBlockEditorSettingsContext::WidgetsEditor),
    };
    let built = endpoint.retrieve_settings(&params).as_str().to_string();
    assert_eq!(
        strip_query(&built),
        published_self_href("/wp-block-editor/v1/settings")
    );
}

#[test]
fn wp_site_health_tests() {
    let endpoint =
        super::wp_site_health_tests_endpoint::WpSiteHealthTestsRequestEndpoint::new(resolver());
    let built = endpoint
        .filter_background_updates(&[
            crate::wp_site_health_tests::SparseWpSiteHealthTestField::Actions,
            crate::wp_site_health_tests::SparseWpSiteHealthTestField::Badge,
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
    let endpoint = super::themes_endpoint::ThemesRequestEndpoint::new(resolver());
    let built = endpoint
        .list_with_edit_context(&crate::themes::ThemeListParams::default())
        .as_str()
        .to_string();
    assert_eq!(strip_query(&built), published_self_href("/wp/v2/themes"));
}

#[test]
fn plugins() {
    let endpoint = super::plugins_endpoint::PluginsRequestEndpoint::new(resolver());
    let built = endpoint
        .list_with_edit_context(&crate::PluginListParams::default())
        .as_str()
        .to_string();
    assert_eq!(strip_query(&built), published_self_href("/wp/v2/plugins"));
}

#[test]
fn templates() {
    let endpoint = super::templates_endpoint::TemplatesRequestEndpoint::new(resolver());
    let built = endpoint
        .list_with_edit_context(&crate::templates::TemplateListParams::default())
        .as_str()
        .to_string();
    assert_eq!(strip_query(&built), published_self_href("/wp/v2/templates"));
}

#[test]
fn template_parts() {
    let endpoint = super::template_parts_endpoint::TemplatePartsRequestEndpoint::new(resolver());
    let built = endpoint
        .list_with_edit_context(&crate::template_parts::TemplatePartListParams::default())
        .as_str()
        .to_string();
    assert_eq!(
        strip_query(&built),
        published_self_href("/wp/v2/template-parts")
    );
}

#[test]
fn nav_menus() {
    let endpoint = super::nav_menus_endpoint::NavMenusRequestEndpoint::new(resolver());
    let built = endpoint
        .list_with_edit_context(&crate::nav_menus::NavMenuListParams::default())
        .as_str()
        .to_string();
    assert_eq!(strip_query(&built), published_self_href("/wp/v2/menus"));
}

#[test]
fn nav_menu_items() {
    let endpoint = super::nav_menu_items_endpoint::NavMenuItemsRequestEndpoint::new(resolver());
    let built = endpoint
        .list_with_edit_context(&crate::nav_menu_items::NavMenuItemListParams::default())
        .as_str()
        .to_string();
    assert_eq!(
        strip_query(&built),
        published_self_href("/wp/v2/menu-items")
    );
}

#[test]
fn categories() {
    let endpoint = super::terms_endpoint::TermsRequestEndpoint::new(resolver());
    let built = endpoint
        .list_with_edit_context(
            &super::terms_endpoint::TermEndpointType::Categories,
            &crate::terms::TermListParams::default(),
        )
        .as_str()
        .to_string();
    assert_eq!(
        strip_query(&built),
        published_self_href("/wp/v2/categories")
    );
}

#[test]
fn tags() {
    let endpoint = super::terms_endpoint::TermsRequestEndpoint::new(resolver());
    let built = endpoint
        .list_with_edit_context(
            &super::terms_endpoint::TermEndpointType::Tags,
            &crate::terms::TermListParams::default(),
        )
        .as_str()
        .to_string();
    assert_eq!(strip_query(&built), published_self_href("/wp/v2/tags"));
}

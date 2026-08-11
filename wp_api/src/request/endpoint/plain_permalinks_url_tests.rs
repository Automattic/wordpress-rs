//! Golden URL table proving every self-hosted (wp.org) endpoint resolves
//! correctly on a *plain permalinks* site — i.e. when the API root is the
//! `?rest_route=` query form rather than `…/wp-json/`. Each test drives the
//! endpoint's real URL builder (the same code path the typed client uses)
//! through a `rest_route`-seeded [`WpOrgSiteApiUrlResolver`] and compares the
//! built URL against a hard-coded expected string.
//!
//! This is the plain-permalinks counterpart to the per-endpoint `wp-json`
//! golden assertions in each `*_endpoint.rs` module. `?rest_route=` requests
//! are accepted by WordPress regardless of a site's permalink setting, so this
//! table also documents the exact URL each endpoint hits on a plain-permalink
//! site. The `// wp-json form:` comment on each case is the pretty-permalink
//! path the sibling test asserts, for cross-reference.

use super::{ApiUrlResolver, WpOrgSiteApiUrlResolver};
use crate::parsed_url::ParsedUrl;
use std::sync::Arc;

/// A resolver seeded with the plain-permalinks (`?rest_route=/`) API root, on
/// the same `example.com` host the `wp-json` fixture uses.
fn resolver() -> Arc<dyn ApiUrlResolver> {
    Arc::new(WpOrgSiteApiUrlResolver::new(
        ParsedUrl::parse("https://example.com/index.php?rest_route=/")
            .expect("valid url")
            .into(),
    ))
}

#[test]
fn api_root() {
    let endpoint = super::api_root_endpoint::ApiRootRequestEndpoint::new(resolver());
    // wp-json form: https://example.com/wp-json
    assert_eq!(
        endpoint.get().as_str(),
        "https://example.com/index.php?rest_route=%2F",
    );
}

#[test]
fn application_passwords() {
    let endpoint =
        super::application_passwords_endpoint::ApplicationPasswordsRequestEndpoint::new(resolver());
    // wp-json form: /users/2/application-passwords?context=edit
    assert_eq!(
        endpoint
            .list_with_edit_context(&crate::users::UserId(2))
            .as_str(),
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fusers%2F2%2Fapplication-passwords&context=edit",
    );
}

#[test]
fn block_autosaves() {
    let endpoint = super::block_autosaves_endpoint::BlockAutosavesRequestEndpoint::new(resolver());
    // wp-json form: /blocks/42/autosaves?context=edit
    assert_eq!(
        endpoint
            .list_with_edit_context(&crate::blocks::BlockId(42))
            .as_str(),
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fblocks%2F42%2Fautosaves&context=edit",
    );
}

#[test]
fn block_directory() {
    let endpoint = super::block_directory_endpoint::BlockDirectoryRequestEndpoint::new(resolver());
    // wp-json form: /block-directory/search?term=coblocks
    assert_eq!(
        endpoint
            .search(&crate::block_directory::BlockDirectorySearchParams::new(
                "coblocks".to_string()
            ))
            .as_str(),
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fblock-directory%2Fsearch&term=coblocks",
    );
}

#[test]
fn block_pattern_categories() {
    let endpoint =
        super::block_pattern_categories_endpoint::BlockPatternCategoriesRequestEndpoint::new(
            resolver(),
        );
    // wp-json form: /block-patterns/categories?context=edit
    assert_eq!(
        endpoint.list_with_edit_context().as_str(),
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fblock-patterns%2Fcategories&context=edit",
    );
}

#[test]
fn block_patterns() {
    let endpoint = super::block_patterns_endpoint::BlockPatternsRequestEndpoint::new(resolver());
    // wp-json form: /block-patterns/patterns?context=edit
    assert_eq!(
        endpoint.list_with_edit_context().as_str(),
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fblock-patterns%2Fpatterns&context=edit",
    );
}

#[test]
fn block_renderer() {
    let endpoint = super::block_renderer_endpoint::BlockRendererRequestEndpoint::new(resolver());
    // wp-json form: /block-renderer/core/paragraph?context=edit
    assert_eq!(
        endpoint
            .render(&crate::block_renderer::BlockName(
                "core/paragraph".to_string()
            ))
            .as_str(),
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fblock-renderer%2Fcore%2Fparagraph&context=edit",
    );
}

#[test]
fn block_revisions() {
    let endpoint = super::block_revisions_endpoint::BlockRevisionsRequestEndpoint::new(resolver());
    // wp-json form: /blocks/42/revisions?context=edit
    assert_eq!(
        endpoint
            .list_with_edit_context(
                &crate::blocks::BlockId(42),
                &crate::block_revisions::BlockRevisionListParams::default()
            )
            .as_str(),
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fblocks%2F42%2Frevisions&context=edit",
    );
}

#[test]
fn block_types() {
    let endpoint = super::block_types_endpoint::BlockTypesRequestEndpoint::new(resolver());
    // wp-json form: /block-types?context=edit
    assert_eq!(
        endpoint.list_with_edit_context().as_str(),
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fblock-types&context=edit",
    );
}

#[test]
fn blocks() {
    let endpoint = super::blocks_endpoint::BlocksRequestEndpoint::new(resolver());
    // wp-json form: /blocks?context=edit
    assert_eq!(
        endpoint
            .list_with_edit_context(&crate::blocks::BlockListParams::default())
            .as_str(),
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fblocks&context=edit",
    );
}

#[test]
fn comments() {
    let endpoint = super::comments_endpoint::CommentsRequestEndpoint::new(resolver());
    // wp-json form: /comments?context=edit
    assert_eq!(
        endpoint
            .list_with_edit_context(&crate::comments::CommentListParams::default())
            .as_str(),
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fcomments&context=edit",
    );
}

#[test]
fn global_styles() {
    let endpoint = super::global_styles_endpoint::GlobalStylesRequestEndpoint::new(resolver());
    // wp-json form: /global-styles/42?context=edit
    assert_eq!(
        endpoint
            .retrieve_with_edit_context(&crate::global_styles::GlobalStylesId(42))
            .as_str(),
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fglobal-styles%2F42&context=edit",
    );
}

#[test]
fn global_styles_revisions() {
    let endpoint =
        super::global_styles_revisions_endpoint::GlobalStylesRevisionsRequestEndpoint::new(
            resolver(),
        );
    // wp-json form: /global-styles/42/revisions?context=edit
    assert_eq!(
        endpoint
            .list_with_edit_context(
                &crate::global_styles::GlobalStylesId(42),
                &crate::global_styles_revisions::GlobalStylesRevisionListParams::default()
            )
            .as_str(),
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fglobal-styles%2F42%2Frevisions&context=edit",
    );
}

#[test]
fn media() {
    let endpoint = super::media_endpoint::MediaRequestEndpoint::new(resolver());
    // wp-json form: /media?context=edit
    assert_eq!(
        endpoint
            .list_with_edit_context(&crate::media::MediaListParams::default())
            .as_str(),
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fmedia&context=edit",
    );
}

#[test]
fn menu_locations() {
    let endpoint = super::menu_locations_endpoint::MenuLocationsRequestEndpoint::new(resolver());
    // wp-json form: /menu-locations?context=edit
    assert_eq!(
        endpoint.list_with_edit_context().as_str(),
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fmenu-locations&context=edit",
    );
}

#[test]
fn nav_menu_item_autosaves() {
    let endpoint =
        super::nav_menu_item_autosaves_endpoint::NavMenuItemAutosavesRequestEndpoint::new(
            resolver(),
        );
    // wp-json form: /menu-items/777/autosaves?context=edit
    assert_eq!(
        endpoint
            .list_with_edit_context(&crate::nav_menu_items::NavMenuItemId(777))
            .as_str(),
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fmenu-items%2F777%2Fautosaves&context=edit",
    );
}

#[test]
fn nav_menu_items() {
    let endpoint = super::nav_menu_items_endpoint::NavMenuItemsRequestEndpoint::new(resolver());
    // wp-json form: /menu-items/54?context=edit
    assert_eq!(
        endpoint
            .retrieve_with_edit_context(&crate::nav_menu_items::NavMenuItemId(54))
            .as_str(),
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fmenu-items%2F54&context=edit",
    );
}

#[test]
fn nav_menus() {
    let endpoint = super::nav_menus_endpoint::NavMenusRequestEndpoint::new(resolver());
    // wp-json form: /menus/54?context=edit
    assert_eq!(
        endpoint
            .retrieve_with_edit_context(&crate::nav_menus::NavMenuId(54))
            .as_str(),
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fmenus%2F54&context=edit",
    );
}

#[test]
fn navigation_autosaves() {
    let endpoint =
        super::navigation_autosaves_endpoint::NavigationAutosavesRequestEndpoint::new(resolver());
    // wp-json form: /navigation/54/autosaves?context=edit
    assert_eq!(
        endpoint
            .list_with_edit_context(&crate::navigations::NavigationId(54))
            .as_str(),
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fnavigation%2F54%2Fautosaves&context=edit",
    );
}

#[test]
fn navigation_revisions() {
    let endpoint =
        super::navigation_revisions_endpoint::NavigationRevisionsRequestEndpoint::new(resolver());
    // wp-json form: /navigation/54/revisions?context=edit
    assert_eq!(
        endpoint
            .list_with_edit_context(
                &crate::navigations::NavigationId(54),
                &crate::navigation_revisions::NavigationRevisionListParams::default()
            )
            .as_str(),
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fnavigation%2F54%2Frevisions&context=edit",
    );
}

#[test]
fn navigations() {
    let endpoint = super::navigations_endpoint::NavigationsRequestEndpoint::new(resolver());
    // wp-json form: /navigation?context=edit
    assert_eq!(
        endpoint
            .list_with_edit_context(&crate::navigations::NavigationListParams::default())
            .as_str(),
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fnavigation&context=edit",
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
    // wp-json form: /pattern-directory/patterns?context=view&per_page=10&category=5
    assert_eq!(
        endpoint.list_with_view_context(&params).as_str(),
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fpattern-directory%2Fpatterns&context=view&per_page=10&category=5",
    );
}

#[test]
fn plugins() {
    let endpoint = super::plugins_endpoint::PluginsRequestEndpoint::new(resolver());
    // wp-json form: /plugins/hello-dolly/hello?context=view
    assert_eq!(
        endpoint
            .retrieve_with_view_context(&crate::PluginSlug::new("hello-dolly/hello".to_string()))
            .as_str(),
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fplugins%2Fhello-dolly%2Fhello&context=view",
    );
}

#[test]
fn post_autosaves() {
    let endpoint = super::post_autosaves_endpoint::AutosavesRequestEndpoint::new(resolver());
    // wp-json form: /posts/777/autosaves?context=edit
    assert_eq!(
        endpoint
            .list_with_edit_context(
                &crate::request::endpoint::posts_endpoint::PostEndpointType::Posts,
                &crate::posts::PostId(777)
            )
            .as_str(),
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fposts%2F777%2Fautosaves&context=edit",
    );
}

#[test]
fn post_revisions() {
    let endpoint = super::post_revisions_endpoint::PostRevisionsRequestEndpoint::new(resolver());
    // wp-json form: /posts/777/revisions/888?context=edit
    assert_eq!(
        endpoint
            .retrieve_with_edit_context(
                &crate::request::endpoint::posts_endpoint::PostEndpointType::Posts,
                &crate::posts::PostId(777),
                &crate::post_revisions::PostRevisionId(888)
            )
            .as_str(),
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fposts%2F777%2Frevisions%2F888&context=edit",
    );
}

#[test]
fn post_statuses() {
    let endpoint = super::post_statuses_endpoint::PostStatusesRequestEndpoint::new(resolver());
    // wp-json form: /statuses?context=edit
    assert_eq!(
        endpoint.list_with_edit_context().as_str(),
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fstatuses&context=edit",
    );
}

#[test]
fn post_types() {
    let endpoint = super::post_types_endpoint::PostTypesRequestEndpoint::new(resolver());
    // wp-json form: /types?context=edit
    assert_eq!(
        endpoint.list_with_edit_context().as_str(),
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Ftypes&context=edit",
    );
}

#[test]
fn posts() {
    let endpoint = super::posts_endpoint::PostsRequestEndpoint::new(resolver());
    // wp-json form: /posts?context=edit
    assert_eq!(
        endpoint
            .list_with_edit_context(
                &crate::request::endpoint::posts_endpoint::PostEndpointType::Posts,
                &crate::posts::PostListParams::default()
            )
            .as_str(),
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fposts&context=edit",
    );
}

#[test]
fn search() {
    let endpoint = super::search_endpoint::SearchRequestEndpoint::new(resolver());
    // wp-json form: /search?context=embed
    assert_eq!(
        endpoint
            .list_with_embed_context(&crate::search_results::SearchListParams::default())
            .as_str(),
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fsearch&context=embed",
    );
}

#[test]
fn sidebars() {
    let endpoint = super::sidebars_endpoint::SidebarsRequestEndpoint::new(resolver());
    // wp-json form: /sidebars?context=edit
    assert_eq!(
        endpoint.list_with_edit_context().as_str(),
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fsidebars&context=edit",
    );
}

#[test]
fn site_settings() {
    let endpoint = super::site_settings_endpoint::SiteSettingsRequestEndpoint::new(resolver());
    // wp-json form: /settings?context=edit
    assert_eq!(
        endpoint.retrieve_with_edit_context().as_str(),
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fsettings&context=edit",
    );
}

#[test]
fn taxonomies() {
    let endpoint = super::taxonomies_endpoint::TaxonomiesRequestEndpoint::new(resolver());
    // wp-json form: /taxonomies?context=edit
    assert_eq!(
        endpoint
            .list_with_edit_context(&crate::taxonomies::TaxonomyListParams::default())
            .as_str(),
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Ftaxonomies&context=edit",
    );
}

#[test]
fn template_autosaves() {
    let endpoint =
        super::template_autosaves_endpoint::TemplateAutosavesRequestEndpoint::new(resolver());
    // wp-json form: /templates/foo/autosaves?context=edit
    assert_eq!(
        endpoint
            .list_with_edit_context(&crate::templates::TemplateId("foo".to_string()))
            .as_str(),
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Ftemplates%2Ffoo%2Fautosaves&context=edit",
    );
}

#[test]
fn template_part_autosaves() {
    let endpoint =
        super::template_part_autosaves_endpoint::TemplatePartAutosavesRequestEndpoint::new(
            resolver(),
        );
    // wp-json form: /template-parts/foo/autosaves?context=edit
    assert_eq!(
        endpoint
            .list_with_edit_context(&crate::template_parts::TemplatePartId("foo".to_string()))
            .as_str(),
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Ftemplate-parts%2Ffoo%2Fautosaves&context=edit",
    );
}

#[test]
fn template_part_revisions() {
    let endpoint =
        super::template_part_revisions_endpoint::TemplatePartRevisionsRequestEndpoint::new(
            resolver(),
        );
    // wp-json form: /template-parts/foo/revisions/42?context=edit
    assert_eq!(
        endpoint
            .retrieve_with_edit_context(
                &crate::template_parts::TemplatePartId("foo".to_string()),
                &crate::template_part_revisions::TemplatePartRevisionId(42)
            )
            .as_str(),
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Ftemplate-parts%2Ffoo%2Frevisions%2F42&context=edit",
    );
}

#[test]
fn template_parts() {
    let endpoint = super::template_parts_endpoint::TemplatePartsRequestEndpoint::new(resolver());
    // wp-json form: /template-parts/foo?context=edit
    assert_eq!(
        endpoint
            .retrieve_with_edit_context(&crate::template_parts::TemplatePartId("foo".to_string()))
            .as_str(),
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Ftemplate-parts%2Ffoo&context=edit",
    );
}

#[test]
fn template_revisions() {
    let endpoint =
        super::template_revisions_endpoint::TemplateRevisionsRequestEndpoint::new(resolver());
    // wp-json form: /templates/foo/revisions/42?context=edit
    assert_eq!(
        endpoint
            .retrieve_with_edit_context(
                &crate::templates::TemplateId("foo".to_string()),
                &crate::template_revisions::TemplateRevisionId(42)
            )
            .as_str(),
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Ftemplates%2Ffoo%2Frevisions%2F42&context=edit",
    );
}

#[test]
fn templates() {
    let endpoint = super::templates_endpoint::TemplatesRequestEndpoint::new(resolver());
    // wp-json form: /templates/foo?context=edit
    assert_eq!(
        endpoint
            .retrieve_with_edit_context(&crate::templates::TemplateId("foo".to_string()))
            .as_str(),
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Ftemplates%2Ffoo&context=edit",
    );
}

#[test]
fn terms() {
    let endpoint = super::terms_endpoint::TermsRequestEndpoint::new(resolver());
    // wp-json form: /categories/54?context=edit
    assert_eq!(
        endpoint
            .retrieve_with_edit_context(
                &crate::request::endpoint::terms_endpoint::TermEndpointType::Categories,
                &crate::terms::TermId(54)
            )
            .as_str(),
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fcategories%2F54&context=edit",
    );
}

#[test]
fn themes() {
    let endpoint = super::themes_endpoint::ThemesRequestEndpoint::new(resolver());
    // wp-json form: /themes/foo?context=edit
    assert_eq!(
        endpoint
            .retrieve_with_edit_context(&crate::themes::ThemeStylesheet::from("foo"))
            .as_str(),
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fthemes%2Ffoo&context=edit",
    );
}

#[test]
fn users() {
    let endpoint = super::users_endpoint::UsersRequestEndpoint::new(resolver());
    // wp-json form: /users?context=edit
    assert_eq!(
        endpoint
            .list_with_edit_context(&crate::UserListParams::default())
            .as_str(),
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fusers&context=edit",
    );
}

#[test]
fn widget_types() {
    let endpoint = super::widget_types_endpoint::WidgetTypesRequestEndpoint::new(resolver());
    // wp-json form: /widget-types?context=edit
    assert_eq!(
        endpoint.list_with_edit_context().as_str(),
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fwidget-types&context=edit",
    );
}

#[test]
fn widgets() {
    let endpoint = super::widgets_endpoint::WidgetsRequestEndpoint::new(resolver());
    // wp-json form: /widgets?context=edit
    assert_eq!(
        endpoint
            .list_with_edit_context(&crate::widgets::WidgetListParams::default())
            .as_str(),
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fwidgets&context=edit",
    );
}

#[test]
fn wp_block_editor() {
    let endpoint = super::wp_block_editor_endpoint::WpBlockEditorRequestEndpoint::new(resolver());
    let params = crate::wp_block_editor::WpBlockEditorSettingsParams {
        context: Some(crate::wp_block_editor::WpBlockEditorSettingsContext::WidgetsEditor),
    };
    // wp-json form: /settings?context=widgets-editor
    assert_eq!(
        endpoint.retrieve_settings(&params).as_str(),
        "https://example.com/index.php?rest_route=%2Fwp-block-editor%2Fv1%2Fsettings&context=widgets-editor",
    );
}

#[test]
fn wp_site_health_tests() {
    let endpoint =
        super::wp_site_health_tests_endpoint::WpSiteHealthTestsRequestEndpoint::new(resolver());
    // wp-json form: /tests/background-updates?_fields=actions%2Cbadge
    assert_eq!(
        endpoint
            .filter_background_updates(&[
                crate::wp_site_health_tests::SparseWpSiteHealthTestField::Actions,
                crate::wp_site_health_tests::SparseWpSiteHealthTestField::Badge
            ])
            .as_str(),
        "https://example.com/index.php?rest_route=%2Fwp-site-health%2Fv1%2Ftests%2Fbackground-updates&_fields=actions%2Cbadge",
    );
}

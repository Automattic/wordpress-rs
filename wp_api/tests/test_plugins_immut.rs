use rstest::*;
use rstest_reuse::{self, apply, template};
use wp_api::{
    generate,
    plugins::{PluginListParams, PluginSlug, PluginStatus, SparsePlugin, SparsePluginField},
    WpContext,
};

use crate::integration_test_common::{
    request_builder, AssertResponse, CLASSIC_EDITOR_PLUGIN_SLUG, HELLO_DOLLY_PLUGIN_SLUG,
};

pub mod integration_test_common;

#[apply(filter_fields_cases)]
#[tokio::test]
async fn filter_plugins(
    #[case] fields: &[SparsePluginField],
    #[values(
        PluginListParams::default(),
        generate!(PluginListParams, (status, Some(PluginStatus::Active))),
        generate!(PluginListParams, (search, Some("foo".to_string())))
    )]
    params: PluginListParams,
) {
    request_builder()
        .plugins()
        .filter_list(WpContext::Edit, &Some(params), fields)
        .await
        .assert_response()
        .iter()
        .for_each(|plugin| validate_sparse_plugin_fields(plugin, fields));
}

#[apply(filter_fields_cases)]
#[tokio::test]
async fn filter_retrieve_plugin(
    #[case] fields: &[SparsePluginField],
    #[values(CLASSIC_EDITOR_PLUGIN_SLUG, HELLO_DOLLY_PLUGIN_SLUG)] slug: &str,
) {
    let response = request_builder()
        .plugins()
        .filter_retrieve(WpContext::Edit, &slug.into(), fields)
        .await
        .assert_response();
    validate_sparse_plugin_fields(&response, fields);
}

#[rstest]
#[case(PluginListParams::default())]
#[case(generate!(PluginListParams, (search, Some("foo".to_string()))))]
#[case(generate!(PluginListParams, (status, Some(PluginStatus::Active))))]
#[case(generate!(PluginListParams, (search, Some("foo".to_string())), (status, Some(PluginStatus::Inactive))))]
#[trace]
#[tokio::test]
async fn list_plugins(
    #[case] params: PluginListParams,
    #[values(WpContext::Edit, WpContext::Embed, WpContext::View)] context: WpContext,
) {
    match context {
        WpContext::Edit => {
            request_builder()
                .plugins()
                .list_with_edit_context(&Some(params))
                .await
                .assert_response();
        }
        WpContext::Embed => {
            request_builder()
                .plugins()
                .list_with_embed_context(&Some(params))
                .await
                .assert_response();
        }
        WpContext::View => {
            request_builder()
                .plugins()
                .list_with_view_context(&Some(params))
                .await
                .assert_response();
        }
    };
}

#[rstest]
#[case(CLASSIC_EDITOR_PLUGIN_SLUG.into(), "WordPress Contributors", "https://wordpress.org/plugins/classic-editor/")]
#[case(HELLO_DOLLY_PLUGIN_SLUG.into(), "Matt Mullenweg", "http://wordpress.org/plugins/hello-dolly/")]
#[trace]
#[tokio::test]
async fn retrieve_plugin(
    #[case] plugin_slug: PluginSlug,
    #[case] expected_author: &str,
    #[case] expected_plugin_uri: &str,
    #[values(WpContext::Edit, WpContext::Embed, WpContext::View)] context: WpContext,
) {
    match context {
        WpContext::Edit => {
            let plugin = request_builder()
                .plugins()
                .retrieve_with_edit_context(&plugin_slug)
                .await
                .assert_response();
            assert_eq!(&plugin_slug, &plugin.plugin);
            assert_eq!(expected_author, plugin.author);
            assert_eq!(expected_plugin_uri, plugin.plugin_uri);
        }
        WpContext::Embed => {
            let plugin = request_builder()
                .plugins()
                .retrieve_with_embed_context(&plugin_slug)
                .await
                .assert_response();
            assert_eq!(&plugin_slug, &plugin.plugin);
        }
        WpContext::View => {
            let plugin = request_builder()
                .plugins()
                .retrieve_with_view_context(&plugin_slug)
                .await
                .assert_response();
            assert_eq!(&plugin_slug, &plugin.plugin);
            assert_eq!(expected_author, plugin.author);
            assert_eq!(expected_plugin_uri, plugin.plugin_uri);
        }
    };
}

fn validate_sparse_plugin_fields(plugin: &SparsePlugin, fields: &[SparsePluginField]) {
    assert_eq!(
        plugin.author.is_some(),
        fields.contains(&SparsePluginField::Author)
    );

    assert_eq!(
        plugin.author.is_some(),
        fields.contains(&SparsePluginField::Author)
    );
    assert_eq!(
        plugin.description.is_some(),
        fields.contains(&SparsePluginField::Description)
    );
    assert_eq!(
        plugin.name.is_some(),
        fields.contains(&SparsePluginField::Name)
    );
    assert_eq!(
        plugin.network_only.is_some(),
        fields.contains(&SparsePluginField::NetworkOnly)
    );
    assert_eq!(
        plugin.plugin.is_some(),
        fields.contains(&SparsePluginField::Plugin)
    );
    assert_eq!(
        plugin.plugin_uri.is_some(),
        fields.contains(&SparsePluginField::PluginUri)
    );
    assert_eq!(
        plugin.requires_php.is_some(),
        fields.contains(&SparsePluginField::RequiresPhp)
    );
    assert_eq!(
        plugin.status.is_some(),
        fields.contains(&SparsePluginField::Status)
    );
    assert_eq!(
        plugin.textdomain.is_some(),
        fields.contains(&SparsePluginField::Textdomain)
    );
    assert_eq!(
        plugin.version.is_some(),
        fields.contains(&SparsePluginField::Version)
    );
}

#[template]
#[rstest]
#[case(&[SparsePluginField::Author])]
#[case(&[SparsePluginField::AuthorUri])]
#[case(&[SparsePluginField::Description])]
#[case(&[SparsePluginField::Name])]
#[case(&[SparsePluginField::NetworkOnly])]
#[case(&[SparsePluginField::Plugin])]
#[case(&[SparsePluginField::PluginUri])]
#[case(&[SparsePluginField::RequiresWp])]
#[case(&[SparsePluginField::RequiresPhp])]
#[case(&[SparsePluginField::Status])]
#[case(&[SparsePluginField::Textdomain])]
#[case(&[SparsePluginField::Version])]
#[case(&[SparsePluginField::Author, SparsePluginField::Plugin])]
#[case(&[SparsePluginField::Status, SparsePluginField::Version])]
fn filter_fields_cases(#[case] fields: &[SparsePluginField]) {}

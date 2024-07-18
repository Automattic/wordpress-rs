use rstest::*;
use rstest_reuse::{self, apply, template};
use serial_test::parallel;
use wp_api::{
    generate,
    plugins::{
        PluginListParams, PluginSlug, PluginStatus, SparsePluginFieldWithEditContext,
        SparsePluginWithEditContext,
    },
    WpContext,
};

use crate::integration_test_common::{
    api_client, AssertResponse, CLASSIC_EDITOR_PLUGIN_SLUG, HELLO_DOLLY_PLUGIN_SLUG,
};

pub mod integration_test_common;

#[apply(filter_fields_cases_with_edit_context)]
#[tokio::test]
#[parallel]
async fn filter_plugins(
    #[case] fields: &[SparsePluginFieldWithEditContext],
    #[values(
        PluginListParams::default(),
        generate!(PluginListParams, (status, Some(PluginStatus::Active))),
        generate!(PluginListParams, (search, Some("foo".to_string())))
    )]
    params: PluginListParams,
) {
    api_client()
        .plugins()
        .filter_list_with_edit_context(&params, fields)
        .await
        .assert_response()
        .iter()
        .for_each(|plugin| validate_sparse_plugin_fields(plugin, fields));
}

#[apply(filter_fields_cases_with_edit_context)]
#[tokio::test]
#[parallel]
async fn filter_retrieve_plugin(
    #[case] fields: &[SparsePluginFieldWithEditContext],
    #[values(CLASSIC_EDITOR_PLUGIN_SLUG, HELLO_DOLLY_PLUGIN_SLUG)] slug: &str,
) {
    let response = api_client()
        .plugins()
        .filter_retrieve_with_edit_context(&slug.into(), fields)
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
#[parallel]
async fn list_plugins(
    #[case] params: PluginListParams,
    #[values(WpContext::Edit, WpContext::Embed, WpContext::View)] context: WpContext,
) {
    match context {
        WpContext::Edit => {
            api_client()
                .plugins()
                .list_with_edit_context(&params)
                .await
                .assert_response();
        }
        WpContext::Embed => {
            api_client()
                .plugins()
                .list_with_embed_context(&params)
                .await
                .assert_response();
        }
        WpContext::View => {
            api_client()
                .plugins()
                .list_with_view_context(&params)
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
#[parallel]
async fn retrieve_plugin(
    #[case] plugin_slug: PluginSlug,
    #[case] expected_author: &str,
    #[case] expected_plugin_uri: &str,
    #[values(WpContext::Edit, WpContext::Embed, WpContext::View)] context: WpContext,
) {
    match context {
        WpContext::Edit => {
            let plugin = api_client()
                .plugins()
                .retrieve_with_edit_context(&plugin_slug)
                .await
                .assert_response();
            assert_eq!(&plugin_slug, &plugin.plugin);
            assert_eq!(expected_author, plugin.author);
            assert_eq!(expected_plugin_uri, plugin.plugin_uri);
        }
        WpContext::Embed => {
            let plugin = api_client()
                .plugins()
                .retrieve_with_embed_context(&plugin_slug)
                .await
                .assert_response();
            assert_eq!(&plugin_slug, &plugin.plugin);
        }
        WpContext::View => {
            let plugin = api_client()
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

fn validate_sparse_plugin_fields(
    plugin: &SparsePluginWithEditContext,
    fields: &[SparsePluginFieldWithEditContext],
) {
    let field_included = |field| {
        // If "fields" is empty the server will return all fields
        fields.is_empty() || fields.contains(&field)
    };
    assert_eq!(
        plugin.author.is_some(),
        field_included(SparsePluginFieldWithEditContext::Author)
    );

    assert_eq!(
        plugin.author.is_some(),
        field_included(SparsePluginFieldWithEditContext::Author)
    );
    assert_eq!(
        plugin.description.is_some(),
        field_included(SparsePluginFieldWithEditContext::Description)
    );
    assert_eq!(
        plugin.name.is_some(),
        field_included(SparsePluginFieldWithEditContext::Name)
    );
    assert_eq!(
        plugin.network_only.is_some(),
        field_included(SparsePluginFieldWithEditContext::NetworkOnly)
    );
    assert_eq!(
        plugin.plugin.is_some(),
        field_included(SparsePluginFieldWithEditContext::Plugin)
    );
    assert_eq!(
        plugin.plugin_uri.is_some(),
        field_included(SparsePluginFieldWithEditContext::PluginUri)
    );
    assert_eq!(
        plugin.requires_php.is_some(),
        field_included(SparsePluginFieldWithEditContext::RequiresPhp)
    );
    assert_eq!(
        plugin.status.is_some(),
        field_included(SparsePluginFieldWithEditContext::Status)
    );
    assert_eq!(
        plugin.textdomain.is_some(),
        field_included(SparsePluginFieldWithEditContext::Textdomain)
    );
    assert_eq!(
        plugin.version.is_some(),
        field_included(SparsePluginFieldWithEditContext::Version)
    );
}

#[template]
#[rstest]
#[case(&[])]
#[case(&[SparsePluginFieldWithEditContext::Author])]
#[case(&[SparsePluginFieldWithEditContext::AuthorUri])]
#[case(&[SparsePluginFieldWithEditContext::Description])]
#[case(&[SparsePluginFieldWithEditContext::Name])]
#[case(&[SparsePluginFieldWithEditContext::NetworkOnly])]
#[case(&[SparsePluginFieldWithEditContext::Plugin])]
#[case(&[SparsePluginFieldWithEditContext::PluginUri])]
#[case(&[SparsePluginFieldWithEditContext::RequiresWp])]
#[case(&[SparsePluginFieldWithEditContext::RequiresPhp])]
#[case(&[SparsePluginFieldWithEditContext::Status])]
#[case(&[SparsePluginFieldWithEditContext::Textdomain])]
#[case(&[SparsePluginFieldWithEditContext::Version])]
#[case(&[SparsePluginFieldWithEditContext::Author, SparsePluginFieldWithEditContext::Plugin])]
#[case(&[SparsePluginFieldWithEditContext::Status, SparsePluginFieldWithEditContext::Version])]
fn filter_fields_cases_with_edit_context(#[case] fields: &[SparsePluginFieldWithEditContext]) {}

use rstest::*;
use rstest_reuse::{self, apply, template};
use serial_test::parallel;
use wp_api::{
    generate,
    plugins::{
        PluginListParams, PluginSlug, PluginStatus, SparsePluginFieldWithEditContext,
        SparsePluginFieldWithEmbedContext, SparsePluginFieldWithViewContext,
    },
    WpContext,
};

use wp_api_integration_tests::{
    api_client, AssertResponse, CLASSIC_EDITOR_PLUGIN_SLUG, HELLO_DOLLY_PLUGIN_SLUG,
};

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

mod filter {
    use super::*;

    wp_api::generate_sparse_plugin_field_with_edit_context_test_cases!();
    wp_api::generate_sparse_plugin_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_plugin_field_with_view_context_test_cases!();

    #[apply(sparse_plugin_field_with_edit_context_test_cases)]
    #[case(&[SparsePluginFieldWithEditContext::Author, SparsePluginFieldWithEditContext::Plugin])]
    #[case(&[SparsePluginFieldWithEditContext::Status, SparsePluginFieldWithEditContext::Version])]
    #[tokio::test]
    #[parallel]
    async fn filter_plugins_with_edit_context(
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
            .for_each(|plugin| {
                plugin.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_plugin_field_with_edit_context_test_cases)]
    #[case(&[SparsePluginFieldWithEditContext::Author, SparsePluginFieldWithEditContext::Plugin])]
    #[case(&[SparsePluginFieldWithEditContext::Status, SparsePluginFieldWithEditContext::Version])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_plugin_with_edit_context(
        #[case] fields: &[SparsePluginFieldWithEditContext],
        #[values(CLASSIC_EDITOR_PLUGIN_SLUG, HELLO_DOLLY_PLUGIN_SLUG)] slug: &str,
    ) {
        let plugin = api_client()
            .plugins()
            .filter_retrieve_with_edit_context(&slug.into(), fields)
            .await
            .assert_response();
        plugin.assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_plugin_field_with_embed_context_test_cases)]
    #[tokio::test]
    #[parallel]
    async fn filter_plugins_with_embed_context(
        #[case] fields: &[SparsePluginFieldWithEmbedContext],
        #[values(
        PluginListParams::default(),
        generate!(PluginListParams, (status, Some(PluginStatus::Active))),
        generate!(PluginListParams, (search, Some("foo".to_string())))
    )]
        params: PluginListParams,
    ) {
        api_client()
            .plugins()
            .filter_list_with_embed_context(&params, fields)
            .await
            .assert_response()
            .iter()
            .for_each(|plugin| {
                plugin.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_plugin_field_with_embed_context_test_cases)]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_plugin_with_embed_context(
        #[case] fields: &[SparsePluginFieldWithEmbedContext],
        #[values(CLASSIC_EDITOR_PLUGIN_SLUG, HELLO_DOLLY_PLUGIN_SLUG)] slug: &str,
    ) {
        let plugin = api_client()
            .plugins()
            .filter_retrieve_with_embed_context(&slug.into(), fields)
            .await
            .assert_response();
        plugin.assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_plugin_field_with_view_context_test_cases)]
    #[tokio::test]
    #[parallel]
    async fn filter_plugins_with_view_context(
        #[case] fields: &[SparsePluginFieldWithViewContext],
        #[values(
        PluginListParams::default(),
        generate!(PluginListParams, (status, Some(PluginStatus::Active))),
        generate!(PluginListParams, (search, Some("foo".to_string())))
    )]
        params: PluginListParams,
    ) {
        api_client()
            .plugins()
            .filter_list_with_view_context(&params, fields)
            .await
            .assert_response()
            .iter()
            .for_each(|plugin| {
                plugin.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_plugin_field_with_view_context_test_cases)]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_plugin_with_view_context(
        #[case] fields: &[SparsePluginFieldWithViewContext],
        #[values(CLASSIC_EDITOR_PLUGIN_SLUG, HELLO_DOLLY_PLUGIN_SLUG)] slug: &str,
    ) {
        let plugin = api_client()
            .plugins()
            .filter_retrieve_with_view_context(&slug.into(), fields)
            .await
            .assert_response();
        plugin.assert_that_instance_fields_nullability_match_provided_fields(fields);
    }
}

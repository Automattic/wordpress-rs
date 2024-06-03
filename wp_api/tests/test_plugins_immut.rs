//use rstest::*;
//use rstest_reuse::{self, apply, template};
//use wp_api::{
//    generate,
//    plugins::{PluginListParams, PluginSlug, PluginStatus, SparsePlugin, SparsePluginField},
//    request::WpNetworkResponse,
//    WpContext,
//};
//
//use crate::integration_test_common::{
//    request_builder, WpNetworkRequestExecutor, CLASSIC_EDITOR_PLUGIN_SLUG, HELLO_DOLLY_PLUGIN_SLUG,
//};
//
//pub mod integration_test_common;
//
//#[apply(filter_fields_cases)]
//#[tokio::test]
//async fn filter_plugins(
//    #[case] fields: &[SparsePluginField],
//    #[values(
//        PluginListParams::default(),
//        generate!(PluginListParams, (status, Some(PluginStatus::Active))),
//        generate!(PluginListParams, (search, Some("foo".to_string())))
//    )]
//    params: PluginListParams,
//) {
//    let parsed_response = request_builder()
//        .plugins()
//        .filter_list(WpContext::Edit, &Some(params), fields)
//        .execute()
//        .await
//        .unwrap()
//        .parse_with(wp_api::plugins::parse_filter_plugins_response);
//    assert!(parsed_response.is_ok());
//    parsed_response
//        .unwrap()
//        .iter()
//        .for_each(|plugin| validate_sparse_plugin_fields(plugin, fields));
//}
//
//#[apply(filter_fields_cases)]
//#[tokio::test]
//async fn filter_retrieve_plugin(
//    #[case] fields: &[SparsePluginField],
//    #[values(CLASSIC_EDITOR_PLUGIN_SLUG, HELLO_DOLLY_PLUGIN_SLUG)] slug: &str,
//) {
//    let plugin_result = request_builder()
//        .plugins()
//        .filter_retrieve(WpContext::Edit, &slug.into(), fields)
//        .execute()
//        .await
//        .unwrap()
//        .parse_with(wp_api::plugins::parse_filter_retrieve_plugin_response);
//    assert!(plugin_result.is_ok());
//    validate_sparse_plugin_fields(&plugin_result.unwrap(), fields);
//}
//
//#[rstest]
//#[case(PluginListParams::default())]
//#[case(generate!(PluginListParams, (search, Some("foo".to_string()))))]
//#[case(generate!(PluginListParams, (status, Some(PluginStatus::Active))))]
//#[case(generate!(PluginListParams, (search, Some("foo".to_string())), (status, Some(PluginStatus::Inactive))))]
//#[trace]
//#[tokio::test]
//async fn list_plugins(
//    #[case] params: PluginListParams,
//    #[values(WpContext::Edit, WpContext::Embed, WpContext::View)] context: WpContext,
//) {
//    let response = request_builder()
//        .plugins()
//        .list(context, &Some(params))
//        .execute()
//        .await
//        .unwrap();
//    match context {
//        WpContext::Edit => {
//            let parsed_response =
//                wp_api::plugins::parse_list_plugins_response_with_edit_context(&response);
//            assert!(
//                parsed_response.is_ok(),
//                "Response was: '{:?}'",
//                parsed_response
//            );
//        }
//        WpContext::Embed => {
//            let parsed_response =
//                wp_api::plugins::parse_list_plugins_response_with_embed_context(&response);
//            assert!(
//                parsed_response.is_ok(),
//                "Response was: '{:?}'",
//                parsed_response
//            );
//        }
//        WpContext::View => {
//            let parsed_response =
//                wp_api::plugins::parse_list_plugins_response_with_view_context(&response);
//            assert!(
//                parsed_response.is_ok(),
//                "Response was: '{:?}'",
//                parsed_response
//            );
//        }
//    };
//}
//
//#[rstest]
//#[case(CLASSIC_EDITOR_PLUGIN_SLUG.into(), "WordPress Contributors", "https://wordpress.org/plugins/classic-editor/")]
//#[case(HELLO_DOLLY_PLUGIN_SLUG.into(), "Matt Mullenweg", "http://wordpress.org/plugins/hello-dolly/")]
//#[trace]
//#[tokio::test]
//async fn retrieve_plugin(
//    #[case] plugin_slug: PluginSlug,
//    #[case] expected_author: &str,
//    #[case] expected_plugin_uri: &str,
//    #[values(WpContext::Edit, WpContext::Embed, WpContext::View)] context: WpContext,
//) {
//    let response = request_builder()
//        .plugins()
//        .retrieve(context, &plugin_slug)
//        .execute()
//        .await;
//    assert!(
//        response.is_ok(),
//        "Retrieve plugin failed!\nContext: {:?}\nPlugin: {:?}\nResponse was: '{:?}'",
//        context,
//        plugin_slug,
//        response
//    );
//    assert_retrieve_plugin_response(
//        &response.unwrap(),
//        context,
//        &plugin_slug,
//        if context == WpContext::Embed {
//            None
//        } else {
//            Some(expected_author)
//        },
//        if context == WpContext::Embed {
//            None
//        } else {
//            Some(expected_plugin_uri)
//        },
//    );
//}
//
//fn assert_retrieve_plugin_response(
//    response: &WpNetworkResponse,
//    context: WpContext,
//    expected_slug: &PluginSlug,
//    expected_author: Option<&str>,
//    expected_plugin_uri: Option<&str>,
//) {
//    let (plugin, author, plugin_uri) = match context {
//        WpContext::Edit => {
//            let parsed_response =
//                wp_api::plugins::parse_retrieve_plugin_response_with_edit_context(response);
//            assert!(parsed_response.is_ok());
//            let p = parsed_response.unwrap();
//            (p.plugin, Some(p.author), Some(p.plugin_uri))
//        }
//        WpContext::Embed => {
//            let parsed_response =
//                wp_api::plugins::parse_retrieve_plugin_response_with_embed_context(response);
//            assert!(parsed_response.is_ok());
//            let p = parsed_response.unwrap();
//            (p.plugin, None, None)
//        }
//        WpContext::View => {
//            let parsed_response =
//                wp_api::plugins::parse_retrieve_plugin_response_with_view_context(response);
//            assert!(parsed_response.is_ok());
//            let p = parsed_response.unwrap();
//            (p.plugin, Some(p.author), Some(p.plugin_uri))
//        }
//    };
//    assert_eq!(expected_slug, &plugin);
//    assert_eq!(expected_author, author.as_deref());
//    assert_eq!(expected_plugin_uri, plugin_uri.as_deref());
//}
//
//fn validate_sparse_plugin_fields(plugin: &SparsePlugin, fields: &[SparsePluginField]) {
//    assert_eq!(
//        plugin.author.is_some(),
//        fields.contains(&SparsePluginField::Author)
//    );
//
//    assert_eq!(
//        plugin.author.is_some(),
//        fields.contains(&SparsePluginField::Author)
//    );
//    assert_eq!(
//        plugin.description.is_some(),
//        fields.contains(&SparsePluginField::Description)
//    );
//    assert_eq!(
//        plugin.name.is_some(),
//        fields.contains(&SparsePluginField::Name)
//    );
//    assert_eq!(
//        plugin.network_only.is_some(),
//        fields.contains(&SparsePluginField::NetworkOnly)
//    );
//    assert_eq!(
//        plugin.plugin.is_some(),
//        fields.contains(&SparsePluginField::Plugin)
//    );
//    assert_eq!(
//        plugin.plugin_uri.is_some(),
//        fields.contains(&SparsePluginField::PluginUri)
//    );
//    assert_eq!(
//        plugin.requires_php.is_some(),
//        fields.contains(&SparsePluginField::RequiresPhp)
//    );
//    assert_eq!(
//        plugin.status.is_some(),
//        fields.contains(&SparsePluginField::Status)
//    );
//    assert_eq!(
//        plugin.textdomain.is_some(),
//        fields.contains(&SparsePluginField::Textdomain)
//    );
//    assert_eq!(
//        plugin.version.is_some(),
//        fields.contains(&SparsePluginField::Version)
//    );
//}
//
//#[template]
//#[rstest]
//#[case(&[SparsePluginField::Author])]
//#[case(&[SparsePluginField::AuthorUri])]
//#[case(&[SparsePluginField::Description])]
//#[case(&[SparsePluginField::Name])]
//#[case(&[SparsePluginField::NetworkOnly])]
//#[case(&[SparsePluginField::Plugin])]
//#[case(&[SparsePluginField::PluginUri])]
//#[case(&[SparsePluginField::RequiresWp])]
//#[case(&[SparsePluginField::RequiresPhp])]
//#[case(&[SparsePluginField::Status])]
//#[case(&[SparsePluginField::Textdomain])]
//#[case(&[SparsePluginField::Version])]
//#[case(&[SparsePluginField::Author, SparsePluginField::Plugin])]
//#[case(&[SparsePluginField::Status, SparsePluginField::Version])]
//fn filter_fields_cases(#[case] fields: &[SparsePluginField]) {}

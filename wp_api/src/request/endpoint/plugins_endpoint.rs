use crate::{plugins::PluginListParams, PluginSlug};
use crate::{
    PluginCreateParams, PluginDeleteResponse, PluginUpdateParams, PluginWithEditContext,
    PluginWithEmbedContext, PluginWithViewContext, SparsePlugin, SparsePluginField,
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
#[Namespace("/wp/v2")]
#[SparseField(SparsePluginField)]
enum PluginsRequest {
    #[post(url = "/plugins", params = &PluginCreateParams, output = PluginWithEditContext)]
    Create,
    #[delete(url = "/plugins/<plugin_slug>", output = PluginDeleteResponse)]
    Delete,
    #[contextual_get(url = "/plugins", params = &PluginListParams, output = Vec<SparsePlugin>)]
    List,
    #[contextual_get(url = "/plugins/<plugin_slug>", output = SparsePlugin)]
    Retrieve,
    #[post(url = "/plugins/<plugin_slug>", params = &PluginUpdateParams, output = PluginWithEditContext)]
    Update,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        generate,
        request::endpoint::{
            tests::{fixture_api_base_url, validate_endpoint},
            ApiBaseUrl,
        },
        PluginStatus, WpContext,
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    fn create_plugin(endpoint: PluginsRequestEndpoint) {
        validate_endpoint(endpoint.create(), "/plugins");
    }

    #[rstest]
    #[case("hello-dolly/hello".into(), "/plugins/hello-dolly/hello")]
    #[case(
        "classic-editor/classic-editor".into(),
        "/plugins/classic-editor/classic-editor"
    )]
    #[case("foo/bar%baz".into(), "/plugins/foo/bar%25baz")]
    #[case("foo/です".into(), "/plugins/foo/%E3%81%A7%E3%81%99")]
    fn delete_plugin(
        endpoint: PluginsRequestEndpoint,
        #[case] plugin_slug: PluginSlug,
        #[case] expected_path: &str,
    ) {
        validate_endpoint(endpoint.delete(&plugin_slug), expected_path);
    }

    #[rstest]
    #[case(PluginListParams::default(), "/plugins?context=edit")]
    #[case(generate!(PluginListParams, (search, Some("foo".to_string()))), "/plugins?context=edit&search=foo")]
    #[case(generate!(PluginListParams, (status, Some(PluginStatus::Active))), "/plugins?context=edit&status=active")]
    #[case(generate!(PluginListParams, (search, Some("foo".to_string())), (status, Some(PluginStatus::Inactive))), "/plugins?context=edit&search=foo&status=inactive")]
    fn list_plugins_with_edit_context(
        endpoint: PluginsRequestEndpoint,
        #[case] params: PluginListParams,
        #[case] expected_path: &str,
    ) {
        validate_endpoint(endpoint.list_with_edit_context(&params), expected_path);
    }

    #[rstest]
    #[case(PluginListParams::default(), "/plugins?context=embed")]
    #[case(generate!(PluginListParams, (search, Some("foo".to_string()))), "/plugins?context=embed&search=foo")]
    fn list_plugins_with_embed_context(
        endpoint: PluginsRequestEndpoint,
        #[case] params: PluginListParams,
        #[case] expected_path: &str,
    ) {
        validate_endpoint(endpoint.list_with_embed_context(&params), expected_path);
    }

    #[rstest]
    #[case(PluginListParams::default(), "/plugins?context=view")]
    #[case(generate!(PluginListParams, (search, Some("foo".to_string()))), "/plugins?context=view&search=foo")]
    fn list_plugins_with_view_context(
        endpoint: PluginsRequestEndpoint,
        #[case] params: PluginListParams,
        #[case] expected_path: &str,
    ) {
        validate_endpoint(endpoint.list_with_view_context(&params), expected_path);
    }

    #[rstest]
    #[case(WpContext::Edit, PluginListParams::default(), &[], "/plugins?context=edit&_fields=")]
    #[case(
        WpContext::Edit,
        generate!(PluginListParams, (search, Some("foo".to_string()))),
        &[SparsePluginField::Author],
        "/plugins?context=edit&search=foo&_fields=author"
    )]
    #[case(
        WpContext::Embed,
        generate!(PluginListParams, (status, Some(PluginStatus::Active))),
        &[SparsePluginField::AuthorUri, SparsePluginField::RequiresWp],
        "/plugins?context=embed&status=active&_fields=author_uri%2Crequires_wp"
    )]
    #[case(
        WpContext::Embed,
        generate!(PluginListParams, (status, Some(PluginStatus::Active))),
        &[SparsePluginField::Name, SparsePluginField::PluginUri],
        "/plugins?context=embed&status=active&_fields=name%2Cplugin_uri"
    )]
    #[case(
        WpContext::View,
        generate!(PluginListParams, (search, Some("foo".to_string())), (status, Some(PluginStatus::Inactive))), 
        &[SparsePluginField::NetworkOnly, SparsePluginField::RequiresPhp, SparsePluginField::Textdomain],
        "/plugins?context=view&search=foo&status=inactive&_fields=network_only%2Crequires_php%2Ctextdomain"
    )]
    fn filter_list_plugins_with_params(
        endpoint: PluginsRequestEndpoint,
        #[case] context: WpContext,
        #[case] params: PluginListParams,
        #[case] fields: &[SparsePluginField],
        #[case] expected_path: &str,
    ) {
        validate_endpoint(
            endpoint.filter_list(context, &params, fields),
            expected_path,
        );
    }

    #[rstest]
    #[case(
        "hello-dolly/hello".into(),
        "/plugins/hello-dolly/hello?context=view"
    )]
    #[case(
        "classic-editor/classic-editor".into(),
        "/plugins/classic-editor/classic-editor?context=view"
    )]
    #[case("foo/bar%baz".into(), "/plugins/foo/bar%25baz?context=view")]
    #[case(
        "foo/です".into(),
        "/plugins/foo/%E3%81%A7%E3%81%99?context=view"
    )]
    fn retrieve_plugin_with_view_context(
        endpoint: PluginsRequestEndpoint,
        #[case] plugin_slug: PluginSlug,
        #[case] expected_path: &str,
    ) {
        validate_endpoint(
            endpoint.retrieve_with_view_context(&plugin_slug),
            expected_path,
        );
    }

    #[rstest]
    #[case(
        "hello-dolly/hello".into(),
        WpContext::View,
        &[SparsePluginField::Name],
        "/plugins/hello-dolly/hello?context=view&_fields=name"
    )]
    #[case(
        "classic-editor/classic-editor".into(),
        WpContext::Embed,
        &[SparsePluginField::Description, SparsePluginField::Plugin],
        "/plugins/classic-editor/classic-editor?context=embed&_fields=description%2Cplugin"
    )]
    #[case(
        "foo/bar%baz".into(),
        WpContext::Edit,
        &[SparsePluginField::Status, SparsePluginField::Version],
        "/plugins/foo/bar%25baz?context=edit&_fields=status%2Cversion"
    )]
    #[case(
        "foo/です".into(),
        WpContext::View,
        &[SparsePluginField::NetworkOnly, SparsePluginField::RequiresPhp, SparsePluginField::Textdomain],
        "/plugins/foo/%E3%81%A7%E3%81%99?context=view&_fields=network_only%2Crequires_php%2Ctextdomain"
    )]
    fn filter_retrieve_plugin(
        endpoint: PluginsRequestEndpoint,
        #[case] plugin_slug: PluginSlug,
        #[case] context: WpContext,
        #[case] fields: &[SparsePluginField],
        #[case] expected_path: &str,
    ) {
        validate_endpoint(
            endpoint.filter_retrieve(&plugin_slug, context, fields),
            expected_path,
        );
    }

    #[rstest]
    #[case("hello-dolly/hello".into(), "/plugins/hello-dolly/hello")]
    #[case(
        "classic-editor/classic-editor".into(),
        "/plugins/classic-editor/classic-editor"
    )]
    #[case("foo/bar%baz".into(), "/plugins/foo/bar%25baz")]
    #[case("foo/です".into(), "/plugins/foo/%E3%81%A7%E3%81%99")]
    fn update_plugin(
        endpoint: PluginsRequestEndpoint,
        #[case] plugin_slug: PluginSlug,
        #[case] expected_path: &str,
    ) {
        validate_endpoint(endpoint.update(&plugin_slug), expected_path);
    }

    #[fixture]
    fn endpoint(fixture_api_base_url: Arc<ApiBaseUrl>) -> PluginsRequestEndpoint {
        PluginsRequestEndpoint::new(fixture_api_base_url)
    }
}

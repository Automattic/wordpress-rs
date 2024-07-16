use crate::{
    PluginSlug, SparseField, SparsePluginFieldWithEditContext, SparsePluginFieldWithEmbedContext,
    SparsePluginFieldWithViewContext,
};
use wp_derive_request_builder::WpDerivedRequest;

use super::{DerivedRequest, Namespace};

#[derive(WpDerivedRequest)]
enum PluginsRequest {
    #[post(url = "/plugins", params = &crate::PluginCreateParams, output = crate::PluginWithEditContext)]
    Create,
    #[delete(url = "/plugins/<plugin_slug>", output = crate::PluginDeleteResponse)]
    Delete,
    #[contextual_get(url = "/plugins", params = &crate::PluginListParams, output = Vec<crate::SparsePlugin>, filter_by = crate::SparsePluginField)]
    List,
    #[contextual_get(url = "/plugins/<plugin_slug>", output = crate::SparsePlugin, filter_by = crate::SparsePluginField)]
    Retrieve,
    #[post(url = "/plugins/<plugin_slug>", params = &crate::PluginUpdateParams, output = crate::PluginWithEditContext)]
    Update,
}

impl DerivedRequest for PluginsRequest {
    fn namespace() -> Namespace {
        Namespace::WpV2
    }
}

super::macros::default_sparse_field_implementation_from_field_name!(
    SparsePluginFieldWithEditContext
);
super::macros::default_sparse_field_implementation_from_field_name!(
    SparsePluginFieldWithEmbedContext
);
super::macros::default_sparse_field_implementation_from_field_name!(
    SparsePluginFieldWithViewContext
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        generate,
        request::endpoint::{
            tests::{fixture_api_base_url, validate_wp_v2_endpoint},
            ApiBaseUrl,
        },
        PluginListParams, PluginStatus,
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    fn create_plugin(endpoint: PluginsRequestEndpoint) {
        validate_wp_v2_endpoint(endpoint.create(), "/plugins");
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
        validate_wp_v2_endpoint(endpoint.delete(&plugin_slug), expected_path);
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
        validate_wp_v2_endpoint(endpoint.list_with_edit_context(&params), expected_path);
    }

    #[rstest]
    #[case(PluginListParams::default(), "/plugins?context=embed")]
    #[case(generate!(PluginListParams, (search, Some("foo".to_string()))), "/plugins?context=embed&search=foo")]
    fn list_plugins_with_embed_context(
        endpoint: PluginsRequestEndpoint,
        #[case] params: PluginListParams,
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(endpoint.list_with_embed_context(&params), expected_path);
    }

    #[rstest]
    #[case(PluginListParams::default(), "/plugins?context=view")]
    #[case(generate!(PluginListParams, (search, Some("foo".to_string()))), "/plugins?context=view&search=foo")]
    fn list_plugins_with_view_context(
        endpoint: PluginsRequestEndpoint,
        #[case] params: PluginListParams,
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(endpoint.list_with_view_context(&params), expected_path);
    }

    #[rstest]
    #[case(PluginListParams::default(), &[], "/plugins?context=edit&_fields=")]
    #[case(generate!(PluginListParams, (search, Some("foo".to_string()))),
        &[SparsePluginFieldWithEditContext::Author],
        "/plugins?context=edit&search=foo&_fields=author"
    )]
    #[case(
        generate!(PluginListParams, (status, Some(PluginStatus::Active))),
        &[SparsePluginFieldWithEditContext::AuthorUri, SparsePluginFieldWithEditContext::RequiresWp],
        "/plugins?context=edit&status=active&_fields=author_uri%2Crequires_wp"
    )]
    #[case(
        generate!(PluginListParams, (status, Some(PluginStatus::Active))),
        &[SparsePluginFieldWithEditContext::Name, SparsePluginFieldWithEditContext::PluginUri],
        "/plugins?context=edit&status=active&_fields=name%2Cplugin_uri"
    )]
    #[case(
        generate!(PluginListParams, (search, Some("foo".to_string())), (status, Some(PluginStatus::Inactive))), 
        &[SparsePluginFieldWithEditContext::NetworkOnly, SparsePluginFieldWithEditContext::RequiresPhp, SparsePluginFieldWithEditContext::Textdomain],
        "/plugins?context=edit&search=foo&status=inactive&_fields=network_only%2Crequires_php%2Ctextdomain"
    )]
    fn filter_list_plugins_with_params(
        endpoint: PluginsRequestEndpoint,
        #[case] params: PluginListParams,
        #[case] fields: &[SparsePluginFieldWithEditContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_edit_context(&params, fields),
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
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_view_context(&plugin_slug),
            expected_path,
        );
    }

    #[rstest]
    #[case(
        "hello-dolly/hello".into(),
        &[SparsePluginFieldWithViewContext::Name],
        "/plugins/hello-dolly/hello?context=view&_fields=name"
    )]
    #[case(
        "classic-editor/classic-editor".into(),
        &[SparsePluginFieldWithViewContext::Description, SparsePluginFieldWithViewContext::Plugin],
        "/plugins/classic-editor/classic-editor?context=view&_fields=description%2Cplugin"
    )]
    #[case(
        "foo/bar%baz".into(),
        &[SparsePluginFieldWithViewContext::Status, SparsePluginFieldWithViewContext::Version],
        "/plugins/foo/bar%25baz?context=view&_fields=status%2Cversion"
    )]
    #[case(
        "foo/です".into(),
        &[SparsePluginFieldWithViewContext::NetworkOnly, SparsePluginFieldWithViewContext::RequiresPhp, SparsePluginFieldWithViewContext::Textdomain],
        "/plugins/foo/%E3%81%A7%E3%81%99?context=view&_fields=network_only%2Crequires_php%2Ctextdomain"
    )]
    fn filter_retrieve_plugin(
        endpoint: PluginsRequestEndpoint,
        #[case] plugin_slug: PluginSlug,
        #[case] fields: &[SparsePluginFieldWithViewContext],
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(
            endpoint.filter_retrieve_with_view_context(&plugin_slug, fields),
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
        validate_wp_v2_endpoint(endpoint.update(&plugin_slug), expected_path);
    }

    #[fixture]
    fn endpoint(fixture_api_base_url: Arc<ApiBaseUrl>) -> PluginsRequestEndpoint {
        PluginsRequestEndpoint::new(fixture_api_base_url)
    }
}

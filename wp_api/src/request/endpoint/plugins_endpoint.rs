use std::sync::Arc;

use url::Url;

use crate::{plugins::PluginListParams, PluginSlug, SparsePluginField, WpContext};

use super::{ApiBaseUrl, ApiEndpointUrl, UrlExtension};

#[derive(Debug)]
pub(crate) struct PluginsEndpoint {
    api_base_url: Arc<ApiBaseUrl>,
}

impl PluginsEndpoint {
    pub fn new(api_base_url: Arc<ApiBaseUrl>) -> Self {
        Self { api_base_url }
    }

    pub fn create(&self) -> ApiEndpointUrl {
        self.plugins_base_url().into()
    }

    pub fn delete(&self, plugin: &PluginSlug) -> ApiEndpointUrl {
        self.plugins_url_with_slug(plugin).into()
    }

    pub fn list(&self, context: WpContext, params: Option<&PluginListParams>) -> ApiEndpointUrl {
        let mut url = self.plugins_base_url();
        url.query_pairs_mut()
            .append_pair("context", context.as_str());
        if let Some(params) = params {
            url.query_pairs_mut().extend_pairs(params.query_pairs());
        }
        url.into()
    }

    pub fn filter_list(
        &self,
        context: WpContext,
        params: Option<&PluginListParams>,
        fields: &[SparsePluginField],
    ) -> ApiEndpointUrl {
        self.list(context, params)
            .url
            .append_filter_fields(fields)
            .into()
    }

    pub fn retrieve(&self, context: WpContext, plugin: &PluginSlug) -> ApiEndpointUrl {
        let mut url = self.plugins_url_with_slug(plugin);
        url.query_pairs_mut()
            .append_pair("context", context.as_str());
        url.into()
    }

    pub fn filter_retrieve(
        &self,
        context: WpContext,
        plugin: &PluginSlug,
        fields: &[SparsePluginField],
    ) -> ApiEndpointUrl {
        self.retrieve(context, plugin)
            .url
            .append_filter_fields(fields)
            .into()
    }

    pub fn update(&self, plugin: &PluginSlug) -> ApiEndpointUrl {
        self.plugins_url_with_slug(plugin).into()
    }

    fn plugins_base_url(&self) -> Url {
        self.api_base_url.by_appending("plugins")
    }

    fn plugins_url_with_slug(&self, plugin: &PluginSlug) -> Url {
        self.api_base_url
            // The '/' character has to be preserved and not get encoded
            .by_extending(["plugins"].into_iter().chain(plugin.slug.split('/')))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        generate,
        request::endpoint::tests::{fixture_api_base_url, validate_endpoint},
        PluginStatus,
    };
    use rstest::*;

    #[rstest]
    fn create_plugin(plugins_endpoint: PluginsEndpoint) {
        validate_endpoint(plugins_endpoint.create(), "/plugins");
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
        plugins_endpoint: PluginsEndpoint,
        #[case] plugin_slug: PluginSlug,
        #[case] expected_path: &str,
    ) {
        validate_endpoint(plugins_endpoint.delete(&plugin_slug), expected_path);
    }

    #[rstest]
    #[case(WpContext::Edit, generate!(PluginListParams, (search, Some("foo".to_string()))), "/plugins?context=edit&search=foo")]
    #[case(WpContext::Embed, generate!(PluginListParams, (status, Some(PluginStatus::Active))), "/plugins?context=embed&status=active")]
    #[case(WpContext::View, generate!(PluginListParams, (search, Some("foo".to_string())), (status, Some(PluginStatus::Inactive))), "/plugins?context=view&search=foo&status=inactive")]
    fn list_plugins_with_params(
        plugins_endpoint: PluginsEndpoint,
        #[case] context: WpContext,
        #[case] params: PluginListParams,
        #[case] expected_path: &str,
    ) {
        validate_endpoint(plugins_endpoint.list(context, Some(&params)), expected_path);
    }

    #[rstest]
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
        plugins_endpoint: PluginsEndpoint,
        #[case] context: WpContext,
        #[case] params: PluginListParams,
        #[case] fields: &[SparsePluginField],
        #[case] expected_path: &str,
    ) {
        validate_endpoint(
            plugins_endpoint.filter_list(context, Some(&params), fields),
            expected_path,
        );
    }

    #[rstest]
    #[case(
        "hello-dolly/hello".into(),
        WpContext::View,
        "/plugins/hello-dolly/hello?context=view"
    )]
    #[case(
        "classic-editor/classic-editor".into(),
        WpContext::Embed,
        "/plugins/classic-editor/classic-editor?context=embed"
    )]
    #[case("foo/bar%baz".into(), WpContext::Edit, "/plugins/foo/bar%25baz?context=edit")]
    #[case(
        "foo/です".into(),
        WpContext::View,
        "/plugins/foo/%E3%81%A7%E3%81%99?context=view"
    )]
    fn retrieve_plugin(
        plugins_endpoint: PluginsEndpoint,
        #[case] plugin_slug: PluginSlug,
        #[case] context: WpContext,
        #[case] expected_path: &str,
    ) {
        validate_endpoint(
            plugins_endpoint.retrieve(context, &plugin_slug),
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
        plugins_endpoint: PluginsEndpoint,
        #[case] plugin_slug: PluginSlug,
        #[case] context: WpContext,
        #[case] fields: &[SparsePluginField],
        #[case] expected_path: &str,
    ) {
        validate_endpoint(
            plugins_endpoint.filter_retrieve(context, &plugin_slug, fields),
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
        plugins_endpoint: PluginsEndpoint,
        #[case] plugin_slug: PluginSlug,
        #[case] expected_path: &str,
    ) {
        validate_endpoint(plugins_endpoint.update(&plugin_slug), expected_path);
    }

    #[fixture]
    fn plugins_endpoint(fixture_api_base_url: Arc<ApiBaseUrl>) -> PluginsEndpoint {
        PluginsEndpoint::new(fixture_api_base_url)
    }
}

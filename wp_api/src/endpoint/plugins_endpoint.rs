use url::Url;

use crate::{plugins::PluginListParams, ApiBaseUrl, WPContext};

pub struct PluginsEndpoint {
    api_base_url: ApiBaseUrl,
}

impl PluginsEndpoint {
    pub fn new(api_base_url: ApiBaseUrl) -> Self {
        Self { api_base_url }
    }

    pub fn create(&self) -> Url {
        self.plugins_base_url()
    }

    pub fn list(&self, context: WPContext, params: Option<&PluginListParams>) -> Url {
        let mut url = self.plugins_base_url();
        url.query_pairs_mut()
            .append_pair("context", context.as_str());
        if let Some(params) = params {
            url.query_pairs_mut().extend_pairs(params.query_pairs());
        }
        url
    }

    pub fn retrieve(&self, context: WPContext, plugin: String) -> Url {
        let mut url = self.plugins_url_with_slug(plugin);
        url.query_pairs_mut()
            .append_pair("context", context.as_str());
        url
    }

    pub fn update(&self, plugin: String) -> Url {
        self.plugins_url_with_slug(plugin)
    }

    fn plugins_base_url(&self) -> Url {
        self.api_base_url.by_appending("plugins")
    }

    fn plugins_url_with_slug(&self, plugin: String) -> Url {
        self.api_base_url
            // The '/' character has to be preserved and not get encoded
            .by_extending(["plugins"].into_iter().chain(plugin.split('/')))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        endpoint::tests::{fixture_api_base_url, validate_endpoint},
        plugins::PluginStatus,
        ApiEndpoint,
    };
    use rstest::*;

    #[rstest]
    fn create_plugin(plugins_endpoint: PluginsEndpoint) {
        validate_endpoint(plugins_endpoint.create(), "/plugins");
    }

    #[rstest]
    fn list_plugins_with_params(plugins_endpoint: PluginsEndpoint) {
        let params = PluginListParams {
            search: Some("foo".to_string()),
            status: Some(PluginStatus::Active),
        };
        validate_endpoint(
            plugins_endpoint.list(WPContext::Edit, Some(&params)),
            "/plugins?context=edit&search=foo&status=active",
        );
    }

    #[rstest]
    fn retrieve_plugin(plugins_endpoint: PluginsEndpoint) {
        validate_endpoint(
            plugins_endpoint.retrieve(WPContext::View, "hello-dolly/hello".to_string()),
            "/plugins/hello-dolly/hello?context=view",
        );
    }

    #[fixture]
    fn plugins_endpoint(fixture_api_base_url: ApiBaseUrl) -> PluginsEndpoint {
        ApiEndpoint::new(fixture_api_base_url).plugins
    }
}

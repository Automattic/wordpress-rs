use url::Url;

use crate::{plugins::PluginListParams, ApiBaseUrl, WPContext};

pub struct PluginsEndpoint {
    api_base_url: ApiBaseUrl,
}

impl PluginsEndpoint {
    pub fn new(api_base_url: ApiBaseUrl) -> Self {
        Self { api_base_url }
    }

    pub fn list(&self, context: WPContext, params: Option<&PluginListParams>) -> Url {
        let mut url = self.api_base_url.by_appending("plugins");
        url.query_pairs_mut()
            .append_pair("context", context.as_str());
        if let Some(params) = params {
            url.query_pairs_mut().extend_pairs(params.query_pairs());
        }
        url
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{plugins::PluginStatus, ApiEndpoint};
    use rstest::*;

    #[rstest]
    fn list_plugins_with_params(api_base_url: ApiBaseUrl, plugins_endpoint: PluginsEndpoint) {
        let params = PluginListParams {
            search: Some("foo".to_string()),
            status: Some(PluginStatus::Active),
        };
        validate_endpoint(
            plugins_endpoint.list(WPContext::Edit, Some(&params)),
            "/plugins?context=edit&search=foo&status=active",
            &api_base_url,
        );
    }

    #[fixture]
    fn api_base_url() -> ApiBaseUrl {
        ApiBaseUrl::new("https://foo.com").unwrap()
    }

    #[fixture]
    fn plugins_endpoint(api_base_url: ApiBaseUrl) -> PluginsEndpoint {
        ApiEndpoint::new(api_base_url).plugins
    }

    fn validate_endpoint(endpoint_url: Url, path: &str, api_base_url: &ApiBaseUrl) {
        assert_eq!(
            endpoint_url.as_str(),
            format!("{}{}", api_base_url.as_str(), path)
        );
    }
}

use std::sync::Arc;
use wp_api::{
    ParsedUrl, WpAuthentication, api_client_generate_api_client, api_client_generate_endpoint_impl,
    request::{RequestExecutor, endpoint::ApiBaseUrl},
};

use super::endpoint::jetpack_connection_endpoint::{
    JetpackConnectionRequestBuilder, JetpackConnectionRequestExecutor,
};

#[derive(Debug, uniffi::Object)]
struct UniffiWpComApiRequestBuilder {
    inner: WpComApiRequestBuilder,
}

#[uniffi::export]
impl UniffiWpComApiRequestBuilder {
    #[uniffi::constructor]
    pub fn new(site_url: Arc<ParsedUrl>, authentication: WpAuthentication) -> Self {
        Self {
            inner: WpComApiRequestBuilder::new(site_url, authentication),
        }
    }
}

#[derive(Debug)]
pub struct WpComApiRequestBuilder {
    jetpack_connection: Arc<JetpackConnectionRequestBuilder>,
}

impl WpComApiRequestBuilder {
    pub fn new(site_url: Arc<ParsedUrl>, authentication: WpAuthentication) -> Self {
        let api_base_url: Arc<ApiBaseUrl> = Arc::new(site_url.inner.clone().into());
        Self {
            jetpack_connection: JetpackConnectionRequestBuilder::new(
                api_base_url.clone(),
                authentication.clone(),
            )
            .into(),
        }
    }
}

#[derive(Debug, uniffi::Object)]
struct UniffiWpComApiClient {
    inner: WpComApiClient,
}

#[uniffi::export]
impl UniffiWpComApiClient {
    #[uniffi::constructor]
    fn new(authentication: WpAuthentication, request_executor: Arc<dyn RequestExecutor>) -> Self {
        Self {
            inner: WpComApiClient::new(authentication, request_executor),
        }
    }
}

#[derive(Debug)]
pub struct WpComApiClient {
    jetpack_connection: Arc<JetpackConnectionRequestExecutor>,
}

impl WpComApiClient {
    pub fn new(
        authentication: WpAuthentication,
        request_executor: Arc<dyn RequestExecutor>,
    ) -> Self {
        let url = url::Url::parse("https://public-api.wordpress.com").expect("This is a valid URL");
        let api_base_url: Arc<ApiBaseUrl> = Arc::new(ApiBaseUrl::with_api_url(url));

        api_client_generate_api_client!(
            api_base_url,
            authentication,
            request_executor;
            jetpack_connection
        )
    }
}
api_client_generate_endpoint_impl!(WpComApi, jetpack_connection);

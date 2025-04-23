use std::sync::Arc;
use wp_api::{
    ParsedUrl, WpAppNotifier, WpAuthentication, api_client_generate_api_client,
    api_client_generate_endpoint_impl, middleware::WpApiMiddlewarePipeline,
    request::RequestExecutor,
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
    pub fn new(api_root_url: Arc<ParsedUrl>, authentication: WpAuthentication) -> Self {
        Self {
            inner: WpComApiRequestBuilder::new(api_root_url, authentication),
        }
    }
}

#[derive(Debug)]
pub struct WpComApiRequestBuilder {
    jetpack_connection: Arc<JetpackConnectionRequestBuilder>,
}

impl WpComApiRequestBuilder {
    pub fn new(api_root_url: Arc<ParsedUrl>, authentication: WpAuthentication) -> Self {
        Self {
            jetpack_connection: JetpackConnectionRequestBuilder::new(
                api_root_url.clone(),
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
    fn new(
        authentication: WpAuthentication,
        request_executor: Arc<dyn RequestExecutor>,
        app_notifier: Arc<dyn WpAppNotifier>,
        middleware_pipeline: Arc<WpApiMiddlewarePipeline>,
    ) -> Self {
        Self {
            inner: WpComApiClient::new(
                authentication,
                request_executor,
                app_notifier,
                middleware_pipeline,
            ),
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
        app_notifier: Arc<dyn WpAppNotifier>,
        middleware_pipeline: Arc<WpApiMiddlewarePipeline>,
    ) -> Self {
        let url = url::Url::parse("https://public-api.wordpress.com").expect("This is a valid URL");
        let api_root_url: Arc<ParsedUrl> = ParsedUrl::new(url).into();

        api_client_generate_api_client!(
            api_root_url,
            authentication,
            request_executor,
            app_notifier,
            middleware_pipeline;
            jetpack_connection
        )
    }
}
api_client_generate_endpoint_impl!(WpComApi, jetpack_connection);

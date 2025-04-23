use std::sync::Arc;
use wp_api::{
    ParsedUrl, WpAppNotifier, WpAuthentication, api_client_generate_api_client,
    api_client_generate_endpoint_impl, middleware::WpApiMiddlewarePipeline,
    request::RequestExecutor,
};

use super::endpoint::connection_endpoint::{ConnectionRequestBuilder, ConnectionRequestExecutor};

#[derive(Debug, uniffi::Object)]
struct UniffiJetpackApiRequestBuilder {
    inner: JetpackApiRequestBuilder,
}

#[uniffi::export]
impl UniffiJetpackApiRequestBuilder {
    #[uniffi::constructor]
    pub fn new(api_root_url: Arc<ParsedUrl>, authentication: WpAuthentication) -> Self {
        Self {
            inner: JetpackApiRequestBuilder::new(api_root_url, authentication),
        }
    }
}

#[derive(Debug)]
pub struct JetpackApiRequestBuilder {
    connection: Arc<ConnectionRequestBuilder>,
}

impl JetpackApiRequestBuilder {
    pub fn new(api_root_url: Arc<ParsedUrl>, authentication: WpAuthentication) -> Self {
        Self {
            connection: ConnectionRequestBuilder::new(api_root_url.clone(), authentication.clone())
                .into(),
        }
    }
}

#[derive(Debug, uniffi::Object)]
struct UniffiJetpackApiClient {
    inner: JetpackApiClient,
}

#[uniffi::export]
impl UniffiJetpackApiClient {
    #[uniffi::constructor]
    fn new(
        site_url: Arc<ParsedUrl>,
        authentication: WpAuthentication,
        request_executor: Arc<dyn RequestExecutor>,
        app_notifier: Arc<dyn WpAppNotifier>,
        middleware_pipeline: Arc<WpApiMiddlewarePipeline>,
    ) -> Self {
        Self {
            inner: JetpackApiClient::new(
                site_url,
                authentication,
                request_executor,
                app_notifier,
                middleware_pipeline,
            ),
        }
    }
}

#[derive(Debug)]
pub struct JetpackApiClient {
    pub connection: Arc<ConnectionRequestExecutor>,
}

impl JetpackApiClient {
    pub fn new(
        api_root_url: Arc<ParsedUrl>,
        authentication: WpAuthentication,
        request_executor: Arc<dyn RequestExecutor>,
        app_notifier: Arc<dyn WpAppNotifier>,
        middleware_pipeline: Arc<WpApiMiddlewarePipeline>,
    ) -> Self {
        api_client_generate_api_client!(
            api_root_url,
            authentication,
            request_executor,
            app_notifier,
            middleware_pipeline;
            connection
        )
    }
}
api_client_generate_endpoint_impl!(JetpackApi, connection);

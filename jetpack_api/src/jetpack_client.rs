use std::sync::Arc;

use crate::request::endpoint::connection_endpoint::{
    ConnectionRequestBuilder, ConnectionRequestExecutor,
};
use crate::request::JetpackRequestExecutor;
use wp_api::{
    api_client_generate_api_client, api_client_generate_endpoint_impl,
    api_client_generate_request_builder, request::endpoint::ApiBaseUrl, ParsedUrl,
    WpAuthentication,
};

#[derive(Debug, uniffi::Object)]
struct UniffiJetpackRequestBuilder {
    inner: JetpackRequestBuilder,
}

#[uniffi::export]
impl UniffiJetpackRequestBuilder {
    #[uniffi::constructor]
    pub fn new(site_url: Arc<ParsedUrl>, authentication: WpAuthentication) -> Self {
        Self {
            inner: JetpackRequestBuilder::new(site_url, authentication),
        }
    }
}

#[derive(Debug)]
pub struct JetpackRequestBuilder {
    connection: Arc<ConnectionRequestBuilder>,
}

impl JetpackRequestBuilder {
    pub fn new(site_url: Arc<ParsedUrl>, authentication: WpAuthentication) -> Self {
        let api_base_url: Arc<ApiBaseUrl> = Arc::new(site_url.inner.clone().into());
        api_client_generate_request_builder!(
            api_base_url,
            authentication;
            connection
        )
    }
}

#[derive(Debug, uniffi::Object)]
struct UniffiJetpackClient {
    inner: JetpackClient,
}

#[uniffi::export]
impl UniffiJetpackClient {
    #[uniffi::constructor]
    fn new(
        site_url: Arc<ParsedUrl>,
        authentication: WpAuthentication,
        request_executor: Arc<dyn JetpackRequestExecutor>,
    ) -> Self {
        Self {
            inner: JetpackClient::new(site_url, authentication, request_executor),
        }
    }
}

#[derive(Debug)]
pub struct JetpackClient {
    connection: Arc<ConnectionRequestExecutor>,
}

impl JetpackClient {
    pub fn new(
        site_url: Arc<ParsedUrl>,
        authentication: WpAuthentication,
        request_executor: Arc<dyn JetpackRequestExecutor>,
    ) -> Self {
        let api_base_url: Arc<ApiBaseUrl> = Arc::new(site_url.inner.clone().into());

        api_client_generate_api_client!(
            api_base_url,
            authentication,
            request_executor;
            connection
        )
    }
}

api_client_generate_endpoint_impl!(Jetpack, connection);

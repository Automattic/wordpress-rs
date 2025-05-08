use super::endpoint::connection_endpoint::{
    JpConnectionRequestBuilder, JpConnectionRequestExecutor,
};
use std::sync::Arc;
use wp_api::{
    ParsedUrl, WpApiClientDelegate, api_client_generate_api_client,
    api_client_generate_endpoint_impl, auth::WpAuthenticationProvider,
};

#[derive(uniffi::Object)]
struct UniffiJetpackApiRequestBuilder {
    inner: JetpackApiRequestBuilder,
}

#[uniffi::export]
impl UniffiJetpackApiRequestBuilder {
    #[uniffi::constructor]
    pub fn new(api_root_url: Arc<ParsedUrl>, auth_provider: Arc<WpAuthenticationProvider>) -> Self {
        Self {
            inner: JetpackApiRequestBuilder::new(api_root_url, auth_provider),
        }
    }
}

pub struct JetpackApiRequestBuilder {
    jp_connection: Arc<JpConnectionRequestBuilder>,
}

impl JetpackApiRequestBuilder {
    pub fn new(api_root_url: Arc<ParsedUrl>, auth_provider: Arc<WpAuthenticationProvider>) -> Self {
        Self {
            jp_connection: JpConnectionRequestBuilder::new(api_root_url, auth_provider).into(),
        }
    }
}

#[derive(uniffi::Object)]
struct UniffiJetpackApiClient {
    inner: JetpackApiClient,
}

#[uniffi::export]
impl UniffiJetpackApiClient {
    #[uniffi::constructor]
    fn new(site_url: Arc<ParsedUrl>, delegate: WpApiClientDelegate) -> Self {
        Self {
            inner: JetpackApiClient::new(site_url, delegate),
        }
    }
}

pub struct JetpackApiClient {
    pub jp_connection: Arc<JpConnectionRequestExecutor>,
}

impl JetpackApiClient {
    pub fn new(api_root_url: Arc<ParsedUrl>, delegate: WpApiClientDelegate) -> Self {
        api_client_generate_api_client!(
            api_root_url,
            delegate;
            jp_connection
        )
    }
}
api_client_generate_endpoint_impl!(JetpackApi, jp_connection);

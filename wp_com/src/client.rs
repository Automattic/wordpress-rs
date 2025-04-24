use std::sync::Arc;
use wp_api::{
    ParsedUrl, WpApiClientDelegate, api_client_generate_api_client,
    api_client_generate_endpoint_impl, auth::WpAuthenticationProvider,
};

use super::endpoint::jetpack_connection_endpoint::{
    JetpackConnectionRequestBuilder, JetpackConnectionRequestExecutor,
};

#[derive(uniffi::Object)]
struct UniffiWpComApiRequestBuilder {
    inner: WpComApiRequestBuilder,
}

#[uniffi::export]
impl UniffiWpComApiRequestBuilder {
    #[uniffi::constructor]
    pub fn new(api_root_url: Arc<ParsedUrl>, auth_provider: Arc<WpAuthenticationProvider>) -> Self {
        Self {
            inner: WpComApiRequestBuilder::new(api_root_url, auth_provider),
        }
    }
}

pub struct WpComApiRequestBuilder {
    jetpack_connection: Arc<JetpackConnectionRequestBuilder>,
}

impl WpComApiRequestBuilder {
    pub fn new(api_root_url: Arc<ParsedUrl>, auth_provider: Arc<WpAuthenticationProvider>) -> Self {
        Self {
            jetpack_connection: JetpackConnectionRequestBuilder::new(api_root_url, auth_provider)
                .into(),
        }
    }
}

#[derive(uniffi::Object)]
struct UniffiWpComApiClient {
    inner: WpComApiClient,
}

#[uniffi::export]
impl UniffiWpComApiClient {
    #[uniffi::constructor]
    fn new(delegate: WpApiClientDelegate) -> Self {
        Self {
            inner: WpComApiClient::new(delegate),
        }
    }
}

pub struct WpComApiClient {
    jetpack_connection: Arc<JetpackConnectionRequestExecutor>,
}

impl WpComApiClient {
    pub fn new(delegate: WpApiClientDelegate) -> Self {
        let url = url::Url::parse("https://public-api.wordpress.com").expect("This is a valid URL");
        let api_root_url: Arc<ParsedUrl> = ParsedUrl::new(url).into();

        api_client_generate_api_client!(
            api_root_url,
            delegate;
            jetpack_connection
        )
    }
}
api_client_generate_endpoint_impl!(WpComApi, jetpack_connection);

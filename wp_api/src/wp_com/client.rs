use super::endpoint::jetpack_connection_endpoint::{
    JetpackConnectionRequestBuilder, JetpackConnectionRequestExecutor,
};
use super::endpoint::oauth2::{Oauth2RequestBuilder, Oauth2RequestExecutor};
use super::endpoint::subscribers::{SubscribersRequestBuilder, SubscribersRequestExecutor};
use super::endpoint::support_bots::SupportBotsRequestExecutor;
use crate::api_client_generate_request_builder;
use crate::wp_com::endpoint::support_bots::SupportBotsRequestBuilder;
use crate::{
    ParsedUrl, WpApiClientDelegate, api_client_generate_api_client,
    api_client_generate_endpoint_impl, auth::WpAuthenticationProvider,
};
use std::sync::Arc;

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
    oauth2: Arc<Oauth2RequestBuilder>,
    subscribers: Arc<SubscribersRequestBuilder>,
    support_bots: Arc<SupportBotsRequestBuilder>,
}

impl WpComApiRequestBuilder {
    pub fn new(api_root_url: Arc<ParsedUrl>, auth_provider: Arc<WpAuthenticationProvider>) -> Self {
        api_client_generate_request_builder!(
            api_root_url,
            auth_provider;
            jetpack_connection,
            oauth2,
            subscribers,
            support_bots
        )
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
    oauth2: Arc<Oauth2RequestExecutor>,
    subscribers: Arc<SubscribersRequestExecutor>,
    support_bots: Arc<SupportBotsRequestExecutor>,
}

impl WpComApiClient {
    pub fn new(delegate: WpApiClientDelegate) -> Self {
        let url = url::Url::parse("https://public-api.wordpress.com").expect("This is a valid URL");
        let api_root_url: Arc<ParsedUrl> = ParsedUrl::new(url).into();

        api_client_generate_api_client!(
            api_root_url,
            delegate;
            jetpack_connection,
            oauth2,
            subscribers,
            support_bots
        )
    }
}
api_client_generate_endpoint_impl!(WpComApi, jetpack_connection);
api_client_generate_endpoint_impl!(WpComApi, oauth2);
api_client_generate_endpoint_impl!(WpComApi, subscribers);
api_client_generate_endpoint_impl!(WpComApi, support_bots);

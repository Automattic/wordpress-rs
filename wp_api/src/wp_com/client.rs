use super::endpoint::{
    followers_endpoint::{FollowersRequestBuilder, FollowersRequestExecutor},
    freshly_pressed::{FreshlyPressedRequestBuilder, FreshlyPressedRequestExecutor},
    jetpack_connection_endpoint::{
        JetpackConnectionRequestBuilder, JetpackConnectionRequestExecutor,
    },
    oauth2::{Oauth2RequestBuilder, Oauth2RequestExecutor},
    subscribers_endpoint::{SubscribersRequestBuilder, SubscribersRequestExecutor},
    support_bots_endpoint::{SupportBotsRequestBuilder, SupportBotsRequestExecutor},
    support_eligibility_endpoint::{
        SupportEligibilityRequestBuilder, SupportEligibilityRequestExecutor,
    },
    support_tickets_endpoint::{SupportTicketsRequestBuilder, SupportTicketsRequestExecutor},
};
use crate::{
    api_client::WpApiClientDelegate, api_client_generate_api_client,
    api_client_generate_endpoint_impl, api_client_generate_request_builder,
    auth::WpAuthenticationProvider, request::endpoint::ApiUrlResolver,
    wp_com::endpoint::WpComApiClientInternalUrlResolver,
};
use std::sync::Arc;

#[derive(uniffi::Object)]
struct UniffiWpComApiRequestBuilder {
    inner: WpComApiRequestBuilder,
}

#[uniffi::export]
impl UniffiWpComApiRequestBuilder {
    #[uniffi::constructor]
    pub fn new(auth_provider: Arc<WpAuthenticationProvider>) -> Self {
        Self {
            inner: WpComApiRequestBuilder::new(auth_provider),
        }
    }
}

pub struct WpComApiRequestBuilder {
    followers: Arc<FollowersRequestBuilder>,
    freshly_pressed: Arc<FreshlyPressedRequestBuilder>,
    jetpack_connection: Arc<JetpackConnectionRequestBuilder>,
    oauth2: Arc<Oauth2RequestBuilder>,
    subscribers: Arc<SubscribersRequestBuilder>,
    support_bots: Arc<SupportBotsRequestBuilder>,
    support_eligibility: Arc<SupportEligibilityRequestBuilder>,
    support_tickets: Arc<SupportTicketsRequestBuilder>,
}

impl WpComApiRequestBuilder {
    pub fn new(auth_provider: Arc<WpAuthenticationProvider>) -> Self {
        let api_url_resolver: Arc<dyn ApiUrlResolver> =
            Arc::new(WpComApiClientInternalUrlResolver::default());
        api_client_generate_request_builder!(
            api_url_resolver,
            auth_provider;
            followers,
            freshly_pressed,
            jetpack_connection,
            oauth2,
            subscribers,
            support_bots,
            support_eligibility,
            support_tickets
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
    followers: Arc<FollowersRequestExecutor>,
    freshly_pressed: Arc<FreshlyPressedRequestExecutor>,
    jetpack_connection: Arc<JetpackConnectionRequestExecutor>,
    oauth2: Arc<Oauth2RequestExecutor>,
    subscribers: Arc<SubscribersRequestExecutor>,
    support_bots: Arc<SupportBotsRequestExecutor>,
    support_eligibility: Arc<SupportEligibilityRequestExecutor>,
    support_tickets: Arc<SupportTicketsRequestExecutor>,
}

impl WpComApiClient {
    pub fn new(delegate: WpApiClientDelegate) -> Self {
        let api_url_resolver: Arc<dyn ApiUrlResolver> =
            Arc::new(WpComApiClientInternalUrlResolver::default());

        api_client_generate_api_client!(
            api_url_resolver,
            delegate;
            followers,
            freshly_pressed,
            jetpack_connection,
            oauth2,
            subscribers,
            support_bots,
            support_eligibility,
            support_tickets
        )
    }
}
api_client_generate_endpoint_impl!(WpComApi, followers);
api_client_generate_endpoint_impl!(WpComApi, freshly_pressed);
api_client_generate_endpoint_impl!(WpComApi, jetpack_connection);
api_client_generate_endpoint_impl!(WpComApi, oauth2);
api_client_generate_endpoint_impl!(WpComApi, subscribers);
api_client_generate_endpoint_impl!(WpComApi, support_bots);
api_client_generate_endpoint_impl!(WpComApi, support_eligibility);
api_client_generate_endpoint_impl!(WpComApi, support_tickets);

use super::endpoint::connection_endpoint::{ConnectionRequestBuilder, ConnectionRequestExecutor};
use crate::{
    api_client::WpApiClientDelegate,
    api_client_generate_api_client, api_client_generate_endpoint_impl,
    auth::WpAuthenticationProvider,
    parsed_url::ParsedUrl,
    request::endpoint::{ApiUrlResolver, WpOrgSiteApiUrlResolver},
};
use std::sync::Arc;

#[derive(uniffi::Object)]
struct UniffiJetpackApiRequestBuilder {
    inner: JetpackApiRequestBuilder,
}

#[uniffi::export]
impl UniffiJetpackApiRequestBuilder {
    #[uniffi::constructor]
    pub fn new(
        api_url_resolver: Arc<dyn ApiUrlResolver>,
        auth_provider: Arc<WpAuthenticationProvider>,
    ) -> Self {
        Self {
            inner: JetpackApiRequestBuilder::new(api_url_resolver, auth_provider),
        }
    }

    #[uniffi::constructor]
    pub fn with_api_root_url(
        api_root_url: Arc<ParsedUrl>,
        auth_provider: Arc<WpAuthenticationProvider>,
    ) -> Self {
        Self::new(jetpack_api_url_resolver(api_root_url), auth_provider)
    }
}

pub struct JetpackApiRequestBuilder {
    connection: Arc<ConnectionRequestBuilder>,
}

impl JetpackApiRequestBuilder {
    pub fn new(
        api_url_resolver: Arc<dyn ApiUrlResolver>,
        auth_provider: Arc<WpAuthenticationProvider>,
    ) -> Self {
        Self {
            connection: ConnectionRequestBuilder::new(api_url_resolver, auth_provider).into(),
        }
    }

    pub fn with_api_root_url(
        api_root_url: Arc<ParsedUrl>,
        auth_provider: Arc<WpAuthenticationProvider>,
    ) -> Self {
        Self::new(jetpack_api_url_resolver(api_root_url), auth_provider)
    }
}

#[derive(uniffi::Object)]
struct UniffiJetpackApiClient {
    inner: JetpackApiClient,
}

#[uniffi::export]
impl UniffiJetpackApiClient {
    #[uniffi::constructor]
    fn new(api_url_resolver: Arc<dyn ApiUrlResolver>, delegate: WpApiClientDelegate) -> Self {
        Self {
            inner: JetpackApiClient::new(api_url_resolver, delegate),
        }
    }

    #[uniffi::constructor]
    fn with_api_root_url(api_root_url: Arc<ParsedUrl>, delegate: WpApiClientDelegate) -> Self {
        Self {
            inner: JetpackApiClient::with_api_root_url(api_root_url, delegate),
        }
    }
}

pub struct JetpackApiClient {
    pub connection: Arc<ConnectionRequestExecutor>,
}

impl JetpackApiClient {
    pub fn new(api_url_resolver: Arc<dyn ApiUrlResolver>, delegate: WpApiClientDelegate) -> Self {
        api_client_generate_api_client!(
            api_url_resolver,
            delegate;
            connection
        )
    }

    pub fn with_api_root_url(api_root_url: Arc<ParsedUrl>, delegate: WpApiClientDelegate) -> Self {
        Self::new(jetpack_api_url_resolver(api_root_url), delegate)
    }
}
api_client_generate_endpoint_impl!(JetpackApi, connection);

fn jetpack_api_url_resolver(api_root_url: Arc<ParsedUrl>) -> Arc<dyn ApiUrlResolver> {
    Arc::new(WpOrgSiteApiUrlResolver::new(api_root_url))
}

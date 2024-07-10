use crate::request::{
    endpoint::{
        application_passwords_endpoint::{
            ApplicationPasswordsRequestBuilder, ApplicationPasswordsRequestExecutor,
        },
        plugins_endpoint::{PluginsRequestBuilder, PluginsRequestExecutor},
        users_endpoint::{UsersRequestBuilder, UsersRequestExecutor},
        wp_site_health_tests_endpoint::{
            WpSiteHealthTestsRequestBuilder, WpSiteHealthTestsRequestExecutor,
        },
        ApiBaseUrl,
    },
    RequestExecutor,
};
use crate::{ParsedUrl, WpAuthentication};
use std::sync::Arc;

#[derive(Debug, uniffi::Object)]
struct UniffiWpApiRequestBuilder {
    inner: WpApiRequestBuilder,
}

#[uniffi::export]
impl UniffiWpApiRequestBuilder {
    #[uniffi::constructor]
    pub fn new(site_url: Arc<ParsedUrl>, authentication: WpAuthentication) -> Self {
        Self {
            inner: WpApiRequestBuilder::new(site_url, authentication),
        }
    }
}

#[derive(Debug)]
pub struct WpApiRequestBuilder {
    application_passwords: Arc<ApplicationPasswordsRequestBuilder>,
    users: Arc<UsersRequestBuilder>,
    plugins: Arc<PluginsRequestBuilder>,
    wp_site_health_tests: Arc<WpSiteHealthTestsRequestBuilder>,
}

impl WpApiRequestBuilder {
    pub fn new(site_url: Arc<ParsedUrl>, authentication: WpAuthentication) -> Self {
        let api_base_url: Arc<ApiBaseUrl> = Arc::new(site_url.inner.clone().into());

        Self {
            application_passwords: ApplicationPasswordsRequestBuilder::new(
                api_base_url.clone(),
                authentication.clone(),
            )
            .into(),
            users: UsersRequestBuilder::new(api_base_url.clone(), authentication.clone()).into(),
            plugins: PluginsRequestBuilder::new(api_base_url.clone(), authentication.clone())
                .into(),
            wp_site_health_tests: WpSiteHealthTestsRequestBuilder::new(
                api_base_url.clone(),
                authentication.clone(),
            )
            .into(),
        }
    }
}

#[derive(Debug, uniffi::Object)]
struct UniffiWpApiClient {
    inner: WpApiClient,
}

#[uniffi::export]
impl UniffiWpApiClient {
    #[uniffi::constructor]
    fn new(
        site_url: Arc<ParsedUrl>,
        authentication: WpAuthentication,
        request_executor: Arc<dyn RequestExecutor>,
    ) -> Self {
        Self {
            inner: WpApiClient::new(site_url, authentication, request_executor),
        }
    }
}

#[derive(Debug)]
pub struct WpApiClient {
    application_passwords: Arc<ApplicationPasswordsRequestExecutor>,
    users: Arc<UsersRequestExecutor>,
    plugins: Arc<PluginsRequestExecutor>,
    wp_site_health_tests: Arc<WpSiteHealthTestsRequestExecutor>,
}

impl WpApiClient {
    pub fn new(
        site_url: Arc<ParsedUrl>,
        authentication: WpAuthentication,
        request_executor: Arc<dyn RequestExecutor>,
    ) -> Self {
        let api_base_url: Arc<ApiBaseUrl> = Arc::new(site_url.inner.clone().into());

        Self {
            application_passwords: ApplicationPasswordsRequestExecutor::new(
                api_base_url.clone(),
                authentication.clone(),
                request_executor.clone(),
            )
            .into(),
            users: UsersRequestExecutor::new(
                api_base_url.clone(),
                authentication.clone(),
                request_executor.clone(),
            )
            .into(),
            plugins: PluginsRequestExecutor::new(
                api_base_url.clone(),
                authentication.clone(),
                request_executor.clone(),
            )
            .into(),
            wp_site_health_tests: WpSiteHealthTestsRequestExecutor::new(
                api_base_url.clone(),
                authentication.clone(),
                request_executor.clone(),
            )
            .into(),
        }
    }
}

macro_helper::generate_endpoint_impl!(application_passwords);
macro_helper::generate_endpoint_impl!(plugins);
macro_helper::generate_endpoint_impl!(users);
macro_helper::generate_endpoint_impl!(wp_site_health_tests);

mod macro_helper {
    macro_rules! generate_endpoint_impl {
        ($ident:ident) => {
            paste::paste! {
                #[uniffi::export]
                impl UniffiWpApiRequestBuilder {

                    fn $ident(&self) -> Arc<[<$ident:camel RequestBuilder>]> {
                        self.inner.$ident.clone()
                    }
                }

                impl WpApiRequestBuilder {
                    pub fn $ident(&self) -> &[<$ident:camel RequestBuilder>] {
                        self.$ident.as_ref()
                    }
                }

                #[uniffi::export]
                impl UniffiWpApiClient {
                    fn $ident(&self) -> Arc<[<$ident:camel RequestExecutor>]> {
                        self.inner.$ident.clone()
                    }
                }

                impl WpApiClient {
                    pub fn $ident(&self) -> &[<$ident:camel RequestExecutor>] {
                        self.$ident.as_ref()
                    }
                }
            }
        };
    }

    pub(super) use generate_endpoint_impl;
}

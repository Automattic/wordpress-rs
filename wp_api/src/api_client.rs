use crate::request::{
    endpoint::{
        application_passwords_endpoint::{
            ApplicationPasswordsRequestBuilder, ApplicationPasswordsRequestExecutor,
        },
        plugins_endpoint::{PluginsRequestBuilder, PluginsRequestExecutor},
        post_types_endpoint::{PostTypesRequestBuilder, PostTypesRequestExecutor},
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
    post_types: Arc<PostTypesRequestBuilder>,
    wp_site_health_tests: Arc<WpSiteHealthTestsRequestBuilder>,
}

impl WpApiRequestBuilder {
    pub fn new(site_url: Arc<ParsedUrl>, authentication: WpAuthentication) -> Self {
        let api_base_url: Arc<ApiBaseUrl> = Arc::new(site_url.inner.clone().into());
        macro_helper::wp_api_request_builder!(
            api_base_url,
            authentication;
            application_passwords,
            users,
            post_types,
            plugins,
            wp_site_health_tests
        )
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
    post_types: Arc<PostTypesRequestExecutor>,
    wp_site_health_tests: Arc<WpSiteHealthTestsRequestExecutor>,
}

impl WpApiClient {
    pub fn new(
        site_url: Arc<ParsedUrl>,
        authentication: WpAuthentication,
        request_executor: Arc<dyn RequestExecutor>,
    ) -> Self {
        let api_base_url: Arc<ApiBaseUrl> = Arc::new(site_url.inner.clone().into());

        macro_helper::wp_api_client!(
            api_base_url,
            authentication,
            request_executor;
            application_passwords,
            users,
            post_types,
            plugins,
            wp_site_health_tests
        )
    }
}

macro_helper::generate_endpoint_impl!(application_passwords);
macro_helper::generate_endpoint_impl!(plugins);
macro_helper::generate_endpoint_impl!(post_types);
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

    macro_rules! wp_api_request_builder {
        ($api_base_url:ident, $authentication:ident; $($element:expr),*) => {
            paste::paste! {
                Self {
                    $($element: [<$element:camel RequestBuilder>]::new(
                        $api_base_url.clone(),
                        $authentication.clone(),
                    )
                    .into(),)*
                }
            }
        };
    }

    macro_rules! wp_api_client {
        ($api_base_url:ident, $authentication:ident, $request_executor:ident; $($element:expr),*) => {
            paste::paste! {
                Self {
                    $($element: [<$element:camel RequestExecutor>]::new(
                        $api_base_url.clone(),
                        $authentication.clone(),
                        $request_executor.clone(),
                    )
                    .into(),)*
                }
            }
        };
    }

    pub(super) use generate_endpoint_impl;
    pub(super) use wp_api_client;
    pub(super) use wp_api_request_builder;
}

use crate::request::{
    endpoint::{
        application_passwords_endpoint::{
            ApplicationPasswordsRequestBuilder, ApplicationPasswordsRequestExecutor,
        },
        plugins_endpoint::{PluginsRequestBuilder, PluginsRequestExecutor},
        post_types_endpoint::{PostTypesRequestBuilder, PostTypesRequestExecutor},
        posts_endpoint::{PostsRequestBuilder, PostsRequestExecutor},
        site_settings_endpoint::{SiteSettingsRequestBuilder, SiteSettingsRequestExecutor},
        users_endpoint::{UsersRequestBuilder, UsersRequestExecutor},
        wp_site_health_tests_endpoint::{
            WpSiteHealthTestsRequestBuilder, WpSiteHealthTestsRequestExecutor,
        },
        ApiBaseUrl,
    },
    RequestExecutor,
};
use crate::{
    api_client_generate_api_client, api_client_generate_endpoint_impl,
    api_client_generate_request_builder, ParsedUrl, WpAuthentication,
};
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
    plugins: Arc<PluginsRequestBuilder>,
    post_types: Arc<PostTypesRequestBuilder>,
    posts: Arc<PostsRequestBuilder>,
    site_settings: Arc<SiteSettingsRequestBuilder>,
    users: Arc<UsersRequestBuilder>,
    wp_site_health_tests: Arc<WpSiteHealthTestsRequestBuilder>,
}

impl WpApiRequestBuilder {
    pub fn new(site_url: Arc<ParsedUrl>, authentication: WpAuthentication) -> Self {
        let api_base_url: Arc<ApiBaseUrl> = Arc::new(site_url.inner.clone().into());
        api_client_generate_request_builder!(
            api_base_url,
            authentication;
            application_passwords,
            plugins,
            post_types,
            posts,
            users,
            site_settings,
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
    plugins: Arc<PluginsRequestExecutor>,
    post_types: Arc<PostTypesRequestExecutor>,
    posts: Arc<PostsRequestExecutor>,
    site_settings: Arc<SiteSettingsRequestExecutor>,
    users: Arc<UsersRequestExecutor>,
    wp_site_health_tests: Arc<WpSiteHealthTestsRequestExecutor>,
}

impl WpApiClient {
    pub fn new(
        site_url: Arc<ParsedUrl>,
        authentication: WpAuthentication,
        request_executor: Arc<dyn RequestExecutor>,
    ) -> Self {
        let api_base_url: Arc<ApiBaseUrl> = Arc::new(site_url.inner.clone().into());

        api_client_generate_api_client!(
            api_base_url,
            authentication,
            request_executor;
            application_passwords,
            plugins,
            post_types,
            posts,
            site_settings,
            users,
            wp_site_health_tests
        )
    }
}

api_client_generate_endpoint_impl!(WpApi, application_passwords);
api_client_generate_endpoint_impl!(WpApi, plugins);
api_client_generate_endpoint_impl!(WpApi, post_types);
api_client_generate_endpoint_impl!(WpApi, posts);
api_client_generate_endpoint_impl!(WpApi, site_settings);
api_client_generate_endpoint_impl!(WpApi, users);
api_client_generate_endpoint_impl!(WpApi, wp_site_health_tests);

#[macro_export]
macro_rules! api_client_generate_endpoint_impl {
    ($client_name_prefix: ident, $feature:ident) => {
        paste::paste! {
            #[uniffi::export]

            impl [<Uniffi $client_name_prefix RequestBuilder>] {
                fn $feature(&self) -> Arc<[<$feature:camel RequestBuilder>]> {
                    self.inner.$feature.clone()
                }
            }

            impl [<$client_name_prefix RequestBuilder>] {
                pub fn $feature(&self) -> &[<$feature:camel RequestBuilder>] {
                    self.$feature.as_ref()
                }
            }

            #[uniffi::export]
            impl [<Uniffi $client_name_prefix Client>] {
                fn $feature(&self) -> Arc<[<$feature:camel RequestExecutor>]> {
                    self.inner.$feature.clone()
                }
            }

            impl [<$client_name_prefix Client>] {
                pub fn $feature(&self) -> &[<$feature:camel RequestExecutor>] {
                    self.$feature.as_ref()
                }
            }
        }
    };
}

#[macro_export]
macro_rules! api_client_generate_request_builder {
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

#[macro_export]
macro_rules! api_client_generate_api_client {
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

use crate::request::{
    endpoint::{
        application_passwords_endpoint::{
            ApplicationPasswordsRequestBuilder, ApplicationPasswordsRequestExecutor,
        },
        plugins_endpoint::{PluginsRequestBuilder, PluginsRequestExecutor},
        users_endpoint::{UsersRequestBuilder, UsersRequestExecutor},
        ApiBaseUrl,
    },
    RequestExecutor,
};
use crate::{ParsedUrl, WpAuthentication};
use std::sync::Arc;

#[derive(Debug, uniffi::Object)]
pub struct WpApiRequestBuilder {
    application_passwords: Arc<ApplicationPasswordsRequestBuilder>,
    users: Arc<UsersRequestBuilder>,
    plugins: Arc<PluginsRequestBuilder>,
}

#[uniffi::export]
impl WpApiRequestBuilder {
    #[uniffi::constructor]
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
        }
    }

    pub fn application_passwords(&self) -> Arc<ApplicationPasswordsRequestBuilder> {
        self.application_passwords.clone()
    }

    pub fn users(&self) -> Arc<UsersRequestBuilder> {
        self.users.clone()
    }

    pub fn plugins(&self) -> Arc<PluginsRequestBuilder> {
        self.plugins.clone()
    }
}

#[derive(Debug, uniffi::Object)]
pub struct WpApiClient {
    application_passwords: Arc<ApplicationPasswordsRequestExecutor>,
    users: Arc<UsersRequestExecutor>,
    plugins: Arc<PluginsRequestExecutor>,
}

#[uniffi::export]
impl WpApiClient {
    #[uniffi::constructor]
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
        }
    }

    pub fn application_passwords(&self) -> Arc<ApplicationPasswordsRequestExecutor> {
        self.application_passwords.clone()
    }

    pub fn users(&self) -> Arc<UsersRequestExecutor> {
        self.users.clone()
    }

    pub fn plugins(&self) -> Arc<PluginsRequestExecutor> {
        self.plugins.clone()
    }
}

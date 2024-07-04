use crate::WpAuthentication;
use crate::{
    request::{
        endpoint::{
            application_passwords_endpoint::ApplicationPasswordsRequestExecutor,
            plugins_endpoint::PluginsRequestExecutor,
            users_endpoint::{UsersRequestBuilder, UsersRequestExecutor},
            ApiBaseUrl,
        },
        RequestExecutor,
    },
    WpApiError,
};
use std::sync::Arc;

#[derive(Debug, uniffi::Object)]
pub struct WpApiRequestBuilder {
    users: Arc<UsersRequestBuilder>,
}

#[uniffi::export]
impl WpApiRequestBuilder {
    #[uniffi::constructor]
    pub fn new(site_url: String, authentication: WpAuthentication) -> Result<Self, WpApiError> {
        let api_base_url: Arc<ApiBaseUrl> = ApiBaseUrl::try_from(site_url.as_str())
            .map_err(|err| WpApiError::SiteUrlParsingError {
                reason: err.to_string(),
            })?
            .into();

        Ok(Self {
            users: UsersRequestBuilder::new(api_base_url.clone(), authentication).into(),
        })
    }

    pub fn users(&self) -> Arc<UsersRequestBuilder> {
        self.users.clone()
    }
}

#[derive(Debug, uniffi::Object)]
pub struct WpRequestBuilder {
    application_passwords: Arc<ApplicationPasswordsRequestExecutor>,
    users: Arc<UsersRequestExecutor>,
    plugins: Arc<PluginsRequestExecutor>,
}

#[uniffi::export]
impl WpRequestBuilder {
    #[uniffi::constructor]
    pub fn new(
        site_url: String,
        authentication: WpAuthentication,
        request_executor: Arc<dyn RequestExecutor>,
    ) -> Result<Self, WpApiError> {
        let api_base_url: Arc<ApiBaseUrl> = ApiBaseUrl::try_from(site_url.as_str())
            .map_err(|err| WpApiError::SiteUrlParsingError {
                reason: err.to_string(),
            })?
            .into();

        Ok(Self {
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
        })
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

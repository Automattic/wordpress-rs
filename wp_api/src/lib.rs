#![allow(dead_code, unused_variables)]

use request::{
    endpoint::{
        application_passwords_endpoint::ApplicationPasswordsRequestExecutor,
        plugins_endpoint::PluginsRequestExecutor,
        users_endpoint::{UsersRequestBuilder, UsersRequestExecutor},
        ApiBaseUrl,
    },
    RequestExecutor,
};
use std::sync::Arc;

pub use api_error::{
    RequestExecutionError, WpApiError, WpRestError, WpRestErrorCode, WpRestErrorWrapper,
};
use plugins::*;
use users::*;

mod api_error; // re-exported relevant types
pub mod application_passwords;
pub mod login;
pub mod plugins;
pub mod request;
pub mod users;

#[cfg(test)]
mod unit_test_common;

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

#[uniffi::export]
fn wp_authentication_from_username_and_password(
    username: String,
    password: String,
) -> WpAuthentication {
    WpAuthentication::from_username_and_password(username, password)
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum WpContext {
    Edit,
    Embed,
    #[default]
    View,
}

impl WpContext {
    fn as_str(&self) -> &str {
        match self {
            Self::Edit => "edit",
            Self::Embed => "embed",
            Self::View => "view",
        }
    }
}

#[derive(Debug, Clone, uniffi::Enum)]
pub enum WpAuthentication {
    AuthorizationHeader { token: String },
    None,
}

impl WpAuthentication {
    pub fn from_username_and_password(username: String, password: String) -> Self {
        use base64::prelude::*;
        WpAuthentication::AuthorizationHeader {
            token: BASE64_STANDARD.encode(format!("{}:{}", username, password)),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum WpApiParamOrder {
    #[default]
    Asc,
    Desc,
}

impl WpApiParamOrder {
    fn as_str(&self) -> &str {
        match self {
            Self::Asc => "asc",
            Self::Desc => "desc",
        }
    }
}

trait SparseField {
    fn as_str(&self) -> &str;
}

#[macro_export]
macro_rules! generate {
    ($type_name:ident) => {
        $type_name::default()
    };
    ($type_name:ident, $(($f:ident, $v:expr)), *) => {{
        let mut obj = $type_name::default();
        $(obj.$f = $v;)*
        obj
    }};
}

uniffi::setup_scaffolding!();

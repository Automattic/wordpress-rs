#![allow(dead_code, unused_variables)]

use request::{
    endpoint::{
        application_passwords_endpoint::{
            ApplicationPasswordsRequestBuilder, ApplicationPasswordsRequestExecutor,
        },
        plugins_endpoint::{PluginsRequestBuilder, PluginsRequestExecutor},
        users_endpoint::{UsersRequestBuilder, UsersRequestExecutor},
        ApiBaseUrl,
    },
    RequestExecutor, WpNetworkResponse,
};
use std::sync::Arc;

pub use api_error::{
    RequestExecutionError, WpApiError, WpRestError, WpRestErrorCode, WpRestErrorWrapper,
};
use login::*;
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

// TODO: This is a temporary type that allows building a request type
// Although we'll have a type that does that, it's unlikely that it'll look like this.
// It still does its job for now to prove that `UsersRequestBuilder2` (temporary) type is
// properly generated and utilized in `test_manual_request_builder_immut` integration tests
#[derive(Debug, uniffi::Object)]
pub struct WpApiRequestBuilder {
    users: Arc<UsersRequestBuilder>,
}

#[uniffi::export]
impl WpApiRequestBuilder {
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
        let request_builder = Arc::new(request::RequestBuilder::new(
            request_executor,
            authentication.clone(),
        ));

        Ok(Self {
            users: UsersRequestBuilder::new(api_base_url.clone(), request_builder.clone()).into(),
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
        let request_builder = Arc::new(request::RequestBuilder::new(
            request_executor.clone(),
            authentication.clone(),
        ));

        Ok(Self {
            application_passwords: ApplicationPasswordsRequestExecutor::new(
                ApplicationPasswordsRequestBuilder::new(
                    api_base_url.clone(),
                    request_builder.clone(),
                ),
                request_executor.clone(),
            )
            .into(),
            users: UsersRequestExecutor::new(
                UsersRequestBuilder::new(api_base_url.clone(), request_builder.clone()),
                request_executor.clone(),
            )
            .into(),
            plugins: PluginsRequestExecutor::new(
                PluginsRequestBuilder::new(api_base_url.clone(), request_builder.clone()),
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

#[uniffi::export]
pub fn parse_api_details_response(response: WpNetworkResponse) -> Result<WpApiDetails, WpApiError> {
    let api_details =
        serde_json::from_slice(&response.body).map_err(|err| WpApiError::ParsingError {
            reason: err.to_string(),
            response: response.body_as_string(),
        })?;

    Ok(api_details)
}

#[uniffi::export]
pub fn get_link_header(response: &WpNetworkResponse, name: &str) -> Option<WpRestApiUrl> {
    if let Some(url) = response.get_link_header(name) {
        return Some(url.into());
    }

    None
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

#![allow(dead_code, unused_variables)]

use request::{
    endpoint::{ApiBaseUrl, ApiEndpointUrl},
    plugins_request_builder::PluginsRequestBuilder,
    users_request_builder::UsersRequestBuilder,
    NetworkRequestError, RequestExecutor, RequestMethod, WpNetworkRequest, WpNetworkResponse,
};
use serde::Serialize;
use std::{collections::HashMap, sync::Arc};

pub use api_error::{WpApiError, WpRestError, WpRestErrorCode, WpRestErrorWrapper};
use login::*;
use plugins::*;
use users::*;

mod api_error; // re-exported relevant types
pub mod login;
pub mod plugins;
pub mod request;
pub mod users;

#[cfg(test)]
mod unit_test_common;

const CONTENT_TYPE_JSON: &str = "application/json";

#[derive(Debug, uniffi::Object)]
pub struct WpRequestBuilder {
    users: Arc<UsersRequestBuilder>,
    plugins: Arc<PluginsRequestBuilder>,
}

#[uniffi::export]
impl WpRequestBuilder {
    #[uniffi::constructor]
    pub fn new(
        site_url: String,
        authentication: WpAuthentication,
        request_executor: Arc<dyn RequestExecutor>,
    ) -> Result<Self, WpApiError> {
        let api_base_url: Arc<ApiBaseUrl> = ApiBaseUrl::new(site_url.as_str())
            .map_err(|err| WpApiError::SiteUrlParsingError {
                reason: err.to_string(),
            })?
            .into();
        let request_builder = Arc::new(RequestBuilder {
            authentication: authentication.clone(),
            executor: request_executor,
        });

        Ok(Self {
            users: UsersRequestBuilder::new(api_base_url.clone(), request_builder.clone()).into(),
            plugins: PluginsRequestBuilder::new(api_base_url.clone(), request_builder.clone())
                .into(),
        })
    }

    pub fn users(&self) -> Arc<UsersRequestBuilder> {
        self.users.clone()
    }

    pub fn plugins(&self) -> Arc<PluginsRequestBuilder> {
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

#[derive(Debug)]
struct RequestBuilder {
    executor: Arc<dyn RequestExecutor>,
    authentication: WpAuthentication,
}

impl RequestBuilder {
    async fn get(&self, url: ApiEndpointUrl) -> Result<WpNetworkResponse, NetworkRequestError> {
        self.executor
            .execute(WpNetworkRequest {
                method: RequestMethod::GET,
                url: url.into(),
                header_map: self.header_map(),
                body: None,
            })
            .await
    }

    async fn post<T>(
        &self,
        url: ApiEndpointUrl,
        json_body: &T,
    ) -> Result<WpNetworkResponse, NetworkRequestError>
    where
        T: ?Sized + Serialize,
    {
        self.executor
            .execute(WpNetworkRequest {
                method: RequestMethod::POST,
                url: url.into(),
                header_map: self.header_map_for_post_request(),
                body: serde_json::to_vec(json_body).ok(),
            })
            .await
    }

    async fn delete(&self, url: ApiEndpointUrl) -> Result<WpNetworkResponse, NetworkRequestError> {
        self.executor
            .execute(WpNetworkRequest {
                method: RequestMethod::DELETE,
                url: url.into(),
                header_map: self.header_map(),
                body: None,
            })
            .await
    }

    fn header_map(&self) -> HashMap<String, String> {
        let mut header_map = HashMap::new();
        header_map.insert(
            http::header::ACCEPT.to_string(),
            CONTENT_TYPE_JSON.to_string(),
        );
        match self.authentication {
            WpAuthentication::None => None,
            WpAuthentication::AuthorizationHeader { ref token } => {
                header_map.insert("Authorization".to_string(), format!("Basic {}", token))
            }
        };
        header_map
    }

    fn header_map_for_post_request(&self) -> HashMap<String, String> {
        let mut header_map = self.header_map();
        header_map.insert(
            http::header::CONTENT_TYPE.to_string(),
            CONTENT_TYPE_JSON.to_string(),
        );
        header_map
    }
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

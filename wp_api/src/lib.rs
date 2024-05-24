#![allow(dead_code, unused_variables)]

use request::{
    endpoint::{ApiBaseUrl, ApiEndpointUrl},
    plugins_request::PluginsRequest,
    users_request::UsersRequest,
    RequestMethod, WPNetworkRequest, WPNetworkResponse,
};
use serde::Serialize;
use std::{collections::HashMap, sync::Arc};
use url::Url;

pub use api_error::{WPApiError, WPRestError, WPRestErrorCode, WPRestErrorWrapper};
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
pub struct WPApiHelper {
    authentication: WPAuthentication,
    users_request: UsersRequest,
    plugins_request: PluginsRequest,
}

#[uniffi::export]
fn wp_authentication_from_username_and_password(
    username: String,
    password: String,
) -> WPAuthentication {
    WPAuthentication::from_username_and_password(username, password)
}

#[uniffi::export]
impl WPApiHelper {
    #[uniffi::constructor]
    pub fn new(site_url: String, authentication: WPAuthentication) -> Self {
        let url = Url::parse(site_url.as_str()).unwrap();
        // TODO: Handle the url parse error
        let api_base_url = ApiBaseUrl::new(site_url.as_str()).unwrap();
        let request_builder = Arc::new(RequestBuilder {
            authentication: authentication.clone(),
        });

        Self {
            authentication: authentication.clone(),
            users_request: UsersRequest::new(api_base_url.clone(), request_builder.clone()),
            plugins_request: PluginsRequest::new(api_base_url.clone(), request_builder.clone()),
        }
    }

    // TODO: Remove this because we want to build all requests within the crate
    pub fn raw_request(&self, url: String) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::GET,
            url: ApiEndpointUrl::new(Url::parse(url.as_str()).unwrap()).into(),
            header_map: header_map(&self.authentication),
            body: None,
        }
    }

    pub fn list_users_request(
        &self,
        context: WPContext,
        params: &Option<UserListParams>, // UniFFI doesn't support Option<&T>
    ) -> WPNetworkRequest {
        self.users_request.list(context, params)
    }

    pub fn filter_list_users_request(
        &self,
        context: WPContext,
        params: &Option<UserListParams>, // UniFFI doesn't support Option<&T>
        fields: &[SparseUserField],
    ) -> WPNetworkRequest {
        self.users_request.filter_list(context, params, fields)
    }

    pub fn retrieve_user_request(&self, user_id: UserId, context: WPContext) -> WPNetworkRequest {
        self.users_request.retrieve(user_id, context)
    }

    pub fn filter_retrieve_user_request(
        &self,
        user_id: UserId,
        context: WPContext,
        fields: &[SparseUserField],
    ) -> WPNetworkRequest {
        self.users_request.filter_retrieve(user_id, context, fields)
    }

    pub fn retrieve_current_user_request(&self, context: WPContext) -> WPNetworkRequest {
        self.users_request.retrieve_me(context)
    }

    pub fn filter_retrieve_current_user_request(
        &self,
        context: WPContext,
        fields: &[SparseUserField],
    ) -> WPNetworkRequest {
        self.users_request.filter_retrieve_me(context, fields)
    }

    pub fn create_user_request(&self, params: &UserCreateParams) -> WPNetworkRequest {
        self.users_request.create(params)
    }

    pub fn update_user_request(
        &self,
        user_id: UserId,
        params: &UserUpdateParams,
    ) -> WPNetworkRequest {
        self.users_request.update(user_id, params)
    }

    pub fn update_current_user_request(&self, params: &UserUpdateParams) -> WPNetworkRequest {
        self.users_request.update_me(params)
    }

    pub fn delete_user_request(
        &self,
        user_id: UserId,
        params: &UserDeleteParams,
    ) -> WPNetworkRequest {
        self.users_request.delete(user_id, params)
    }

    pub fn delete_current_user_request(&self, params: &UserDeleteParams) -> WPNetworkRequest {
        self.users_request.delete_me(params)
    }

    pub fn list_plugins_request(
        &self,
        context: WPContext,
        params: &Option<PluginListParams>, // UniFFI doesn't support Option<&T>
    ) -> WPNetworkRequest {
        self.plugins_request.list(context, params)
    }

    pub fn filter_list_plugins_request(
        &self,
        context: WPContext,
        params: &Option<PluginListParams>, // UniFFI doesn't support Option<&T>
        fields: &[SparsePluginField],
    ) -> WPNetworkRequest {
        self.plugins_request.filter_list(context, params, fields)
    }

    pub fn create_plugin_request(&self, params: &PluginCreateParams) -> WPNetworkRequest {
        self.plugins_request.create(params)
    }

    pub fn retrieve_plugin_request(
        &self,
        context: WPContext,
        plugin: &PluginSlug,
    ) -> WPNetworkRequest {
        self.plugins_request.retrieve(context, plugin)
    }

    pub fn filter_retrieve_plugin_request(
        &self,
        context: WPContext,
        plugin: &PluginSlug,
        fields: &[SparsePluginField],
    ) -> WPNetworkRequest {
        self.plugins_request
            .filter_retrieve(context, plugin, fields)
    }

    pub fn update_plugin_request(
        &self,
        plugin: &PluginSlug,
        params: &PluginUpdateParams,
    ) -> WPNetworkRequest {
        self.plugins_request.update(plugin, params)
    }

    pub fn delete_plugin_request(&self, plugin: &PluginSlug) -> WPNetworkRequest {
        self.plugins_request.delete(plugin)
    }
}

#[derive(Debug)]
struct RequestBuilder {
    authentication: WPAuthentication,
}

impl RequestBuilder {
    fn get(&self, url: ApiEndpointUrl) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::GET,
            url: url.into(),
            header_map: header_map(&self.authentication),
            body: None,
        }
    }

    fn post<T>(&self, url: ApiEndpointUrl, json_body: &T) -> WPNetworkRequest
    where
        T: ?Sized + Serialize,
    {
        WPNetworkRequest {
            method: RequestMethod::POST,
            url: url.into(),
            header_map: header_map_for_post_request(&self.authentication),
            body: serde_json::to_vec(json_body).ok(),
        }
    }

    fn delete(&self, url: ApiEndpointUrl) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::DELETE,
            url: url.into(),
            header_map: header_map(&self.authentication),
            body: None,
        }
    }
}

fn header_map(authentication: &WPAuthentication) -> HashMap<String, String> {
    let mut header_map = HashMap::new();
    header_map.insert(
        http::header::ACCEPT.to_string(),
        CONTENT_TYPE_JSON.to_string(),
    );
    match authentication {
        WPAuthentication::None => None,
        WPAuthentication::AuthorizationHeader { ref token } => {
            header_map.insert("Authorization".to_string(), format!("Basic {}", token))
        }
    };
    header_map
}

fn header_map_for_post_request(authentication: &WPAuthentication) -> HashMap<String, String> {
    let mut header_map = header_map(authentication);
    header_map.insert(
        http::header::CONTENT_TYPE.to_string(),
        CONTENT_TYPE_JSON.to_string(),
    );
    header_map
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum WPContext {
    Edit,
    Embed,
    #[default]
    View,
}

impl WPContext {
    fn as_str(&self) -> &str {
        match self {
            Self::Edit => "edit",
            Self::Embed => "embed",
            Self::View => "view",
        }
    }
}

#[derive(Debug, Clone, uniffi::Enum)]
pub enum WPAuthentication {
    AuthorizationHeader { token: String },
    None,
}

impl WPAuthentication {
    pub fn from_username_and_password(username: String, password: String) -> Self {
        use base64::prelude::*;
        WPAuthentication::AuthorizationHeader {
            token: BASE64_STANDARD.encode(format!("{}:{}", username, password)),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum WPApiParamOrder {
    #[default]
    Asc,
    Desc,
}

impl WPApiParamOrder {
    fn as_str(&self) -> &str {
        match self {
            Self::Asc => "asc",
            Self::Desc => "desc",
        }
    }
}

#[uniffi::export]
pub fn parse_api_details_response(response: WPNetworkResponse) -> Result<WPAPIDetails, WPApiError> {
    let api_details =
        serde_json::from_slice(&response.body).map_err(|err| WPApiError::ParsingError {
            reason: err.to_string(),
            response: response.body_as_string(),
        })?;

    Ok(api_details)
}

// TODO: Figure out why we can't expose this method on `WPNetworkResponse` via UniFFI
#[uniffi::export]
pub fn get_link_header(response: &WPNetworkResponse, name: &str) -> Option<WPRestAPIURL> {
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

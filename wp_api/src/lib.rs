#![allow(dead_code, unused_variables)]

use request::{
    endpoint::{ApiEndpoint, ApiEndpointUrl},
    RequestMethod, WPNetworkRequest, WPNetworkResponse,
};
use std::collections::HashMap;
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
    api_endpoint: ApiEndpoint,
    authentication: WPAuthentication,
}

#[uniffi::export]
fn wp_authentication_from_username_and_password(
    username: String,
    password: String,
) -> WPAuthentication {
    WPAuthentication::from_username_and_password(username, password)
}

struct RequestBuilder {}

struct PluginRequestBuilder {}

impl PluginRequestBuilder {
    pub fn list(
        &self,
        context: WPContext,
        params: &Option<PluginListParams>, // UniFFI doesn't support Option<&T>
    ) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::GET,
            url: self
                .api_endpoint
                .plugins
                .list(context, params.as_ref())
                .into(),
            header_map: self.header_map(),
            body: None,
        }
    }

    pub fn filter_list(
        &self,
        context: WPContext,
        params: &Option<PluginListParams>, // UniFFI doesn't support Option<&T>
        fields: &[SparsePluginField],
    ) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::GET,
            url: self
                .api_endpoint
                .plugins
                .filter_list(context, params.as_ref(), fields)
                .into(),
            header_map: self.header_map(),
            body: None,
        }
    }

    pub fn create(&self, params: &PluginCreateParams) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::POST,
            url: self.api_endpoint.plugins.create().into(),
            header_map: self.header_map_for_post_request(),
            body: serde_json::to_vec(&params).ok(),
        }
    }

    pub fn retrieve(&self, context: WPContext, plugin: &PluginSlug) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::GET,
            url: self.api_endpoint.plugins.retrieve(context, plugin).into(),
            header_map: self.header_map(),
            body: None,
        }
    }

    pub fn filter_retrieve(
        &self,
        context: WPContext,
        plugin: &PluginSlug,
        fields: &[SparsePluginField],
    ) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::GET,
            url: self
                .api_endpoint
                .plugins
                .filter_retrieve(context, plugin, fields)
                .into(),
            header_map: self.header_map(),
            body: None,
        }
    }

    pub fn update(&self, plugin: &PluginSlug, params: &PluginUpdateParams) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::POST,
            url: self.api_endpoint.plugins.update(plugin).into(),
            header_map: self.header_map_for_post_request(),
            body: serde_json::to_vec(&params).ok(),
        }
    }

    pub fn delete(&self, plugin: &PluginSlug) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::DELETE,
            url: self.api_endpoint.plugins.delete(plugin).into(),
            header_map: self.header_map(),
            body: None,
        }
    }
}

struct UserRequestBuilder {}

impl UserRequestBuilder {
    pub fn list(
        &self,
        context: WPContext,
        params: &Option<UserListParams>, // UniFFI doesn't support Option<&T>
    ) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::GET,
            url: self
                .api_endpoint
                .users
                .list(context, params.as_ref())
                .into(),
            header_map: self.header_map(),
            body: None,
        }
    }

    pub fn filter_list(
        &self,
        context: WPContext,
        params: &Option<UserListParams>, // UniFFI doesn't support Option<&T>
        fields: &[SparseUserField],
    ) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::GET,
            url: self
                .api_endpoint
                .users
                .filter_list(context, params.as_ref(), fields)
                .into(),
            header_map: self.header_map(),
            body: None,
        }
    }

    pub fn retrieve(&self, user_id: UserId, context: WPContext) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::GET,
            url: self.api_endpoint.users.retrieve(user_id, context).into(),
            header_map: self.header_map(),
            body: None,
        }
    }

    pub fn filter_retrieve(
        &self,
        user_id: UserId,
        context: WPContext,
        fields: &[SparseUserField],
    ) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::GET,
            url: self
                .api_endpoint
                .users
                .filter_retrieve(user_id, context, fields)
                .into(),
            header_map: self.header_map(),
            body: None,
        }
    }

    pub fn retrieve_me(&self, context: WPContext) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::GET,
            url: self.api_endpoint.users.retrieve_me(context).into(),
            header_map: self.header_map(),
            body: None,
        }
    }

    pub fn filter_retrieve_me(
        &self,
        context: WPContext,
        fields: &[SparseUserField],
    ) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::GET,
            url: self
                .api_endpoint
                .users
                .filter_retrieve_me(context, fields)
                .into(),
            header_map: self.header_map(),
            body: None,
        }
    }

    pub fn create(&self, params: &UserCreateParams) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::POST,
            url: self.api_endpoint.users.create().into(),
            header_map: self.header_map_for_post_request(),
            body: serde_json::to_vec(&params).ok(),
        }
    }

    pub fn update(&self, user_id: UserId, params: &UserUpdateParams) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::POST,
            url: self.api_endpoint.users.update(user_id).into(),
            header_map: self.header_map_for_post_request(),
            body: serde_json::to_vec(&params).ok(),
        }
    }

    pub fn update_me(&self, params: &UserUpdateParams) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::POST,
            url: self.api_endpoint.users.update_me().into(),
            header_map: self.header_map_for_post_request(),
            body: serde_json::to_vec(&params).ok(),
        }
    }

    pub fn delete(&self, user_id: UserId, params: &UserDeleteParams) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::DELETE,
            url: self.api_endpoint.users.delete(user_id, params).into(),
            header_map: self.header_map(),
            body: None,
        }
    }

    pub fn delete_me(&self, params: &UserDeleteParams) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::DELETE,
            url: self.api_endpoint.users.delete_me(params).into(),
            header_map: self.header_map(),
            body: None,
        }
    }
}

#[uniffi::export]
impl WPApiHelper {
    #[uniffi::constructor]
    pub fn new(site_url: String, authentication: WPAuthentication) -> Self {
        let url = Url::parse(site_url.as_str()).unwrap();
        // TODO: Handle the url parse error
        let api_endpoint = ApiEndpoint::new_from_str(site_url.as_str()).unwrap();

        Self {
            api_endpoint,
            authentication,
        }
    }

    // TODO: Remove this because we want to build all requests within the crate
    pub fn raw_request(&self, url: String) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::GET,
            url: ApiEndpointUrl::new(Url::parse(url.as_str()).unwrap()).into(),
            header_map: self.header_map(),
            body: None,
        }
    }

    fn header_map(&self) -> HashMap<String, String> {
        let mut header_map = HashMap::new();
        header_map.insert(
            http::header::ACCEPT.to_string(),
            CONTENT_TYPE_JSON.to_string(),
        );
        match &self.authentication {
            WPAuthentication::None => None,
            WPAuthentication::AuthorizationHeader { token } => {
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

#![allow(dead_code, unused_variables)]

use serde::Deserialize;
use std::collections::HashMap;

pub use api_error::*;
pub use endpoint::*;
pub use login::*;
pub use pages::*;
pub use plugins::*;
pub use posts::*;
pub use url::*;
pub use users::*;

pub mod api_error;
pub mod endpoint;
pub mod login;
pub mod pages;
pub mod plugins;
pub mod posts;
pub mod url;
pub mod users;

#[cfg(test)]
mod test_helpers;

const CONTENT_TYPE_JSON: &str = "application/json";

#[derive(uniffi::Object)]
pub struct WPApiHelper {
    api_endpoint: ApiEndpoint,
    site_url: Url,
    authentication: WPAuthentication,
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
        let api_endpoint = ApiEndpoint::new_from_str(site_url.as_str()).unwrap();

        Self {
            api_endpoint,
            site_url: url,
            authentication,
        }
    }

    pub fn raw_request(&self, url: String) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::GET,
            url: Url::parse(url.as_str()).unwrap().into(),
            header_map: self.header_map(),
            body: None,
        }
    }

    pub fn post_list_request(&self, params: PostListParams) -> WPNetworkRequest {
        let mut url = self
            .site_url
            .join("/wp-json/wp/v2/posts?context=edit")
            .unwrap();

        url.query_pairs_mut()
            .append_pair("page", params.page.to_string().as_str());
        url.query_pairs_mut()
            .append_pair("per_page", params.per_page.to_string().as_str());

        WPNetworkRequest {
            method: RequestMethod::GET,
            url: url.into(),
            header_map: self.header_map(),
            body: None,
        }
    }

    pub fn list_users_request(
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

    pub fn filter_list_users_request(
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

    pub fn retrieve_user_request(&self, user_id: UserId, context: WPContext) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::GET,
            url: self.api_endpoint.users.retrieve(user_id, context).into(),
            header_map: self.header_map(),
            body: None,
        }
    }

    pub fn filter_retrieve_user_request(
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

    pub fn retrieve_current_user_request(&self, context: WPContext) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::GET,
            url: self.api_endpoint.users.retrieve_me(context).into(),
            header_map: self.header_map(),
            body: None,
        }
    }

    pub fn filter_retrieve_current_user_request(
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

    pub fn create_user_request(&self, params: &UserCreateParams) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::POST,
            url: self.api_endpoint.users.create().into(),
            header_map: self.header_map_for_post_request(),
            body: serde_json::to_vec(&params).ok(),
        }
    }

    pub fn update_user_request(
        &self,
        user_id: UserId,
        params: &UserUpdateParams,
    ) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::POST,
            url: self.api_endpoint.users.update(user_id).into(),
            header_map: self.header_map_for_post_request(),
            body: serde_json::to_vec(&params).ok(),
        }
    }

    pub fn update_current_user_request(&self, params: &UserUpdateParams) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::POST,
            url: self.api_endpoint.users.update_me().into(),
            header_map: self.header_map_for_post_request(),
            body: serde_json::to_vec(&params).ok(),
        }
    }

    pub fn delete_user_request(
        &self,
        user_id: UserId,
        params: &UserDeleteParams,
    ) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::DELETE,
            url: self.api_endpoint.users.delete(user_id, params).into(),
            header_map: self.header_map(),
            body: None,
        }
    }

    pub fn delete_current_user_request(&self, params: &UserDeleteParams) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::DELETE,
            url: self.api_endpoint.users.delete_me(params).into(),
            header_map: self.header_map(),
            body: None,
        }
    }

    pub fn list_plugins_request(
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum WPContext {
    Edit,
    Embed,
    View,
}

impl Default for WPContext {
    fn default() -> Self {
        Self::View
    }
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

#[derive(uniffi::Enum)]
pub enum RequestMethod {
    GET,
    POST,
    PUT,
    DELETE,
    HEAD,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum WPApiParamOrder {
    Asc,
    Desc,
}

impl Default for WPApiParamOrder {
    fn default() -> Self {
        Self::Asc
    }
}

impl WPApiParamOrder {
    fn as_str(&self) -> &str {
        match self {
            Self::Asc => "asc",
            Self::Desc => "desc",
        }
    }
}

#[derive(uniffi::Record)]
pub struct WPNetworkRequest {
    pub method: RequestMethod,
    pub url: String,
    // TODO: We probably want to implement a specific type for these headers instead of using a
    // regular HashMap.
    //
    // It could be something similar to `reqwest`'s [`header`](https://docs.rs/reqwest/latest/reqwest/header/index.html)
    // module.
    pub header_map: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
}

#[derive(uniffi::Record)]
pub struct WPNetworkResponse {
    pub body: Vec<u8>,
    pub status_code: u16,
    // TODO: We probably want to implement a specific type for these headers instead of using a
    // regular HashMap.
    //
    // It could be something similar to `reqwest`'s [`header`](https://docs.rs/reqwest/latest/reqwest/header/index.html)
    // module.
    pub header_map: Option<HashMap<String, String>>,
}

impl WPNetworkResponse {
    pub fn get_link_header(&self, name: &str) -> Option<Url> {
        if let Some(headers) = self.header_map.clone() {
            // TODO: This is inefficient
            if headers.contains_key("Link") {
                if let Ok(res) = parse_link_header::parse_with_rel(&headers["Link"]) {
                    if let Some(next) = res.get(name) {
                        if let Ok(url) = Url::parse(next.raw_uri.as_str()) {
                            return Some(url);
                        }
                    }
                }
            }
        }

        None
    }
}

#[uniffi::export]
pub fn parse_post_list_response(
    response: WPNetworkResponse,
) -> Result<PostListResponse, WPApiError> {
    parse_response_for_generic_errors(&response)?;
    let post_list: Vec<PostObject> =
        serde_json::from_slice(&response.body).map_err(|err| WPApiError::ParsingError {
            reason: err.to_string(),
            response: String::from_utf8_lossy(&response.body).to_string(),
        })?;

    let mut next_page: Option<String> = None;

    if let Some(link_header) = response.get_link_header("next") {
        next_page = Some(link_header.to_string())
    }

    Ok(PostListResponse {
        post_list: Some(post_list),
        next_page,
    })
}

#[uniffi::export]
pub fn parse_api_details_response(response: WPNetworkResponse) -> Result<WPAPIDetails, WPApiError> {
    let api_details =
        serde_json::from_slice(&response.body).map_err(|err| WPApiError::ParsingError {
            reason: err.to_string(),
            response: String::from_utf8_lossy(&response.body).to_string(),
        })?;

    Ok(api_details)
}

pub fn parse_wp_response<'de, T: Deserialize<'de>>(
    response: &'de WPNetworkResponse,
) -> Result<T, WPApiError> {
    parse_response_for_generic_errors(response)?;
    serde_json::from_slice(&response.body).map_err(|err| WPApiError::ParsingError {
        reason: err.to_string(),
        response: String::from_utf8_lossy(&response.body).to_string(),
    })
}

pub fn parse_response_for_generic_errors(response: &WPNetworkResponse) -> Result<(), WPApiError> {
    let response_str = String::from_utf8_lossy(&response.body).to_string();
    // TODO: Further parse the response body to include error message
    // TODO: Lots of unwraps to get a basic setup working
    let status = http::StatusCode::from_u16(response.status_code).unwrap();
    if let Ok(rest_error) = serde_json::from_slice(&response.body) {
        Err(WPApiError::RestError {
            rest_error,
            status_code: response.status_code,
            response: response_str,
        })
    } else if status.is_client_error() || status.is_server_error() {
        Err(WPApiError::UnknownError {
            status_code: response.status_code,
            response: response_str,
        })
    } else {
        Ok(())
    }
}

// TODO: Figure out why we can't expose this method on `WPNetworkResponse` via UniFFI
#[uniffi::export]
pub fn get_link_header(response: &WPNetworkResponse, name: &str) -> Option<WPRestAPIURL> {
    if let Some(url) = response.get_link_header(name) {
        return Some(url.into());
    }

    None
}

#[macro_export]
macro_rules! add_uniffi_exported_parser {
    ($fn_name:ident, $return_type: ty) => {
        #[uniffi::export]
        pub fn $fn_name(response: &WPNetworkResponse) -> Result<$return_type, WPApiError> {
            parse_wp_response(response)
        }
    };
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

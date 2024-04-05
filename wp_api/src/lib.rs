#![allow(dead_code, unused_variables)]

use std::{collections::HashMap, fmt::Display};

pub use api_error::*;
pub use login::*;
pub use pages::*;
pub use posts::*;
use serde::Deserialize;
pub use url::*;
pub use users::*;

pub mod api_error;
pub mod login;
pub mod pages;
pub mod posts;
pub mod url;
pub mod users;

#[derive(uniffi::Object)]
pub struct WPApiHelper {
    site_url: Url,
    authentication: WPAuthentication,
}

#[uniffi::export]
impl WPApiHelper {
    #[uniffi::constructor]
    pub fn new(site_url: String, authentication: WPAuthentication) -> Self {
        let url = Url::parse(site_url.as_str()).unwrap();

        Self {
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
        params: Option<UserListParams>,
    ) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::GET,
            url: UsersEndpoint::list_users(&self.site_url, context, params.as_ref()).into(),
            header_map: self.header_map(),
            body: None,
        }
    }

    pub fn retrieve_user_request(&self, user_id: UserId, context: WPContext) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::GET,
            url: UsersEndpoint::retrieve_user(&self.site_url, user_id, context).into(),
            header_map: self.header_map(),
            body: None,
        }
    }

    pub fn retrieve_current_user(&self, context: WPContext) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::GET,
            url: UsersEndpoint::retrieve_current_user(&self.site_url, context).into(),
            header_map: self.header_map(),
            body: None,
        }
    }

    pub fn create_user_request(&self, params: UserCreateParams) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::POST,
            url: UsersEndpoint::create_user(&self.site_url).into(),
            header_map: self.header_map(),
            body: serde_json::to_vec(&params).ok(),
        }
    }

    pub fn update_user_request(
        &self,
        user_id: UserId,
        params: UserUpdateParams,
    ) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::POST,
            url: UsersEndpoint::update_user(&self.site_url, user_id, &params).into(),
            header_map: self.header_map(),
            body: serde_json::to_vec(&params).ok(),
        }
    }

    pub fn delete_user_request(
        &self,
        user_id: UserId,
        params: UserDeleteParams,
    ) -> WPNetworkRequest {
        WPNetworkRequest {
            method: RequestMethod::DELETE,
            url: UsersEndpoint::delete_user(&self.site_url, user_id, &params).into(),
            header_map: self.header_map(),
            body: None,
        }
    }

    fn header_map(&self) -> Option<HashMap<String, String>> {
        match &self.authentication {
            WPAuthentication::None => None,
            WPAuthentication::AuthorizationHeader { token } => Some(HashMap::from([(
                "Authorization".into(),
                format!("Basic {}", token),
            )])),
        }
    }
}

#[derive(Debug, Clone, Copy, uniffi::Enum)]
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

impl Display for WPContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Edit => "edit",
                Self::Embed => "embed",
                Self::View => "view",
            }
        )
    }
}

#[derive(Debug, Clone, uniffi::Enum)]
pub enum WPAuthentication {
    AuthorizationHeader { token: String },
    None,
}

#[derive(uniffi::Enum)]
pub enum RequestMethod {
    GET,
    POST,
    PUT,
    DELETE,
    HEAD,
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
    pub header_map: Option<HashMap<String, String>>,
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
    // TODO: Further parse the response body to include error message
    // TODO: Lots of unwraps to get a basic setup working
    if let Some(client_error_type) = ClientErrorType::from_status_code(response.status_code) {
        return Err(WPApiError::ClientError {
            error_type: client_error_type,
            status_code: response.status_code,
        });
    }
    let status = http::StatusCode::from_u16(response.status_code).unwrap();
    if status.is_server_error() {
        return Err(WPApiError::ServerError {
            status_code: response.status_code,
        });
    }
    let post_list: Vec<PostObject> =
        serde_json::from_slice(&response.body).map_err(|err| WPApiError::ParsingError {
            reason: err.to_string(),
            response: std::str::from_utf8(&response.body).unwrap().to_string(),
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

pub fn parse_user_list_response<'de, T: Deserialize<'de>>(
    response: &'de WPNetworkResponse,
) -> Result<Vec<T>, WPApiError> {
    if let Some(client_error_type) = ClientErrorType::from_status_code(response.status_code) {
        return Err(WPApiError::ClientError {
            error_type: client_error_type,
            status_code: response.status_code,
        });
    }
    let status = http::StatusCode::from_u16(response.status_code).unwrap();
    if status.is_server_error() {
        return Err(WPApiError::ServerError {
            status_code: response.status_code,
        });
    }

    let user_list: Vec<T> =
        serde_json::from_slice(&response.body).map_err(|err| WPApiError::ParsingError {
            reason: err.to_string(),
            response: std::str::from_utf8(&response.body).unwrap().to_string(),
        })?;

    Ok(user_list)
}

#[uniffi::export]
pub fn parse_user_list_response_with_edit_context(
    response: &WPNetworkResponse,
) -> Result<Vec<UserWithEditContext>, WPApiError> {
    parse_user_list_response(response)
}

#[uniffi::export]
pub fn parse_user_list_response_with_embed_context(
    response: &WPNetworkResponse,
) -> Result<Vec<UserWithEmbedContext>, WPApiError> {
    parse_user_list_response(response)
}

#[uniffi::export]
pub fn parse_user_list_response_with_view_context(
    response: &WPNetworkResponse,
) -> Result<Vec<UserWithViewContext>, WPApiError> {
    parse_user_list_response(response)
}

pub fn parse_user_retrieve_response<'de, T: Deserialize<'de> + std::fmt::Debug>(
    response: &'de WPNetworkResponse,
) -> Result<T, WPApiError> {
    if let Some(client_error_type) = ClientErrorType::from_status_code(response.status_code) {
        return Err(WPApiError::ClientError {
            error_type: client_error_type,
            status_code: response.status_code,
        });
    }
    let status = http::StatusCode::from_u16(response.status_code).unwrap();
    if status.is_server_error() {
        return Err(WPApiError::ServerError {
            status_code: response.status_code,
        });
    }

    let user: T =
        serde_json::from_slice(&response.body).map_err(|err| WPApiError::ParsingError {
            reason: err.to_string(),
            response: std::str::from_utf8(&response.body).unwrap().to_string(),
        })?;
    Ok(user)
}

#[uniffi::export]
pub fn parse_user_retrieve_response_with_edit_context(
    response: &WPNetworkResponse,
) -> Result<Option<UserWithEditContext>, WPApiError> {
    parse_user_retrieve_response(response)
}

#[uniffi::export]
pub fn parse_user_retrieve_response_with_embed_context(
    response: &WPNetworkResponse,
) -> Result<Option<UserWithEmbedContext>, WPApiError> {
    parse_user_retrieve_response(response)
}

#[uniffi::export]
pub fn parse_user_retrieve_response_with_view_context(
    response: &WPNetworkResponse,
) -> Result<Option<UserWithViewContext>, WPApiError> {
    parse_user_retrieve_response(response)
}

#[uniffi::export]
pub fn parse_api_details_response(response: WPNetworkResponse) -> Result<WPAPIDetails, WPApiError> {
    let api_details =
        serde_json::from_slice(&response.body).map_err(|err| WPApiError::ParsingError {
            reason: err.to_string(),
            response: std::str::from_utf8(&response.body).unwrap().to_string(),
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

uniffi::setup_scaffolding!();

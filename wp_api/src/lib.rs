#![allow(dead_code, unused_variables)]

use std::collections::HashMap;

pub use api_error::*;
pub use pages::*;
pub use posts::*;

pub mod api_error;
pub mod pages;
pub mod posts;

pub struct WPApiHelper {
    site_url: String,
    authentication: WPAuthentication,
}

impl WPApiHelper {
    pub fn new(site_url: String, authentication: WPAuthentication) -> Self {
        Self {
            site_url,
            authentication,
        }
    }

    pub fn post_list_request(&self) -> WPNetworkRequest {
        let url = format!("{}/wp-json/wp/v2/posts?context=edit", self.site_url);

        let mut header_map = HashMap::new();
        header_map.insert(
            "Authorization".into(),
            format!("Basic {}", self.authentication.auth_token).into(),
        );
        WPNetworkRequest {
            method: RequestMethod::GET,
            url,
            header_map: Some(header_map),
        }
    }
}

#[derive(Debug, Clone)]
// TODO: This will probably become an `enum` where we support multiple authentication types.
pub struct WPAuthentication {
    pub auth_token: String,
}

pub enum RequestMethod {
    GET,
    POST,
    PUT,
    DELETE,
}

pub struct WPNetworkRequest {
    pub method: RequestMethod,
    pub url: String,
    // TODO: We probably want to implement a specific type for these headers instead of using a
    // regular HashMap.
    //
    // It could be something similar to `reqwest`'s [`header`](https://docs.rs/reqwest/latest/reqwest/header/index.html)
    // module.
    pub header_map: Option<HashMap<String, String>>,
}

pub struct WPNetworkResponse {
    pub status_code: u16,
    pub body: Vec<u8>,
}

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
    let post_list: Vec<PostObject> = serde_json::from_slice(&response.body).or_else(|err| {
        Err(WPApiError::ParsingError {
            reason: err.to_string(),
            response: std::str::from_utf8(&response.body).unwrap().to_string(),
        })
    })?;
    Ok(PostListResponse {
        post_list: Some(post_list),
    })
}

uniffi::include_scaffolding!("wp_api");

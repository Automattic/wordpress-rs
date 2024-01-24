#![allow(dead_code, unused_variables)]

use http::HeaderMap;
use reqwest::blocking::Client;
use wp_api::{
    ClientErrorType, PostListParams, PostListResponse, PostObject, WPApiError, WPApiHelper,
    WPAuthentication,
};

pub struct WPNetworking {
    client: Client,
    helper: WPApiHelper,
}

impl WPNetworking {
    pub fn new(site_url: String, authentication: WPAuthentication) -> Self {
        Self {
            client: reqwest::blocking::Client::new(),
            helper: WPApiHelper::new(site_url, authentication),
        }
    }

    pub fn list_posts(
        &self,
        params: Option<PostListParams>,
    ) -> Result<PostListResponse, WPApiError> {
        let wp_request = self.helper.post_list_request();
        let request_headers: HeaderMap = (&wp_request.header_map.unwrap()).try_into().unwrap();
        let response = self
            .client
            .request(request_method(wp_request.method), wp_request.url)
            .headers(request_headers)
            .send()
            .unwrap();
        parse_list_posts_response(response)
    }
}

fn request_method(method: wp_api::RequestMethod) -> http::Method {
    match method {
        wp_api::RequestMethod::GET => reqwest::Method::GET,
        wp_api::RequestMethod::POST => reqwest::Method::POST,
        wp_api::RequestMethod::PUT => reqwest::Method::PUT,
        wp_api::RequestMethod::DELETE => reqwest::Method::DELETE,
    }
}

fn parse_list_posts_response(
    response: reqwest::blocking::Response,
) -> Result<PostListResponse, WPApiError> {
    let status_code = response.status().as_u16();
    // TODO: Further parse the response body to include error message
    if let Some(client_error_type) = ClientErrorType::from_status_code(status_code) {
        return Err(WPApiError::ClientError {
            error_type: client_error_type,
            status_code,
        });
    }
    if response.status().is_server_error() {
        return Err(WPApiError::ServerError { status_code });
    }
    let body = response.text().unwrap();
    let post_list: Vec<PostObject> = serde_json::from_str(&body).or_else(|err| {
        Err(WPApiError::ParsingError {
            reason: err.to_string(),
            response: body,
        })
    })?;
    Ok(PostListResponse {
        post_list: Some(post_list),
    })
}

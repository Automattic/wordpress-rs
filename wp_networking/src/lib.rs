#![allow(dead_code, unused_variables)]

use http::HeaderMap;
use reqwest::blocking::Client;
use wp_api::{
    PostListParams, PostListResponse, WPApiError, WPApiHelper, WPAuthentication, WPNetworkResponse,
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
        let wp_request = self.helper.post_list_request(params.unwrap_or_default());
        let request_headers: HeaderMap = (&wp_request.header_map.unwrap()).try_into().unwrap();
        let response = self
            .client
            .request(request_method(wp_request.method), wp_request.url)
            .headers(request_headers)
            .send()
            .unwrap();
        wp_api::parse_post_list_response(wp_network_response(response))
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

fn wp_network_response(response: reqwest::blocking::Response) -> WPNetworkResponse {
    WPNetworkResponse {
        status_code: response.status().as_u16(),
        body: response.bytes().unwrap().to_vec(),
        header_map: None, // TODO: Properly read the headers
    }
}

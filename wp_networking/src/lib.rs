#![allow(dead_code, unused_variables)]

use http::HeaderMap;
use wp_api::{WPNetworkRequest, WPNetworkResponse};

pub struct AsyncWPNetworking {
    client: reqwest::Client,
}

impl AsyncWPNetworking {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    pub async fn async_request(
        &self,
        wp_request: WPNetworkRequest,
    ) -> Result<WPNetworkResponse, reqwest::Error> {
        let request_headers: HeaderMap = (&wp_request.header_map).try_into().unwrap();

        let mut request = self
            .client
            .request(request_method(wp_request.method), wp_request.url)
            .headers(request_headers);
        if let Some(body) = wp_request.body {
            request = request.body(body);
        }
        let response = request.send().await?;

        Ok(WPNetworkResponse {
            status_code: response.status().as_u16(),
            body: response.bytes().await.unwrap().to_vec(),
            header_map: None, // TODO: Properly read the headers
        })
    }
}

fn request_method(method: wp_api::RequestMethod) -> http::Method {
    match method {
        wp_api::RequestMethod::GET => reqwest::Method::GET,
        wp_api::RequestMethod::POST => reqwest::Method::POST,
        wp_api::RequestMethod::PUT => reqwest::Method::PUT,
        wp_api::RequestMethod::DELETE => reqwest::Method::DELETE,
        wp_api::RequestMethod::HEAD => reqwest::Method::HEAD,
    }
}

fn wp_network_response(response: reqwest::blocking::Response) -> WPNetworkResponse {
    WPNetworkResponse {
        status_code: response.status().as_u16(),
        body: response.bytes().unwrap().to_vec(),
        header_map: None, // TODO: Properly read the headers
    }
}

#![allow(dead_code, unused_variables)]

use std::{collections::HashMap, sync::Arc};

use http::HeaderMap;
use reqwest::blocking::Client;
use wp_api::{
    ClientErrorType, PageListParams, PageListResponse, PostCreateParams, PostCreateResponse,
    PostDeleteParams, PostDeleteResponse, PostListParams, PostListResponse, PostObject,
    PostRetrieveParams, PostRetrieveResponse, PostUpdateParams, PostUpdateResponse, WPApiError,
    WPAuthentication,
};

pub struct WPNetworking {
    client: Client,
}

impl Default for WPNetworking {
    fn default() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

impl WPNetworkingInterface for WPNetworking {
    fn request(&self, request: WPNetworkRequest) -> WPNetworkResponse {
        let method = match request.method {
            RequestMethod::GET => reqwest::Method::GET,
            RequestMethod::POST => reqwest::Method::POST,
            RequestMethod::PUT => reqwest::Method::PUT,
            RequestMethod::DELETE => reqwest::Method::DELETE,
        };

        let request_headers: HeaderMap = (&request.header_map.unwrap()).try_into().unwrap();

        // TODO: Error handling
        let response = self
            .client
            .request(method, request.url)
            .headers(request_headers)
            .send()
            .unwrap();
        WPNetworkResponse {
            status: Arc::new(response.status()),
            body: response.text().unwrap().as_bytes().to_vec(),
        }
    }
}

pub trait WPNetworkingInterface: Send + Sync {
    fn request(&self, request: WPNetworkRequest) -> WPNetworkResponse;
}

pub trait NetworkResponseStatus: Send + Sync {
    fn as_u16(&self) -> u16;
    fn is_informational(&self) -> bool;
    fn is_success(&self) -> bool;
    fn is_redirection(&self) -> bool;
    fn is_client_error(&self) -> bool;
    fn is_server_error(&self) -> bool;
}

impl NetworkResponseStatus for http::StatusCode {
    fn as_u16(&self) -> u16 {
        self.as_u16()
    }

    fn is_informational(&self) -> bool {
        self.is_informational()
    }

    fn is_success(&self) -> bool {
        self.is_success()
    }

    fn is_redirection(&self) -> bool {
        self.is_redirection()
    }

    fn is_client_error(&self) -> bool {
        self.is_client_error()
    }

    fn is_server_error(&self) -> bool {
        self.is_informational()
    }
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
    pub status: Arc<dyn NetworkResponseStatus>,
    pub body: Vec<u8>,
}

pub trait WPApiInterface: Send + Sync {
    fn list_posts(&self, params: Option<PostListParams>) -> Result<PostListResponse, WPApiError>;
    fn create_post(&self, params: Option<PostCreateParams>) -> PostCreateResponse;
    fn retrieve_post(
        &self,
        post_id: u32,
        params: Option<PostRetrieveParams>,
    ) -> PostRetrieveResponse;

    fn update_post(&self, post_id: u32, params: Option<PostUpdateParams>) -> PostUpdateResponse;

    fn delete_post(&self, post_id: u32, params: Option<PostDeleteParams>) -> PostDeleteResponse;

    fn list_pages(&self, params: Option<PageListParams>) -> PageListResponse;
}

pub fn wp_api_with_custom_networking(
    site_url: String,
    authentication: WPAuthentication,
    networking_interface: Arc<dyn WPNetworkingInterface>,
) -> Arc<dyn WPApiInterface> {
    Arc::new(WPApi {
        site_url,
        authentication,
        networking_interface: networking_interface.clone(),
    })
}

struct WPApi {
    site_url: String,
    authentication: WPAuthentication,
    networking_interface: Arc<dyn WPNetworkingInterface>,
}

impl WPApiInterface for WPApi {
    fn list_posts(&self, params: Option<PostListParams>) -> Result<PostListResponse, WPApiError> {
        let mut header_map = HashMap::new();
        // // TODO: Authorization headers should be generated through its type not like a cave man
        header_map.insert(
            "Authorization".into(),
            format!("Basic {}", self.authentication.auth_token).into(),
        );

        let response = self.networking_interface.request(WPNetworkRequest {
            method: RequestMethod::GET,
            // TODO: Centralize the endpoints
            url: format!("{}/wp-json/wp/v2/posts?context=edit", self.site_url).into(),
            header_map: Some(header_map),
        });
        parse_list_posts_response(&response)
    }

    fn create_post(&self, params: Option<PostCreateParams>) -> PostCreateResponse {
        todo!()
    }

    fn retrieve_post(
        &self,
        post_id: u32,
        params: Option<PostRetrieveParams>,
    ) -> PostRetrieveResponse {
        todo!()
    }

    fn update_post(&self, post_id: u32, params: Option<PostUpdateParams>) -> PostUpdateResponse {
        todo!()
    }

    fn delete_post(&self, post_id: u32, params: Option<PostDeleteParams>) -> PostDeleteResponse {
        todo!()
    }

    fn list_pages(&self, params: Option<PageListParams>) -> PageListResponse {
        todo!()
    }
}

fn parse_list_posts_response(response: &WPNetworkResponse) -> Result<PostListResponse, WPApiError> {
    let status_code = response.status.as_u16();
    // TODO: Further parse the response body to include error message
    if let Some(client_error_type) = ClientErrorType::from_status_code(status_code) {
        return Err(WPApiError::ClientError {
            error_type: client_error_type,
            status_code,
        });
    }
    if response.status.is_server_error() {
        return Err(WPApiError::ServerError { status_code });
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

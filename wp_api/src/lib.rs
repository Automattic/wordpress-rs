#![allow(dead_code, unused_variables)]

use std::{collections::HashMap, sync::Arc};

pub use api_error::*;
pub use pages::*;
pub use posts::*;

pub mod api_error;
pub mod pages;
pub mod posts;

pub trait WPNetworkingInterface: Send + Sync {
    fn request(&self, request: WPNetworkRequest) -> WPNetworkResponse;
}

pub enum RequestMethod {
    GET,
    POST,
    PUT,
    DELETE,
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

#[derive(Debug, Clone)]
// TODO: This will probably become an `enum` where we support multiple authentication types.
pub struct WPAuthentication {
    pub auth_token: String,
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

uniffi::include_scaffolding!("wp_api");

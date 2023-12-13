#![allow(dead_code, unused_variables)]

use std::collections::HashMap;

pub use pages::*;
pub use posts::*;

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
    // TODO: This is a placeholder for now to get a basic setup working
    pub json: String,
}

#[derive(Debug, Clone)]
// TODO: This will probably become an `enum` where we support multiple authentication types.
pub struct WPAuthentication {
    pub auth_token: String,
}

pub trait WPApiInterface: Send + Sync {
    fn list_posts(&self, params: Option<PostListParams>) -> PostListResponse;
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

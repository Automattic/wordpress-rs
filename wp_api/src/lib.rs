#![allow(dead_code, unused_variables)]

pub use pages::*;
pub use posts::*;

pub mod pages;
pub mod posts;

pub trait WPNetworkingInterface: Send + Sync {
    fn request(&self, request: WPNetworkRequest) -> WPNetworkResponse;
}

pub struct WPNetworkRequest {}
pub struct WPNetworkResponse {}

#[derive(Debug, Clone)]
pub struct WPAuthentication {
    pub auth_token: String,
}

pub trait WPApiInterface: Send + Sync {
    fn list_posts(&self, params: Option<PostListParams>) -> ParsedPostListResponse;
    fn create_post(&self, params: Option<PostCreateParams>) -> ParsedPostCreateResponse;
    fn retrieve_post(
        &self,
        post_id: u32,
        params: Option<PostRetrieveParams>,
    ) -> ParsedPostRetrieveResponse;

    fn update_post(
        &self,
        post_id: u32,
        params: Option<PostUpdateParams>,
    ) -> ParsedPostUpdateResponse;

    fn delete_post(
        &self,
        post_id: u32,
        params: Option<PostDeleteParams>,
    ) -> ParsedPostDeleteResponse;

    fn list_pages(&self, params: Option<PageListParams>) -> ParsedPageListResponse;
}

uniffi::include_scaffolding!("wp_api");

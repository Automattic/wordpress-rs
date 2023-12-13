use serde::{Deserialize, Serialize};

pub trait PostNetworkingInterface: Send + Sync {}

pub struct PostListParams {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

pub struct PostCreateParams {
    pub title: Option<String>,
    pub content: Option<String>,
}

pub struct PostRetrieveParams {
    pub password: Option<String>,
}

pub struct PostUpdateParams {
    pub title: Option<String>,
    pub content: Option<String>,
}

pub struct PostDeleteParams {
    pub force: Option<bool>,
}

pub struct PostListRequest {
    pub params: Option<String>,
}
pub struct PostCreateRequest {
    pub params: Option<String>,
}
pub struct PostRetrieveRequest {
    pub params: Option<String>,
}
pub struct PostUpdateRequest {
    pub params: Option<String>,
}
pub struct PostDeleteRequest {
    pub params: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ParsedPostListResponse {
    pub post_list: Option<Vec<PostObject>>,
}

impl ParsedPostListResponse {
    pub fn new(post_list: Option<Vec<PostObject>>) -> Self {
        Self { post_list }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ParsedPostCreateResponse {
    pub post: Option<PostObject>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct ParsedPostRetrieveResponse {
    pub post: Option<PostObject>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct ParsedPostUpdateResponse {
    pub post: Option<PostObject>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct ParsedPostDeleteResponse {
    pub post: Option<PostObject>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PostObject {
    pub id: Option<u32>,
    pub title: Option<PostTitle>,
    pub content: Option<PostContent>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PostTitle {
    pub raw: Option<String>,
    pub rendered: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PostContent {
    pub raw: Option<String>,
    pub rendered: Option<String>,
}

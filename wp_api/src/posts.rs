use serde::{Deserialize, Serialize};

pub trait PostNetworkingInterface: Send + Sync {}

#[derive(Default)] // The default has `None` for all
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
pub struct PostListResponse {
    pub post_list: Option<Vec<PostObject>>,
    pub next_page: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PostCreateResponse {
    pub post: Option<PostObject>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct PostRetrieveResponse {
    pub post: Option<PostObject>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct PostUpdateResponse {
    pub post: Option<PostObject>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct PostDeleteResponse {
    pub post: Option<PostObject>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PostObject {
    pub id: Option<u32>,
    pub date: Option<String>,
    pub date_gmt: Option<String>,
    pub guid: Option<PostGuid>,
    pub modified: Option<String>,
    pub modified_gmt: Option<String>,
    pub password: Option<String>,
    pub slug: Option<String>,
    pub status: Option<String>,
    pub link: Option<String>,
    pub title: Option<PostTitle>,
    pub content: Option<PostContent>,
    pub excerpt: Option<PostExcerpt>,
    pub author: Option<u32>,
    pub featured_media: Option<u32>,
    pub comment_status: Option<String>,
    pub ping_status: Option<String>,
    pub sticky: Option<bool>,
    pub template: Option<String>,
    pub format: Option<String>,
    pub meta: Option<PostMeta>,
    pub categories: Option<Vec<u32>>,
    pub tags: Option<Vec<u32>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PostGuid {
    pub raw: Option<String>,
    pub rendered: Option<String>,
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
    pub protected: Option<bool>,
    pub block_version: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PostExcerpt {
    pub raw: Option<String>,
    pub rendered: Option<String>,
    pub protected: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PostMeta {
    pub footnotes: Option<String>,
}

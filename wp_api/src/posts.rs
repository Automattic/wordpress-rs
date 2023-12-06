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

pub struct PostListResponse {}
pub struct PostCreateResponse {}
pub struct PostRetrieveResponse {}
pub struct PostUpdateResponse {}
pub struct PostDeleteResponse {}

#[derive(Debug)]
pub struct ParsedPostListResponse {
    pub post_list: Option<Vec<PostObject>>,
}
#[derive(Debug)]
pub struct ParsedPostCreateResponse {
    pub post: Option<PostObject>,
}
#[derive(Debug)]
pub struct ParsedPostRetrieveResponse {
    pub post: Option<PostObject>,
}
#[derive(Debug)]
pub struct ParsedPostUpdateResponse {
    pub post: Option<PostObject>,
}
#[derive(Debug)]
pub struct ParsedPostDeleteResponse {
    pub post: Option<PostObject>,
}

#[derive(Debug)]
pub struct PostObject {
    pub id: Option<u32>,
    pub title: Option<String>,
    pub content: Option<String>,
}

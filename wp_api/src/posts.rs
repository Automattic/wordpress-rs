pub trait PostNetworkingInterface: Send + Sync {
    fn list(&self, request: PostListRequest) -> PostListResponse;
    fn create(&self, request: PostCreateRequest) -> PostCreateResponse;
    fn retrieve(&self, request: PostRetrieveRequest) -> PostRetrieveResponse;
    fn update(&self, request: PostUpdateRequest) -> PostUpdateResponse;
    fn delete(&self, request: PostDeleteRequest) -> PostDeleteResponse;
}

pub struct PostRequestBuilder {}

impl PostRequestBuilder {
    pub fn list(&self, params: Option<PostListParams>) -> PostListRequest {
        todo!()
    }

    pub fn create(&self, params: Option<PostCreateParams>) -> PostCreateRequest {
        todo!()
    }

    pub fn retrieve(
        &self,
        post_id: u32,
        params: Option<PostRetrieveParams>,
    ) -> PostRetrieveRequest {
        todo!()
    }

    pub fn update(&self, post_id: u32, params: Option<PostUpdateParams>) -> PostUpdateRequest {
        todo!()
    }

    pub fn delete(&self, post_id: u32, params: Option<PostDeleteParams>) -> PostDeleteRequest {
        todo!()
    }
}

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

pub struct ParsedPostListResponse {
    pub post_list: Option<Vec<PostObject>>,
}
pub struct ParsedPostCreateResponse {
    pub post: Option<PostObject>,
}
pub struct ParsedPostRetrieveResponse {
    pub post: Option<PostObject>,
}
pub struct ParsedPostUpdateResponse {
    pub post: Option<PostObject>,
}
pub struct ParsedPostDeleteResponse {
    pub post: Option<PostObject>,
}

pub struct PostObject {
    pub id: Option<u32>,
    pub title: Option<String>,
    pub content: Option<String>,
}

use serde::{Deserialize, Serialize};
use wp_derive::WPContextual;

use crate::{parse_response_for_generic_errors, WPApiError, WPNetworkResponse};

pub trait PostNetworkingInterface: Send + Sync {}

#[derive(uniffi::Record)]
pub struct PostListParams {
    #[uniffi(default = 1)]
    pub page: u32,
    #[uniffi(default = 10)]
    pub per_page: u32,
}

impl PostListParams {
    pub fn query_pairs(&self) -> impl IntoIterator<Item = (&str, String)> {
        [
            ("page", self.page.to_string()),
            ("per_page", self.per_page.to_string()),
        ]
        .into_iter()
    }
}

impl Default for PostListParams {
    fn default() -> Self {
        Self {
            page: 1,
            per_page: 10,
        }
    }
}

#[derive(Serialize, uniffi::Record)]
pub struct PostCreateParams {
    pub title: Option<String>,
    pub content: Option<String>,
}

#[derive(uniffi::Record)]
pub struct PostRetrieveParams {
    pub password: Option<String>,
}

#[derive(Serialize, uniffi::Record)]
pub struct PostUpdateParams {
    pub title: Option<String>,
    pub content: Option<String>,
}

#[derive(uniffi::Record)]
pub struct PostDeleteParams {
    pub force: Option<bool>,
}

impl PostDeleteParams {
    pub fn query_pairs(&self) -> impl IntoIterator<Item = (&str, String)> {
        [("force", self.force.map(|x| x.to_string()))]
            .into_iter()
            // Remove `None` values
            .filter_map(|(k, opt_v)| opt_v.map(|v| (k, v)))
    }
}

#[derive(uniffi::Record)]
pub struct PostListRequest {
    pub params: Option<String>,
}
#[derive(uniffi::Record)]
pub struct PostCreateRequest {
    pub params: Option<String>,
}
#[derive(uniffi::Record)]
pub struct PostRetrieveRequest {
    pub params: Option<String>,
}
#[derive(uniffi::Record)]
pub struct PostUpdateRequest {
    pub params: Option<String>,
}
#[derive(uniffi::Record)]
pub struct PostDeleteRequest {
    pub params: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct PostListResponse {
    pub post_list: Option<Vec<SparsePost>>,
    pub next_page: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct PostCreateResponse {
    pub post: Option<SparsePost>,
}
#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct PostRetrieveResponse {
    pub post: Option<SparsePost>,
}
#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct PostUpdateResponse {
    pub post: Option<SparsePost>,
}
#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct PostDeleteResponse {
    pub post: Option<SparsePost>,
}

uniffi::custom_newtype!(PostId, u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostId(pub u32);

impl std::fmt::Display for PostId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WPContextual)]
pub struct SparsePost {
    #[WPContext(edit, embed, view)]
    pub id: Option<u32>,
    #[WPContext(edit, embed, view)]
    pub date: Option<String>,
    #[WPContext(edit, view)]
    pub date_gmt: Option<String>,
    #[WPContext(edit, view)]
    #[WPContextualField]
    pub guid: Option<SparsePostGuid>,
    #[WPContext(edit, view)]
    pub modified: Option<String>,
    #[WPContext(edit, view)]
    pub modified_gmt: Option<String>,
    #[WPContext(edit)]
    pub password: Option<String>,
    #[WPContext(edit, embed, view)]
    pub slug: Option<String>,
    #[WPContext(edit, view)]
    pub status: Option<String>,
    #[WPContext(edit, embed, view)]
    pub link: Option<String>,
    #[WPContext(edit, embed, view)]
    #[WPContextualField]
    pub title: Option<SparsePostTitle>,
    #[WPContext(edit, view)]
    #[WPContextualField]
    pub content: Option<SparsePostContent>,
    #[WPContext(edit, embed, view)]
    #[WPContextualField]
    pub excerpt: Option<SparsePostExcerpt>,
    #[WPContext(edit, embed, view)]
    pub author: Option<u32>,
    #[WPContext(edit, embed, view)]
    pub featured_media: Option<u32>,
    #[WPContext(edit, view)]
    pub comment_status: Option<String>,
    #[WPContext(edit, view)]
    pub ping_status: Option<String>,
    pub sticky: Option<bool>,
    #[WPContext(edit, view)]
    pub template: Option<String>,
    #[WPContext(edit, view)]
    pub format: Option<String>,
    #[WPContext(edit, view)]
    pub meta: Option<PostMeta>,
    #[WPContext(edit, view)]
    pub categories: Option<Vec<u32>>,
    #[WPContext(edit, view)]
    pub tags: Option<Vec<u32>>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WPContextual)]
pub struct SparsePostGuid {
    #[WPContext(edit)]
    pub raw: Option<String>,
    #[WPContext(edit, view)]
    pub rendered: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WPContextual)]
pub struct SparsePostTitle {
    #[WPContext(edit)]
    pub raw: Option<String>,
    #[WPContext(edit, embed, view)]
    pub rendered: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WPContextual)]
pub struct SparsePostContent {
    #[WPContext(edit)]
    pub raw: Option<String>,
    #[WPContext(edit, view)]
    pub rendered: Option<String>,
    #[WPContext(edit, embed, view)]
    pub protected: Option<bool>,
    #[WPContext(edit)]
    pub block_version: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WPContextual)]
pub struct SparsePostExcerpt {
    #[WPContext(edit)]
    pub raw: Option<String>,
    #[WPContext(edit, embed, view)]
    pub rendered: Option<String>,
    #[WPContext(edit, embed, view)]
    pub protected: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct PostMeta {
    pub footnotes: Option<String>,
}

#[uniffi::export]
pub fn parse_list_posts_response_with_edit_context(
    response: &WPNetworkResponse,
) -> Result<Vec<PostWithEditContext>, WPApiError> {
    parse_posts_response(response)
}

#[uniffi::export]
pub fn parse_list_posts_response_with_embed_context(
    response: &WPNetworkResponse,
) -> Result<Vec<PostWithEmbedContext>, WPApiError> {
    parse_posts_response(response)
}

#[uniffi::export]
pub fn parse_list_posts_response_with_view_context(
    response: &WPNetworkResponse,
) -> Result<Vec<PostWithViewContext>, WPApiError> {
    parse_posts_response(response)
}

#[uniffi::export]
pub fn parse_retrieve_post_response_with_edit_context(
    response: &WPNetworkResponse,
) -> Result<PostWithEditContext, WPApiError> {
    parse_posts_response(response)
}

#[uniffi::export]
pub fn parse_retrieve_post_response_with_embed_context(
    response: &WPNetworkResponse,
) -> Result<PostWithEmbedContext, WPApiError> {
    parse_posts_response(response)
}

#[uniffi::export]
pub fn parse_retrieve_post_response_with_view_context(
    response: &WPNetworkResponse,
) -> Result<PostWithViewContext, WPApiError> {
    parse_posts_response(response)
}

pub fn parse_posts_response<'de, T: Deserialize<'de>>(
    response: &'de WPNetworkResponse,
) -> Result<T, WPApiError> {
    parse_response_for_generic_errors(response)?;
    serde_json::from_slice(&response.body).map_err(|err| WPApiError::ParsingError {
        reason: err.to_string(),
        response: String::from_utf8_lossy(&response.body).to_string(),
    })
}

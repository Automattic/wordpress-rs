use serde::{Deserialize, Serialize};
use wp_contextual::WpContextual;

use crate::UserId;

#[derive(Debug, Default, uniffi::Record)]
pub struct PostListParams {
    /// Current page of the collection.
    /// Default: `1`
    #[uniffi(default = None)]
    pub page: Option<u32>,
}

impl PostListParams {
    pub fn query_pairs(&self) -> impl IntoIterator<Item = (&str, String)> {
        [("page", self.page.map(|x| x.to_string()))]
            .into_iter()
            // Remove `None` values
            .filter_map(|(k, opt_v)| opt_v.map(|v| (k, v)))
    }
}

uniffi::custom_newtype!(PostId, i32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostId(pub i32);

uniffi::custom_newtype!(PostTag, i32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostTag(pub i32);

impl std::fmt::Display for PostId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparsePost {
    #[WpContext(edit, embed, view)]
    pub id: Option<PostId>,
    #[WpContext(edit, view)]
    pub date: Option<String>,
    #[WpContext(edit, view)]
    pub date_gmt: Option<String>,
    #[WpContext(edit, view)]
    pub guid: Option<PostGuid>,
    #[WpContext(edit, embed, view)]
    pub link: Option<String>,
    #[WpContext(edit, view)]
    pub modified: Option<String>,
    #[WpContext(edit, view)]
    pub modified_gmt: Option<String>,
    #[WpContext(edit, embed, view)]
    pub slug: Option<String>,
    #[WpContext(edit, view)]
    pub status: Option<PostStatus>,
    #[serde(rename = "type")]
    #[WpContext(edit, embed, view)]
    pub post_type: Option<String>,
    #[WpContext(edit)]
    pub password: Option<String>,
    #[WpContext(edit)]
    pub permalink_template: Option<String>,
    #[WpContext(edit)]
    pub generated_slug: Option<String>,
    #[WpContext(edit, embed, view)]
    pub title: Option<PostTitle>,
    #[WpContext(edit, view)]
    pub content: Option<PostContent>,
    #[WpContext(edit, embed, view)]
    pub author: Option<UserId>,
    #[WpContext(edit, embed, view)]
    pub excerpt: Option<PostExcerpt>,
    #[WpContext(edit, embed, view)]
    pub featured_media: Option<i64>,
    #[WpContext(edit, view)]
    pub comment_status: Option<String>,
    #[WpContext(edit, view)]
    pub ping_status: Option<String>,
    #[WpContext(edit, view)]
    pub format: Option<String>,
    #[WpContext(edit, view)]
    pub meta: Option<PostMeta>,
    #[WpContext(edit, view)]
    pub sticky: Option<bool>,
    #[WpContext(edit, view)]
    pub template: Option<String>,
    #[WpContext(edit, view)]
    pub categories: Option<Vec<i64>>,
    #[WpContext(edit, view)]
    pub tags: Option<Vec<PostTag>>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct PostGuid {
    pub rendered: String,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct PostTitle {
    pub rendered: String,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct PostContent {
    pub rendered: String,
    pub protected: bool,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct PostExcerpt {
    pub rendered: String,
    pub protected: bool,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct PostMeta {
    pub footnotes: String,
}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, uniffi::Enum,
)]
#[serde(rename_all = "snake_case")]
pub enum PostStatus {
    Publish,
    Future,
    Draft,
    Pending,
    Private,
    #[serde(untagged)]
    Custom(String),
}

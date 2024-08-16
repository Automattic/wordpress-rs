use serde::{Deserialize, Serialize};
use wp_contextual::WpContextual;

use crate::{
    url_query::{AppendUrlQueryPairs, QueryPairs, QueryPairsExtension},
    UserId, WpApiParamOrder,
};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum WpApiParamPostsOrderBy {
    Author,
    #[default]
    Date,
    Id,
    Include,
    IncludeSlugs,
    Modified,
    Parent,
    Relevance,
    Slug,
    Title,
}

impl WpApiParamPostsOrderBy {
    fn as_str(&self) -> &str {
        match self {
            Self::Author => "author",
            Self::Date => "date",
            Self::Id => "id",
            Self::Include => "include",
            Self::IncludeSlugs => "include_slugs",
            Self::Modified => "modified",
            Self::Parent => "parent",
            Self::Relevance => "relevance",
            Self::Slug => "slug",
            Self::Title => "title",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum WpApiParamPostsTaxRelation {
    And,
    Or,
}

impl WpApiParamPostsTaxRelation {
    fn as_str(&self) -> &str {
        match self {
            Self::And => "AND",
            Self::Or => "OR",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum WpApiParamPostsSearchColumn {
    PostContent,
    PostExcerpt,
    PostTitle,
}

impl WpApiParamPostsSearchColumn {
    fn as_str(&self) -> &str {
        match self {
            Self::PostContent => "post_content",
            Self::PostExcerpt => "post_excerpt",
            Self::PostTitle => "post_title",
        }
    }
}

#[derive(Debug, Default, uniffi::Record)]
pub struct PostListParams {
    /// Current page of the collection.
    /// Default: `1`
    #[uniffi(default = None)]
    pub page: Option<u32>,
    /// Maximum number of items to be returned in result set.
    /// Default: `10`
    #[uniffi(default = None)]
    pub per_page: Option<u32>,
    /// Limit results to those matching a string.
    #[uniffi(default = None)]
    pub search: Option<String>,
    /// Limit response to posts published after a given ISO8601 compliant date.
    #[uniffi(default = None)]
    pub after: Option<String>,
    /// Limit response to posts modified after a given ISO8601 compliant date.
    #[uniffi(default = None)]
    pub modified_after: Option<String>,
    /// Limit result set to posts assigned to specific authors.
    #[uniffi(default = [])]
    pub author: Vec<UserId>,
    /// Ensure result set excludes posts assigned to specific authors.
    #[uniffi(default = [])]
    pub author_exclude: Vec<UserId>,
    /// Limit response to posts published before a given ISO8601 compliant date.
    #[uniffi(default = None)]
    pub before: Option<String>,
    /// Limit response to posts modified before a given ISO8601 compliant date.
    #[uniffi(default = None)]
    pub modified_before: Option<String>,
    /// Ensure result set excludes specific IDs.
    #[uniffi(default = [])]
    pub exclude: Vec<PostId>,
    /// Limit result set to specific IDs.
    #[uniffi(default = [])]
    pub include: Vec<PostId>,
    /// Offset the result set by a specific number of items.
    #[uniffi(default = None)]
    pub offset: Option<u32>,
    /// Order sort attribute ascending or descending.
    /// Default: desc
    /// One of: asc, desc
    #[uniffi(default = None)]
    pub order: Option<WpApiParamOrder>,
    /// Sort collection by post attribute.
    /// Default: date
    /// One of: author, date, id, include, modified, parent, relevance, slug, include_slugs, title
    #[uniffi(default = None)]
    pub orderby: Option<WpApiParamPostsOrderBy>,
    /// Array of column names to be searched.
    #[uniffi(default = [])]
    pub search_columns: Vec<WpApiParamPostsSearchColumn>,
    /// Limit result set to posts with one or more specific slugs.
    #[uniffi(default = [])]
    pub slug: Vec<String>,
    /// Limit result set to posts assigned one or more statuses.
    /// Default: publish
    #[uniffi(default = [])]
    pub status: Vec<PostStatus>,
    /// Limit result set based on relationship between multiple taxonomies.
    /// One of: AND, OR
    #[uniffi(default = None)]
    pub tax_relation: Option<WpApiParamPostsTaxRelation>,
    /// Limit result set to items with specific terms assigned in the categories taxonomy.
    #[uniffi(default = [])]
    pub categories: Vec<CategoryId>,
    /// Limit result set to items except those with specific terms assigned in the categories taxonomy.
    #[uniffi(default = [])]
    pub categories_exclude: Vec<CategoryId>,
    /// Limit result set to items with specific terms assigned in the tags taxonomy.
    #[uniffi(default = [])]
    pub tags: Vec<TagId>,
    /// Limit result set to items except those with specific terms assigned in the tags taxonomy.
    #[uniffi(default = [])]
    pub tags_exclude: Vec<TagId>,
    /// Limit result set to items that are sticky.
    #[uniffi(default = None)]
    pub sticky: Option<bool>,
}

impl AppendUrlQueryPairs for PostListParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut
            .append_option_query_value_pair("page", self.page.as_ref())
            .append_option_query_value_pair("per_page", self.per_page.as_ref())
            .append_option_query_value_pair("search", self.search.as_ref())
            .append_option_query_value_pair("after", self.after.as_ref())
            .append_option_query_value_pair("modified_after", self.modified_after.as_ref())
            .append_vec_query_value_pair("author", &self.author)
            .append_vec_query_value_pair("author_exclude", &self.author_exclude)
            .append_option_query_value_pair("before", self.before.as_ref())
            .append_option_query_value_pair("modified_before", self.modified_before.as_ref())
            //.append_vec_query_value_pair("exclude", &self.exclude)
            //.append_vec_query_value_pair("include", &self.include)
            .append_option_query_value_pair("offset", self.offset.as_ref())
            .append_option_query_value_pair("order", self.order.as_ref())
            //.append_option_query_value_pair("orderby", self.orderby.as_ref())
            //.append_vec_query_value_pair("search_columns", &self.search_columns)
            .append_vec_query_value_pair("slug", &self.slug)
            //.append_vec_query_value_pair("status", &self.status)
            //.append_option_query_value_pair("tax_relation", self.tax_relation.as_ref())
            //.append_vec_query_value_pair("categories", &self.categories)
            //.append_vec_query_value_pair("categories_exclude", &self.categories_exclude)
            //.append_vec_query_value_pair("tags", &self.tags)
            //.append_vec_query_value_pair("tags_exclude", &self.tags_exclude)
            .append_option_query_value_pair("sticky", self.sticky.as_ref());
    }
}

uniffi::custom_newtype!(PostId, i32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostId(pub i32);

uniffi::custom_newtype!(TagId, i32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagId(pub i32);

uniffi::custom_newtype!(CategoryId, i32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategoryId(pub i32);

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
    pub comment_status: Option<PostCommentStatus>,
    #[WpContext(edit, view)]
    pub ping_status: Option<PostPingStatus>,
    #[WpContext(edit, view)]
    pub format: Option<PostFormat>,
    #[WpContext(edit, view)]
    pub meta: Option<PostMeta>,
    #[WpContext(edit, view)]
    pub sticky: Option<bool>,
    #[WpContext(edit, view)]
    pub template: Option<String>,
    #[WpContext(edit, view)]
    pub categories: Option<Vec<i64>>,
    #[WpContext(edit, view)]
    pub tags: Option<Vec<TagId>>,
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
    Debug,
    Default,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    uniffi::Enum,
)]
#[serde(rename_all = "snake_case")]
pub enum PostStatus {
    Draft,
    Future,
    Pending,
    Private,
    #[default]
    Publish,
    #[serde(untagged)]
    Custom(String),
}

impl PostStatus {
    fn as_str(&self) -> &str {
        match self {
            Self::Draft => "draft",
            Self::Future => "future",
            Self::Pending => "pending",
            Self::Private => "private",
            Self::Publish => "publish",
            Self::Custom(status) => status,
        }
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, uniffi::Enum,
)]
#[serde(rename_all = "snake_case")]
pub enum PostCommentStatus {
    Open,
    Closed,
    #[serde(untagged)]
    Custom(String),
}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, uniffi::Enum,
)]
#[serde(rename_all = "snake_case")]
pub enum PostPingStatus {
    Open,
    Closed,
    #[serde(untagged)]
    Custom(String),
}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, uniffi::Enum,
)]
#[serde(rename_all = "snake_case")]
pub enum PostFormat {
    Standard,
    Aside,
    Chat,
    Gallery,
    Link,
    Image,
    Quote,
    Status,
    Video,
    Audio,
    #[serde(untagged)]
    Custom(String),
}

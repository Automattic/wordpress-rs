use std::{num::ParseIntError, str::FromStr};

use serde::{Deserialize, Serialize};
use strum_macros::IntoStaticStr;
use wp_contextual::WpContextual;
use wp_serde_helper::{deserialize_from_string_of_json_array, serialize_as_json_string};

use crate::{
    impl_as_query_value_for_new_type, impl_as_query_value_from_as_str,
    media::MediaId,
    url_query::{
        AppendUrlQueryPairs, AsQueryValue, FromUrlQueryPairs, QueryPairs, QueryPairsExtension,
        UrlQueryPairsMap,
    },
    EnumFromStrParsingError, UserId, WpApiParamOrder,
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

impl_as_query_value_from_as_str!(WpApiParamPostsOrderBy);

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

impl FromStr for WpApiParamPostsOrderBy {
    type Err = EnumFromStrParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "author" => Ok(Self::Author),
            "date" => Ok(Self::Date),
            "id" => Ok(Self::Id),
            "include" => Ok(Self::Include),
            "include_slugs" => Ok(Self::IncludeSlugs),
            "modified" => Ok(Self::Modified),
            "parent" => Ok(Self::Parent),
            "relevance" => Ok(Self::Relevance),
            "slug" => Ok(Self::Slug),
            "title" => Ok(Self::Title),
            value => Err(EnumFromStrParsingError::UnknownVariant {
                value: value.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum WpApiParamPostsTaxRelation {
    And,
    Or,
}

impl_as_query_value_from_as_str!(WpApiParamPostsTaxRelation);

impl WpApiParamPostsTaxRelation {
    fn as_str(&self) -> &str {
        match self {
            Self::And => "AND",
            Self::Or => "OR",
        }
    }
}

impl FromStr for WpApiParamPostsTaxRelation {
    type Err = EnumFromStrParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "AND" => Ok(Self::And),
            "OR" => Ok(Self::Or),
            value => Err(EnumFromStrParsingError::UnknownVariant {
                value: value.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum WpApiParamPostsSearchColumn {
    PostContent,
    PostExcerpt,
    PostTitle,
}

impl_as_query_value_from_as_str!(WpApiParamPostsSearchColumn);

impl WpApiParamPostsSearchColumn {
    fn as_str(&self) -> &str {
        match self {
            Self::PostContent => "post_content",
            Self::PostExcerpt => "post_excerpt",
            Self::PostTitle => "post_title",
        }
    }
}

impl FromStr for WpApiParamPostsSearchColumn {
    type Err = EnumFromStrParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "post_content" => Ok(Self::PostContent),
            "post_excerpt" => Ok(Self::PostExcerpt),
            "post_title" => Ok(Self::PostTitle),
            value => Err(EnumFromStrParsingError::UnknownVariant {
                value: value.to_string(),
            }),
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq, uniffi::Record)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, IntoStaticStr)]
enum PostListParamsField {
    #[strum(serialize = "page")]
    Page,
    #[strum(serialize = "per_page")]
    PerPage,
    #[strum(serialize = "search")]
    Search,
    #[strum(serialize = "after")]
    After,
    #[strum(serialize = "modified_after")]
    ModifiedAfter,
    #[strum(serialize = "author")]
    Author,
    #[strum(serialize = "author_exclude")]
    AuthorExclude,
    #[strum(serialize = "before")]
    Before,
    #[strum(serialize = "modified_before")]
    ModifiedBefore,
    #[strum(serialize = "exclude")]
    Exclude,
    #[strum(serialize = "include")]
    Include,
    #[strum(serialize = "offset")]
    Offset,
    #[strum(serialize = "order")]
    Order,
    #[strum(serialize = "orderby")]
    Orderby,
    #[strum(serialize = "search_columns")]
    SearchColumns,
    #[strum(serialize = "slug")]
    Slug,
    #[strum(serialize = "status")]
    Status,
    #[strum(serialize = "tax_relation")]
    TaxRelation,
    #[strum(serialize = "categories")]
    Categories,
    #[strum(serialize = "categories_exclude")]
    CategoriesExclude,
    #[strum(serialize = "tags")]
    Tags,
    #[strum(serialize = "tags_exclude")]
    TagsExclude,
    #[strum(serialize = "sticky")]
    Sticky,
}

impl AppendUrlQueryPairs for PostListParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut
            .append_option_query_value_pair(PostListParamsField::Page, self.page.as_ref())
            .append_option_query_value_pair(PostListParamsField::PerPage, self.per_page.as_ref())
            .append_option_query_value_pair(PostListParamsField::Search, self.search.as_ref())
            .append_option_query_value_pair(PostListParamsField::After, self.after.as_ref())
            .append_option_query_value_pair(
                PostListParamsField::ModifiedAfter,
                self.modified_after.as_ref(),
            )
            .append_vec_query_value_pair(PostListParamsField::Author, &self.author)
            .append_vec_query_value_pair(PostListParamsField::AuthorExclude, &self.author_exclude)
            .append_option_query_value_pair(PostListParamsField::Before, self.before.as_ref())
            .append_option_query_value_pair(
                PostListParamsField::ModifiedBefore,
                self.modified_before.as_ref(),
            )
            .append_vec_query_value_pair(PostListParamsField::Exclude, &self.exclude)
            .append_vec_query_value_pair(PostListParamsField::Include, &self.include)
            .append_option_query_value_pair(PostListParamsField::Offset, self.offset.as_ref())
            .append_option_query_value_pair(PostListParamsField::Order, self.order.as_ref())
            .append_option_query_value_pair(PostListParamsField::Orderby, self.orderby.as_ref())
            .append_vec_query_value_pair(PostListParamsField::SearchColumns, &self.search_columns)
            .append_vec_query_value_pair(PostListParamsField::Slug, &self.slug)
            .append_vec_query_value_pair(PostListParamsField::Status, &self.status)
            .append_option_query_value_pair(
                PostListParamsField::TaxRelation,
                self.tax_relation.as_ref(),
            )
            .append_vec_query_value_pair(PostListParamsField::Categories, &self.categories)
            .append_vec_query_value_pair(
                PostListParamsField::CategoriesExclude,
                &self.categories_exclude,
            )
            .append_vec_query_value_pair(PostListParamsField::Tags, &self.tags)
            .append_vec_query_value_pair(PostListParamsField::TagsExclude, &self.tags_exclude)
            .append_option_query_value_pair(PostListParamsField::Sticky, self.sticky.as_ref());
    }
}

impl FromUrlQueryPairs for PostListParams {
    fn from_url_query_pairs(query_pairs: UrlQueryPairsMap) -> Option<Self> {
        Some(Self {
            page: query_pairs.get(PostListParamsField::Page),
            per_page: query_pairs.get(PostListParamsField::PerPage),
            search: query_pairs.get(PostListParamsField::Search),
            after: query_pairs.get(PostListParamsField::After),
            modified_after: query_pairs.get(PostListParamsField::ModifiedAfter),
            author: query_pairs.get_csv(PostListParamsField::Author),
            author_exclude: query_pairs.get_csv(PostListParamsField::AuthorExclude),
            before: query_pairs.get(PostListParamsField::Before),
            modified_before: query_pairs.get(PostListParamsField::ModifiedBefore),
            exclude: query_pairs.get_csv(PostListParamsField::Exclude),
            include: query_pairs.get_csv(PostListParamsField::Include),
            offset: query_pairs.get(PostListParamsField::Offset),
            order: query_pairs.get(PostListParamsField::Order),
            orderby: query_pairs.get(PostListParamsField::Orderby),
            search_columns: query_pairs.get_csv(PostListParamsField::SearchColumns),
            slug: query_pairs.get_csv(PostListParamsField::Slug),
            status: query_pairs.get_csv(PostListParamsField::Status),
            tax_relation: query_pairs.get(PostListParamsField::TaxRelation),
            categories: query_pairs.get_csv(PostListParamsField::Categories),
            categories_exclude: query_pairs.get_csv(PostListParamsField::CategoriesExclude),
            tags: query_pairs.get_csv(PostListParamsField::Tags),
            tags_exclude: query_pairs.get_csv(PostListParamsField::TagsExclude),
            sticky: query_pairs.get(PostListParamsField::Sticky),
        })
    }

    fn supports_pagination() -> bool {
        true
    }
}

#[derive(Debug, Default, uniffi::Record)]
pub struct PostRetrieveParams {
    /// The password for the post if it is password protected.
    #[uniffi(default = None)]
    pub password: Option<String>,
}

impl AppendUrlQueryPairs for PostRetrieveParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut.append_option_query_value_pair("password", self.password.as_ref());
    }
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct PostDeleteResponse {
    pub deleted: bool,
    pub previous: PostWithEditContext,
}

#[derive(Debug, Default, Serialize, uniffi::Record)]
pub struct PostCreateParams {
    // The date the post was published, in the site's timezone.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    // The date the post was published, as GMT.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_gmt: Option<String>,
    // An alphanumeric identifier for the post unique to its type.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    // A named status for the post.
    // One of: publish, future, draft, pending, private
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<PostStatus>,
    // A password to protect access to the content and excerpt.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    // The title for the post.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    // The content for the post.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    // The ID for the author of the post.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<UserId>,
    // The excerpt for the post.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excerpt: Option<String>,
    // The ID of the featured media for the post.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub featured_media: Option<MediaId>,
    // Whether or not comments are open on the post.
    // One of: open, closed
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment_status: Option<PostCommentStatus>,
    // Whether or not the post can be pinged.
    // One of: open, closed
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ping_status: Option<PostPingStatus>,
    // The format for the post.
    // One of: standard, aside, chat, gallery, link, image, quote, status, video, audio
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<PostFormat>,
    // Meta fields.
    pub meta: Option<PostMeta>,
    // Whether or not the post should be treated as sticky.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sticky: Option<bool>,
    // The theme file to use to display the post.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    // The terms assigned to the post in the category taxonomy.
    #[uniffi(default = [])]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub categories: Vec<CategoryId>,
    // The terms assigned to the post in the post_tag taxonomy.
    #[uniffi(default = [])]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<TagId>,
}

#[derive(Debug, Default, Serialize, uniffi::Record)]
pub struct PostUpdateParams {
    // The date the post was published, in the site's timezone.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    // The date the post was published, as GMT.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_gmt: Option<String>,
    // An alphanumeric identifier for the post unique to its type.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    // A named status for the post.
    // One of: publish, future, draft, pending, private
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<PostStatus>,
    // A password to protect access to the content and excerpt.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    // The title for the post.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    // The content for the post.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    // The ID for the author of the post.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<UserId>,
    // The excerpt for the post.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excerpt: Option<String>,
    // The ID of the featured media for the post.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub featured_media: Option<MediaId>,
    // Whether or not comments are open on the post.
    // One of: open, closed
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment_status: Option<PostCommentStatus>,
    // Whether or not the post can be pinged.
    // One of: open, closed
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ping_status: Option<PostPingStatus>,
    // The format for the post.
    // One of: standard, aside, chat, gallery, link, image, quote, status, video, audio
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<PostFormat>,
    // Meta fields.
    pub meta: Option<PostMeta>,
    // Whether or not the post should be treated as sticky.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sticky: Option<bool>,
    // The theme file to use to display the post.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    // The terms assigned to the post in the category taxonomy.
    #[uniffi(default = [])]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub categories: Vec<CategoryId>,
    // The terms assigned to the post in the post_tag taxonomy.
    #[uniffi(default = [])]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<TagId>,
}

impl_as_query_value_for_new_type!(PostId);
uniffi::custom_newtype!(PostId, i64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostId(pub i64);

impl FromStr for PostId {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self)
    }
}

impl_as_query_value_for_new_type!(TagId);
uniffi::custom_newtype!(TagId, i64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagId(pub i64);

impl FromStr for TagId {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self)
    }
}

impl_as_query_value_for_new_type!(CategoryId);
uniffi::custom_newtype!(CategoryId, i64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategoryId(pub i64);

impl FromStr for CategoryId {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self)
    }
}

impl std::fmt::Display for PostId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparsePost {
    #[WpContext(edit, embed, view)]
    pub id: Option<PostId>,
    #[WpContext(edit, embed, view)]
    pub date: Option<String>,
    #[WpContext(edit, view)]
    pub date_gmt: Option<String>,
    #[WpContext(edit, view)]
    #[WpContextualField]
    pub guid: Option<SparsePostGuid>,
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
    #[WpContextualField]
    pub title: Option<SparsePostTitle>,
    #[WpContext(edit, view)]
    #[WpContextualField]
    pub content: Option<SparsePostContent>,
    #[WpContext(edit, embed, view)]
    pub author: Option<UserId>,
    #[WpContext(edit, embed, view)]
    #[WpContextualField]
    pub excerpt: Option<SparsePostExcerpt>,
    #[WpContext(edit, embed, view)]
    pub featured_media: Option<MediaId>,
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
    pub categories: Option<Vec<CategoryId>>,
    #[WpContext(edit, view)]
    pub tags: Option<Vec<TagId>>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparsePostGuid {
    #[WpContext(edit)]
    pub raw: Option<String>,
    #[WpContext(edit, view)]
    pub rendered: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparsePostTitle {
    #[WpContext(edit)]
    pub raw: Option<String>,
    #[WpContext(edit, embed, view)]
    pub rendered: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparsePostContent {
    #[WpContext(edit)]
    pub raw: Option<String>,
    #[WpContext(edit, view)]
    pub rendered: Option<String>,
    #[WpContext(edit, view)]
    pub protected: Option<bool>,
    #[WpContext(edit)]
    pub block_version: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparsePostExcerpt {
    #[WpContext(edit)]
    pub raw: Option<String>,
    #[WpContext(edit, embed, view)]
    pub rendered: Option<String>,
    #[WpContext(edit, embed, view)]
    pub protected: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct PostMeta {
    #[serde(deserialize_with = "deserialize_from_string_of_json_array")]
    #[serde(serialize_with = "serialize_as_json_string")]
    pub footnotes: Vec<PostFootnote>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, uniffi::Record)]
pub struct PostFootnote {
    pub id: String,
    pub content: String,
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

impl_as_query_value_from_as_str!(PostStatus);

impl PostStatus {
    pub fn as_str(&self) -> &str {
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

impl FromStr for PostStatus {
    type Err = EnumFromStrParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "draft" => Ok(Self::Draft),
            "future" => Ok(Self::Future),
            "pending" => Ok(Self::Pending),
            "private" => Ok(Self::Private),
            "publish" => Ok(Self::Publish),
            value => Ok(Self::Custom(value.to_string())),
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

impl PostCommentStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Open => "open",
            Self::Closed => "closed",
            Self::Custom(comment_status) => comment_status,
        }
    }
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

impl PostPingStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Open => "open",
            Self::Closed => "closed",
            Self::Custom(ping_status) => ping_status,
        }
    }
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

impl PostFormat {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Standard => "standard",
            Self::Aside => "aside",
            Self::Chat => "chat",
            Self::Gallery => "gallery",
            Self::Link => "link",
            Self::Image => "image",
            Self::Quote => "quote",
            Self::Status => "status",
            Self::Video => "video",
            Self::Audio => "audio",
            Self::Custom(post_format) => post_format,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{generate, unit_test_common::assert_expected_and_from_query_pairs};
    use rstest::*;

    #[rstest]
    #[case(PostListParams::default(), "")]
    #[case(generate!(PostListParams, (page, Some(2))), "page=2")]
    #[case(generate!(PostListParams, (per_page, Some(2))), "per_page=2")]
    #[case(generate!(PostListParams, (search, Some("foo".to_string()))), "search=foo")]
    #[case(generate!(PostListParams, (after, Some("2023-08-14 17:00:00.000".to_string()))), "after=2023-08-14+17%3A00%3A00.000")]
    #[case(generate!(PostListParams, (modified_after, Some("2023-08-14 17:00:00.000".to_string()))), "modified_after=2023-08-14+17%3A00%3A00.000")]
    #[case(generate!(PostListParams, (author, vec![UserId(1), UserId(2)])), "author=1%2C2")]
    #[case(generate!(PostListParams, (author_exclude, vec![UserId(1), UserId(2)])), "author_exclude=1%2C2")]
    #[case(generate!(PostListParams, (before, Some("2023-08-14 17:00:00.000".to_string()))), "before=2023-08-14+17%3A00%3A00.000")]
    #[case(generate!(PostListParams, (modified_before, Some("2023-08-14 17:00:00.000".to_string()))), "modified_before=2023-08-14+17%3A00%3A00.000")]
    #[case(generate!(PostListParams, (exclude, vec![PostId(1), PostId(2)])), "exclude=1%2C2")]
    #[case(generate!(PostListParams, (include, vec![PostId(1), PostId(2)])), "include=1%2C2")]
    #[case(generate!(PostListParams, (offset, Some(2))), "offset=2")]
    #[case(generate!(PostListParams, (order, Some(WpApiParamOrder::Asc))), "order=asc")]
    #[case(generate!(PostListParams, (order, Some(WpApiParamOrder::Desc))), "order=desc")]
    #[case(generate!(PostListParams, (orderby, Some(WpApiParamPostsOrderBy::Author))), "orderby=author")]
    #[case(generate!(PostListParams, (orderby, Some(WpApiParamPostsOrderBy::Date))), "orderby=date")]
    #[case(generate!(PostListParams, (orderby, Some(WpApiParamPostsOrderBy::Id))), "orderby=id")]
    #[case(generate!(PostListParams, (orderby, Some(WpApiParamPostsOrderBy::Include))), "orderby=include")]
    #[case(generate!(PostListParams, (orderby, Some(WpApiParamPostsOrderBy::IncludeSlugs))), "orderby=include_slugs")]
    #[case(generate!(PostListParams, (orderby, Some(WpApiParamPostsOrderBy::Modified))), "orderby=modified")]
    #[case(generate!(PostListParams, (orderby, Some(WpApiParamPostsOrderBy::Parent))), "orderby=parent")]
    #[case(generate!(PostListParams, (orderby, Some(WpApiParamPostsOrderBy::Relevance))), "orderby=relevance")]
    #[case(generate!(PostListParams, (orderby, Some(WpApiParamPostsOrderBy::Slug))), "orderby=slug")]
    #[case(generate!(PostListParams, (orderby, Some(WpApiParamPostsOrderBy::Title))), "orderby=title")]
    #[case(generate!(PostListParams, (search_columns, vec![WpApiParamPostsSearchColumn::PostContent])), "search_columns=post_content")]
    #[case(generate!(PostListParams, (search_columns, vec![WpApiParamPostsSearchColumn::PostExcerpt])), "search_columns=post_excerpt")]
    #[case(generate!(PostListParams, (search_columns, vec![WpApiParamPostsSearchColumn::PostTitle])), "search_columns=post_title")]
    #[case(generate!(PostListParams, (search_columns, vec![WpApiParamPostsSearchColumn::PostContent, WpApiParamPostsSearchColumn::PostExcerpt, WpApiParamPostsSearchColumn::PostTitle])), "search_columns=post_content%2Cpost_excerpt%2Cpost_title")]
    #[case(generate!(PostListParams, (slug, vec!["foo".to_string(), "bar".to_string()])), "slug=foo%2Cbar")]
    #[case(generate!(PostListParams, (status, vec![PostStatus::Draft])), "status=draft")]
    #[case(generate!(PostListParams, (status, vec![PostStatus::Future])), "status=future")]
    #[case(generate!(PostListParams, (status, vec![PostStatus::Pending])), "status=pending")]
    #[case(generate!(PostListParams, (status, vec![PostStatus::Private])), "status=private")]
    #[case(generate!(PostListParams, (status, vec![PostStatus::Publish])), "status=publish")]
    #[case(generate!(PostListParams, (status, vec![PostStatus::Custom("foo".to_string())])), "status=foo")]
    #[case(generate!(PostListParams, (status, vec![PostStatus::Draft, PostStatus::Future, PostStatus::Pending, PostStatus::Private, PostStatus::Publish, PostStatus::Custom("foo".to_string())])), "status=draft%2Cfuture%2Cpending%2Cprivate%2Cpublish%2Cfoo")]
    #[case(generate!(PostListParams, (tax_relation, Some(WpApiParamPostsTaxRelation::And))), "tax_relation=AND")]
    #[case(generate!(PostListParams, (tax_relation, Some(WpApiParamPostsTaxRelation::Or))), "tax_relation=OR")]
    #[case(generate!(PostListParams, (categories, vec![CategoryId(1), CategoryId(2)])), "categories=1%2C2")]
    #[case(generate!(PostListParams, (categories_exclude, vec![CategoryId(1), CategoryId(2)])), "categories_exclude=1%2C2")]
    #[case(generate!(PostListParams, (tags, vec![TagId(1), TagId(2)])), "tags=1%2C2")]
    #[case(generate!(PostListParams, (tags_exclude, vec![TagId(1), TagId(2)])), "tags_exclude=1%2C2")]
    #[case(generate!(PostListParams, (sticky, Some(true))), "sticky=true")]
    #[case(PostListParams {
        page: Some(11),
        per_page: Some(22),
        search: Some("s_q".to_string()),
        after: Some("d_a".to_string()),
        modified_after: Some("d_m_a".to_string()),
        author: vec![UserId(111), UserId(112)],
        author_exclude: vec![UserId(211), UserId(212)],
        before: Some("d_b".to_string()),
        modified_before: Some("d_m_b".to_string()),
        exclude: vec![PostId(1111), PostId(1112)],
        include: vec![PostId(2111), PostId(2112)],
        offset: Some(11111),
        order: Some(WpApiParamOrder::Desc),
        orderby: Some(WpApiParamPostsOrderBy::Slug),
        search_columns: vec![
            WpApiParamPostsSearchColumn::PostContent,
            WpApiParamPostsSearchColumn::PostExcerpt,
        ],
        slug: vec!["sl_1".to_string(), "sl_2".to_string()],
        status: vec![PostStatus::Draft, PostStatus::Future],
        tax_relation: Some(WpApiParamPostsTaxRelation::Or),
        categories: vec![CategoryId(333333), CategoryId(333334)],
        categories_exclude: vec![CategoryId(444444), CategoryId(444445)],
        tags: vec![TagId(555555), TagId(555556)],
        tags_exclude: vec![TagId(666666), TagId(666667)],
        sticky: Some(true),
        },
        "page=11&per_page=22&search=s_q&after=d_a&modified_after=d_m_a&author=111%2C112&author_exclude=211%2C212&before=d_b&modified_before=d_m_b&exclude=1111%2C1112&include=2111%2C2112&offset=11111&order=desc&orderby=slug&search_columns=post_content%2Cpost_excerpt&slug=sl_1%2Csl_2&status=draft%2Cfuture&tax_relation=OR&categories=333333%2C333334&categories_exclude=444444%2C444445&tags=555555%2C555556&tags_exclude=666666%2C666667&sticky=true"
    )]
    #[trace]
    fn test_post_list_query_pairs(#[case] params: PostListParams, #[case] expected_query: &str) {
        assert_expected_and_from_query_pairs(params, expected_query);
    }
}

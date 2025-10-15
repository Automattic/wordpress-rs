use crate::{
    UserId, WpApiParamOrder,
    date::WpGmtDateTime,
    impl_as_query_value_from_to_string,
    media::MediaId,
    terms::TermId,
    url_query::{
        AppendUrlQueryPairs, FromUrlQueryPairs, QueryPairs, QueryPairsExtension, UrlQueryPairsMap,
    },
    wp_content_i64_id,
};
use serde::{Deserialize, Serialize};
use wp_contextual::WpContextual;
use wp_derive::{WpDeriveParamsField, WpDeserialize};
use wp_serde_helper::{deserialize_from_string_of_json_array, serialize_as_json_string};

#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    uniffi::Enum,
    strum_macros::EnumString,
    strum_macros::Display,
)]
#[strum(serialize_all = "snake_case")]
pub enum WpApiParamPostsOrderBy {
    Author,
    #[default]
    Date,
    Id,
    Include,
    IncludeSlugs,
    MenuOrder,
    Modified,
    Parent,
    Relevance,
    Slug,
    Title,
}

impl_as_query_value_from_to_string!(WpApiParamPostsOrderBy);

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum, strum_macros::EnumString, strum_macros::Display,
)]
pub enum WpApiParamPostsTaxRelation {
    #[strum(serialize = "AND")]
    And,
    #[strum(serialize = "OR")]
    Or,
}

impl_as_query_value_from_to_string!(WpApiParamPostsTaxRelation);

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum, strum_macros::EnumString, strum_macros::Display,
)]
#[strum(serialize_all = "snake_case")]
pub enum WpApiParamPostsSearchColumn {
    PostContent,
    PostExcerpt,
    PostTitle,
}

impl_as_query_value_from_to_string!(WpApiParamPostsSearchColumn);

#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(true)]
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
    pub after: Option<WpGmtDateTime>,
    /// Limit response to posts modified after a given ISO8601 compliant date.
    #[uniffi(default = None)]
    pub modified_after: Option<WpGmtDateTime>,
    /// Limit result set to posts assigned to specific authors.
    #[uniffi(default = [])]
    pub author: Vec<UserId>,
    /// Ensure result set excludes posts assigned to specific authors.
    #[uniffi(default = [])]
    pub author_exclude: Vec<UserId>,
    /// Limit response to posts published before a given ISO8601 compliant date.
    #[uniffi(default = None)]
    pub before: Option<WpGmtDateTime>,
    /// Limit response to posts modified before a given ISO8601 compliant date.
    #[uniffi(default = None)]
    pub modified_before: Option<WpGmtDateTime>,
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
    /// One of: author, date, id, include, modified, parent, relevance, slug, include_slugs, title, menu_order
    #[uniffi(default = None)]
    #[field_name("orderby")]
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
    pub categories: Vec<TermId>,
    /// Limit result set to items except those with specific terms assigned in the categories taxonomy.
    #[uniffi(default = [])]
    pub categories_exclude: Vec<TermId>,
    /// Limit result set to items with specific terms assigned in the tags taxonomy.
    #[uniffi(default = [])]
    pub tags: Vec<TermId>,
    /// Limit result set to items except those with specific terms assigned in the tags taxonomy.
    #[uniffi(default = [])]
    pub tags_exclude: Vec<TermId>,
    /// Limit result set to items that are sticky.
    #[uniffi(default = None)]
    pub sticky: Option<bool>,
    // Page-specific fields (for hierarchical post types)
    /// Limit result set to items with a specific parent.
    #[uniffi(default = None)]
    pub parent: Option<PostId>,
    /// Limit result set to items except those of a specific parent.
    #[uniffi(default = [])]
    pub parent_exclude: Vec<PostId>,
    /// Limit result set by menu order.
    #[uniffi(default = None)]
    pub menu_order: Option<u32>,
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
    pub previous: AnyPostWithEditContext,
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
    pub date_gmt: Option<WpGmtDateTime>,
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
    pub categories: Vec<TermId>,
    // The terms assigned to the post in the post_tag taxonomy.
    #[uniffi(default = [])]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<TermId>,
    // Page-specific fields (for hierarchical post types)
    // The ID for the parent of the post.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<PostId>,
    // The order of the post in relation to other posts.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub menu_order: Option<u32>,
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
    pub date_gmt: Option<WpGmtDateTime>,
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
    pub categories: Vec<TermId>,
    // The terms assigned to the post in the post_tag taxonomy.
    #[uniffi(default = [])]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<TermId>,
    // Page-specific fields (for hierarchical post types)
    // The ID for the parent of the post.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<PostId>,
    // The order of the post in relation to other posts.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub menu_order: Option<u32>,
}

wp_content_i64_id!(PostId);

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparseAnyPost {
    #[WpContext(edit, embed, view)]
    pub id: Option<PostId>,
    #[WpContext(edit, embed, view)]
    pub date: Option<String>,
    #[WpContext(edit, view)]
    pub date_gmt: Option<WpGmtDateTime>,
    #[WpContext(edit, view)]
    #[WpContextualField]
    pub guid: Option<SparsePostGuid>,
    #[WpContext(edit, embed, view)]
    pub link: Option<String>,
    #[WpContext(edit, view)]
    pub modified: Option<String>,
    #[WpContext(edit, view)]
    pub modified_gmt: Option<WpGmtDateTime>,
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
    #[WpContextualOption]
    pub permalink_template: Option<String>,
    #[WpContext(edit)]
    #[WpContextualOption]
    pub generated_slug: Option<String>,
    #[WpContext(edit, embed, view)]
    #[WpContextualField]
    pub title: Option<SparsePostTitle>,
    #[WpContext(edit, view)]
    #[WpContextualField]
    pub content: Option<SparsePostContent>,
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
    pub author: Option<UserId>,
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
    pub excerpt: Option<SparsePostExcerpt>,
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
    pub featured_media: Option<MediaId>,
    #[WpContext(edit, view)]
    #[WpContextualOption]
    pub comment_status: Option<PostCommentStatus>,
    #[WpContext(edit, view)]
    #[WpContextualOption]
    pub ping_status: Option<PostPingStatus>,
    #[WpContext(edit, view)]
    #[WpContextualOption]
    pub format: Option<PostFormat>,
    #[WpContext(edit, view)]
    #[WpContextualOption]
    pub meta: Option<PostMeta>,
    #[WpContext(edit, view)]
    #[WpContextualOption]
    pub sticky: Option<bool>,
    #[WpContext(edit, view)]
    pub template: Option<String>,
    #[WpContext(edit, view)]
    #[WpContextualOption]
    pub categories: Option<Vec<TermId>>,
    #[WpContext(edit, view)]
    #[WpContextualOption]
    pub tags: Option<Vec<TermId>>,
    // Page-specific fields (optional for pages, None for posts)
    #[WpContext(edit, view)]
    #[WpContextualOption]
    pub parent: Option<PostId>,
    #[WpContext(edit, view)]
    #[WpContextualOption]
    pub menu_order: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparsePostGuid {
    #[WpContext(edit)]
    #[WpContextualOption]
    pub raw: Option<String>,
    #[WpContext(edit, view)]
    pub rendered: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparsePostTitle {
    #[WpContext(edit)]
    #[WpContextualOption]
    pub raw: Option<String>,
    #[WpContext(edit, embed, view)]
    pub rendered: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparsePostContent {
    #[WpContext(edit)]
    #[WpContextualOption]
    pub raw: Option<String>,
    #[WpContext(edit, view)]
    pub rendered: Option<String>,
    #[WpContext(edit, view)]
    #[WpContextualOption]
    pub protected: Option<bool>,
    #[WpContext(edit)]
    #[WpContextualOption]
    pub block_version: Option<u32>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct SparsePostExcerpt {
    pub raw: Option<String>,
    pub rendered: Option<String>,
    pub protected: Option<bool>,
}

#[derive(Debug, PartialEq, Eq, Serialize, WpDeserialize, uniffi::Record)]
pub struct PostMeta {
    #[serde(deserialize_with = "deserialize_from_string_of_json_array")]
    #[serde(serialize_with = "serialize_as_json_string")]
    pub footnotes: Option<Vec<PostFootnote>>,
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
    strum_macros::EnumString,
    strum_macros::Display,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum PostStatus {
    Draft,
    Future,
    Pending,
    Private,
    #[default]
    Publish,
    #[serde(untagged)]
    #[strum(default)]
    Custom(String),
}

impl_as_query_value_from_to_string!(PostStatus);

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    uniffi::Enum,
    strum_macros::EnumString,
    strum_macros::Display,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum PostCommentStatus {
    Open,
    Closed,
    #[serde(untagged)]
    #[strum(default)]
    Custom(String),
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    uniffi::Enum,
    strum_macros::EnumString,
    strum_macros::Display,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum PostPingStatus {
    Open,
    Closed,
    #[serde(untagged)]
    #[strum(default)]
    Custom(String),
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    uniffi::Enum,
    strum_macros::EnumString,
    strum_macros::Display,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
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
    #[strum(default)]
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
    use crate::{
        SparseField, generate,
        terms::TermId,
        unit_test_common::{
            assert_expected_and_from_query_pairs, unit_test_example_date_as_option,
            unit_test_example_date_as_query_value,
        },
    };
    use rstest::*;

    #[rstest]
    #[case(PostListParams::default(), "")]
    #[case(generate!(PostListParams, (page, Some(2))), "page=2")]
    #[case(generate!(PostListParams, (per_page, Some(2))), "per_page=2")]
    #[case(generate!(PostListParams, (search, Some("foo".to_string()))), "search=foo")]
    #[case(generate!(PostListParams, (after, unit_test_example_date_as_option())), &unit_test_example_date_as_query_value("after"))]
    #[case(generate!(PostListParams, (modified_after, unit_test_example_date_as_option())), &unit_test_example_date_as_query_value("modified_after"))]
    #[case(generate!(PostListParams, (author, vec![UserId(1), UserId(2)])), "author=1%2C2")]
    #[case(generate!(PostListParams, (author_exclude, vec![UserId(1), UserId(2)])), "author_exclude=1%2C2")]
    #[case(generate!(PostListParams, (before, unit_test_example_date_as_option())), &unit_test_example_date_as_query_value("before"))]
    #[case(generate!(PostListParams, (modified_before, unit_test_example_date_as_option())), &unit_test_example_date_as_query_value("modified_before"))]
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
    #[case(generate!(PostListParams, (status, vec![PostStatus::Custom("foo-bar".to_string())])), "status=foo-bar")]
    #[case(generate!(PostListParams, (status, vec![PostStatus::Custom("foo_bar".to_string())])), "status=foo_bar")]
    #[case(generate!(PostListParams, (status, vec![PostStatus::Custom("FooBar".to_string())])), "status=FooBar")]
    #[case(generate!(PostListParams, (status, vec![PostStatus::Draft, PostStatus::Future, PostStatus::Pending, PostStatus::Private, PostStatus::Publish, PostStatus::Custom("foo".to_string())])), "status=draft%2Cfuture%2Cpending%2Cprivate%2Cpublish%2Cfoo")]
    #[case(generate!(PostListParams, (tax_relation, Some(WpApiParamPostsTaxRelation::And))), "tax_relation=AND")]
    #[case(generate!(PostListParams, (tax_relation, Some(WpApiParamPostsTaxRelation::Or))), "tax_relation=OR")]
    #[case(generate!(PostListParams, (categories, vec![TermId(1), TermId(2)])), "categories=1%2C2")]
    #[case(generate!(PostListParams, (categories_exclude, vec![TermId(1), TermId(2)])), "categories_exclude=1%2C2")]
    #[case(generate!(PostListParams, (tags, vec![TermId(1), TermId(2)])), "tags=1%2C2")]
    #[case(generate!(PostListParams, (tags_exclude, vec![TermId(1), TermId(2)])), "tags_exclude=1%2C2")]
    #[case(generate!(PostListParams, (sticky, Some(true))), "sticky=true")]
    #[case(
        post_list_params_with_all_fields(),
        &expected_query_pairs_for_post_list_params_with_all_fields()
    )]
    #[trace]
    fn test_post_list_query_pairs(#[case] params: PostListParams, #[case] expected_query: &str) {
        assert_expected_and_from_query_pairs(params, expected_query);
    }

    #[rstest]
    #[case(SparseAnyPostFieldWithEditContext::Id, "id")]
    #[case(SparseAnyPostFieldWithEditContext::PostType, "type")]
    fn test_as_mapped_field_name_for_edit_context(
        #[case] field: SparseAnyPostFieldWithEditContext,
        #[case] expected_mapped_field_name: &str,
    ) {
        assert_eq!(field.as_mapped_field_name(), expected_mapped_field_name);
    }

    #[rstest]
    #[case(SparseAnyPostFieldWithEmbedContext::Id, "id")]
    #[case(SparseAnyPostFieldWithEmbedContext::PostType, "type")]
    fn test_as_mapped_field_name_for_embed_context(
        #[case] field: SparseAnyPostFieldWithEmbedContext,
        #[case] expected_mapped_field_name: &str,
    ) {
        assert_eq!(field.as_mapped_field_name(), expected_mapped_field_name);
    }

    #[rstest]
    #[case(SparseAnyPostFieldWithViewContext::Id, "id")]
    #[case(SparseAnyPostFieldWithViewContext::PostType, "type")]
    fn test_as_mapped_field_name_for_view_context(
        #[case] field: SparseAnyPostFieldWithViewContext,
        #[case] expected_mapped_field_name: &str,
    ) {
        assert_eq!(field.as_mapped_field_name(), expected_mapped_field_name);
    }

    fn expected_query_pairs_for_post_list_params_with_all_fields() -> String {
        let after = unit_test_example_date_as_query_value("after");
        let modified_after = unit_test_example_date_as_query_value("modified_after");
        let before = unit_test_example_date_as_query_value("before");
        let modified_before = unit_test_example_date_as_query_value("modified_before");
        format!(
            "page=2&per_page=2&search=foo&{after}&{modified_after}&author=1%2C2&author_exclude=1%2C2&{before}&{modified_before}&exclude=1%2C2&include=1%2C2&offset=2&order=asc&orderby=author&search_columns=post_content%2Cpost_excerpt%2Cpost_title&slug=foo%2Cbar&status=draft%2Cfuture%2Cpending%2Cprivate%2Cpublish%2Cfoo&tax_relation=AND&categories=1%2C2&categories_exclude=1%2C2&tags=1%2C2&tags_exclude=1%2C2&sticky=true&parent=1&parent_exclude=1%2C2&menu_order=1"
        )
    }

    fn post_list_params_with_all_fields() -> PostListParams {
        PostListParams {
            after: unit_test_example_date_as_option(),
            author: vec![UserId(1), UserId(2)],
            author_exclude: vec![UserId(1), UserId(2)],
            before: unit_test_example_date_as_option(),
            categories: vec![TermId(1), TermId(2)],
            categories_exclude: vec![TermId(1), TermId(2)],
            exclude: vec![PostId(1), PostId(2)],
            include: vec![PostId(1), PostId(2)],
            modified_after: unit_test_example_date_as_option(),
            modified_before: unit_test_example_date_as_option(),
            offset: Some(2),
            order: Some(WpApiParamOrder::Asc),
            orderby: Some(WpApiParamPostsOrderBy::Author),
            page: Some(2),
            per_page: Some(2),
            search: Some("foo".to_string()),
            search_columns: vec![
                WpApiParamPostsSearchColumn::PostContent,
                WpApiParamPostsSearchColumn::PostExcerpt,
                WpApiParamPostsSearchColumn::PostTitle,
            ],
            slug: vec!["foo".to_string(), "bar".to_string()],
            status: vec![
                PostStatus::Draft,
                PostStatus::Future,
                PostStatus::Pending,
                PostStatus::Private,
                PostStatus::Publish,
                PostStatus::Custom("foo".to_string()),
            ],
            sticky: Some(true),
            tags: vec![TermId(1), TermId(2)],
            tags_exclude: vec![TermId(1), TermId(2)],
            tax_relation: Some(WpApiParamPostsTaxRelation::And),
            parent: Some(PostId(1)),
            parent_exclude: vec![PostId(1), PostId(2)],
            menu_order: Some(1),
        }
    }
}

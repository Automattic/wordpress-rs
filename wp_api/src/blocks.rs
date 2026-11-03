use crate::{
    WpApiParamOrder,
    date::{WpDateString, WpGmtDateTime},
    impl_as_query_value_from_to_string,
    url_query::{
        AppendUrlQueryPairs, FromUrlQueryPairs, QueryPairs, QueryPairsExtension, UrlQueryPairsMap,
    },
    wp_content_i64_id,
};
use serde::{Deserialize, Serialize};
use wp_contextual::WpContextual;
use wp_derive::WpDeriveParamsField;

wp_content_i64_id!(BlockId);

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
pub enum BlockStatus {
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

impl_as_query_value_from_to_string!(BlockStatus);

#[derive(Debug, Serialize, Deserialize, WpContextual)]
pub struct SparseBlock {
    #[WpContext(edit, embed, view)]
    pub date: Option<WpDateString>,
    #[WpContext(edit, view)]
    pub date_gmt: Option<WpGmtDateTime>,
    #[WpContext(edit, view)]
    #[WpContextualField]
    pub guid: Option<SparseBlockGuid>,
    #[WpContext(edit, embed, view)]
    pub id: Option<BlockId>,
    #[WpContext(edit, embed, view)]
    pub link: Option<String>,
    #[WpContext(edit, view)]
    pub modified: Option<WpDateString>,
    #[WpContext(edit, view)]
    pub modified_gmt: Option<WpGmtDateTime>,
    #[WpContext(edit, embed, view)]
    pub slug: Option<String>,
    #[WpContext(edit, view)]
    pub status: Option<BlockStatus>,
    #[WpContext(edit, embed, view)]
    #[serde(rename = "type")]
    pub block_type: Option<String>,
    #[WpContext(edit)]
    #[WpContextualOption]
    pub password: Option<String>,
    #[WpContext(edit, embed, view)]
    pub title: Option<SparseBlockTitle>,
    #[WpContext(edit, view)]
    pub content: Option<SparseBlockContent>,
    #[WpContext(edit, view)]
    #[WpContextualOption]
    pub template: Option<String>,
    #[WpContext(edit, view)]
    #[WpContextualOption]
    pub wp_pattern_sync_status: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, WpContextual, uniffi::Record)]
pub struct SparseBlockGuid {
    #[WpContext(edit)]
    #[WpContextualOption]
    pub raw: Option<String>,
    #[WpContext(edit, view)]
    pub rendered: Option<String>,
}

#[derive(Debug, PartialEq, Serialize, wp_derive::WpDeserialize, uniffi::Record)]
pub struct SparseBlockTitle {
    pub raw: Option<String>,
}

#[derive(Debug, PartialEq, Serialize, wp_derive::WpDeserialize, uniffi::Record)]
pub struct SparseBlockContent {
    pub raw: Option<String>,
    pub protected: Option<bool>,
    pub block_version: Option<u32>,
}

#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(true)]
pub struct BlockListParams {
    /// Current page of the collection.
    #[uniffi(default = None)]
    pub page: Option<u32>,
    /// Maximum number of items to be returned in result set.
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
    /// Limit response to posts published before a given ISO8601 compliant date.
    #[uniffi(default = None)]
    pub before: Option<WpGmtDateTime>,
    /// Limit response to posts modified before a given ISO8601 compliant date.
    #[uniffi(default = None)]
    pub modified_before: Option<WpGmtDateTime>,
    /// Ensure result set excludes specific IDs.
    #[uniffi(default = [])]
    pub exclude: Vec<BlockId>,
    /// Limit result set to specific IDs.
    #[uniffi(default = [])]
    pub include: Vec<BlockId>,
    /// Offset the result set by a specific number of items.
    #[uniffi(default = None)]
    pub offset: Option<u32>,
    /// Order sort attribute ascending or descending.
    #[uniffi(default = None)]
    pub order: Option<WpApiParamOrder>,
    /// Sort collection by post attribute.
    #[uniffi(default = None)]
    #[field_name("orderby")]
    pub order_by: Option<WpApiParamBlocksOrderBy>,
    /// Array of column names to be searched.
    #[uniffi(default = [])]
    pub search_columns: Vec<String>,
    /// Limit result set to posts with one or more specific slugs.
    #[uniffi(default = [])]
    pub slug: Vec<String>,
    /// Limit result set to posts assigned one or more statuses.
    #[uniffi(default = [])]
    pub status: Vec<BlockStatus>,
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
pub enum WpApiParamBlocksOrderBy {
    Author,
    #[default]
    Date,
    Id,
    Include,
    Modified,
    Parent,
    Relevance,
    Slug,
    IncludeSlugs,
    Title,
}

impl_as_query_value_from_to_string!(WpApiParamBlocksOrderBy);

#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(false)]
pub struct BlockRetrieveParams {
    /// The password for the post if it is password protected.
    #[uniffi(default = None)]
    pub password: Option<String>,
}

#[derive(Debug, Default, Serialize, uniffi::Record)]
pub struct BlockCreateParams {
    /// The date the post was published, in the site's timezone.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<WpDateString>,
    /// The date the post was published, as GMT.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_gmt: Option<WpGmtDateTime>,
    /// An alphanumeric identifier for the post unique to its type.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    /// A named status for the post.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<BlockStatus>,
    /// A password to protect access to the content and excerpt.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    /// The title for the post.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The content for the post.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// The theme file to use to display the post.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
}

#[derive(Debug, Default, Serialize, uniffi::Record)]
pub struct BlockUpdateParams {
    /// The date the post was published, in the site's timezone.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<WpDateString>,
    /// The date the post was published, as GMT.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_gmt: Option<WpGmtDateTime>,
    /// An alphanumeric identifier for the post unique to its type.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    /// A named status for the post.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<BlockStatus>,
    /// A password to protect access to the content and excerpt.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    /// The title for the post.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The content for the post.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// The theme file to use to display the post.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct BlockDeleteResponse {
    pub deleted: bool,
    pub previous: BlockWithEditContext,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SparseField;
    use rstest::*;

    #[rstest]
    #[case(SparseBlockFieldWithEditContext::Id, "id")]
    #[case(SparseBlockFieldWithEditContext::BlockType, "type")]
    fn test_as_mapped_field_name_for_edit_context(
        #[case] field: SparseBlockFieldWithEditContext,
        #[case] expected_mapped_field_name: &str,
    ) {
        assert_eq!(field.as_mapped_field_name(), expected_mapped_field_name);
    }

    #[rstest]
    #[case(SparseBlockFieldWithEmbedContext::Id, "id")]
    #[case(SparseBlockFieldWithEmbedContext::BlockType, "type")]
    fn test_as_mapped_field_name_for_embed_context(
        #[case] field: SparseBlockFieldWithEmbedContext,
        #[case] expected_mapped_field_name: &str,
    ) {
        assert_eq!(field.as_mapped_field_name(), expected_mapped_field_name);
    }

    #[rstest]
    #[case(SparseBlockFieldWithViewContext::Id, "id")]
    #[case(SparseBlockFieldWithViewContext::BlockType, "type")]
    fn test_as_mapped_field_name_for_view_context(
        #[case] field: SparseBlockFieldWithViewContext,
        #[case] expected_mapped_field_name: &str,
    ) {
        assert_eq!(field.as_mapped_field_name(), expected_mapped_field_name);
    }
}

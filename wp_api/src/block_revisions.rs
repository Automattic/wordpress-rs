use crate::{
    UserId, WpApiParamOrder,
    blocks::BlockId,
    date::WpGmtDateTime,
    impl_as_query_value_from_to_string,
    url_query::{
        AppendUrlQueryPairs, FromUrlQueryPairs, QueryPairs, QueryPairsExtension, UrlQueryPairsMap,
    },
    wp_content_i64_id,
};
use serde::{Deserialize, Serialize};
use wp_contextual::WpContextual;
use wp_derive::WpDeriveParamsField;

wp_content_i64_id!(BlockRevisionId);

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
pub enum WpApiParamBlockRevisionsOrderBy {
    #[default]
    Date,
    Id,
    Include,
    IncludeSlugs,
    Relevance,
    Slug,
    Title,
}

impl_as_query_value_from_to_string!(WpApiParamBlockRevisionsOrderBy);

#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(true)]
pub struct BlockRevisionListParams {
    /// Current page of the collection.
    #[uniffi(default = None)]
    pub page: Option<u32>,
    /// Maximum number of items to be returned in result set.
    #[uniffi(default = None)]
    pub per_page: Option<u32>,
    /// Limit results to those matching a string.
    #[uniffi(default = None)]
    pub search: Option<String>,
    /// Ensure result set excludes specific IDs.
    #[uniffi(default = [])]
    pub exclude: Vec<BlockRevisionId>,
    /// Limit result set to specific IDs.
    #[uniffi(default = [])]
    pub include: Vec<BlockRevisionId>,
    /// Offset the result set by a specific number of items.
    #[uniffi(default = None)]
    pub offset: Option<u32>,
    /// Order sort attribute ascending or descending.
    #[uniffi(default = None)]
    pub order: Option<WpApiParamOrder>,
    /// Sort collection by object attribute.
    #[uniffi(default = None)]
    #[field_name("orderby")]
    pub orderby: Option<WpApiParamBlockRevisionsOrderBy>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparseBlockRevision {
    #[WpContext(edit, embed, view)]
    pub id: Option<BlockRevisionId>,
    #[WpContext(edit, embed, view)]
    pub author: Option<UserId>,
    #[WpContext(edit, embed, view)]
    pub date: Option<String>,
    #[WpContext(edit, view)]
    pub date_gmt: Option<WpGmtDateTime>,
    #[WpContext(edit, view)]
    pub modified: Option<String>,
    #[WpContext(edit, view)]
    pub modified_gmt: Option<WpGmtDateTime>,
    #[WpContext(edit, embed, view)]
    pub parent: Option<BlockId>,
    #[WpContext(edit, embed, view)]
    pub slug: Option<String>,
    #[WpContext(edit, view)]
    #[WpContextualField]
    pub guid: Option<crate::blocks::SparseBlockGuid>,
    #[WpContext(edit, embed, view)]
    #[WpContextualField]
    pub title: Option<SparseBlockRevisionTitle>,
    #[WpContext(edit, view)]
    #[WpContextualField]
    pub content: Option<SparseBlockRevisionContent>,
    // _meta field omitted
}

#[derive(Debug, PartialEq, Serialize, Deserialize, WpContextual, uniffi::Record)]
pub struct SparseBlockRevisionTitle {
    #[WpContext(edit)]
    #[WpContextualOption]
    pub raw: Option<String>,
    #[WpContext(edit, embed, view)]
    pub rendered: Option<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, WpContextual, uniffi::Record)]
pub struct SparseBlockRevisionContent {
    #[WpContext(edit)]
    #[WpContextualOption]
    pub raw: Option<String>,
    #[WpContext(edit, view)]
    pub rendered: Option<String>,
    #[WpContext(edit, view)]
    #[WpContextualOption]
    pub protected: Option<bool>,
    #[WpContext(edit, view)]
    #[WpContextualOption]
    pub block_version: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct BlockRevisionDeleteResponse {
    pub deleted: bool,
    pub previous: BlockRevisionWithEditContext,
}

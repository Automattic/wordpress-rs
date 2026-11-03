use crate::{
    UserId, WpApiParamOrder,
    date::WpDateString,
    impl_as_query_value_from_to_string,
    posts::PostId,
    templates::{
        SparseTemplateContentWrapper, SparseTemplateTitleWrapper, TemplateId, TemplateStatus,
    },
    url_query::{
        AppendUrlQueryPairs, FromUrlQueryPairs, QueryPairs, QueryPairsExtension, UrlQueryPairsMap,
    },
    wp_content_i64_id,
};
use serde::{Deserialize, Serialize};
use wp_contextual::WpContextual;
use wp_derive::WpDeriveParamsField;

wp_content_i64_id!(TemplateRevisionId);

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
pub enum WpApiParamTemplateRevisionsOrderBy {
    #[default]
    Date,
    Id,
    Include,
    IncludeSlugs,
    Relevance,
    Slug,
    Title,
}

impl_as_query_value_from_to_string!(WpApiParamTemplateRevisionsOrderBy);

#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(true)]
pub struct TemplateRevisionListParams {
    /// Current page of the collection.
    /// Default: `1`
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
    pub exclude: Vec<TemplateRevisionId>,
    /// Limit result set to specific IDs.
    #[uniffi(default = [])]
    pub include: Vec<TemplateRevisionId>,
    /// Offset the result set by a specific number of items.
    #[uniffi(default = None)]
    pub offset: Option<u32>,
    /// Order sort attribute ascending or descending.
    /// Default: desc
    /// One of: asc, desc
    #[uniffi(default = None)]
    pub order: Option<WpApiParamOrder>,
    /// Sort collection by object attribute.
    /// Default: date
    /// One of: date, id, include, relevance, slug, include_slugs, title
    #[uniffi(default = None)]
    #[field_name("orderby")]
    pub orderby: Option<WpApiParamTemplateRevisionsOrderBy>,
}

// The template revision response uses the parent template controller's schema
// plus a `parent` field. Fields match `SparseTemplate` with revision additions.
#[derive(Debug, Serialize, Deserialize, WpContextual)]
pub struct SparseTemplateRevision {
    #[WpContext(edit, embed, view)]
    pub id: Option<TemplateId>,
    #[WpContext(edit, embed, view)]
    pub slug: Option<String>,
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
    pub theme: Option<String>,
    #[serde(rename = "type")]
    #[WpContext(edit, embed, view)]
    pub template_type: Option<String>,
    #[WpContext(edit, embed, view)]
    pub source: Option<String>,
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
    pub origin: Option<String>,
    #[WpContext(edit, embed, view)]
    pub content: Option<SparseTemplateContentWrapper>,
    #[WpContext(edit, embed, view)]
    pub title: Option<SparseTemplateTitleWrapper>,
    #[WpContext(edit, embed, view)]
    pub description: Option<String>,
    #[WpContext(edit, embed, view)]
    pub status: Option<TemplateStatus>,
    #[serde(rename = "wp_id")]
    #[WpContext(edit, embed, view)]
    pub post_id: Option<PostId>,
    #[WpContext(edit, embed, view)]
    pub has_theme_file: Option<bool>,
    #[WpContext(edit, embed, view)]
    pub author: Option<UserId>,
    #[WpContext(edit, view)]
    #[WpContextualOption]
    #[serde(
        default,
        deserialize_with = "crate::date::deserialize_optional_date_string"
    )]
    pub modified: Option<WpDateString>,
    #[WpContext(edit, view, embed)]
    pub is_custom: Option<bool>,
    #[WpContext(edit, view, embed)]
    pub author_text: Option<String>,
    #[WpContext(edit, view, embed)]
    pub original_source: Option<String>,
    // Revision-specific: the wp_id of the parent template post
    #[WpContext(edit, embed, view)]
    pub parent: Option<PostId>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct TemplateRevisionDeleteResponse {
    pub deleted: bool,
    pub previous: TemplateRevisionWithEditContext,
}

use crate::{
    UserId, WpApiParamOrder,
    date::{WpDateString, WpGmtDateTime},
    impl_as_query_value_from_to_string,
    navigations::NavigationId,
    url_query::{
        AppendUrlQueryPairs, FromUrlQueryPairs, QueryPairs, QueryPairsExtension, UrlQueryPairsMap,
    },
    wp_content_i64_id,
};
use serde::{Deserialize, Serialize};
use wp_contextual::WpContextual;
use wp_derive::WpDeriveParamsField;

wp_content_i64_id!(NavigationRevisionId);

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
pub enum WpApiParamNavigationRevisionsOrderBy {
    #[default]
    Date,
    Id,
    Include,
    IncludeSlugs,
    Relevance,
    Slug,
    Title,
}

impl_as_query_value_from_to_string!(WpApiParamNavigationRevisionsOrderBy);

#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(true)]
pub struct NavigationRevisionListParams {
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
    pub exclude: Vec<NavigationRevisionId>,
    /// Limit result set to specific IDs.
    #[uniffi(default = [])]
    pub include: Vec<NavigationRevisionId>,
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
    pub orderby: Option<WpApiParamNavigationRevisionsOrderBy>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparseNavigationRevision {
    #[WpContext(edit, embed, view)]
    pub id: Option<NavigationRevisionId>,
    #[WpContext(edit, embed, view)]
    pub author: Option<UserId>,
    #[WpContext(edit, embed, view)]
    pub date: Option<WpDateString>,
    #[WpContext(edit, view)]
    pub date_gmt: Option<WpGmtDateTime>,
    #[WpContext(edit, view)]
    pub modified: Option<WpDateString>,
    #[WpContext(edit, view)]
    pub modified_gmt: Option<WpGmtDateTime>,
    #[WpContext(edit, embed, view)]
    pub parent: Option<NavigationId>,
    #[WpContext(edit, embed, view)]
    pub slug: Option<String>,
    #[WpContext(edit, view)]
    #[WpContextualField]
    pub guid: Option<crate::navigations::SparseNavigationGuid>,
    #[WpContext(edit, embed, view)]
    #[WpContextualField]
    pub title: Option<crate::navigations::SparseNavigationTitle>,
    #[WpContext(edit, view)]
    #[WpContextualField]
    pub content: Option<crate::navigations::SparseNavigationContent>,
    // meta field omitted for now
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct NavigationRevisionDeleteResponse {
    pub deleted: bool,
    pub previous: NavigationRevisionWithEditContext,
}

use crate::{
    UserId, WpApiParamOrder,
    date::WpGmtDateTime,
    impl_as_query_value_from_to_string,
    posts::PostId,
    url_query::{
        AppendUrlQueryPairs, FromUrlQueryPairs, QueryPairs, QueryPairsExtension, UrlQueryPairsMap,
    },
    wp_content_i64_id,
};
use serde::{Deserialize, Serialize};
use wp_contextual::WpContextual;
use wp_derive::WpDeriveParamsField;

wp_content_i64_id!(PostRevisionId);

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
pub enum WpApiParamPostRevisionsOrderBy {
    #[default]
    Date,
    Id,
    Include,
    IncludeSlugs,
    Relevance,
    Slug,
    Title,
}

impl_as_query_value_from_to_string!(WpApiParamPostRevisionsOrderBy);

#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(true)]
pub struct AnyPostRevisionListParams {
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
    pub exclude: Vec<PostRevisionId>,
    /// Limit result set to specific IDs.
    #[uniffi(default = [])]
    pub include: Vec<PostRevisionId>,
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
    pub orderby: Option<WpApiParamPostRevisionsOrderBy>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparseAnyPostRevision {
    #[WpContext(edit, embed, view)]
    pub id: Option<PostRevisionId>,
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
    pub parent: Option<PostId>,
    #[WpContext(edit, embed, view)]
    pub slug: Option<String>,
    #[WpContext(edit, view)]
    #[WpContextualField]
    pub guid: Option<crate::posts::SparsePostGuid>,
    #[WpContext(edit, embed, view)]
    #[WpContextualField]
    pub title: Option<crate::posts::SparsePostTitle>,
    #[WpContext(edit, view)]
    #[WpContextualField]
    pub content: Option<crate::posts::SparsePostContent>,
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
    pub excerpt: Option<crate::posts::SparsePostExcerpt>,
    #[WpContext(edit, view)]
    pub meta: Option<crate::posts::PostMeta>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct AnyPostRevisionDeleteResponse {
    pub deleted: bool,
    pub previous: AnyPostRevisionWithEditContext,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{generate, unit_test_common::assert_expected_and_from_query_pairs};
    use rstest::*;

    #[rstest]
    #[case(AnyPostRevisionListParams::default(), "")]
    #[case(generate!(AnyPostRevisionListParams, (page, Some(2))), "page=2")]
    #[case(generate!(AnyPostRevisionListParams, (per_page, Some(2))), "per_page=2")]
    #[case(generate!(AnyPostRevisionListParams, (search, Some("foo".to_string()))), "search=foo")]
    #[case(generate!(AnyPostRevisionListParams, (exclude, vec![PostRevisionId(1), PostRevisionId(2)])), "exclude=1%2C2")]
    #[case(generate!(AnyPostRevisionListParams, (include, vec![PostRevisionId(1), PostRevisionId(2)])), "include=1%2C2")]
    #[case(generate!(AnyPostRevisionListParams, (offset, Some(2))), "offset=2")]
    #[case(generate!(AnyPostRevisionListParams, (order, Some(WpApiParamOrder::Asc))), "order=asc")]
    #[case(generate!(AnyPostRevisionListParams, (order, Some(WpApiParamOrder::Desc))), "order=desc")]
    #[case(generate!(AnyPostRevisionListParams, (orderby, Some(WpApiParamPostRevisionsOrderBy::Date))), "orderby=date")]
    #[case(generate!(AnyPostRevisionListParams, (orderby, Some(WpApiParamPostRevisionsOrderBy::Id))), "orderby=id")]
    #[case(generate!(AnyPostRevisionListParams, (orderby, Some(WpApiParamPostRevisionsOrderBy::Include))), "orderby=include")]
    #[case(generate!(AnyPostRevisionListParams, (orderby, Some(WpApiParamPostRevisionsOrderBy::IncludeSlugs))), "orderby=include_slugs")]
    #[case(generate!(AnyPostRevisionListParams, (orderby, Some(WpApiParamPostRevisionsOrderBy::Relevance))), "orderby=relevance")]
    #[case(generate!(AnyPostRevisionListParams, (orderby, Some(WpApiParamPostRevisionsOrderBy::Slug))), "orderby=slug")]
    #[case(generate!(AnyPostRevisionListParams, (orderby, Some(WpApiParamPostRevisionsOrderBy::Title))), "orderby=title")]
    #[case(
        post_revision_list_params_with_all_fields(),
        &expected_query_pairs_for_post_revision_list_params_with_all_fields()
    )]
    #[trace]
    fn test_post_list_query_pairs(
        #[case] params: AnyPostRevisionListParams,
        #[case] expected_query: &str,
    ) {
        assert_expected_and_from_query_pairs(params, expected_query);
    }

    fn expected_query_pairs_for_post_revision_list_params_with_all_fields() -> String {
        "page=2&per_page=2&search=foo&exclude=1%2C2&include=1%2C2&offset=2&order=asc&orderby=id"
            .to_string()
    }

    fn post_revision_list_params_with_all_fields() -> AnyPostRevisionListParams {
        AnyPostRevisionListParams {
            page: Some(2),
            per_page: Some(2),
            search: Some("foo".to_string()),
            exclude: vec![PostRevisionId(1), PostRevisionId(2)],
            include: vec![PostRevisionId(1), PostRevisionId(2)],
            offset: Some(2),
            order: Some(WpApiParamOrder::Asc),
            orderby: Some(WpApiParamPostRevisionsOrderBy::Id),
        }
    }
}

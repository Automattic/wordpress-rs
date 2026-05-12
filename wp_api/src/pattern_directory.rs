use crate::{
    WpApiParamOrder, impl_as_query_value_from_to_string,
    url_query::{
        AppendUrlQueryPairs, FromUrlQueryPairs, QueryPairs, QueryPairsExtension, UrlQueryPairsMap,
    },
    wp_content_u64_id,
};

wp_content_u64_id!(PatternDirectoryItemId);
wp_content_u64_id!(PatternDirectoryCategoryId);
wp_content_u64_id!(PatternDirectoryKeywordId);
use serde::{Deserialize, Serialize};
use wp_contextual::WpContextual;
use wp_derive::WpDeriveParamsField;

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
pub enum WpApiParamPatternDirectoryOrderBy {
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
    FavoriteCount,
}

impl_as_query_value_from_to_string!(WpApiParamPatternDirectoryOrderBy);

#[derive(Debug, Serialize, Deserialize, WpContextual)]
pub struct SparsePatternDirectoryItem {
    #[WpContext(edit, embed, view)]
    pub id: Option<PatternDirectoryItemId>,
    #[WpContext(edit, embed, view)]
    pub title: Option<String>,
    #[WpContext(edit, embed, view)]
    pub content: Option<String>,
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
    pub categories: Option<Vec<String>>,
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
    pub keywords: Option<Vec<String>>,
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
    pub description: Option<String>,
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
    pub viewport_width: Option<u64>,
    #[WpContext(embed, view)]
    #[WpContextualOption]
    pub block_types: Option<Vec<String>>,
}

#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(true)]
pub struct PatternDirectoryListParams {
    /// Current page of the collection.
    /// Default: `1`
    #[uniffi(default = None)]
    pub page: Option<u32>,
    /// Maximum number of items to be returned in result set.
    /// Default: `100`
    #[uniffi(default = None)]
    pub per_page: Option<u32>,
    /// Limit results to those matching a string.
    #[uniffi(default = None)]
    pub search: Option<String>,
    /// Limit results to those matching a category ID.
    #[uniffi(default = None)]
    pub category: Option<PatternDirectoryCategoryId>,
    /// Limit results to those matching a keyword ID.
    #[uniffi(default = None)]
    pub keyword: Option<PatternDirectoryKeywordId>,
    /// Limit results to those matching a pattern slug.
    #[uniffi(default = [])]
    pub slug: Vec<String>,
    /// Offset the result set by a specific number of items.
    #[uniffi(default = None)]
    pub offset: Option<u32>,
    /// Order sort attribute ascending or descending.
    /// Default: `desc`
    /// One of: `asc`, `desc`
    #[uniffi(default = None)]
    pub order: Option<WpApiParamOrder>,
    /// Sort collection by pattern attribute.
    /// Default: `date`
    /// One of: `author`, `date`, `id`, `include`, `modified`, `parent`, `relevance`, `slug`, `include_slugs`, `title`, `favorite_count`
    #[uniffi(default = None)]
    pub orderby: Option<WpApiParamPatternDirectoryOrderBy>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SparseField, generate, unit_test_common::assert_expected_and_from_query_pairs};
    use rstest::*;

    #[rstest]
    #[case(PatternDirectoryListParams::default(), "")]
    #[case(generate!(PatternDirectoryListParams, (page, Some(2))), "page=2")]
    #[case(generate!(PatternDirectoryListParams, (per_page, Some(50))), "per_page=50")]
    #[case(generate!(PatternDirectoryListParams, (search, Some("hero".to_string()))), "search=hero")]
    #[case(generate!(PatternDirectoryListParams, (category, Some(PatternDirectoryCategoryId(5)))), "category=5")]
    #[case(generate!(PatternDirectoryListParams, (keyword, Some(PatternDirectoryKeywordId(3)))), "keyword=3")]
    #[case(generate!(PatternDirectoryListParams, (slug, vec!["my-pattern".to_string()])), "slug=my-pattern")]
    #[case(generate!(PatternDirectoryListParams, (slug, vec!["pattern-a".to_string(), "pattern-b".to_string()])), "slug=pattern-a%2Cpattern-b")]
    #[case(generate!(PatternDirectoryListParams, (offset, Some(10))), "offset=10")]
    #[case(generate!(PatternDirectoryListParams, (order, Some(WpApiParamOrder::Asc))), "order=asc")]
    #[case(generate!(PatternDirectoryListParams, (order, Some(WpApiParamOrder::Desc))), "order=desc")]
    #[case(generate!(PatternDirectoryListParams, (orderby, Some(WpApiParamPatternDirectoryOrderBy::Date))), "orderby=date")]
    #[case(generate!(PatternDirectoryListParams, (orderby, Some(WpApiParamPatternDirectoryOrderBy::Title))), "orderby=title")]
    #[case(generate!(PatternDirectoryListParams, (orderby, Some(WpApiParamPatternDirectoryOrderBy::FavoriteCount))), "orderby=favorite_count")]
    #[case(generate!(PatternDirectoryListParams, (orderby, Some(WpApiParamPatternDirectoryOrderBy::Author))), "orderby=author")]
    #[case(generate!(PatternDirectoryListParams, (orderby, Some(WpApiParamPatternDirectoryOrderBy::Id))), "orderby=id")]
    #[case(generate!(PatternDirectoryListParams, (orderby, Some(WpApiParamPatternDirectoryOrderBy::Include))), "orderby=include")]
    #[case(generate!(PatternDirectoryListParams, (orderby, Some(WpApiParamPatternDirectoryOrderBy::Modified))), "orderby=modified")]
    #[case(generate!(PatternDirectoryListParams, (orderby, Some(WpApiParamPatternDirectoryOrderBy::Parent))), "orderby=parent")]
    #[case(generate!(PatternDirectoryListParams, (orderby, Some(WpApiParamPatternDirectoryOrderBy::Relevance))), "orderby=relevance")]
    #[case(generate!(PatternDirectoryListParams, (orderby, Some(WpApiParamPatternDirectoryOrderBy::Slug))), "orderby=slug")]
    #[case(generate!(PatternDirectoryListParams, (orderby, Some(WpApiParamPatternDirectoryOrderBy::IncludeSlugs))), "orderby=include_slugs")]
    #[case(PatternDirectoryListParams {
            page: Some(2),
            per_page: Some(10),
            search: Some("hero".to_string()),
            category: Some(PatternDirectoryCategoryId(5)),
            keyword: Some(PatternDirectoryKeywordId(3)),
            slug: vec!["my-pattern".to_string()],
            offset: Some(10),
            order: Some(WpApiParamOrder::Asc),
            orderby: Some(WpApiParamPatternDirectoryOrderBy::Title),
        },
        "page=2&per_page=10&search=hero&category=5&keyword=3&slug=my-pattern&offset=10&order=asc&orderby=title"
    )]
    #[trace]
    fn test_pattern_directory_list_query_pairs(
        #[case] params: PatternDirectoryListParams,
        #[case] expected_query: &str,
    ) {
        assert_expected_and_from_query_pairs(params, expected_query);
    }

    #[rstest]
    #[case(SparsePatternDirectoryItemFieldWithEditContext::Id, "id")]
    #[case(SparsePatternDirectoryItemFieldWithEditContext::Title, "title")]
    #[case(SparsePatternDirectoryItemFieldWithEditContext::Content, "content")]
    #[case(
        SparsePatternDirectoryItemFieldWithEditContext::Categories,
        "categories"
    )]
    #[case(SparsePatternDirectoryItemFieldWithEditContext::Keywords, "keywords")]
    #[case(
        SparsePatternDirectoryItemFieldWithEditContext::Description,
        "description"
    )]
    #[case(
        SparsePatternDirectoryItemFieldWithEditContext::ViewportWidth,
        "viewport_width"
    )]
    fn test_sparse_field_edit_context(
        #[case] field: SparsePatternDirectoryItemFieldWithEditContext,
        #[case] expected: &str,
    ) {
        assert_eq!(field.as_mapped_field_name(), expected);
    }

    #[rstest]
    #[case(
        SparsePatternDirectoryItemFieldWithViewContext::BlockTypes,
        "block_types"
    )]
    fn test_sparse_field_view_context_includes_block_types(
        #[case] field: SparsePatternDirectoryItemFieldWithViewContext,
        #[case] expected: &str,
    ) {
        assert_eq!(field.as_mapped_field_name(), expected);
    }
}

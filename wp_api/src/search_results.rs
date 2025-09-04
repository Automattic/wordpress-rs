use crate::{
    IntegerOrString, SparseField, impl_as_query_value_from_to_string,
    url_query::{
        AppendUrlQueryPairs, FromUrlQueryPairs, QueryPairs, QueryPairsExtension, UrlQueryPairsMap,
    },
};
use serde::{Deserialize, Serialize};
use wp_contextual::WpContextual;
use wp_derive::WpDeriveParamsField;

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
pub enum SearchResultType {
    #[default]
    #[serde(rename = "post")]
    #[strum(serialize = "post")]
    Post,
    #[serde(rename = "term")]
    #[strum(serialize = "term")]
    Term,
    #[serde(rename = "post-format")]
    #[strum(serialize = "post-format")]
    PostFormat,
    #[serde(untagged)]
    #[strum(default)]
    Custom(String),
}

impl_as_query_value_from_to_string!(SearchResultType);

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
pub enum SearchResultSubtype {
    Post,
    Page,
    Category,
    PostTag,
    #[serde(untagged)]
    #[strum(default)]
    Custom(String),
}

impl_as_query_value_from_to_string!(SearchResultSubtype);

#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(true)]
pub struct SearchListParams {
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
    /// Limit results to items of an object type.
    /// Default: `post`
    /// One of: `post`, `term`, `post-format`
    #[uniffi(default = None)]
    #[field_name("type")]
    pub object_type: Option<SearchResultType>,
    /// Limit results to items of one or more object subtypes.
    /// Default: `any`
    #[uniffi(default = None)]
    #[field_name("subtype")]
    pub object_subtype: Option<SearchResultSubtype>,
    /// Ensure result set excludes specific IDs.
    #[uniffi(default = [])]
    pub exclude: Vec<i64>,
    /// Limit result set to specific IDs.
    #[uniffi(default = [])]
    pub include: Vec<i64>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparseSearchResult {
    #[WpContext(edit, embed, view)]
    pub id: Option<IntegerOrString>,
    #[WpContext(edit, embed, view)]
    pub title: Option<String>,
    #[WpContext(edit, embed, view)]
    pub url: Option<String>,
    #[serde(rename = "type")]
    #[WpContext(edit, embed, view)]
    pub object_type: Option<SearchResultType>,
    #[serde(rename = "subtype")]
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
    pub object_subtype: Option<SearchResultSubtype>,
}

impl SparseField for SparseSearchResultFieldWithEmbedContext {
    fn as_str(&self) -> &str {
        match self {
            Self::ObjectType => "type",
            Self::ObjectSubtype => "subtype",
            _ => self.as_field_name(),
        }
    }
}

impl SparseField for SparseSearchResultFieldWithViewContext {
    fn as_str(&self) -> &str {
        match self {
            Self::ObjectType => "type",
            Self::ObjectSubtype => "subtype",
            _ => self.as_field_name(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[rstest]
    #[case(SearchResultType::Post)]
    #[case(SearchResultType::Term)]
    #[case(SearchResultType::PostFormat)]
    #[case(SearchResultType::Custom("foo_bar".to_string()))]
    #[case(SearchResultType::Custom("foo-bar".to_string()))]
    fn test_search_result_type_string_conversion(#[case] value: SearchResultType) {
        assert_eq!(value, value.to_string().parse().unwrap());
    }

    #[rstest]
    #[case(SparseSearchResultFieldWithEmbedContext::Id, "id")]
    #[case(SparseSearchResultFieldWithEmbedContext::ObjectType, "type")]
    #[case(SparseSearchResultFieldWithEmbedContext::ObjectSubtype, "subtype")]
    fn test_as_mapped_field_name_for_embed_context(
        #[case] field: SparseSearchResultFieldWithEmbedContext,
        #[case] expected_mapped_field_name: &str,
    ) {
        assert_eq!(field.as_str(), expected_mapped_field_name);
    }

    #[rstest]
    #[case(SparseSearchResultFieldWithViewContext::Id, "id")]
    #[case(SparseSearchResultFieldWithViewContext::ObjectType, "type")]
    #[case(SparseSearchResultFieldWithViewContext::ObjectSubtype, "subtype")]
    fn test_as_mapped_field_name_for_view_context(
        #[case] field: SparseSearchResultFieldWithViewContext,
        #[case] expected_mapped_field_name: &str,
    ) {
        assert_eq!(field.as_str(), expected_mapped_field_name);
    }
}

use crate::{
    impl_as_query_value_from_as_str,
    url_query::{
        AppendUrlQueryPairs, FromUrlQueryPairs, QueryPairs, QueryPairsExtension, UrlQueryPairsMap,
    },
    AsQueryValue, EnumFromStrParsingError, IntegerOrString,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use strum_macros::IntoStaticStr;
use wp_contextual::WpContextual;

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
pub enum SearchResultType {
    #[default]
    #[serde(rename = "post")]
    Post,
    #[serde(rename = "term")]
    Term,
    #[serde(rename = "post-format")]
    PostFormat,
    #[serde(untagged)]
    Custom(String),
}

impl_as_query_value_from_as_str!(SearchResultType);

impl SearchResultType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Post => "post",
            Self::Term => "term",
            Self::PostFormat => "post-format",
            Self::Custom(result_type) => result_type,
        }
    }
}

impl FromStr for SearchResultType {
    type Err = EnumFromStrParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "post" => Ok(Self::Post),
            "term" => Ok(Self::Term),
            "post-format" => Ok(Self::PostFormat),
            value => Ok(Self::Custom(value.to_string())),
        }
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, uniffi::Enum,
)]
pub enum SearchResultSubtype {
    #[serde(rename = "post")]
    Post,
    #[serde(rename = "page")]
    Page,
    #[serde(rename = "category")]
    Category,
    #[serde(rename = "post_tag")]
    PostTag,
    #[serde(untagged)]
    Custom(String),
}

impl_as_query_value_from_as_str!(SearchResultSubtype);

impl SearchResultSubtype {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Post => "post",
            Self::Page => "page",
            Self::Category => "category",
            Self::PostTag => "post_tag",
            Self::Custom(result_type) => result_type,
        }
    }
}

impl FromStr for SearchResultSubtype {
    type Err = EnumFromStrParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "post" => Ok(Self::Post),
            "page" => Ok(Self::Page),
            "category" => Ok(Self::Category),
            "post_tag" => Ok(Self::PostTag),
            value => Ok(Self::Custom(value.to_string())),
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq, uniffi::Record)]
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
    pub object_type: Option<SearchResultType>,
    /// Limit results to items of one or more object subtypes.
    /// Default: `any`
    #[uniffi(default = None)]
    pub object_subtype: Option<SearchResultSubtype>,
    /// Ensure result set excludes specific IDs.
    #[uniffi(default = [])]
    pub exclude: Vec<i64>,
    /// Limit result set to specific IDs.
    #[uniffi(default = [])]
    pub include: Vec<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, IntoStaticStr)]
enum SearchListParamsField {
    #[strum(serialize = "page")]
    Page,
    #[strum(serialize = "per_page")]
    PerPage,
    #[strum(serialize = "search")]
    Search,
    #[strum(serialize = "type")]
    ObjectType,
    #[strum(serialize = "subtype")]
    ObjectSubtype,
    #[strum(serialize = "exclude")]
    Exclude,
    #[strum(serialize = "include")]
    Include,
}

impl AppendUrlQueryPairs for SearchListParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut
            .append_option_query_value_pair(SearchListParamsField::Page, self.page.as_ref())
            .append_option_query_value_pair(SearchListParamsField::PerPage, self.per_page.as_ref())
            .append_option_query_value_pair(SearchListParamsField::Search, self.search.as_ref())
            .append_option_query_value_pair(
                SearchListParamsField::ObjectType,
                self.object_type.as_ref(),
            )
            .append_option_query_value_pair(
                SearchListParamsField::ObjectSubtype,
                self.object_subtype.as_ref(),
            )
            .append_vec_query_value_pair(SearchListParamsField::Exclude, &self.exclude)
            .append_vec_query_value_pair(SearchListParamsField::Include, &self.include);
    }
}

impl FromUrlQueryPairs for SearchListParams {
    fn from_url_query_pairs(query_pairs: UrlQueryPairsMap) -> Option<Self> {
        Some(Self {
            page: query_pairs.get(SearchListParamsField::Page),
            per_page: query_pairs.get(SearchListParamsField::PerPage),
            search: query_pairs.get(SearchListParamsField::Search),
            object_type: query_pairs.get(SearchListParamsField::ObjectType),
            object_subtype: query_pairs.get(SearchListParamsField::ObjectSubtype),
            exclude: query_pairs.get_csv(SearchListParamsField::Exclude),
            include: query_pairs.get_csv(SearchListParamsField::Include),
        })
    }

    fn supports_pagination() -> bool {
        true
    }
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

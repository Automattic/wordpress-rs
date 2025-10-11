use crate::{
    WpApiParamOrder, impl_as_query_value_from_to_string,
    posts::PostId,
    taxonomies::TaxonomyType,
    url_query::{
        AppendUrlQueryPairs, FromUrlQueryPairs, QueryPairs, QueryPairsExtension, UrlQueryPairsMap,
    },
    wp_content_i64_id,
};
use serde::{Deserialize, Serialize};
use wp_contextual::WpContextual;
use wp_derive::WpDeriveParamsField;

wp_content_i64_id!(TermId);

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
pub enum WpApiParamTermsOrderBy {
    Id,
    Include,
    #[default]
    Name,
    Slug,
    IncludeSlugs,
    TermGroup,
    Description,
    Count,
}

impl_as_query_value_from_to_string!(WpApiParamTermsOrderBy);

#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(true)]
pub struct TermListParams {
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
    /// Ensure result set excludes specific IDs.
    #[uniffi(default = [])]
    pub exclude: Vec<TermId>,
    /// Limit result set to specific IDs.
    #[uniffi(default = [])]
    pub include: Vec<TermId>,
    /// Offset the result set by a specific number of items.
    #[uniffi(default = None)]
    pub offset: Option<u32>,
    /// Order sort attribute ascending or descending.
    /// Default: `asc`
    /// One of: `asc`, `desc`
    #[uniffi(default = None)]
    pub order: Option<WpApiParamOrder>,
    /// Sort collection by term attribute.
    /// Default: `name`
    /// One of: `id`, `include`, `name`, `slug`, `include_slugs`, `term_group`, `description`, `count`
    #[uniffi(default = None)]
    #[field_name("orderby")]
    pub orderby: Option<WpApiParamTermsOrderBy>,
    /// Whether to hide terms not assigned to any posts.
    #[uniffi(default = None)]
    pub hide_empty: Option<bool>,
    /// Limit result set to terms assigned to a specific parent.
    /// Category-specific fields
    #[uniffi(default = None)]
    pub parent: Option<TermId>,
    /// Limit result set to terms assigned to a specific post.
    #[uniffi(default = None)]
    pub post: Option<PostId>,
    /// Limit result set to users with one or more specific slugs.
    #[uniffi(default = [])]
    pub slug: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct TermDeleteResponse {
    pub deleted: bool,
    pub previous: AnyTermWithEditContext,
}

#[derive(Debug, Serialize, uniffi::Record)]
pub struct TermCreateParams {
    /// HTML title for the term.
    pub name: String,
    /// HTML description of the term.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// An alphanumeric identifier for the term unique to its type.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    /// The parent term ID.
    /// Category-specific fields
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<TermId>,
    // meta field is omitted for now: https://github.com/Automattic/wordpress-rs/issues/470
}

#[derive(Debug, Default, Serialize, uniffi::Record)]
pub struct TermUpdateParams {
    /// HTML title for the term.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// HTML description of the term.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// An alphanumeric identifier for the term unique to its type.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    /// The parent term ID.
    /// Category-specific field.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<TermId>,
    // meta field is omitted for now: https://github.com/Automattic/wordpress-rs/issues/470
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparseAnyTerm {
    #[WpContext(edit, embed, view)]
    pub id: Option<TermId>,
    #[WpContext(edit, view)]
    pub count: Option<i64>,
    #[WpContext(edit, view)]
    pub description: Option<String>,
    #[WpContext(edit, embed, view)]
    pub link: Option<String>,
    #[WpContext(edit, embed, view)]
    pub name: Option<String>,
    #[WpContext(edit, embed, view)]
    pub slug: Option<String>,
    #[WpContext(edit, embed, view)]
    pub taxonomy: Option<TaxonomyType>,
    // Category-specific fields
    #[WpContextualOption]
    #[WpContext(edit, view)]
    pub parent: Option<TermId>,
    // meta field is omitted for now: https://github.com/Automattic/wordpress-rs/issues/470
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use url::Url;

    #[rstest]
    #[case(WpApiParamTermsOrderBy::Id, "id")]
    #[case(WpApiParamTermsOrderBy::Include, "include")]
    #[case(WpApiParamTermsOrderBy::Name, "name")]
    #[case(WpApiParamTermsOrderBy::Slug, "slug")]
    #[case(WpApiParamTermsOrderBy::IncludeSlugs, "include_slugs")]
    #[case(WpApiParamTermsOrderBy::TermGroup, "term_group")]
    #[case(WpApiParamTermsOrderBy::Description, "description")]
    #[case(WpApiParamTermsOrderBy::Count, "count")]
    fn test_orderby_url_query(#[case] orderby: WpApiParamTermsOrderBy, #[case] expected: &str) {
        let mut url = Url::parse("https://example.com").unwrap();
        url.query_pairs_mut()
            .append_query_value_pair("orderby", &orderby);
        assert_eq!(
            url.query().map(|x| x.to_string()),
            Some(format!("orderby={expected}"))
        );
    }

    #[rstest]
    #[case(WpApiParamTermsOrderBy::Id)]
    #[case(WpApiParamTermsOrderBy::Include)]
    #[case(WpApiParamTermsOrderBy::Name)]
    #[case(WpApiParamTermsOrderBy::Slug)]
    #[case(WpApiParamTermsOrderBy::IncludeSlugs)]
    #[case(WpApiParamTermsOrderBy::TermGroup)]
    #[case(WpApiParamTermsOrderBy::Description)]
    #[case(WpApiParamTermsOrderBy::Count)]
    fn test_orderby_string_conversion(#[case] orderby: WpApiParamTermsOrderBy) {
        assert_eq!(orderby, orderby.to_string().parse().unwrap());
    }
}

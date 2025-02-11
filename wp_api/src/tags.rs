use crate::{
    impl_as_query_value_for_new_type, impl_as_query_value_from_to_string,
    posts::PostId,
    taxonomies::TaxonomyType,
    url_query::{
        AppendUrlQueryPairs, FromUrlQueryPairs, QueryPairs, QueryPairsExtension, UrlQueryPairsMap,
    },
    WpApiParamOrder,
};
use serde::{Deserialize, Serialize};
use std::{num::ParseIntError, str::FromStr};
use strum_macros::IntoStaticStr;
use wp_contextual::WpContextual;

impl_as_query_value_for_new_type!(TagId);
uniffi::custom_newtype!(TagId, i64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagId(pub i64);

impl FromStr for TagId {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self)
    }
}

impl std::fmt::Display for TagId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

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
pub enum WpApiParamTagsOrderBy {
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

impl_as_query_value_from_to_string!(WpApiParamTagsOrderBy);

#[derive(Debug, Default, PartialEq, Eq, uniffi::Record)]
pub struct TagListParams {
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
    pub exclude: Vec<TagId>,
    /// Limit result set to specific IDs.
    #[uniffi(default = [])]
    pub include: Vec<TagId>,
    /// Offset the result set by a specific number of items.
    #[uniffi(default = None)]
    pub offset: Option<u32>,
    /// Order sort attribute ascending or descending.
    /// Default: `asc`
    /// One of: `asc`, `desc`
    #[uniffi(default = None)]
    pub order: Option<WpApiParamOrder>,
    /// Sort collection by user attribute.
    /// Default: `name`
    /// One of: `id`, `include`, `name`, `slug`, `include_slugs`, `term_group`, `description`, `count`
    #[uniffi(default = None)]
    pub orderby: Option<WpApiParamTagsOrderBy>,
    /// Whether to hide terms not assigned to any posts.
    #[uniffi(default = None)]
    pub hide_empty: Option<bool>,
    /// Limit result set to terms assigned to a specific post.
    #[uniffi(default = None)]
    pub post: Option<PostId>,
    /// Limit result set to users with one or more specific slugs.
    #[uniffi(default = [])]
    pub slug: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, IntoStaticStr)]
enum TagListParamsField {
    #[strum(serialize = "page")]
    Page,
    #[strum(serialize = "per_page")]
    PerPage,
    #[strum(serialize = "search")]
    Search,
    #[strum(serialize = "exclude")]
    Exclude,
    #[strum(serialize = "include")]
    Include,
    #[strum(serialize = "offset")]
    Offset,
    #[strum(serialize = "order")]
    Order,
    #[strum(serialize = "orderby")]
    Orderby,
    #[strum(serialize = "hide_empty")]
    HideEmpty,
    #[strum(serialize = "post")]
    Post,
    #[strum(serialize = "slug")]
    Slug,
}

impl AppendUrlQueryPairs for TagListParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut
            .append_option_query_value_pair(TagListParamsField::Page, self.page.as_ref())
            .append_option_query_value_pair(TagListParamsField::PerPage, self.per_page.as_ref())
            .append_option_query_value_pair(TagListParamsField::Search, self.search.as_ref())
            .append_vec_query_value_pair(TagListParamsField::Exclude, &self.exclude)
            .append_vec_query_value_pair(TagListParamsField::Include, &self.include)
            .append_option_query_value_pair(TagListParamsField::Offset, self.offset.as_ref())
            .append_option_query_value_pair(TagListParamsField::Order, self.order.as_ref())
            .append_option_query_value_pair(TagListParamsField::Orderby, self.orderby.as_ref())
            .append_option_query_value_pair(TagListParamsField::HideEmpty, self.hide_empty.as_ref())
            .append_option_query_value_pair(TagListParamsField::Post, self.post.as_ref())
            .append_vec_query_value_pair(TagListParamsField::Slug, &self.slug);
    }
}

impl FromUrlQueryPairs for TagListParams {
    fn from_url_query_pairs(query_pairs: UrlQueryPairsMap) -> Option<Self> {
        Some(Self {
            page: query_pairs.get(TagListParamsField::Page),
            per_page: query_pairs.get(TagListParamsField::PerPage),
            search: query_pairs.get(TagListParamsField::Search),
            exclude: query_pairs.get_csv(TagListParamsField::Exclude),
            include: query_pairs.get_csv(TagListParamsField::Include),
            offset: query_pairs.get(TagListParamsField::Offset),
            order: query_pairs.get(TagListParamsField::Order),
            orderby: query_pairs.get(TagListParamsField::Orderby),
            hide_empty: query_pairs.get(TagListParamsField::HideEmpty),
            post: query_pairs.get(TagListParamsField::Post),
            slug: query_pairs.get_csv(TagListParamsField::Slug),
        })
    }

    fn supports_pagination() -> bool {
        true
    }
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct TagDeleteResponse {
    pub deleted: bool,
    pub previous: TagWithEditContext,
}

#[derive(Debug, Serialize, uniffi::Record)]
pub struct TagCreateParams {
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
    // meta field is omitted for now: https://github.com/Automattic/wordpress-rs/issues/463
}

#[derive(Debug, Default, Serialize, uniffi::Record)]
pub struct TagUpdateParams {
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
    // meta field is omitted for now: https://github.com/Automattic/wordpress-rs/issues/463
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparseTag {
    #[WpContext(edit, embed, view)]
    pub id: Option<TagId>,
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
    // meta field is omitted for now: https://github.com/Automattic/wordpress-rs/issues/463
}

use crate::{
    WpApiParamOrder, impl_as_query_value_from_to_string,
    posts::PostId,
    url_query::{
        AppendUrlQueryPairs, FromUrlQueryPairs, QueryPairs, QueryPairsExtension, UrlQueryPairsMap,
    },
    wp_content_i64_id,
};
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
pub enum WpApiParamNavMenusOrderBy {
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

impl_as_query_value_from_to_string!(WpApiParamNavMenusOrderBy);

#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(true)]
pub struct NavMenuListParams {
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
    pub exclude: Vec<NavMenuId>,
    /// Limit result set to specific IDs.
    #[uniffi(default = [])]
    pub include: Vec<NavMenuId>,
    /// Offset the result set by a specific number of items.
    #[uniffi(default = None)]
    pub offset: Option<u32>,
    /// Order sort attribute ascending or descending.
    /// Default: asc
    /// One of: asc, desc
    #[uniffi(default = None)]
    pub order: Option<WpApiParamOrder>,
    /// Sort collection by term attribute.
    /// Default: name
    /// One of: id, include, name, slug, include_slugs, term_group, description, count
    #[uniffi(default = None)]
    #[field_name("orderby")]
    pub orderby: Option<WpApiParamNavMenusOrderBy>,
    /// Whether to hide terms not assigned to any posts.
    #[uniffi(default = None)]
    pub hide_empty: Option<bool>,
    /// Limit result set to terms assigned to a specific post.
    /// This param must be a valid nav_menu_item ID.
    #[uniffi(default = None)]
    pub post: Option<PostId>,
    /// Limit result set to terms with one or more specific slugs.
    #[uniffi(default = [])]
    pub slug: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct NavMenuDeleteResponse {
    pub deleted: bool,
    pub previous: NavMenuWithEditContext,
}

#[derive(Debug, Default, Serialize, uniffi::Record)]
pub struct NavMenuCreateParams {
    /// HTML description of the term.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// HTML title for the term.
    /// Required
    pub name: String,
    /// An alphanumeric identifier for the term unique to its type.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    /// The locations assigned to the menu.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locations: Option<Vec<String>>,
    /// Whether to automatically add top level pages to this menu.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_add: Option<bool>,
    // meta field is omitted for now: https://github.com/Automattic/wordpress-rs/issues/925
}

#[derive(Debug, Default, Serialize, uniffi::Record)]
pub struct NavMenuUpdateParams {
    /// HTML description of the term.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// HTML title for the term.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The locations assigned to the menu.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locations: Option<Vec<String>>,
    /// Whether to automatically add top level pages to this menu.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_add: Option<bool>,
    // meta field is omitted for now: https://github.com/Automattic/wordpress-rs/issues/925
    //
    // Note that even though the documentation states `slug` is updatable, it's not:
    // https://github.com/WordPress/WordPress/blob/b27e369cb25445784bc014d6fa731558beaf320d/wp-includes/nav-menu.php#L325
}

wp_content_i64_id!(NavMenuId);

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparseNavMenu {
    /// Unique identifier for the term.
    #[WpContext(edit, embed, view)]
    pub id: Option<NavMenuId>,
    /// HTML description of the term.
    #[WpContext(edit, view)]
    pub description: Option<String>,
    /// HTML title for the term.
    #[WpContext(edit, embed, view)]
    pub name: Option<String>,
    /// An alphanumeric identifier for the term unique to its type.
    #[WpContext(edit, embed, view)]
    pub slug: Option<String>,
    /// The locations assigned to the menu.
    #[WpContext(edit, view)]
    pub locations: Option<Vec<String>>,
    /// Whether to automatically add top level pages to this menu.
    #[WpContext(edit, view)]
    pub auto_add: Option<bool>,
    // meta field is omitted for now: https://github.com/Automattic/wordpress-rs/issues/925
}

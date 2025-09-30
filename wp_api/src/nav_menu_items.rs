use crate::{
    WpApiParamOrder,
    date::WpGmtDateTime,
    impl_as_query_value_from_to_string,
    nav_menus::NavMenuId,
    posts::{WpApiParamPostsOrderBy, WpApiParamPostsSearchColumn, WpApiParamPostsTaxRelation},
    url_query::{
        AppendUrlQueryPairs, FromUrlQueryPairs, QueryPairs, QueryPairsExtension, UrlQueryPairsMap,
    },
    wp_content_i64_id,
};
use serde::{Deserialize, Serialize};
use wp_contextual::WpContextual;
use wp_derive::WpDeriveParamsField;

wp_content_i64_id!(NavMenuItemId);

/// Status of a nav menu item.
///
/// WordPress only supports two statuses for nav menu items: `publish` and `draft`.
/// Any other status values will be coerced to one of these by WordPress core.
///
/// See:
/// https://github.com/WordPress/WordPress/blob/b27e369cb25445784bc014d6fa731558beaf320d/wp-includes/nav-menu.php#L548
/// https://github.com/WordPress/WordPress/blob/b27e369cb25445784bc014d6fa731558beaf320d/wp-includes/nav-menu.php#L612
#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    uniffi::Enum,
    strum_macros::EnumString,
    strum_macros::Display,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum NavMenuItemStatus {
    #[default]
    Publish,
    Draft,
}

impl_as_query_value_from_to_string!(NavMenuItemStatus);

#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(true)]
pub struct NavMenuItemListParams {
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
    /// Limit response to posts published after a given ISO8601 compliant date.
    #[uniffi(default = None)]
    pub after: Option<WpGmtDateTime>,
    /// Limit response to posts modified after a given ISO8601 compliant date.
    #[uniffi(default = None)]
    pub modified_after: Option<WpGmtDateTime>,
    /// Limit response to posts published before a given ISO8601 compliant date.
    #[uniffi(default = None)]
    pub before: Option<WpGmtDateTime>,
    /// Limit response to posts modified before a given ISO8601 compliant date.
    #[uniffi(default = None)]
    pub modified_before: Option<WpGmtDateTime>,
    /// Ensure result set excludes specific IDs.
    #[uniffi(default = [])]
    pub exclude: Vec<NavMenuItemId>,
    /// Limit result set to specific IDs.
    #[uniffi(default = [])]
    pub include: Vec<NavMenuItemId>,
    /// Offset the result set by a specific number of items.
    #[uniffi(default = None)]
    pub offset: Option<u32>,
    /// Order sort attribute ascending or descending.
    /// Default: asc
    /// One of: asc, desc
    #[uniffi(default = None)]
    pub order: Option<WpApiParamOrder>,
    /// Sort collection by object attribute.
    /// Default: menu_order
    /// One of: author, date, id, include, modified, parent, relevance, slug, include_slugs, title, menu_order
    #[uniffi(default = None)]
    #[field_name("orderby")]
    pub orderby: Option<WpApiParamPostsOrderBy>,
    /// Array of column names to be searched.
    #[uniffi(default = [])]
    pub search_columns: Vec<WpApiParamPostsSearchColumn>,
    /// Limit result set to posts with one or more specific slugs.
    #[uniffi(default = [])]
    pub slug: Vec<String>,
    /// Limit result set to posts assigned one or more statuses.
    /// Default: publish
    #[uniffi(default = [])]
    pub status: Vec<NavMenuItemStatus>,
    /// Limit result set based on relationship between multiple taxonomies.
    /// One of: AND, OR
    #[uniffi(default = None)]
    pub tax_relation: Option<WpApiParamPostsTaxRelation>,
    /// Limit result set to items with specific terms assigned in the menus taxonomy.
    #[uniffi(default = [])]
    pub menus: Vec<NavMenuId>,
    /// Limit result set to items except those with specific terms assigned in the menus taxonomy.
    #[uniffi(default = [])]
    pub menus_exclude: Vec<NavMenuId>,
    /// Limit result set to posts with a specific menu_order value.
    #[uniffi(default = None)]
    pub menu_order: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct NavMenuItemDeleteResponse {
    pub deleted: bool,
    pub previous: NavMenuItemWithEditContext,
}

#[derive(Debug, Default, Serialize, uniffi::Record)]
pub struct NavMenuItemCreateParams {
    /// The title for the object.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The family of objects originally represented, such as "post_type" or "taxonomy".
    /// Default: custom
    /// One of: taxonomy, post_type, post_type_archive, custom
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub nav_menu_item_type: Option<NavMenuItemType>,
    /// A named status for the object.
    /// Default: publish
    /// One of: publish, draft
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<NavMenuItemStatus>,
    /// The ID for the parent of the object.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<NavMenuItemId>,
    /// Text for the title attribute of the link element for this menu item.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attr_title: Option<String>,
    /// Class names for the link element of this menu item.
    #[uniffi(default = [])]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub classes: Vec<String>,
    /// The description of this menu item.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The DB ID of the nav_menu_item that is this item's menu parent, if any, otherwise 0.
    /// Default: 1
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub menu_order: Option<i64>,
    /// The type of object originally represented, such as "category", "post", or "attachment".
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object: Option<String>,
    /// The database ID of the original object this menu item represents, for example the ID for posts or the term_id for categories.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_id: Option<i64>,
    /// The target attribute of the link element for this menu item.
    /// One of: _blank, (empty string)
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// The URL to which this menu item points.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// The XFN relationship expressed in the link of this menu item.
    #[uniffi(default = [])]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub xfn: Vec<String>,
    /// The terms assigned to the object in the nav_menu taxonomy.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub menus: Option<NavMenuId>,
    // meta field is omitted for now: https://github.com/Automattic/wordpress-rs/issues/931
}

#[derive(Debug, Default, Serialize, uniffi::Record)]
pub struct NavMenuItemUpdateParams {
    /// The title for the object.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The family of objects originally represented, such as "post_type" or "taxonomy".
    /// One of: taxonomy, post_type, post_type_archive, custom
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub nav_menu_item_type: Option<NavMenuItemType>,
    /// A named status for the object.
    /// One of: publish, draft
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<NavMenuItemStatus>,
    /// The ID for the parent of the object.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<NavMenuItemId>,
    /// Text for the title attribute of the link element for this menu item.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attr_title: Option<String>,
    /// Class names for the link element of this menu item.
    #[uniffi(default = [])]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub classes: Vec<String>,
    /// The description of this menu item.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The DB ID of the nav_menu_item that is this item's menu parent, if any, otherwise 0.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub menu_order: Option<i64>,
    /// The type of object originally represented, such as "category", "post", or "attachment".
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object: Option<String>,
    /// The database ID of the original object this menu item represents, for example the ID for posts or the term_id for categories.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_id: Option<i64>,
    /// The target attribute of the link element for this menu item.
    /// One of: _blank, (empty string)
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// The URL to which this menu item points.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// The XFN relationship expressed in the link of this menu item.
    #[uniffi(default = [])]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub xfn: Vec<String>,
    /// The terms assigned to the object in the nav_menu taxonomy.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub menus: Option<NavMenuId>,
    // meta field is omitted for now: https://github.com/Automattic/wordpress-rs/issues/931
}

#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    uniffi::Enum,
    strum_macros::EnumString,
    strum_macros::Display,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum NavMenuItemType {
    Taxonomy,
    PostType,
    PostTypeArchive,
    #[default]
    Custom,
}

impl_as_query_value_from_to_string!(NavMenuItemType);

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparseNavMenuItem {
    #[WpContext(edit, embed, view)]
    #[WpContextualField]
    pub title: Option<SparseNavMenuItemTitle>,
    #[WpContext(edit, embed, view)]
    pub id: Option<NavMenuItemId>,
    #[WpContext(edit, embed, view)]
    pub type_label: Option<String>,
    #[serde(rename = "type")]
    #[WpContext(edit, embed, view)]
    pub item_type: Option<NavMenuItemType>,
    #[WpContext(edit, embed, view)]
    pub status: Option<NavMenuItemStatus>,
    #[WpContext(edit, embed, view)]
    pub parent: Option<NavMenuItemId>,
    #[WpContext(edit, embed, view)]
    pub attr_title: Option<String>,
    #[WpContext(edit, embed, view)]
    pub classes: Option<Vec<String>>,
    #[WpContext(edit, embed, view)]
    pub description: Option<String>,
    #[WpContext(edit, embed, view)]
    pub menu_order: Option<i64>,
    #[WpContext(edit, embed, view)]
    pub object: Option<String>,
    #[WpContext(edit, embed, view)]
    pub object_id: Option<i64>,
    #[WpContext(edit, embed, view)]
    pub target: Option<String>,
    #[WpContext(edit, embed, view)]
    pub url: Option<String>,
    #[WpContext(edit, embed, view)]
    pub xfn: Option<Vec<String>>,
    #[WpContext(edit, embed, view)]
    pub invalid: Option<bool>,
    #[WpContext(edit, view)]
    #[WpContextualOption]
    pub menus: Option<NavMenuId>,
    // meta field is omitted for now: https://github.com/Automattic/wordpress-rs/issues/931
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparseNavMenuItemTitle {
    #[WpContext(edit)]
    #[WpContextualOption]
    pub raw: Option<String>,
    #[WpContext(edit, embed, view)]
    pub rendered: Option<String>,
}

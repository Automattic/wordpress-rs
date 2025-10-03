use crate::{
    UserId,
    date::WpGmtDateTime,
    nav_menu_items::{NavMenuItemId, NavMenuItemStatus, NavMenuItemType},
    nav_menus::NavMenuId,
    wp_content_i64_id,
};
use serde::{Deserialize, Serialize};
use wp_contextual::WpContextual;

wp_content_i64_id!(NavMenuItemRevisionId);

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparseNavMenuItemRevision {
    #[WpContext(edit, embed, view)]
    pub author: Option<UserId>,
    #[WpContext(edit, embed, view)]
    pub date: Option<String>,
    #[WpContext(edit, view)]
    pub date_gmt: Option<WpGmtDateTime>,
    #[WpContext(edit, view)]
    #[WpContextualField]
    pub guid: Option<crate::posts::SparsePostGuid>,
    #[WpContext(edit, embed, view)]
    pub id: Option<NavMenuItemRevisionId>,
    #[WpContext(edit, view)]
    pub modified: Option<String>,
    #[WpContext(edit, view)]
    pub modified_gmt: Option<WpGmtDateTime>,
    #[WpContext(edit, embed, view)]
    pub parent: Option<NavMenuItemId>,
    #[WpContext(edit, embed, view)]
    pub slug: Option<String>,
    #[WpContext(edit, embed, view)]
    #[WpContextualField]
    pub title: Option<crate::posts::SparsePostTitle>,
    #[WpContext(edit)]
    pub preview_link: Option<String>,
    // meta field is omitted for now: https://github.com/Automattic/wordpress-rs/issues/931
}

#[derive(Debug, Default, Serialize, uniffi::Record)]
pub struct NavMenuItemRevisionCreateParams {
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

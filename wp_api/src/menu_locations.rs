use crate::{nav_menus::NavMenuId, wp_content_string_id};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wp_contextual::WpContextual;

wp_content_string_id!(MenuLocation);

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
#[serde(transparent)]
pub struct SparseMenuLocationsResponse {
    #[WpContext(edit, embed, view)]
    #[WpContextualField]
    pub locations: Option<HashMap<MenuLocation, SparseMenuLocation>>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparseMenuLocation {
    #[WpContext(edit, embed, view)]
    pub name: Option<String>,
    #[WpContext(edit, embed, view)]
    pub description: Option<String>,
    #[WpContext(edit, embed, view)]
    pub menu: Option<NavMenuId>,
}

use crate::{widgets::WidgetId, wp_content_string_id};
use serde::{Deserialize, Serialize};
use wp_contextual::WpContextual;

wp_content_string_id!(SidebarId);

#[derive(
    Debug,
    Clone,
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
pub enum SidebarStatus {
    Active,
    Inactive,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparseSidebar {
    #[WpContext(edit, embed, view)]
    pub id: Option<SidebarId>,
    #[WpContext(edit, embed, view)]
    pub name: Option<String>,
    #[WpContext(edit, embed, view)]
    pub description: Option<String>,
    #[WpContext(edit, embed, view)]
    pub class: Option<String>,
    #[WpContext(edit, embed, view)]
    pub before_widget: Option<String>,
    #[WpContext(edit, embed, view)]
    pub after_widget: Option<String>,
    #[WpContext(edit, embed, view)]
    pub before_title: Option<String>,
    #[WpContext(edit, embed, view)]
    pub after_title: Option<String>,
    #[WpContext(edit, embed, view)]
    pub status: Option<SidebarStatus>,
    #[WpContext(edit, embed, view)]
    pub widgets: Option<Vec<WidgetId>>,
}

#[derive(Debug, Default, Serialize, uniffi::Record)]
pub struct SidebarUpdateParams {
    pub widgets: Vec<WidgetId>,
}

use serde::{Deserialize, Serialize};
use wp_derive::WPContextual;

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WPContextual)]
pub struct SparsePlugin {
    #[WPContext(edit, embed, view)]
    pub plugin: Option<String>,
    #[WPContext(edit, embed, view)]
    pub status: Option<PluginStatus>,
    #[WPContext(edit, embed, view)]
    pub name: Option<String>,
    #[WPContext(edit, view)]
    // TODO: Custom URI type?
    pub plugin_uri: Option<String>,
    #[WPContext(edit, view)]
    pub author: Option<PluginAuthor>,
    #[WPContext(edit, view)]
    pub description: Option<PluginDescription>,
    #[WPContext(edit, view)]
    pub version: Option<String>,
    #[WPContext(edit, embed, view)]
    pub network_only: Option<bool>,
    #[WPContext(edit, embed, view)]
    pub requires_php: Option<String>,
    #[WPContext(edit, view)]
    pub textdomain: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, uniffi::Enum)]
pub enum PluginStatus {
    Active,
    Inactive,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct PluginAuthor {
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct PluginDescription {
    pub name: Option<String>,
}

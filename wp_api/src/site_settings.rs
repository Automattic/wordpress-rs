use serde::{Deserialize, Serialize};
use wp_contextual::WpContextual;

#[derive(Debug, Default, Serialize, uniffi::Record)]
pub struct SiteSettingsUpdateParams {
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_format: Option<String>,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_format: Option<String>,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_of_week: Option<u64>,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_smilies: Option<bool>,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_category: Option<u64>,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_post_format: Option<String>,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub posts_per_page: Option<u64>,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_on_front: Option<String>,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_on_front: Option<u64>,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_for_posts: Option<u64>,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_ping_status: Option<SiteSettingsPingStatus>,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_comment_status: Option<SiteSettingsCommentStatus>,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site_logo: Option<u64>,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site_icon: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparseSiteSettings {
    #[WpContext(edit, embed, view)]
    pub title: Option<String>,
    #[WpContext(edit, embed, view)]
    pub description: Option<String>,
    #[WpContext(edit, embed, view)]
    pub url: Option<String>,
    #[WpContext(edit, embed, view)]
    pub email: Option<String>,
    #[WpContext(edit, embed, view)]
    pub timezone: Option<String>,
    #[WpContext(edit, embed, view)]
    pub date_format: Option<String>,
    #[WpContext(edit, embed, view)]
    pub time_format: Option<String>,
    #[WpContext(edit, embed, view)]
    pub start_of_week: Option<u64>,
    #[WpContext(edit, embed, view)]
    pub language: Option<String>,
    #[WpContext(edit, embed, view)]
    pub use_smilies: Option<bool>,
    #[WpContext(edit, embed, view)]
    pub default_category: Option<u64>,
    #[WpContext(edit, embed, view)]
    pub default_post_format: Option<String>,
    #[WpContext(edit, embed, view)]
    pub posts_per_page: Option<u64>,
    #[WpContext(edit, embed, view)]
    pub show_on_front: Option<String>,
    #[WpContext(edit, embed, view)]
    pub page_on_front: Option<u64>,
    #[WpContext(edit, embed, view)]
    pub page_for_posts: Option<u64>,
    #[WpContext(edit, embed, view)]
    pub default_ping_status: Option<SiteSettingsPingStatus>,
    #[WpContext(edit, embed, view)]
    pub default_comment_status: Option<SiteSettingsCommentStatus>,
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
    pub site_logo: Option<u64>,
    #[WpContext(edit, embed, view)]
    pub site_icon: Option<u64>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, uniffi::Enum,
)]
#[serde(rename_all = "snake_case")]
pub enum SiteSettingsPingStatus {
    Open,
    Closed,
    #[serde(untagged)]
    Custom(String),
}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, uniffi::Enum,
)]
#[serde(rename_all = "snake_case")]
pub enum SiteSettingsCommentStatus {
    Open,
    Closed,
    #[serde(untagged)]
    Custom(String),
}

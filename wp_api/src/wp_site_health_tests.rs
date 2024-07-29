use serde::{Deserialize, Serialize};

use crate::SparseField;

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct SparseWpSiteHealthTest {
    pub actions: Option<String>,
    pub badge: Option<WpSiteHealthTestBadge>,
    pub description: Option<String>,
    pub label: Option<String>,
    pub status: Option<String>,
    pub test: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct WpSiteHealthTest {
    pub actions: String,
    pub badge: WpSiteHealthTestBadge,
    pub description: String,
    pub label: String,
    pub status: String,
    pub test: String,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct WpSiteHealthTestBadge {
    pub color: String,
    pub label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum SparseWpSiteHealthTestField {
    Actions,
    Badge,
    Description,
    Label,
    Status,
    Test,
}

impl SparseField for SparseWpSiteHealthTestField {
    fn as_str(&self) -> &str {
        match self {
            Self::Actions => "actions",
            Self::Badge => "badge",
            Self::Description => "description",
            Self::Label => "label",
            Self::Status => "status",
            Self::Test => "test",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct SparseWpSiteHealthDirectorySizes {
    pub database_size: Option<WpSiteHealthDirectorySizeInfo>,
    pub fonts_size: Option<WpSiteHealthDirectorySizeInfo>,
    pub plugins_size: Option<WpSiteHealthDirectorySizeInfo>,
    pub themes_size: Option<WpSiteHealthDirectorySizeInfo>,
    pub total_size: Option<WpSiteHealthDirectorySizeInfo>,
    pub uploads_size: Option<WpSiteHealthDirectorySizeInfo>,
    pub wordpress_size: Option<WpSiteHealthDirectorySizeInfo>,
    pub raw: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct WpSiteHealthDirectorySizes {
    pub database_size: WpSiteHealthDirectorySizeInfo,
    pub fonts_size: WpSiteHealthDirectorySizeInfo,
    pub plugins_size: WpSiteHealthDirectorySizeInfo,
    pub themes_size: WpSiteHealthDirectorySizeInfo,
    pub total_size: WpSiteHealthDirectorySizeInfo,
    pub uploads_size: WpSiteHealthDirectorySizeInfo,
    pub wordpress_size: WpSiteHealthDirectorySizeInfo,
    pub raw: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct WpSiteHealthDirectorySizeInfo {
    pub debug: String,
    pub size: String,
    // `raw` is missing from `fonts_size` in our local WordPress test site. It's possible that it
    // might be missing from other size types in different WordPress installations. We use
    // `Option<u64>` for all of them to make sure we don't have parsing errors.
    pub raw: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum SparseWpSiteHealthDirectorySizesField {
    DatabaseSize,
    FontsSize,
    PluginsSize,
    ThemesSize,
    TotalSize,
    UploadsSize,
    WordpressSize,
    Raw,
}

impl SparseField for SparseWpSiteHealthDirectorySizesField {
    fn as_str(&self) -> &str {
        match self {
            Self::DatabaseSize => "database_size",
            Self::FontsSize => "fonts_size",
            Self::PluginsSize => "plugins_size",
            Self::ThemesSize => "themes_size",
            Self::TotalSize => "total_size",
            Self::UploadsSize => "uploads_size",
            Self::WordpressSize => "wordpress_size",
            Self::Raw => "raw",
        }
    }
}

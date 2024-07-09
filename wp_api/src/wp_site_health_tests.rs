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

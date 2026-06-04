use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Display};
use wp_contextual::WpContextual;

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
#[serde(transparent)]
pub struct SparsePostStatusesResponse {
    #[WpContext(edit, embed, view)]
    #[WpContextualField]
    pub post_statuses: Option<HashMap<PostStatusSlug, SparsePostStatus>>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparsePostStatus {
    /// The title for the status.
    #[WpContext(edit, embed, view)]
    pub name: Option<String>,
    /// Whether posts with this status should be private.
    #[WpContext(edit)]
    pub private: Option<bool>,
    /// Whether posts with this status should be protected.
    #[WpContext(edit)]
    pub protected: Option<bool>,
    /// Whether posts of this status should be shown in the front end of the site.
    #[WpContext(edit, view)]
    pub public: Option<bool>,
    /// Whether posts with this status should be publicly-queryable.
    #[WpContext(edit, view)]
    pub queryable: Option<bool>,
    /// Whether to include posts in the edit listing for their post type.
    #[WpContext(edit)]
    pub show_in_list: Option<bool>,
    /// An alphanumeric identifier for the status.
    #[WpContext(edit, embed, view)]
    pub slug: Option<String>,
    /// Whether posts of this status may have floating published dates.
    #[WpContext(edit, view)]
    pub date_floating: Option<bool>,
}

#[derive(
    Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, uniffi::Record,
)]
#[uniffi::export(Display)]
#[serde(transparent)]
pub struct PostStatusSlug {
    pub slug: String,
}

impl PostStatusSlug {
    pub fn new(name: String) -> Self {
        Self { slug: name }
    }
}

impl From<&str> for PostStatusSlug {
    fn from(value: &str) -> Self {
        Self {
            slug: value.to_string(),
        }
    }
}

impl Display for PostStatusSlug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.slug)
    }
}

use std::fmt::Display;

use serde::{Deserialize, Serialize};
use wp_contextual::WpContextual;

use crate::{
    impl_as_query_value_from_as_str,
    url_query::{AppendUrlQueryPairs, AsQueryValue, QueryPairs, QueryPairsExtension},
};

#[derive(Debug, Default, uniffi::Record)]
pub struct PluginListParams {
    /// Limit results to those matching a string.
    #[uniffi(default = None)]
    pub search: Option<String>,
    /// Limits results to plugins with the given status.
    #[uniffi(default = None)]
    pub status: Option<PluginStatus>,
}

impl AppendUrlQueryPairs for PluginListParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut
            .append_option_query_value_pair("search", self.search.as_ref())
            .append_option_query_value_pair("status", self.status.as_ref());
    }
}

#[derive(Debug, Serialize, uniffi::Record)]
pub struct PluginCreateParams {
    /// WordPress.org plugin directory slug.
    pub slug: PluginWpOrgDirectorySlug,
    /// The plugin activation status.
    pub status: PluginStatus,
}

#[derive(Debug, Serialize, uniffi::Record)]
pub struct PluginUpdateParams {
    /// The plugin activation status.
    pub status: PluginStatus,
    // According to the documentation: https://developer.wordpress.org/rest-api/reference/plugins/#update-a-plugin
    // There is supposed to be a `context` parameter as well, but this parameter doesn't seem to
    // modify the response fields as promised in the documentation.
    // In order to avoid confusion, this parameter is not included in this implementation.
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparsePlugin {
    #[WpContext(edit, embed, view)]
    pub plugin: Option<PluginSlug>,
    #[WpContext(edit, embed, view)]
    pub status: Option<PluginStatus>,
    #[WpContext(edit, embed, view)]
    pub name: Option<String>,
    #[WpContext(edit, view)]
    pub plugin_uri: Option<String>,
    #[WpContext(edit, view)]
    pub author: Option<String>,
    #[WpContext(edit, view)]
    pub author_uri: Option<String>,
    #[WpContext(edit, view)]
    pub description: Option<PluginDescription>,
    #[WpContext(edit, view)]
    pub version: Option<String>,
    #[WpContext(edit, embed, view)]
    pub network_only: Option<bool>,
    #[WpContext(edit, embed, view)]
    pub requires_wp: Option<String>,
    #[WpContext(edit, embed, view)]
    pub requires_php: Option<String>,
    #[WpContext(edit, view)]
    pub textdomain: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct PluginDeleteResponse {
    pub deleted: bool,
    pub previous: PluginWithEditContext,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, uniffi::Record)]
#[serde(transparent)]
pub struct PluginSlug {
    pub slug: String,
}

impl PluginSlug {
    pub fn new(slug: String) -> Self {
        Self { slug }
    }
}

impl From<&str> for PluginSlug {
    fn from(value: &str) -> Self {
        Self {
            slug: value.to_string(),
        }
    }
}

impl Display for PluginSlug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.slug)
    }
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
#[serde(transparent)]
pub struct PluginWpOrgDirectorySlug {
    pub slug: String,
}

impl From<&str> for PluginWpOrgDirectorySlug {
    fn from(value: &str) -> Self {
        Self {
            slug: value.to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, uniffi::Enum)]
pub enum PluginStatus {
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "inactive")]
    Inactive,
    #[serde(rename = "network-active")]
    NetworkActive,
}

impl_as_query_value_from_as_str!(PluginStatus);

impl PluginStatus {
    fn as_str(&self) -> &str {
        match self {
            Self::Active => "active",
            Self::Inactive => "inactive",
            Self::NetworkActive => "network-active",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct PluginDescription {
    pub raw: String,
    pub rendered: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{generate, unit_test_common::assert_expected_query_pairs};
    use rstest::*;

    #[rstest]
    #[case(PluginListParams::default(), "")]
    #[case(generate!(PluginListParams, (search, Some("foo".to_string()))), "search=foo")]
    #[case(generate!(PluginListParams, (status, Some(PluginStatus::Active))), "status=active")]
    #[case(generate!(PluginListParams, (search, Some("foo".to_string())), (status, Some(PluginStatus::Inactive))), "search=foo&status=inactive")]
    #[trace]
    fn test_plugin_list_params(#[case] params: PluginListParams, #[case] expected_query: &str) {
        assert_expected_query_pairs(params, expected_query);
    }
}

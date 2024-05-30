use serde::{Deserialize, Serialize};
use wp_contextual::WpContextual;

use crate::{add_uniffi_exported_parser, SparseField, WpApiError, WpNetworkResponse};

add_uniffi_exported_parser!(parse_filter_plugins_response, Vec<SparsePlugin>);
add_uniffi_exported_parser!(parse_filter_retrieve_plugin_response, SparsePlugin);
add_uniffi_exported_parser!(
    parse_list_plugins_response_with_edit_context,
    Vec<PluginWithEditContext>
);
add_uniffi_exported_parser!(
    parse_list_plugins_response_with_embed_context,
    Vec<PluginWithEmbedContext>
);
add_uniffi_exported_parser!(
    parse_list_plugins_response_with_view_context,
    Vec<PluginWithViewContext>
);
add_uniffi_exported_parser!(
    parse_retrieve_plugin_response_with_edit_context,
    PluginWithEditContext
);
add_uniffi_exported_parser!(
    parse_retrieve_plugin_response_with_embed_context,
    PluginWithEmbedContext
);
add_uniffi_exported_parser!(
    parse_retrieve_plugin_response_with_view_context,
    PluginWithViewContext
);
add_uniffi_exported_parser!(parse_create_plugin_response, PluginWithEditContext);
add_uniffi_exported_parser!(parse_update_plugin_response, PluginWithEditContext);
add_uniffi_exported_parser!(parse_delete_plugin_response, PluginDeleteResponse);

#[derive(Debug, Default, uniffi::Record)]
pub struct PluginListParams {
    /// Limit results to those matching a string.
    pub search: Option<String>,
    /// Limits results to plugins with the given status.
    pub status: Option<PluginStatus>,
}

impl PluginListParams {
    pub fn query_pairs(&self) -> impl IntoIterator<Item = (&str, String)> {
        [
            ("search", self.search.clone()),
            ("status", self.status.map(|x| x.as_str().to_string())),
        ]
        .into_iter()
        // Remove `None` values
        .filter_map(|(k, opt_v)| opt_v.map(|v| (k, v)))
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum SparsePluginField {
    Author,
    AuthorUri,
    Description,
    Name,
    NetworkOnly,
    Plugin,
    PluginUri,
    RequiresPhp,
    RequiresWp,
    Status,
    Textdomain,
    Version,
}

impl SparseField for SparsePluginField {
    fn as_str(&self) -> &str {
        match self {
            Self::Author => "author",
            Self::AuthorUri => "author_uri",
            Self::Description => "description",
            Self::Name => "name",
            Self::NetworkOnly => "network_only",
            Self::Plugin => "plugin",
            Self::PluginUri => "plugin_uri",
            Self::RequiresPhp => "requires_php",
            Self::RequiresWp => "requires_wp",
            Self::Status => "status",
            Self::Textdomain => "textdomain",
            Self::Version => "version",
        }
    }
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

impl From<&str> for PluginSlug {
    fn from(value: &str) -> Self {
        Self {
            slug: value.to_string(),
        }
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
    #[case(PluginListParams::default(), &[])]
    #[case(generate!(PluginListParams, (search, Some("foo".to_string()))), &[("search", "foo")])]
    #[case(generate!(PluginListParams, (status, Some(PluginStatus::Active))), &[("status", "active")])]
    #[case(generate!(PluginListParams, (search, Some("foo".to_string())), (status, Some(PluginStatus::Inactive))), &[("search", "foo"), ("status", "inactive")])]
    #[trace]
    fn test_plugin_list_params(
        #[case] params: PluginListParams,
        #[case] expected_pairs: &[(&str, &str)],
    ) {
        assert_expected_query_pairs(params.query_pairs(), expected_pairs);
    }
}

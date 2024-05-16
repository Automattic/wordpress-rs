use serde::{Deserialize, Serialize};
use wp_derive::WPContextual;

use crate::{add_uniffi_exported_parser, parse_wp_response, WPApiError, WPNetworkResponse};

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

#[derive(Default, Debug, uniffi::Record)]
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

#[derive(Serialize, Debug, uniffi::Record)]
pub struct PluginCreateParams {
    /// WordPress.org plugin directory slug.
    pub slug: String,
    /// The plugin activation status.
    pub status: PluginStatus,
}

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
    pub author: Option<String>,
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
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "inactive")]
    Inactive,
}

impl PluginStatus {
    fn as_str(&self) -> &str {
        match self {
            Self::Active => "active",
            Self::Inactive => "inactive",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct PluginAuthor {
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct PluginDescription {
    pub raw: String,
    pub rendered: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{generate, test_helpers::assert_expected_query_pairs};
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

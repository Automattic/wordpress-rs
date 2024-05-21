use wp_api::{PluginCreateParams, PluginStatus, WPContext, WPRestErrorCode};

use crate::test_helpers::{
    api_as_subscriber, AssertWpError, WPNetworkRequestExecutor, WPNetworkResponseParser,
    WP_ORG_PLUGIN_SLUG_CLASSIC_WIDGETS,
};

pub mod test_helpers;

#[tokio::test]
async fn list_plugins_err_cannot_view_plugins() {
    api_as_subscriber()
        .list_plugins_request(WPContext::Edit, &None)
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_retrieve_plugin_response_with_edit_context)
        .assert_wp_error(WPRestErrorCode::CannotViewPlugins);
}

#[tokio::test]
async fn create_plugin_err_cannot_install_plugin() {
    api_as_subscriber()
        .create_plugin_request(&PluginCreateParams {
            slug: WP_ORG_PLUGIN_SLUG_CLASSIC_WIDGETS.into(),
            status: PluginStatus::Active,
        })
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_create_plugin_response)
        .assert_wp_error(WPRestErrorCode::CannotInstallPlugin);
}

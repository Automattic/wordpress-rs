use wp_api::{PluginCreateParams, PluginStatus, PluginUpdateParams, WPContext, WPRestErrorCode};

use crate::test_helpers::{
    api, api_as_subscriber, AssertWpError, WPNetworkRequestExecutor, WPNetworkResponseParser,
    HELLO_DOLLY_PLUGIN_SLUG, WP_ORG_PLUGIN_SLUG_CLASSIC_WIDGETS,
};

pub mod test_helpers;

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

#[tokio::test]
async fn delete_plugin_err_cannot_delete_active_plugin() {
    api()
        .delete_plugin_request(&HELLO_DOLLY_PLUGIN_SLUG.into())
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_delete_plugin_response)
        .assert_wp_error(WPRestErrorCode::CannotDeleteActivePlugin);
}

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
async fn update_plugin_err_plugin_not_found() {
    api()
        .update_plugin_request(
            &"foo".into(),
            &PluginUpdateParams {
                status: PluginStatus::Active,
            },
        )
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_update_plugin_response)
        .assert_wp_error(WPRestErrorCode::PluginNotFound);
}

#[tokio::test]
async fn update_plugin_err_cannot_manage_plugins() {
    api_as_subscriber()
        .update_plugin_request(
            &HELLO_DOLLY_PLUGIN_SLUG.into(),
            &PluginUpdateParams {
                status: PluginStatus::Active,
            },
        )
        .execute()
        .await
        .unwrap()
        .parse(wp_api::parse_update_plugin_response)
        .assert_wp_error(WPRestErrorCode::CannotManagePlugins);
}

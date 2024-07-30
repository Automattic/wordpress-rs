use wp_api::plugins::{PluginCreateParams, PluginListParams, PluginStatus, PluginUpdateParams};
use wp_api::WpErrorCode;

use wp_api_integration_tests::{
    api_client, api_client_as_subscriber, AssertWpError, HELLO_DOLLY_PLUGIN_SLUG,
    WP_ORG_PLUGIN_SLUG_CLASSIC_WIDGETS,
};

#[tokio::test]
async fn create_plugin_err_cannot_install_plugin() {
    api_client_as_subscriber()
        .plugins()
        .create(&PluginCreateParams {
            slug: WP_ORG_PLUGIN_SLUG_CLASSIC_WIDGETS.into(),
            status: PluginStatus::Active,
        })
        .await
        .assert_wp_error(WpErrorCode::CannotInstallPlugin);
}

#[tokio::test]
async fn delete_plugin_err_cannot_delete_active_plugin() {
    api_client()
        .plugins()
        .delete(&HELLO_DOLLY_PLUGIN_SLUG.into())
        .await
        .assert_wp_error(WpErrorCode::CannotDeleteActivePlugin);
}

#[tokio::test]
async fn list_plugins_err_cannot_view_plugins() {
    api_client_as_subscriber()
        .plugins()
        .list_with_edit_context(&PluginListParams::default())
        .await
        .assert_wp_error(WpErrorCode::CannotViewPlugins);
}

#[tokio::test]
async fn retrieve_plugin_err_cannot_view_plugin() {
    api_client_as_subscriber()
        .plugins()
        .retrieve_with_edit_context(&HELLO_DOLLY_PLUGIN_SLUG.into())
        .await
        .assert_wp_error(WpErrorCode::CannotViewPlugin);
}

#[tokio::test]
async fn update_plugin_err_plugin_not_found() {
    api_client()
        .plugins()
        .update(
            &"foo".into(),
            &PluginUpdateParams {
                status: PluginStatus::Active,
            },
        )
        .await
        .assert_wp_error(WpErrorCode::PluginNotFound);
}

#[tokio::test]
async fn update_plugin_err_cannot_manage_plugins() {
    api_client_as_subscriber()
        .plugins()
        .update(
            &HELLO_DOLLY_PLUGIN_SLUG.into(),
            &PluginUpdateParams {
                status: PluginStatus::Active,
            },
        )
        .await
        .assert_wp_error(WpErrorCode::CannotManagePlugins);
}

use wp_api::plugins::{PluginCreateParams, PluginListParams, PluginStatus, PluginUpdateParams};
use wp_api::WpRestErrorCode;

use crate::integration_test_common::{
    request_builder, request_builder_as_subscriber, AssertWpError, HELLO_DOLLY_PLUGIN_SLUG,
    WP_ORG_PLUGIN_SLUG_CLASSIC_WIDGETS,
};

pub mod integration_test_common;

#[tokio::test]
async fn create_plugin_err_cannot_install_plugin() {
    request_builder_as_subscriber()
        .plugins()
        .create(&PluginCreateParams {
            slug: WP_ORG_PLUGIN_SLUG_CLASSIC_WIDGETS.into(),
            status: PluginStatus::Active,
        })
        .await
        .assert_wp_error(WpRestErrorCode::CannotInstallPlugin);
}

#[tokio::test]
async fn delete_plugin_err_cannot_delete_active_plugin() {
    request_builder()
        .plugins()
        .delete(&HELLO_DOLLY_PLUGIN_SLUG.into())
        .await
        .assert_wp_error(WpRestErrorCode::CannotDeleteActivePlugin);
}

#[tokio::test]
async fn list_plugins_err_cannot_view_plugins() {
    request_builder_as_subscriber()
        .plugins()
        .list_with_edit_context(&PluginListParams::default())
        .await
        .assert_wp_error(WpRestErrorCode::CannotViewPlugins);
}

#[tokio::test]
async fn retrieve_plugin_err_cannot_view_plugin() {
    request_builder_as_subscriber()
        .plugins()
        .retrieve_with_edit_context(&HELLO_DOLLY_PLUGIN_SLUG.into())
        .await
        .assert_wp_error(WpRestErrorCode::CannotViewPlugin);
}

#[tokio::test]
async fn update_plugin_err_plugin_not_found() {
    request_builder()
        .plugins()
        .update(
            &"foo".into(),
            &PluginUpdateParams {
                status: PluginStatus::Active,
            },
        )
        .await
        .assert_wp_error(WpRestErrorCode::PluginNotFound);
}

#[tokio::test]
async fn update_plugin_err_cannot_manage_plugins() {
    request_builder_as_subscriber()
        .plugins()
        .update(
            &HELLO_DOLLY_PLUGIN_SLUG.into(),
            &PluginUpdateParams {
                status: PluginStatus::Active,
            },
        )
        .await
        .assert_wp_error(WpRestErrorCode::CannotManagePlugins);
}

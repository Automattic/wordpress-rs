use wp_api::plugins::{PluginCreateParams, PluginStatus, PluginUpdateParams};
use wp_api::{WPContext, WPRestErrorCode};

use crate::integration_test_common::{
    request_builder, request_builder_as_subscriber, AssertWpError, WPNetworkRequestExecutor,
    HELLO_DOLLY_PLUGIN_SLUG, WP_ORG_PLUGIN_SLUG_CLASSIC_WIDGETS,
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
        .execute()
        .await
        .unwrap()
        .parse_with(wp_api::plugins::parse_create_plugin_response)
        .assert_wp_error(WPRestErrorCode::CannotInstallPlugin);
}

#[tokio::test]
async fn delete_plugin_err_cannot_delete_active_plugin() {
    request_builder()
        .plugins()
        .delete(&HELLO_DOLLY_PLUGIN_SLUG.into())
        .execute()
        .await
        .unwrap()
        .parse_with(wp_api::plugins::parse_delete_plugin_response)
        .assert_wp_error(WPRestErrorCode::CannotDeleteActivePlugin);
}

#[tokio::test]
async fn list_plugins_err_cannot_view_plugins() {
    request_builder_as_subscriber()
        .plugins()
        .list(WPContext::Edit, &None)
        .execute()
        .await
        .unwrap()
        .parse_with(wp_api::plugins::parse_retrieve_plugin_response_with_edit_context)
        .assert_wp_error(WPRestErrorCode::CannotViewPlugins);
}

#[tokio::test]
async fn retrieve_plugin_err_cannot_view_plugin() {
    request_builder_as_subscriber()
        .plugins()
        .retrieve(WPContext::Edit, &HELLO_DOLLY_PLUGIN_SLUG.into())
        .execute()
        .await
        .unwrap()
        .parse_with(wp_api::plugins::parse_retrieve_plugin_response_with_edit_context)
        .assert_wp_error(WPRestErrorCode::CannotViewPlugin);
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
        .execute()
        .await
        .unwrap()
        .parse_with(wp_api::plugins::parse_update_plugin_response)
        .assert_wp_error(WPRestErrorCode::PluginNotFound);
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
        .execute()
        .await
        .unwrap()
        .parse_with(wp_api::plugins::parse_update_plugin_response)
        .assert_wp_error(WPRestErrorCode::CannotManagePlugins);
}

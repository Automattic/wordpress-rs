use wp_api::plugins::{PluginCreateParams, PluginStatus, PluginUpdateParams};

use crate::integration_test_common::{
    request_builder, run_and_restore_wp_content_plugins, WpNetworkRequestExecutor,
    CLASSIC_EDITOR_PLUGIN_SLUG, HELLO_DOLLY_PLUGIN_SLUG, WP_ORG_PLUGIN_SLUG_CLASSIC_WIDGETS,
};

pub mod integration_test_common;
pub mod wp_db;

#[tokio::test]
async fn create_plugin() {
    run_and_restore_wp_content_plugins(|| {
        wp_db::run_and_restore(|mut _db| async move {
            let status = PluginStatus::Active;
            let params = PluginCreateParams {
                slug: WP_ORG_PLUGIN_SLUG_CLASSIC_WIDGETS.into(),
                status,
            };
            let created_plugin = request_builder()
                .plugins()
                .create(&params)
                .execute()
                .await
                .unwrap()
                .parse_with(wp_api::plugins::parse_create_plugin_response)
                .unwrap();
            println!("Created Plugin: {:?}", created_plugin);
        })
    })
    .await;
}

#[tokio::test]
async fn update_plugin() {
    run_and_restore_wp_content_plugins(|| {
        wp_db::run_and_restore(|mut _db| async move {
            let status = PluginStatus::Active;
            let updated_plugin = request_builder()
                .plugins()
                .update(
                    &HELLO_DOLLY_PLUGIN_SLUG.into(),
                    &PluginUpdateParams { status },
                )
                .execute()
                .await
                .unwrap()
                .parse_with(wp_api::plugins::parse_update_plugin_response)
                .unwrap();
            println!("Updated Plugin: {:?}", updated_plugin);
        })
    })
    .await;
}

#[tokio::test]
async fn delete_plugin() {
    run_and_restore_wp_content_plugins(|| {
        wp_db::run_and_restore(|mut _db| async move {
            let deleted_plugin = request_builder()
                .plugins()
                .delete(&CLASSIC_EDITOR_PLUGIN_SLUG.into())
                .execute()
                .await
                .unwrap()
                .parse_with(wp_api::plugins::parse_delete_plugin_response)
                .unwrap();
            println!("Deleted Plugin: {:?}", deleted_plugin);
        })
    })
    .await;
}

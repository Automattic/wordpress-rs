use wp_api::{PluginCreateParams, PluginStatus, PluginUpdateParams};

use crate::test_helpers::{
    api, run_and_restore_wp_content_plugins, WPNetworkRequestExecutor, WPNetworkResponseParser,
    CLASSIC_EDITOR_PLUGIN_SLUG, HELLO_DOLLY_PLUGIN_SLUG, WP_ORG_PLUGIN_SLUG_CLASSIC_WIDGETS,
};

pub mod test_helpers;
pub mod wp_db;

#[tokio::test]
async fn create_plugin() {
    run_and_restore_wp_content_plugins(|| {
        wp_db::run_and_restore(|mut _db| async move {
            let status = PluginStatus::Active;
            let params = PluginCreateParams {
                slug: WP_ORG_PLUGIN_SLUG_CLASSIC_WIDGETS.to_string(),
                status,
            };
            let created_plugin = api()
                .create_plugin_request(&params)
                .execute()
                .await
                .unwrap()
                .parse(wp_api::parse_create_plugin_response)
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
            let updated_plugin = api()
                .update_plugin_request(HELLO_DOLLY_PLUGIN_SLUG, PluginUpdateParams { status })
                .execute()
                .await
                .unwrap()
                .parse(wp_api::parse_update_plugin_response)
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
            let deleted_plugin = api()
                .delete_plugin_request(CLASSIC_EDITOR_PLUGIN_SLUG)
                .execute()
                .await
                .unwrap()
                .parse(wp_api::parse_delete_plugin_response)
                .unwrap();
            println!("Deleted Plugin: {:?}", deleted_plugin);
        })
    })
    .await;
}

use wp_api::{PluginCreateParams, PluginStatus, PluginUpdateParams};

use crate::test_helpers::{
    api, run_and_restore_wp_content_plugins, WPNetworkRequestExecutor, WPNetworkResponseParser,
};

pub mod test_helpers;
pub mod wp_db;

#[tokio::test]
async fn create_plugin() {
    run_and_restore_wp_content_plugins(|| {
        wp_db::run_and_restore(|mut _db| async move {
            let slug = "jetpack".to_string();
            let status = PluginStatus::Active;
            let params = PluginCreateParams { slug, status };
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
            let slug = "hello-dolly/hello".to_string();
            let status = PluginStatus::Active;
            let updated_plugin = api()
                .update_plugin_request(slug, PluginUpdateParams { status })
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

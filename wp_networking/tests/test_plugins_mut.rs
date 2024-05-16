use wp_api::{PluginCreateParams, PluginStatus};

use crate::test_helpers::{api, WPNetworkRequestExecutor, WPNetworkResponseParser};

pub mod test_helpers;
pub mod wp_db;

#[tokio::test]
async fn create_plugin() {
    wp_db::run_and_restore(|mut _db| async move {
        let slug = "jetpack".to_string();
        let status = PluginStatus::Active;

        let params = PluginCreateParams { slug, status };
        let _created_plugin = api()
            .create_plugin_request(&params)
            .execute()
            .await
            .unwrap()
            .parse(wp_api::parse_retrieve_plugin_response_with_edit_context)
            .unwrap();
        println!("{:?}", _created_plugin);
    })
    .await;
}

use wp_api::wp_block_editor::WpBlockEditorSettingsParams;
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[serial]

async fn test_get_wp_block_editor_settings<F>() {
    api_client()
        .wp_block_editor()
        .get_settings(&WpBlockEditorSettingsParams::default())
        .await
        .assert_response();

    RestoreServer::db().await;
}

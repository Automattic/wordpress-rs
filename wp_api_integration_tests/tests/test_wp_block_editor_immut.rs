use wp_api::{JsonValue, wp_block_editor::WpBlockEditorSettingsParams};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn test_retrieve_wp_block_editor_settings() {
    let json = api_client()
        .wp_block_editor()
        .retrieve_settings(&WpBlockEditorSettingsParams::default())
        .await
        .assert_response()
        .data
        .payload
        .as_json();

    if let JsonValue::Object(obj) = &json {
        assert_eq!(
            obj.get("alignWide"),
            Some(&JsonValue::Bool(false)),
            "alignWide should be false in test environment"
        );
    } else {
        panic!("Expected JSON object but got {:?}", json);
    }
}

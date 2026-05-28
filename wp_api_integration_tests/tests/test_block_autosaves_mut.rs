use wp_api::blocks::{BlockCreateParams, BlockId};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
async fn create_autosave() {
    let title = "Test Autosave Block Title".to_string();
    let content =
        "<!-- wp:paragraph --><p>Test autosave block content</p><!-- /wp:paragraph -->".to_string();
    let params = BlockCreateParams {
        title: Some(title.clone()),
        content: Some(content.clone()),
        ..Default::default()
    };

    let autosave = api_client()
        .block_autosaves()
        .create(&autosaved_block_id(), &params)
        .await
        .assert_response()
        .data;

    assert_eq!(autosave.title.raw, Some(title));
    assert_eq!(autosave.content.raw, Some(content));
}

fn autosaved_block_id() -> BlockId {
    BlockId(TestCredentials::instance().autosaved_block_id)
}

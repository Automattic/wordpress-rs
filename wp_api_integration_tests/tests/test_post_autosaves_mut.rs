use wp_api::{posts::PostCreateParams, posts::PostId};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
async fn create_autosave() {
    let title = "Test Autosave Title".to_string();
    let content = "Test autosave content created by integration test".to_string();
    let params = PostCreateParams {
        title: Some(title.clone()),
        content: Some(content.clone()),
        ..Default::default()
    };

    let autosave = api_client()
        .autosaves()
        .create(&autosaved_post_id(), &params)
        .await
        .assert_response()
        .data;

    // Verify the autosave was created successfully
    assert_eq!(autosave.title.raw, Some(title));
    assert_eq!(autosave.content.raw, Some(content));
}

fn autosaved_post_id() -> PostId {
    PostId(TestCredentials::instance().autosaved_post_id)
}

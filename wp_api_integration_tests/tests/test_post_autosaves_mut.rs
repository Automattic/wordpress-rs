use wp_api::{
    posts::{PostCreateParams, PostId},
    request::endpoint::posts_endpoint::PostEndpointType,
};
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
        .create(&PostEndpointType::Posts, &autosaved_post_id(), &params)
        .await
        .assert_response()
        .data;

    // Verify the autosave was created successfully
    assert_eq!(autosave.title.raw, Some(title));
    assert_eq!(autosave.content.raw, Some(content));

    RestoreServer::db().await;
}

fn autosaved_post_id() -> PostId {
    PostId(TestCredentials::instance().autosaved_post_id)
}

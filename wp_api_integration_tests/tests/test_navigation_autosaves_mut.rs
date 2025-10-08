use wp_api::navigations::{NavigationCreateParams, NavigationId};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
async fn create_autosave() {
    let title = "Test Autosave Navigation Title".to_string();
    let content =
        "<!-- wp:navigation -->Test autosave navigation content<!-- /wp:navigation -->".to_string();
    let params = NavigationCreateParams {
        title: Some(title.clone()),
        content: Some(content.clone()),
        ..Default::default()
    };

    let autosave = api_client()
        .navigation_autosaves()
        .create(&autosaved_navigation_id(), &params)
        .await
        .assert_response()
        .data;

    // Verify the autosave was created successfully
    assert_eq!(autosave.title.raw, Some(title));
    assert_eq!(autosave.content.raw, Some(content));
}

fn autosaved_navigation_id() -> NavigationId {
    NavigationId(TestCredentials::instance().autosaved_navigation_id)
}

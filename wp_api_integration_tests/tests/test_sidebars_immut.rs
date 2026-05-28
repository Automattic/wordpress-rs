use wp_api::sidebars::SidebarId;
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn list_with_edit_context() {
    let response = api_client()
        .sidebars()
        .list_with_edit_context()
        .await
        .assert_response();
    assert!(!response.data.is_empty());
}

#[tokio::test]
#[parallel]
async fn list_with_embed_context() {
    api_client()
        .sidebars()
        .list_with_embed_context()
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn list_with_view_context() {
    api_client()
        .sidebars()
        .list_with_view_context()
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_edit_context() {
    let sidebar_id = &SidebarId("wp_inactive_widgets".to_string());
    api_client()
        .sidebars()
        .retrieve_with_edit_context(sidebar_id)
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_embed_context() {
    let sidebar_id = &SidebarId("wp_inactive_widgets".to_string());
    api_client()
        .sidebars()
        .retrieve_with_embed_context(sidebar_id)
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_view_context() {
    let sidebar_id = &SidebarId("wp_inactive_widgets".to_string());
    api_client()
        .sidebars()
        .retrieve_with_view_context(sidebar_id)
        .await
        .assert_response();
}

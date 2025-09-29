use wp_api::menu_locations::MenuLocation;
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn list_with_edit_context() {
    let response = api_client()
        .menu_locations()
        .list_with_edit_context()
        .await
        .assert_response();
    assert!(!response.data.locations.is_empty());
}

#[tokio::test]
#[parallel]
async fn list_with_embed_context() {
    api_client()
        .menu_locations()
        .list_with_embed_context()
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn list_with_view_context() {
    api_client()
        .menu_locations()
        .list_with_view_context()
        .await
        .assert_response();
}

#[tokio::test]
#[apply(retrieve_cases)]
#[parallel]
async fn retrieve_with_edit_context(location: &str) {
    api_client()
        .menu_locations()
        .retrieve_with_edit_context(&MenuLocation(location.to_string()))
        .await
        .assert_response();
}

#[tokio::test]
#[apply(retrieve_cases)]
#[parallel]
async fn retrieve_with_embed_context(location: &str) {
    api_client()
        .menu_locations()
        .retrieve_with_embed_context(&MenuLocation(location.to_string()))
        .await
        .assert_response();
}

#[tokio::test]
#[apply(retrieve_cases)]
#[parallel]
async fn retrieve_with_view_context(location: &str) {
    api_client()
        .menu_locations()
        .retrieve_with_view_context(&MenuLocation(location.to_string()))
        .await
        .assert_response();
}

#[template]
#[rstest]
#[case::primary(TestCredentials::instance().primary_menu_location)]
#[case::footer(TestCredentials::instance().footer_menu_location)]
fn retrieve_cases(#[case] location: &str) {}

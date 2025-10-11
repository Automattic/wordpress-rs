use wp_api::menu_locations::MenuLocation;
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn list_menu_locations_err_rest_cannot_view() {
    api_client_as_subscriber()
        .menu_locations()
        .list_with_edit_context()
        .await
        .assert_wp_error(WpErrorCode::CannotView);
}

#[tokio::test]
#[parallel]
async fn retrieve_menu_location_err_rest_cannot_view() {
    api_client_as_subscriber()
        .menu_locations()
        .retrieve_with_edit_context(&MenuLocation(
            TestCredentials::instance()
                .primary_menu_location
                .to_string(),
        ))
        .await
        .assert_wp_error(WpErrorCode::CannotView);
}

#[tokio::test]
#[parallel]
async fn retrieve_menu_location_err_rest_menu_location_invalid() {
    api_client()
        .menu_locations()
        .retrieve_with_edit_context(&MenuLocation(
            "invalid_location_that_does_not_exist".to_string(),
        ))
        .await
        .assert_wp_error(WpErrorCode::MenuLocationInvalid);
}

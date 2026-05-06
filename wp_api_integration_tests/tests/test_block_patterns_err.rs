use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn list_err_cannot_view_as_subscriber() {
    api_client_as_subscriber()
        .block_patterns()
        .list_with_edit_context()
        .await
        .assert_wp_error(WpErrorCode::CannotView)
}

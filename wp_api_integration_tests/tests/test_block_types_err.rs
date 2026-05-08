use wp_api::block_types::{BlockTypeName, BlockTypeNamespace};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn retrieve_err_block_type_invalid() {
    api_client()
        .block_types()
        .retrieve_with_edit_context(
            &BlockTypeNamespace("nonexistent".to_string()),
            &BlockTypeName("nonexistent".to_string()),
        )
        .await
        .assert_wp_error(WpErrorCode::BlockTypeInvalid)
}

#[tokio::test]
#[parallel]
async fn list_err_cannot_view_as_subscriber() {
    api_client_as_subscriber()
        .block_types()
        .list_with_edit_context()
        .await
        .assert_wp_error(WpErrorCode::BlockTypeCannotView)
}

#[tokio::test]
#[parallel]
async fn retrieve_err_cannot_view_as_subscriber() {
    api_client_as_subscriber()
        .block_types()
        .retrieve_with_edit_context(
            &BlockTypeNamespace("core".to_string()),
            &BlockTypeName("paragraph".to_string()),
        )
        .await
        .assert_wp_error(WpErrorCode::BlockTypeCannotView)
}

use wp_api::block_directory::BlockDirectorySearchParams;
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn search_err_cannot_view_as_subscriber() {
    api_client_as_subscriber()
        .block_directory()
        .search(&BlockDirectorySearchParams::new("test".to_string()))
        .await
        .assert_wp_error(WpErrorCode::BlockDirectoryCannotView)
}

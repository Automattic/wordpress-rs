use wp_api::block_directory::{BlockDirectorySearchParams, SparseBlockDirectoryItemField};
use wp_api_integration_tests::prelude::*;

// Note: This endpoint queries the wordpress.org plugin directory via an external
// API call. Tests may be affected by network availability.

#[tokio::test]
#[parallel]
async fn search_block_directory() {
    let results = api_client()
        .block_directory()
        .search(&BlockDirectorySearchParams::new("coblocks".to_string()))
        .await
        .assert_response()
        .data;
    assert!(!results.is_empty());
}

#[tokio::test]
#[parallel]
async fn filter_search_block_directory() {
    let results = api_client()
        .block_directory()
        .filter_search(
            &BlockDirectorySearchParams::new("coblocks".to_string()),
            &[
                SparseBlockDirectoryItemField::Name,
                SparseBlockDirectoryItemField::Title,
            ],
        )
        .await
        .assert_response()
        .data;
    assert!(!results.is_empty());
    // Fields not requested should be None
    assert!(
        results
            .first()
            .expect("should have results")
            .description
            .is_none()
    );
}

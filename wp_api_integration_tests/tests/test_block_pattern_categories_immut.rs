use wp_api::block_pattern_categories::SparseBlockPatternCategoryFieldWithViewContext;
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn list_with_edit_context() {
    let categories = api_client()
        .block_pattern_categories()
        .list_with_edit_context()
        .await
        .assert_response()
        .data;
    assert!(!categories.is_empty());
}

#[tokio::test]
#[parallel]
async fn list_with_embed_context() {
    api_client()
        .block_pattern_categories()
        .list_with_embed_context()
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn list_with_view_context() {
    api_client()
        .block_pattern_categories()
        .list_with_view_context()
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn filter_list_with_view_context() {
    let categories = api_client()
        .block_pattern_categories()
        .filter_list_with_view_context(&[SparseBlockPatternCategoryFieldWithViewContext::Name])
        .await
        .assert_response()
        .data;
    assert!(!categories.is_empty());
    assert!(
        categories
            .first()
            .expect("should have results")
            .label
            .is_none()
    );
}

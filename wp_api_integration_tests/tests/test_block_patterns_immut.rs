use wp_api::block_patterns::SparseBlockPatternFieldWithViewContext;
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn list_with_edit_context() {
    let patterns = api_client()
        .block_patterns()
        .list_with_edit_context()
        .await
        .assert_response()
        .data;
    assert!(!patterns.is_empty());
}

#[tokio::test]
#[parallel]
async fn list_with_embed_context() {
    api_client()
        .block_patterns()
        .list_with_embed_context()
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn list_with_view_context() {
    api_client()
        .block_patterns()
        .list_with_view_context()
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn filter_list_with_view_context() {
    let patterns = api_client()
        .block_patterns()
        .filter_list_with_view_context(&[
            SparseBlockPatternFieldWithViewContext::Name,
            SparseBlockPatternFieldWithViewContext::Title,
        ])
        .await
        .assert_response()
        .data;
    assert!(!patterns.is_empty());
    // Fields not requested should be None
    assert!(
        patterns
            .first()
            .expect("should have results")
            .content
            .is_none()
    );
}

use wp_api::pattern_directory::{
    PatternDirectoryCategoryId, PatternDirectoryListParams,
    SparsePatternDirectoryItemFieldWithEditContext,
    SparsePatternDirectoryItemFieldWithEmbedContext,
    SparsePatternDirectoryItemFieldWithViewContext,
};
use wp_api_integration_tests::prelude::*;

// Note: This endpoint queries the wordpress.org pattern directory via an external
// API call. Tests may be affected by network availability.

#[tokio::test]
#[parallel]
async fn list_with_edit_context() {
    let patterns = api_client()
        .pattern_directory()
        .list_with_edit_context(&PatternDirectoryListParams::default())
        .await
        .assert_response()
        .data;
    assert!(!patterns.is_empty());
}

#[tokio::test]
#[parallel]
async fn list_with_embed_context() {
    api_client()
        .pattern_directory()
        .list_with_embed_context(&PatternDirectoryListParams::default())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn list_with_view_context() {
    api_client()
        .pattern_directory()
        .list_with_view_context(&PatternDirectoryListParams::default())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn list_with_category_param() {
    let params = PatternDirectoryListParams {
        category: Some(PatternDirectoryCategoryId(2)),
        per_page: Some(5),
        ..Default::default()
    };
    api_client()
        .pattern_directory()
        .list_with_view_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn list_with_per_page_param() {
    let params = PatternDirectoryListParams {
        per_page: Some(3),
        ..Default::default()
    };
    let patterns = api_client()
        .pattern_directory()
        .list_with_view_context(&params)
        .await
        .assert_response()
        .data;
    assert!(patterns.len() <= 3);
}

mod filter {
    use super::*;

    wp_api::generate_sparse_pattern_directory_item_field_with_edit_context_test_cases!();
    wp_api::generate_sparse_pattern_directory_item_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_pattern_directory_item_field_with_view_context_test_cases!();

    #[apply(sparse_pattern_directory_item_field_with_edit_context_test_cases)]
    #[case(&[SparsePatternDirectoryItemFieldWithEditContext::Id, SparsePatternDirectoryItemFieldWithEditContext::Title])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_edit_context(
        #[case] fields: &[SparsePatternDirectoryItemFieldWithEditContext],
    ) {
        api_client()
            .pattern_directory()
            .filter_list_with_edit_context(&PatternDirectoryListParams::default(), fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|pattern| {
                pattern.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_pattern_directory_item_field_with_embed_context_test_cases)]
    #[case(&[SparsePatternDirectoryItemFieldWithEmbedContext::Id, SparsePatternDirectoryItemFieldWithEmbedContext::Title])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_embed_context(
        #[case] fields: &[SparsePatternDirectoryItemFieldWithEmbedContext],
    ) {
        api_client()
            .pattern_directory()
            .filter_list_with_embed_context(&PatternDirectoryListParams::default(), fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|pattern| {
                pattern.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_pattern_directory_item_field_with_view_context_test_cases)]
    #[case(&[SparsePatternDirectoryItemFieldWithViewContext::Id, SparsePatternDirectoryItemFieldWithViewContext::Title])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_view_context(
        #[case] fields: &[SparsePatternDirectoryItemFieldWithViewContext],
    ) {
        api_client()
            .pattern_directory()
            .filter_list_with_view_context(&PatternDirectoryListParams::default(), fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|pattern| {
                pattern.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }
}

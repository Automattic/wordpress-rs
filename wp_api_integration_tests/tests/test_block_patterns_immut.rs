use wp_api::block_patterns::{
    SparseBlockPatternFieldWithEditContext, SparseBlockPatternFieldWithEmbedContext,
    SparseBlockPatternFieldWithViewContext,
};
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

mod filter {
    use super::*;

    wp_api::generate_sparse_block_pattern_field_with_edit_context_test_cases!();
    wp_api::generate_sparse_block_pattern_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_block_pattern_field_with_view_context_test_cases!();

    #[apply(sparse_block_pattern_field_with_edit_context_test_cases)]
    #[case(&[SparseBlockPatternFieldWithEditContext::Name, SparseBlockPatternFieldWithEditContext::Title])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_edit_context(
        #[case] fields: &[SparseBlockPatternFieldWithEditContext],
    ) {
        api_client()
            .block_patterns()
            .filter_list_with_edit_context(fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|pattern| {
                pattern.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_block_pattern_field_with_embed_context_test_cases)]
    #[case(&[SparseBlockPatternFieldWithEmbedContext::Name, SparseBlockPatternFieldWithEmbedContext::Title])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_embed_context(
        #[case] fields: &[SparseBlockPatternFieldWithEmbedContext],
    ) {
        api_client()
            .block_patterns()
            .filter_list_with_embed_context(fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|pattern| {
                pattern.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_block_pattern_field_with_view_context_test_cases)]
    #[case(&[SparseBlockPatternFieldWithViewContext::Name, SparseBlockPatternFieldWithViewContext::Title])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_view_context(
        #[case] fields: &[SparseBlockPatternFieldWithViewContext],
    ) {
        api_client()
            .block_patterns()
            .filter_list_with_view_context(fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|pattern| {
                pattern.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }
}

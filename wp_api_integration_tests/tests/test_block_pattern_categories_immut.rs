use wp_api::block_pattern_categories::{
    SparseBlockPatternCategoryFieldWithEditContext,
    SparseBlockPatternCategoryFieldWithEmbedContext,
    SparseBlockPatternCategoryFieldWithViewContext,
};
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

mod filter {
    use super::*;

    wp_api::generate_sparse_block_pattern_category_field_with_edit_context_test_cases!();
    wp_api::generate_sparse_block_pattern_category_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_block_pattern_category_field_with_view_context_test_cases!();

    #[apply(sparse_block_pattern_category_field_with_edit_context_test_cases)]
    #[case(&[SparseBlockPatternCategoryFieldWithEditContext::Name, SparseBlockPatternCategoryFieldWithEditContext::Label])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_edit_context(
        #[case] fields: &[SparseBlockPatternCategoryFieldWithEditContext],
    ) {
        api_client()
            .block_pattern_categories()
            .filter_list_with_edit_context(fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|category| {
                category.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_block_pattern_category_field_with_embed_context_test_cases)]
    #[case(&[SparseBlockPatternCategoryFieldWithEmbedContext::Name, SparseBlockPatternCategoryFieldWithEmbedContext::Label])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_embed_context(
        #[case] fields: &[SparseBlockPatternCategoryFieldWithEmbedContext],
    ) {
        api_client()
            .block_pattern_categories()
            .filter_list_with_embed_context(fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|category| {
                category.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_block_pattern_category_field_with_view_context_test_cases)]
    #[case(&[SparseBlockPatternCategoryFieldWithViewContext::Name, SparseBlockPatternCategoryFieldWithViewContext::Label])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_view_context(
        #[case] fields: &[SparseBlockPatternCategoryFieldWithViewContext],
    ) {
        api_client()
            .block_pattern_categories()
            .filter_list_with_view_context(fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|category| {
                category.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }
}

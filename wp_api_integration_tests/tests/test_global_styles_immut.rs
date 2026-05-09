use integration_test_credentials::TestCredentials;
use wp_api::global_styles::{
    GlobalStylesId, SparseGlobalStylesFieldWithEditContext,
    SparseGlobalStylesFieldWithEmbedContext, SparseGlobalStylesFieldWithViewContext,
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn retrieve_with_edit_context() {
    api_client()
        .global_styles()
        .retrieve_with_edit_context(&global_styles_id())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_embed_context() {
    api_client()
        .global_styles()
        .retrieve_with_embed_context(&global_styles_id())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_view_context() {
    api_client()
        .global_styles()
        .retrieve_with_view_context(&global_styles_id())
        .await
        .assert_response();
}

mod filter {
    use super::*;

    wp_api::generate_sparse_global_styles_field_with_edit_context_test_cases!();
    wp_api::generate_sparse_global_styles_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_global_styles_field_with_view_context_test_cases!();

    #[apply(sparse_global_styles_field_with_edit_context_test_cases)]
    #[case(&[SparseGlobalStylesFieldWithEditContext::Id, SparseGlobalStylesFieldWithEditContext::Title])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_edit_context(
        #[case] fields: &[SparseGlobalStylesFieldWithEditContext],
    ) {
        api_client()
            .global_styles()
            .filter_retrieve_with_edit_context(&global_styles_id(), fields)
            .await
            .assert_response()
            .data
            .assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_global_styles_field_with_embed_context_test_cases)]
    #[case(&[SparseGlobalStylesFieldWithEmbedContext::Id, SparseGlobalStylesFieldWithEmbedContext::Title])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_embed_context(
        #[case] fields: &[SparseGlobalStylesFieldWithEmbedContext],
    ) {
        api_client()
            .global_styles()
            .filter_retrieve_with_embed_context(&global_styles_id(), fields)
            .await
            .assert_response()
            .data
            .assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_global_styles_field_with_view_context_test_cases)]
    #[case(&[SparseGlobalStylesFieldWithViewContext::Id, SparseGlobalStylesFieldWithViewContext::Title])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_view_context(
        #[case] fields: &[SparseGlobalStylesFieldWithViewContext],
    ) {
        api_client()
            .global_styles()
            .filter_retrieve_with_view_context(&global_styles_id(), fields)
            .await
            .assert_response()
            .data
            .assert_that_instance_fields_nullability_match_provided_fields(fields);
    }
}

fn global_styles_id() -> GlobalStylesId {
    GlobalStylesId(TestCredentials::instance().global_styles_id)
}

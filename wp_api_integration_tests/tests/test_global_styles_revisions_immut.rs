use integration_test_credentials::TestCredentials;
use wp_api::{
    global_styles::GlobalStylesId,
    global_styles_revisions::{
        GlobalStylesRevisionId, GlobalStylesRevisionListParams,
        SparseGlobalStylesRevisionFieldWithEditContext,
        SparseGlobalStylesRevisionFieldWithEmbedContext,
        SparseGlobalStylesRevisionFieldWithViewContext,
    },
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_edit_context(#[case] params: GlobalStylesRevisionListParams) {
    api_client()
        .global_styles_revisions()
        .list_with_edit_context(&global_styles_id(), &params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_embed_context(#[case] params: GlobalStylesRevisionListParams) {
    api_client()
        .global_styles_revisions()
        .list_with_embed_context(&global_styles_id(), &params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_view_context(#[case] params: GlobalStylesRevisionListParams) {
    api_client()
        .global_styles_revisions()
        .list_with_view_context(&global_styles_id(), &params)
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_edit_context() {
    api_client()
        .global_styles_revisions()
        .retrieve_with_edit_context(&global_styles_id(), &revision_id())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_embed_context() {
    api_client()
        .global_styles_revisions()
        .retrieve_with_embed_context(&global_styles_id(), &revision_id())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_view_context() {
    api_client()
        .global_styles_revisions()
        .retrieve_with_view_context(&global_styles_id(), &revision_id())
        .await
        .assert_response();
}

fn global_styles_id() -> GlobalStylesId {
    GlobalStylesId(TestCredentials::instance().global_styles_id)
}

fn revision_id() -> GlobalStylesRevisionId {
    GlobalStylesRevisionId(TestCredentials::instance().revision_id_for_global_styles_id)
}

#[template]
#[rstest]
#[case::default(GlobalStylesRevisionListParams::default())]
#[case::page(generate!(GlobalStylesRevisionListParams, (page, Some(1))))]
#[case::per_page(generate!(GlobalStylesRevisionListParams, (per_page, Some(3))))]
#[case::offset(generate!(GlobalStylesRevisionListParams, (offset, Some(2))))]
fn list_cases(#[case] params: GlobalStylesRevisionListParams) {}

mod filter {
    use super::*;

    wp_api::generate_sparse_global_styles_revision_field_with_edit_context_test_cases!();
    wp_api::generate_sparse_global_styles_revision_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_global_styles_revision_field_with_view_context_test_cases!();

    #[apply(sparse_global_styles_revision_field_with_edit_context_test_cases)]
    #[case(&[SparseGlobalStylesRevisionFieldWithEditContext::Id, SparseGlobalStylesRevisionFieldWithEditContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_edit_context(
        #[case] fields: &[SparseGlobalStylesRevisionFieldWithEditContext],
        #[values(GlobalStylesRevisionListParams::default())] params: GlobalStylesRevisionListParams,
    ) {
        api_client()
            .global_styles_revisions()
            .filter_list_with_edit_context(&global_styles_id(), &params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|revision| {
                revision.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_global_styles_revision_field_with_embed_context_test_cases)]
    #[case(&[SparseGlobalStylesRevisionFieldWithEmbedContext::Id, SparseGlobalStylesRevisionFieldWithEmbedContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_embed_context(
        #[case] fields: &[SparseGlobalStylesRevisionFieldWithEmbedContext],
        #[values(GlobalStylesRevisionListParams::default())] params: GlobalStylesRevisionListParams,
    ) {
        api_client()
            .global_styles_revisions()
            .filter_list_with_embed_context(&global_styles_id(), &params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|revision| {
                revision.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_global_styles_revision_field_with_view_context_test_cases)]
    #[case(&[SparseGlobalStylesRevisionFieldWithViewContext::Id, SparseGlobalStylesRevisionFieldWithViewContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_view_context(
        #[case] fields: &[SparseGlobalStylesRevisionFieldWithViewContext],
        #[values(GlobalStylesRevisionListParams::default())] params: GlobalStylesRevisionListParams,
    ) {
        api_client()
            .global_styles_revisions()
            .filter_list_with_view_context(&global_styles_id(), &params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|revision| {
                revision.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_global_styles_revision_field_with_edit_context_test_cases)]
    #[case(&[SparseGlobalStylesRevisionFieldWithEditContext::Id, SparseGlobalStylesRevisionFieldWithEditContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_edit_context(
        #[case] fields: &[SparseGlobalStylesRevisionFieldWithEditContext],
    ) {
        api_client()
            .global_styles_revisions()
            .filter_retrieve_with_edit_context(&global_styles_id(), &revision_id(), fields)
            .await
            .assert_response()
            .data
            .assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_global_styles_revision_field_with_embed_context_test_cases)]
    #[case(&[SparseGlobalStylesRevisionFieldWithEmbedContext::Id, SparseGlobalStylesRevisionFieldWithEmbedContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_embed_context(
        #[case] fields: &[SparseGlobalStylesRevisionFieldWithEmbedContext],
    ) {
        api_client()
            .global_styles_revisions()
            .filter_retrieve_with_embed_context(&global_styles_id(), &revision_id(), fields)
            .await
            .assert_response()
            .data
            .assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_global_styles_revision_field_with_view_context_test_cases)]
    #[case(&[SparseGlobalStylesRevisionFieldWithViewContext::Id, SparseGlobalStylesRevisionFieldWithViewContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_view_context(
        #[case] fields: &[SparseGlobalStylesRevisionFieldWithViewContext],
    ) {
        api_client()
            .global_styles_revisions()
            .filter_retrieve_with_view_context(&global_styles_id(), &revision_id(), fields)
            .await
            .assert_response()
            .data
            .assert_that_instance_fields_nullability_match_provided_fields(fields);
    }
}

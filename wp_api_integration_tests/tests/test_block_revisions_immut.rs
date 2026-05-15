use wp_api::block_revisions::{
    BlockRevisionId, BlockRevisionListParams, SparseBlockRevisionFieldWithEditContext,
    SparseBlockRevisionFieldWithEmbedContext, SparseBlockRevisionFieldWithViewContext,
    WpApiParamBlockRevisionsOrderBy,
};
use wp_api::blocks::BlockId;
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_edit_context(#[case] params: BlockRevisionListParams) {
    api_client()
        .block_revisions()
        .list_with_edit_context(&block_id(), &params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_embed_context(#[case] params: BlockRevisionListParams) {
    api_client()
        .block_revisions()
        .list_with_embed_context(&block_id(), &params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_view_context(#[case] params: BlockRevisionListParams) {
    api_client()
        .block_revisions()
        .list_with_view_context(&block_id(), &params)
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_edit_context() {
    api_client()
        .block_revisions()
        .retrieve_with_edit_context(&block_id(), &revision_id_for_block_id())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_embed_context() {
    api_client()
        .block_revisions()
        .retrieve_with_embed_context(&block_id(), &revision_id_for_block_id())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_view_context() {
    api_client()
        .block_revisions()
        .retrieve_with_view_context(&block_id(), &revision_id_for_block_id())
        .await
        .assert_response();
}

fn block_id() -> BlockId {
    BlockId(TestCredentials::instance().block_id)
}

fn revision_id_for_block_id() -> BlockRevisionId {
    BlockRevisionId(TestCredentials::instance().revision_id_for_block_id)
}

#[template]
#[rstest]
#[case::default(BlockRevisionListParams::default())]
#[case::page(generate!(BlockRevisionListParams, (page, Some(1))))]
#[case::per_page(generate!(BlockRevisionListParams, (per_page, Some(3))))]
#[case::search(generate!(BlockRevisionListParams, (search, Some("foo".to_string()))))]
#[case::exclude(generate!(BlockRevisionListParams, (exclude, vec![BlockRevisionId(1), BlockRevisionId(2)])))]
#[case::include(generate!(BlockRevisionListParams, (include, vec![BlockRevisionId(1)])))]
#[case::offset(generate!(BlockRevisionListParams, (offset, Some(2))))]
#[case::order(generate!(BlockRevisionListParams, (order, Some(WpApiParamOrder::Asc))))]
#[case::orderby(generate!(BlockRevisionListParams, (orderby, Some(WpApiParamBlockRevisionsOrderBy::Id))))]
fn list_cases(#[case] params: BlockRevisionListParams) {}

mod filter {
    use super::*;

    wp_api::generate_sparse_block_revision_field_with_edit_context_test_cases!();
    wp_api::generate_sparse_block_revision_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_block_revision_field_with_view_context_test_cases!();

    #[apply(sparse_block_revision_field_with_edit_context_test_cases)]
    #[case(&[SparseBlockRevisionFieldWithEditContext::Id, SparseBlockRevisionFieldWithEditContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_block_revisions_with_edit_context(
        #[case] fields: &[SparseBlockRevisionFieldWithEditContext],
    ) {
        api_client()
            .block_revisions()
            .filter_list_with_edit_context(&block_id(), &BlockRevisionListParams::default(), fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|revision| {
                revision.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_block_revision_field_with_edit_context_test_cases)]
    #[case(&[SparseBlockRevisionFieldWithEditContext::Id, SparseBlockRevisionFieldWithEditContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_block_revisions_with_edit_context(
        #[case] fields: &[SparseBlockRevisionFieldWithEditContext],
    ) {
        let revision = api_client()
            .block_revisions()
            .filter_retrieve_with_edit_context(&block_id(), &revision_id_for_block_id(), fields)
            .await
            .assert_response()
            .data;
        revision.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }

    #[apply(sparse_block_revision_field_with_embed_context_test_cases)]
    #[case(&[SparseBlockRevisionFieldWithEmbedContext::Id, SparseBlockRevisionFieldWithEmbedContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_block_revisions_with_embed_context(
        #[case] fields: &[SparseBlockRevisionFieldWithEmbedContext],
    ) {
        api_client()
            .block_revisions()
            .filter_list_with_embed_context(
                &block_id(),
                &BlockRevisionListParams::default(),
                fields,
            )
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|revision| {
                revision.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_block_revision_field_with_embed_context_test_cases)]
    #[case(&[SparseBlockRevisionFieldWithEmbedContext::Id, SparseBlockRevisionFieldWithEmbedContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_block_revisions_with_embed_context(
        #[case] fields: &[SparseBlockRevisionFieldWithEmbedContext],
    ) {
        let revision = api_client()
            .block_revisions()
            .filter_retrieve_with_embed_context(&block_id(), &revision_id_for_block_id(), fields)
            .await
            .assert_response()
            .data;
        revision.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }

    #[apply(sparse_block_revision_field_with_view_context_test_cases)]
    #[case(&[SparseBlockRevisionFieldWithViewContext::Id, SparseBlockRevisionFieldWithViewContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_block_revisions_with_view_context(
        #[case] fields: &[SparseBlockRevisionFieldWithViewContext],
    ) {
        api_client()
            .block_revisions()
            .filter_list_with_view_context(&block_id(), &BlockRevisionListParams::default(), fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|revision| {
                revision.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_block_revision_field_with_view_context_test_cases)]
    #[case(&[SparseBlockRevisionFieldWithViewContext::Id, SparseBlockRevisionFieldWithViewContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_block_revisions_with_view_context(
        #[case] fields: &[SparseBlockRevisionFieldWithViewContext],
    ) {
        let revision = api_client()
            .block_revisions()
            .filter_retrieve_with_view_context(&block_id(), &revision_id_for_block_id(), fields)
            .await
            .assert_response()
            .data;
        revision.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }
}

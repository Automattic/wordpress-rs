use wp_api::{
    block_revisions::{
        BlockRevisionId, SparseBlockRevisionFieldWithEditContext,
        SparseBlockRevisionFieldWithEmbedContext, SparseBlockRevisionFieldWithViewContext,
    },
    blocks::BlockId,
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn list_with_edit_context() {
    api_client()
        .block_autosaves()
        .list_with_edit_context(&autosaved_block_id())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn list_with_embed_context() {
    api_client()
        .block_autosaves()
        .list_with_embed_context(&autosaved_block_id())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn list_with_view_context() {
    api_client()
        .block_autosaves()
        .list_with_view_context(&autosaved_block_id())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_edit_context() {
    api_client()
        .block_autosaves()
        .retrieve_with_edit_context(&autosaved_block_id(), &autosave_id_for_autosaved_block_id())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_embed_context() {
    api_client()
        .block_autosaves()
        .retrieve_with_embed_context(&autosaved_block_id(), &autosave_id_for_autosaved_block_id())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_view_context() {
    api_client()
        .block_autosaves()
        .retrieve_with_view_context(&autosaved_block_id(), &autosave_id_for_autosaved_block_id())
        .await
        .assert_response();
}

fn autosaved_block_id() -> BlockId {
    BlockId(TestCredentials::instance().autosaved_block_id)
}

fn autosave_id_for_autosaved_block_id() -> BlockRevisionId {
    BlockRevisionId(TestCredentials::instance().autosave_id_for_autosaved_block_id)
}

mod filter {
    use super::*;

    wp_api::generate_sparse_block_revision_field_with_edit_context_test_cases!();
    wp_api::generate_sparse_block_revision_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_block_revision_field_with_view_context_test_cases!();

    #[apply(sparse_block_revision_field_with_edit_context_test_cases)]
    #[case(&[SparseBlockRevisionFieldWithEditContext::Id, SparseBlockRevisionFieldWithEditContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_edit_context(
        #[case] fields: &[SparseBlockRevisionFieldWithEditContext],
    ) {
        api_client()
            .block_autosaves()
            .filter_list_with_edit_context(&autosaved_block_id(), fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|autosave| {
                autosave.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_block_revision_field_with_embed_context_test_cases)]
    #[case(&[SparseBlockRevisionFieldWithEmbedContext::Id, SparseBlockRevisionFieldWithEmbedContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_embed_context(
        #[case] fields: &[SparseBlockRevisionFieldWithEmbedContext],
    ) {
        api_client()
            .block_autosaves()
            .filter_list_with_embed_context(&autosaved_block_id(), fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|autosave| {
                autosave.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_block_revision_field_with_view_context_test_cases)]
    #[case(&[SparseBlockRevisionFieldWithViewContext::Id, SparseBlockRevisionFieldWithViewContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_view_context(
        #[case] fields: &[SparseBlockRevisionFieldWithViewContext],
    ) {
        api_client()
            .block_autosaves()
            .filter_list_with_view_context(&autosaved_block_id(), fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|autosave| {
                autosave.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_block_revision_field_with_edit_context_test_cases)]
    #[case(&[SparseBlockRevisionFieldWithEditContext::Id, SparseBlockRevisionFieldWithEditContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_edit_context(
        #[case] fields: &[SparseBlockRevisionFieldWithEditContext],
    ) {
        api_client()
            .block_autosaves()
            .filter_retrieve_with_edit_context(
                &autosaved_block_id(),
                &autosave_id_for_autosaved_block_id(),
                fields,
            )
            .await
            .assert_response()
            .data
            .assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_block_revision_field_with_embed_context_test_cases)]
    #[case(&[SparseBlockRevisionFieldWithEmbedContext::Id, SparseBlockRevisionFieldWithEmbedContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_embed_context(
        #[case] fields: &[SparseBlockRevisionFieldWithEmbedContext],
    ) {
        api_client()
            .block_autosaves()
            .filter_retrieve_with_embed_context(
                &autosaved_block_id(),
                &autosave_id_for_autosaved_block_id(),
                fields,
            )
            .await
            .assert_response()
            .data
            .assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_block_revision_field_with_view_context_test_cases)]
    #[case(&[SparseBlockRevisionFieldWithViewContext::Id, SparseBlockRevisionFieldWithViewContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_view_context(
        #[case] fields: &[SparseBlockRevisionFieldWithViewContext],
    ) {
        api_client()
            .block_autosaves()
            .filter_retrieve_with_view_context(
                &autosaved_block_id(),
                &autosave_id_for_autosaved_block_id(),
                fields,
            )
            .await
            .assert_response()
            .data
            .assert_that_instance_fields_nullability_match_provided_fields(fields);
    }
}

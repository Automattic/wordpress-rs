use wp_api::{
    post_revisions::{
        PostRevisionId, SparsePostRevisionFieldWithEditContext,
        SparsePostRevisionFieldWithEmbedContext, SparsePostRevisionFieldWithViewContext,
    },
    posts::PostId,
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn list_with_edit_context() {
    api_client()
        .autosaves()
        .list_with_edit_context(&autosaved_post_id())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn list_with_embed_context() {
    api_client()
        .autosaves()
        .list_with_embed_context(&autosaved_post_id())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn list_with_view_context() {
    api_client()
        .autosaves()
        .list_with_view_context(&autosaved_post_id())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_edit_context() {
    api_client()
        .autosaves()
        .retrieve_with_edit_context(&autosaved_post_id(), &autosave_id_for_autosaved_post_id())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_embed_context() {
    api_client()
        .autosaves()
        .retrieve_with_embed_context(&autosaved_post_id(), &autosave_id_for_autosaved_post_id())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_view_context() {
    api_client()
        .autosaves()
        .retrieve_with_view_context(&autosaved_post_id(), &autosave_id_for_autosaved_post_id())
        .await
        .assert_response();
}

fn autosaved_post_id() -> PostId {
    PostId(TestCredentials::instance().autosaved_post_id)
}

fn autosave_id_for_autosaved_post_id() -> PostRevisionId {
    PostRevisionId(TestCredentials::instance().autosave_id_for_autosaved_post_id)
}

mod filter {
    use super::*;

    wp_api::generate_sparse_post_revision_field_with_edit_context_test_cases!();
    wp_api::generate_sparse_post_revision_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_post_revision_field_with_view_context_test_cases!();

    #[apply(sparse_post_revision_field_with_edit_context_test_cases)]
    #[case(&[SparsePostRevisionFieldWithEditContext::Id, SparsePostRevisionFieldWithEditContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_edit_context(
        #[case] fields: &[SparsePostRevisionFieldWithEditContext],
    ) {
        api_client()
            .autosaves()
            .filter_list_with_edit_context(&autosaved_post_id(), fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|autosave| {
                autosave.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_post_revision_field_with_embed_context_test_cases)]
    #[case(&[SparsePostRevisionFieldWithEmbedContext::Id, SparsePostRevisionFieldWithEmbedContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_embed_context(
        #[case] fields: &[SparsePostRevisionFieldWithEmbedContext],
    ) {
        api_client()
            .autosaves()
            .filter_list_with_embed_context(&autosaved_post_id(), fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|autosave| {
                autosave.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_post_revision_field_with_view_context_test_cases)]
    #[case(&[SparsePostRevisionFieldWithViewContext::Id, SparsePostRevisionFieldWithViewContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_view_context(
        #[case] fields: &[SparsePostRevisionFieldWithViewContext],
    ) {
        api_client()
            .autosaves()
            .filter_list_with_view_context(&autosaved_post_id(), fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|autosave| {
                autosave.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_post_revision_field_with_edit_context_test_cases)]
    #[case(&[SparsePostRevisionFieldWithEditContext::Id, SparsePostRevisionFieldWithEditContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_edit_context(
        #[case] fields: &[SparsePostRevisionFieldWithEditContext],
    ) {
        api_client()
            .autosaves()
            .filter_retrieve_with_edit_context(
                &autosaved_post_id(),
                &autosave_id_for_autosaved_post_id(),
                fields,
            )
            .await
            .assert_response()
            .data
            .assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_post_revision_field_with_embed_context_test_cases)]
    #[case(&[SparsePostRevisionFieldWithEmbedContext::Id, SparsePostRevisionFieldWithEmbedContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_embed_context(
        #[case] fields: &[SparsePostRevisionFieldWithEmbedContext],
    ) {
        api_client()
            .autosaves()
            .filter_retrieve_with_embed_context(
                &autosaved_post_id(),
                &autosave_id_for_autosaved_post_id(),
                fields,
            )
            .await
            .assert_response()
            .data
            .assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_post_revision_field_with_view_context_test_cases)]
    #[case(&[SparsePostRevisionFieldWithViewContext::Id, SparsePostRevisionFieldWithViewContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_view_context(
        #[case] fields: &[SparsePostRevisionFieldWithViewContext],
    ) {
        api_client()
            .autosaves()
            .filter_retrieve_with_view_context(
                &autosaved_post_id(),
                &autosave_id_for_autosaved_post_id(),
                fields,
            )
            .await
            .assert_response()
            .data
            .assert_that_instance_fields_nullability_match_provided_fields(fields);
    }
}

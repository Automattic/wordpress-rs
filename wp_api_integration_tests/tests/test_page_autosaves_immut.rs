use wp_api::{
    post_revisions::{
        PostRevisionId, SparseAnyPostRevisionFieldWithEditContext,
        SparseAnyPostRevisionFieldWithEmbedContext, SparseAnyPostRevisionFieldWithViewContext,
    },
    posts::PostId,
    request::endpoint::posts_endpoint::PostEndpointType,
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn list_with_edit_context() {
    api_client()
        .autosaves()
        .list_with_edit_context(&PostEndpointType::Pages, &autosaved_page_id())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn list_with_embed_context() {
    api_client()
        .autosaves()
        .list_with_embed_context(&PostEndpointType::Pages, &autosaved_page_id())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn list_with_view_context() {
    api_client()
        .autosaves()
        .list_with_view_context(&PostEndpointType::Pages, &autosaved_page_id())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_edit_context() {
    api_client()
        .autosaves()
        .retrieve_with_edit_context(
            &PostEndpointType::Pages,
            &autosaved_page_id(),
            &autosave_id_for_autosaved_page_id(),
        )
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_embed_context() {
    api_client()
        .autosaves()
        .retrieve_with_embed_context(
            &PostEndpointType::Pages,
            &autosaved_page_id(),
            &autosave_id_for_autosaved_page_id(),
        )
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_view_context() {
    api_client()
        .autosaves()
        .retrieve_with_view_context(
            &PostEndpointType::Pages,
            &autosaved_page_id(),
            &autosave_id_for_autosaved_page_id(),
        )
        .await
        .assert_response();
}

fn autosaved_page_id() -> PostId {
    PostId(TestCredentials::instance().autosaved_page_id)
}

fn autosave_id_for_autosaved_page_id() -> PostRevisionId {
    PostRevisionId(TestCredentials::instance().autosave_id_for_autosaved_page_id)
}

mod filter {
    use super::*;

    wp_api::generate_sparse_any_post_revision_field_with_edit_context_test_cases!();
    wp_api::generate_sparse_any_post_revision_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_any_post_revision_field_with_view_context_test_cases!();

    #[apply(sparse_any_post_revision_field_with_edit_context_test_cases)]
    #[case(&[SparseAnyPostRevisionFieldWithEditContext::Id, SparseAnyPostRevisionFieldWithEditContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_edit_context(
        #[case] fields: &[SparseAnyPostRevisionFieldWithEditContext],
    ) {
        api_client()
            .autosaves()
            .filter_list_with_edit_context(&PostEndpointType::Pages, &autosaved_page_id(), fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|autosave| {
                autosave.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_any_post_revision_field_with_embed_context_test_cases)]
    #[case(&[SparseAnyPostRevisionFieldWithEmbedContext::Id, SparseAnyPostRevisionFieldWithEmbedContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_embed_context(
        #[case] fields: &[SparseAnyPostRevisionFieldWithEmbedContext],
    ) {
        api_client()
            .autosaves()
            .filter_list_with_embed_context(&PostEndpointType::Pages, &autosaved_page_id(), fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|autosave| {
                autosave.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_any_post_revision_field_with_view_context_test_cases)]
    #[case(&[SparseAnyPostRevisionFieldWithViewContext::Id, SparseAnyPostRevisionFieldWithViewContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_view_context(
        #[case] fields: &[SparseAnyPostRevisionFieldWithViewContext],
    ) {
        api_client()
            .autosaves()
            .filter_list_with_view_context(&PostEndpointType::Pages, &autosaved_page_id(), fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|autosave| {
                autosave.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_any_post_revision_field_with_edit_context_test_cases)]
    #[case(&[SparseAnyPostRevisionFieldWithEditContext::Id, SparseAnyPostRevisionFieldWithEditContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_edit_context(
        #[case] fields: &[SparseAnyPostRevisionFieldWithEditContext],
    ) {
        api_client()
            .autosaves()
            .filter_retrieve_with_edit_context(
                &PostEndpointType::Pages,
                &autosaved_page_id(),
                &autosave_id_for_autosaved_page_id(),
                fields,
            )
            .await
            .assert_response()
            .data
            .assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_any_post_revision_field_with_embed_context_test_cases)]
    #[case(&[SparseAnyPostRevisionFieldWithEmbedContext::Id, SparseAnyPostRevisionFieldWithEmbedContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_embed_context(
        #[case] fields: &[SparseAnyPostRevisionFieldWithEmbedContext],
    ) {
        api_client()
            .autosaves()
            .filter_retrieve_with_embed_context(
                &PostEndpointType::Pages,
                &autosaved_page_id(),
                &autosave_id_for_autosaved_page_id(),
                fields,
            )
            .await
            .assert_response()
            .data
            .assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_any_post_revision_field_with_view_context_test_cases)]
    #[case(&[SparseAnyPostRevisionFieldWithViewContext::Id, SparseAnyPostRevisionFieldWithViewContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_view_context(
        #[case] fields: &[SparseAnyPostRevisionFieldWithViewContext],
    ) {
        api_client()
            .autosaves()
            .filter_retrieve_with_view_context(
                &PostEndpointType::Pages,
                &autosaved_page_id(),
                &autosave_id_for_autosaved_page_id(),
                fields,
            )
            .await
            .assert_response()
            .data
            .assert_that_instance_fields_nullability_match_provided_fields(fields);
    }
}

use wp_api::{
    navigation_revisions::{
        NavigationRevisionId, SparseNavigationRevisionFieldWithEditContext,
        SparseNavigationRevisionFieldWithEmbedContext,
        SparseNavigationRevisionFieldWithViewContext,
    },
    navigations::NavigationId,
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn list_with_edit_context() {
    api_client()
        .navigation_autosaves()
        .list_with_edit_context(&autosaved_navigation_id())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn list_with_embed_context() {
    api_client()
        .navigation_autosaves()
        .list_with_embed_context(&autosaved_navigation_id())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn list_with_view_context() {
    api_client()
        .navigation_autosaves()
        .list_with_view_context(&autosaved_navigation_id())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_edit_context() {
    api_client()
        .navigation_autosaves()
        .retrieve_with_edit_context(
            &autosaved_navigation_id(),
            &autosave_id_for_autosaved_navigation_id(),
        )
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_embed_context() {
    api_client()
        .navigation_autosaves()
        .retrieve_with_embed_context(
            &autosaved_navigation_id(),
            &autosave_id_for_autosaved_navigation_id(),
        )
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_view_context() {
    api_client()
        .navigation_autosaves()
        .retrieve_with_view_context(
            &autosaved_navigation_id(),
            &autosave_id_for_autosaved_navigation_id(),
        )
        .await
        .assert_response();
}

fn autosaved_navigation_id() -> NavigationId {
    NavigationId(TestCredentials::instance().autosaved_navigation_id)
}

fn autosave_id_for_autosaved_navigation_id() -> NavigationRevisionId {
    NavigationRevisionId(TestCredentials::instance().autosave_id_for_autosaved_navigation_id)
}

mod filter {
    use super::*;

    wp_api::generate_sparse_navigation_revision_field_with_edit_context_test_cases!();
    wp_api::generate_sparse_navigation_revision_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_navigation_revision_field_with_view_context_test_cases!();

    #[apply(sparse_navigation_revision_field_with_edit_context_test_cases)]
    #[case(&[SparseNavigationRevisionFieldWithEditContext::Id, SparseNavigationRevisionFieldWithEditContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_edit_context(
        #[case] fields: &[SparseNavigationRevisionFieldWithEditContext],
    ) {
        api_client()
            .navigation_autosaves()
            .filter_list_with_edit_context(&autosaved_navigation_id(), fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|autosave| {
                autosave.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_navigation_revision_field_with_embed_context_test_cases)]
    #[case(&[SparseNavigationRevisionFieldWithEmbedContext::Id, SparseNavigationRevisionFieldWithEmbedContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_embed_context(
        #[case] fields: &[SparseNavigationRevisionFieldWithEmbedContext],
    ) {
        api_client()
            .navigation_autosaves()
            .filter_list_with_embed_context(&autosaved_navigation_id(), fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|autosave| {
                autosave.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_navigation_revision_field_with_view_context_test_cases)]
    #[case(&[SparseNavigationRevisionFieldWithViewContext::Id, SparseNavigationRevisionFieldWithViewContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_view_context(
        #[case] fields: &[SparseNavigationRevisionFieldWithViewContext],
    ) {
        api_client()
            .navigation_autosaves()
            .filter_list_with_view_context(&autosaved_navigation_id(), fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|autosave| {
                autosave.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_navigation_revision_field_with_edit_context_test_cases)]
    #[case(&[SparseNavigationRevisionFieldWithEditContext::Id, SparseNavigationRevisionFieldWithEditContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_edit_context(
        #[case] fields: &[SparseNavigationRevisionFieldWithEditContext],
    ) {
        api_client()
            .navigation_autosaves()
            .filter_retrieve_with_edit_context(
                &autosaved_navigation_id(),
                &autosave_id_for_autosaved_navigation_id(),
                fields,
            )
            .await
            .assert_response()
            .data
            .assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_navigation_revision_field_with_embed_context_test_cases)]
    #[case(&[SparseNavigationRevisionFieldWithEmbedContext::Id, SparseNavigationRevisionFieldWithEmbedContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_embed_context(
        #[case] fields: &[SparseNavigationRevisionFieldWithEmbedContext],
    ) {
        api_client()
            .navigation_autosaves()
            .filter_retrieve_with_embed_context(
                &autosaved_navigation_id(),
                &autosave_id_for_autosaved_navigation_id(),
                fields,
            )
            .await
            .assert_response()
            .data
            .assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_navigation_revision_field_with_view_context_test_cases)]
    #[case(&[SparseNavigationRevisionFieldWithViewContext::Id, SparseNavigationRevisionFieldWithViewContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_view_context(
        #[case] fields: &[SparseNavigationRevisionFieldWithViewContext],
    ) {
        api_client()
            .navigation_autosaves()
            .filter_retrieve_with_view_context(
                &autosaved_navigation_id(),
                &autosave_id_for_autosaved_navigation_id(),
                fields,
            )
            .await
            .assert_response()
            .data
            .assert_that_instance_fields_nullability_match_provided_fields(fields);
    }
}

use wp_api::nav_menu_item_revisions::{
    NavMenuItemRevisionId, SparseNavMenuItemRevisionFieldWithEditContext,
    SparseNavMenuItemRevisionFieldWithEmbedContext, SparseNavMenuItemRevisionFieldWithViewContext,
};
use wp_api::nav_menu_items::NavMenuItemId;
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn list_with_edit_context() {
    let test_credentials = TestCredentials::instance();
    let response = api_client()
        .nav_menu_item_autosaves()
        .list_with_edit_context(&NavMenuItemId(test_credentials.nav_menu_item_id))
        .await
        .assert_response();
    assert!(!response.data.is_empty());
}

#[tokio::test]
#[parallel]
async fn list_with_embed_context() {
    let test_credentials = TestCredentials::instance();
    api_client()
        .nav_menu_item_autosaves()
        .list_with_embed_context(&NavMenuItemId(test_credentials.nav_menu_item_id))
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn list_with_view_context() {
    let test_credentials = TestCredentials::instance();
    api_client()
        .nav_menu_item_autosaves()
        .list_with_view_context(&NavMenuItemId(test_credentials.nav_menu_item_id))
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_edit_context() {
    let test_credentials = TestCredentials::instance();
    api_client()
        .nav_menu_item_autosaves()
        .retrieve_with_edit_context(
            &NavMenuItemId(test_credentials.nav_menu_item_id),
            &NavMenuItemRevisionId(test_credentials.autosave_id_for_nav_menu_item_id),
        )
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_embed_context() {
    let test_credentials = TestCredentials::instance();
    api_client()
        .nav_menu_item_autosaves()
        .retrieve_with_embed_context(
            &NavMenuItemId(test_credentials.nav_menu_item_id),
            &NavMenuItemRevisionId(test_credentials.autosave_id_for_nav_menu_item_id),
        )
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_view_context() {
    let test_credentials = TestCredentials::instance();
    api_client()
        .nav_menu_item_autosaves()
        .retrieve_with_view_context(
            &NavMenuItemId(test_credentials.nav_menu_item_id),
            &NavMenuItemRevisionId(test_credentials.autosave_id_for_nav_menu_item_id),
        )
        .await
        .assert_response();
}

mod filter {
    use super::*;

    wp_api::generate_sparse_nav_menu_item_revision_field_with_edit_context_test_cases!();
    wp_api::generate_sparse_nav_menu_item_revision_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_nav_menu_item_revision_field_with_view_context_test_cases!();

    #[apply(sparse_nav_menu_item_revision_field_with_edit_context_test_cases)]
    #[case(&[SparseNavMenuItemRevisionFieldWithEditContext::Id, SparseNavMenuItemRevisionFieldWithEditContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_nav_menu_item_autosaves_with_edit_context(
        #[case] fields: &[SparseNavMenuItemRevisionFieldWithEditContext],
    ) {
        let test_credentials = TestCredentials::instance();
        api_client()
            .nav_menu_item_autosaves()
            .filter_list_with_edit_context(
                &NavMenuItemId(test_credentials.nav_menu_item_id),
                fields,
            )
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|autosave| {
                autosave.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_nav_menu_item_revision_field_with_edit_context_test_cases)]
    #[case(&[SparseNavMenuItemRevisionFieldWithEditContext::Id, SparseNavMenuItemRevisionFieldWithEditContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_nav_menu_item_autosaves_with_edit_context(
        #[case] fields: &[SparseNavMenuItemRevisionFieldWithEditContext],
    ) {
        let test_credentials = TestCredentials::instance();
        let autosave = api_client()
            .nav_menu_item_autosaves()
            .filter_retrieve_with_edit_context(
                &NavMenuItemId(test_credentials.nav_menu_item_id),
                &NavMenuItemRevisionId(test_credentials.autosave_id_for_nav_menu_item_id),
                fields,
            )
            .await
            .assert_response()
            .data;
        autosave.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }

    #[apply(sparse_nav_menu_item_revision_field_with_embed_context_test_cases)]
    #[case(&[SparseNavMenuItemRevisionFieldWithEmbedContext::Id, SparseNavMenuItemRevisionFieldWithEmbedContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_nav_menu_item_autosaves_with_embed_context(
        #[case] fields: &[SparseNavMenuItemRevisionFieldWithEmbedContext],
    ) {
        let test_credentials = TestCredentials::instance();
        api_client()
            .nav_menu_item_autosaves()
            .filter_list_with_embed_context(
                &NavMenuItemId(test_credentials.nav_menu_item_id),
                fields,
            )
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|autosave| {
                autosave.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_nav_menu_item_revision_field_with_embed_context_test_cases)]
    #[case(&[SparseNavMenuItemRevisionFieldWithEmbedContext::Id, SparseNavMenuItemRevisionFieldWithEmbedContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_nav_menu_item_autosaves_with_embed_context(
        #[case] fields: &[SparseNavMenuItemRevisionFieldWithEmbedContext],
    ) {
        let test_credentials = TestCredentials::instance();
        let autosave = api_client()
            .nav_menu_item_autosaves()
            .filter_retrieve_with_embed_context(
                &NavMenuItemId(test_credentials.nav_menu_item_id),
                &NavMenuItemRevisionId(test_credentials.autosave_id_for_nav_menu_item_id),
                fields,
            )
            .await
            .assert_response()
            .data;
        autosave.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }

    #[apply(sparse_nav_menu_item_revision_field_with_view_context_test_cases)]
    #[case(&[SparseNavMenuItemRevisionFieldWithViewContext::Id, SparseNavMenuItemRevisionFieldWithViewContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_nav_menu_item_autosaves_with_view_context(
        #[case] fields: &[SparseNavMenuItemRevisionFieldWithViewContext],
    ) {
        let test_credentials = TestCredentials::instance();
        api_client()
            .nav_menu_item_autosaves()
            .filter_list_with_view_context(
                &NavMenuItemId(test_credentials.nav_menu_item_id),
                fields,
            )
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|autosave| {
                autosave.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_nav_menu_item_revision_field_with_view_context_test_cases)]
    #[case(&[SparseNavMenuItemRevisionFieldWithViewContext::Id, SparseNavMenuItemRevisionFieldWithViewContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_nav_menu_item_autosaves_with_view_context(
        #[case] fields: &[SparseNavMenuItemRevisionFieldWithViewContext],
    ) {
        let test_credentials = TestCredentials::instance();
        let autosave = api_client()
            .nav_menu_item_autosaves()
            .filter_retrieve_with_view_context(
                &NavMenuItemId(test_credentials.nav_menu_item_id),
                &NavMenuItemRevisionId(test_credentials.autosave_id_for_nav_menu_item_id),
                fields,
            )
            .await
            .assert_response()
            .data;
        autosave.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }
}

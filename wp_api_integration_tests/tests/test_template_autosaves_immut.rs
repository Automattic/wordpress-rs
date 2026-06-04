use wp_api::{
    template_autosaves::{
        SparseTemplateAutosaveFieldWithEditContext, SparseTemplateAutosaveFieldWithEmbedContext,
        SparseTemplateAutosaveFieldWithViewContext, TemplateAutosaveId,
    },
    templates::TemplateId,
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn list_with_edit_context() {
    api_client()
        .template_autosaves()
        .list_with_edit_context(&autosaved_template_id())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn list_with_embed_context() {
    api_client()
        .template_autosaves()
        .list_with_embed_context(&autosaved_template_id())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn list_with_view_context() {
    api_client()
        .template_autosaves()
        .list_with_view_context(&autosaved_template_id())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_edit_context() {
    api_client()
        .template_autosaves()
        .retrieve_with_edit_context(&autosaved_template_id(), &autosave_id())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_embed_context() {
    api_client()
        .template_autosaves()
        .retrieve_with_embed_context(&autosaved_template_id(), &autosave_id())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_view_context() {
    api_client()
        .template_autosaves()
        .retrieve_with_view_context(&autosaved_template_id(), &autosave_id())
        .await
        .assert_response();
}

fn autosaved_template_id() -> TemplateId {
    TemplateId(
        TestCredentials::instance()
            .autosaved_template_id
            .to_string(),
    )
}

fn autosave_id() -> TemplateAutosaveId {
    TemplateAutosaveId(TestCredentials::instance().autosave_id_for_autosaved_template)
}

mod filter {
    use super::*;

    wp_api::generate_sparse_template_autosave_field_with_edit_context_test_cases!();
    wp_api::generate_sparse_template_autosave_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_template_autosave_field_with_view_context_test_cases!();

    #[apply(sparse_template_autosave_field_with_edit_context_test_cases)]
    #[case(&[SparseTemplateAutosaveFieldWithEditContext::Id, SparseTemplateAutosaveFieldWithEditContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_edit_context(
        #[case] fields: &[SparseTemplateAutosaveFieldWithEditContext],
    ) {
        api_client()
            .template_autosaves()
            .filter_list_with_edit_context(&autosaved_template_id(), fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|autosave| {
                autosave.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_template_autosave_field_with_embed_context_test_cases)]
    #[case(&[SparseTemplateAutosaveFieldWithEmbedContext::Id, SparseTemplateAutosaveFieldWithEmbedContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_embed_context(
        #[case] fields: &[SparseTemplateAutosaveFieldWithEmbedContext],
    ) {
        api_client()
            .template_autosaves()
            .filter_list_with_embed_context(&autosaved_template_id(), fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|autosave| {
                autosave.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_template_autosave_field_with_view_context_test_cases)]
    #[case(&[SparseTemplateAutosaveFieldWithViewContext::Id, SparseTemplateAutosaveFieldWithViewContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_view_context(
        #[case] fields: &[SparseTemplateAutosaveFieldWithViewContext],
    ) {
        api_client()
            .template_autosaves()
            .filter_list_with_view_context(&autosaved_template_id(), fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|autosave| {
                autosave.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_template_autosave_field_with_edit_context_test_cases)]
    #[case(&[SparseTemplateAutosaveFieldWithEditContext::Id, SparseTemplateAutosaveFieldWithEditContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_edit_context(
        #[case] fields: &[SparseTemplateAutosaveFieldWithEditContext],
    ) {
        api_client()
            .template_autosaves()
            .filter_retrieve_with_edit_context(&autosaved_template_id(), &autosave_id(), fields)
            .await
            .assert_response()
            .data
            .assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_template_autosave_field_with_embed_context_test_cases)]
    #[case(&[SparseTemplateAutosaveFieldWithEmbedContext::Id, SparseTemplateAutosaveFieldWithEmbedContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_embed_context(
        #[case] fields: &[SparseTemplateAutosaveFieldWithEmbedContext],
    ) {
        api_client()
            .template_autosaves()
            .filter_retrieve_with_embed_context(&autosaved_template_id(), &autosave_id(), fields)
            .await
            .assert_response()
            .data
            .assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_template_autosave_field_with_view_context_test_cases)]
    #[case(&[SparseTemplateAutosaveFieldWithViewContext::Id, SparseTemplateAutosaveFieldWithViewContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_view_context(
        #[case] fields: &[SparseTemplateAutosaveFieldWithViewContext],
    ) {
        api_client()
            .template_autosaves()
            .filter_retrieve_with_view_context(&autosaved_template_id(), &autosave_id(), fields)
            .await
            .assert_response()
            .data
            .assert_that_instance_fields_nullability_match_provided_fields(fields);
    }
}

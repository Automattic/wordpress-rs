use wp_api::{
    template_part_autosaves::{
        SparseTemplatePartAutosaveFieldWithEditContext,
        SparseTemplatePartAutosaveFieldWithEmbedContext,
        SparseTemplatePartAutosaveFieldWithViewContext, TemplatePartAutosaveId,
    },
    template_parts::TemplatePartId,
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn list_with_edit_context() {
    api_client()
        .template_part_autosaves()
        .list_with_edit_context(&autosaved_template_part_id())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn list_with_embed_context() {
    api_client()
        .template_part_autosaves()
        .list_with_embed_context(&autosaved_template_part_id())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn list_with_view_context() {
    api_client()
        .template_part_autosaves()
        .list_with_view_context(&autosaved_template_part_id())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_edit_context() {
    api_client()
        .template_part_autosaves()
        .retrieve_with_edit_context(&autosaved_template_part_id(), &autosave_id())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_embed_context() {
    api_client()
        .template_part_autosaves()
        .retrieve_with_embed_context(&autosaved_template_part_id(), &autosave_id())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_view_context() {
    api_client()
        .template_part_autosaves()
        .retrieve_with_view_context(&autosaved_template_part_id(), &autosave_id())
        .await
        .assert_response();
}

fn autosaved_template_part_id() -> TemplatePartId {
    TemplatePartId(
        TestCredentials::instance()
            .autosaved_template_part_id
            .to_string(),
    )
}

fn autosave_id() -> TemplatePartAutosaveId {
    TemplatePartAutosaveId(TestCredentials::instance().autosave_id_for_autosaved_template_part)
}

mod filter {
    use super::*;

    wp_api::generate_sparse_template_part_autosave_field_with_edit_context_test_cases!();
    wp_api::generate_sparse_template_part_autosave_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_template_part_autosave_field_with_view_context_test_cases!();

    #[apply(sparse_template_part_autosave_field_with_edit_context_test_cases)]
    #[case(&[SparseTemplatePartAutosaveFieldWithEditContext::Id, SparseTemplatePartAutosaveFieldWithEditContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_edit_context(
        #[case] fields: &[SparseTemplatePartAutosaveFieldWithEditContext],
    ) {
        api_client()
            .template_part_autosaves()
            .filter_list_with_edit_context(&autosaved_template_part_id(), fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|autosave| {
                autosave.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_template_part_autosave_field_with_embed_context_test_cases)]
    #[case(&[SparseTemplatePartAutosaveFieldWithEmbedContext::Id, SparseTemplatePartAutosaveFieldWithEmbedContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_embed_context(
        #[case] fields: &[SparseTemplatePartAutosaveFieldWithEmbedContext],
    ) {
        api_client()
            .template_part_autosaves()
            .filter_list_with_embed_context(&autosaved_template_part_id(), fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|autosave| {
                autosave.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_template_part_autosave_field_with_view_context_test_cases)]
    #[case(&[SparseTemplatePartAutosaveFieldWithViewContext::Id, SparseTemplatePartAutosaveFieldWithViewContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_view_context(
        #[case] fields: &[SparseTemplatePartAutosaveFieldWithViewContext],
    ) {
        api_client()
            .template_part_autosaves()
            .filter_list_with_view_context(&autosaved_template_part_id(), fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|autosave| {
                autosave.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_template_part_autosave_field_with_edit_context_test_cases)]
    #[case(&[SparseTemplatePartAutosaveFieldWithEditContext::Id, SparseTemplatePartAutosaveFieldWithEditContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_edit_context(
        #[case] fields: &[SparseTemplatePartAutosaveFieldWithEditContext],
    ) {
        api_client()
            .template_part_autosaves()
            .filter_retrieve_with_edit_context(
                &autosaved_template_part_id(),
                &autosave_id(),
                fields,
            )
            .await
            .assert_response()
            .data
            .assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_template_part_autosave_field_with_embed_context_test_cases)]
    #[case(&[SparseTemplatePartAutosaveFieldWithEmbedContext::Id, SparseTemplatePartAutosaveFieldWithEmbedContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_embed_context(
        #[case] fields: &[SparseTemplatePartAutosaveFieldWithEmbedContext],
    ) {
        api_client()
            .template_part_autosaves()
            .filter_retrieve_with_embed_context(
                &autosaved_template_part_id(),
                &autosave_id(),
                fields,
            )
            .await
            .assert_response()
            .data
            .assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_template_part_autosave_field_with_view_context_test_cases)]
    #[case(&[SparseTemplatePartAutosaveFieldWithViewContext::Id, SparseTemplatePartAutosaveFieldWithViewContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_view_context(
        #[case] fields: &[SparseTemplatePartAutosaveFieldWithViewContext],
    ) {
        api_client()
            .template_part_autosaves()
            .filter_retrieve_with_view_context(
                &autosaved_template_part_id(),
                &autosave_id(),
                fields,
            )
            .await
            .assert_response()
            .data
            .assert_that_instance_fields_nullability_match_provided_fields(fields);
    }
}

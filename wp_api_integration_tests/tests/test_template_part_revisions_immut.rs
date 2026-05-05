use wp_api::{
    template_part_revisions::{
        SparseTemplatePartRevisionFieldWithEditContext,
        SparseTemplatePartRevisionFieldWithEmbedContext,
        SparseTemplatePartRevisionFieldWithViewContext, TemplatePartRevisionId,
        TemplatePartRevisionListParams, WpApiParamTemplatePartRevisionsOrderBy,
    },
    template_parts::TemplatePartId,
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_edit_context(#[case] params: TemplatePartRevisionListParams) {
    api_client()
        .template_part_revisions()
        .list_with_edit_context(&template_part_id(), &params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_embed_context(#[case] params: TemplatePartRevisionListParams) {
    api_client()
        .template_part_revisions()
        .list_with_embed_context(&template_part_id(), &params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_view_context(#[case] params: TemplatePartRevisionListParams) {
    api_client()
        .template_part_revisions()
        .list_with_view_context(&template_part_id(), &params)
        .await
        .assert_response();
}

fn template_part_id() -> TemplatePartId {
    TemplatePartId(
        TestCredentials::instance()
            .integration_test_custom_template_part_id
            .to_string(),
    )
}

fn revision_id() -> TemplatePartRevisionId {
    TemplatePartRevisionId(TestCredentials::instance().revision_id_for_custom_template_part)
}

#[tokio::test]
#[parallel]
async fn retrieve_with_edit_context() {
    api_client()
        .template_part_revisions()
        .retrieve_with_edit_context(&template_part_id(), &revision_id())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_embed_context() {
    api_client()
        .template_part_revisions()
        .retrieve_with_embed_context(&template_part_id(), &revision_id())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_view_context() {
    api_client()
        .template_part_revisions()
        .retrieve_with_view_context(&template_part_id(), &revision_id())
        .await
        .assert_response();
}

#[template]
#[rstest]
#[case::default(TemplatePartRevisionListParams::default())]
#[case::page(generate!(TemplatePartRevisionListParams, (page, Some(1))))]
#[case::per_page(generate!(TemplatePartRevisionListParams, (per_page, Some(3))))]
#[case::search(generate!(TemplatePartRevisionListParams, (search, Some("foo".to_string()))))]
#[case::exclude(generate!(TemplatePartRevisionListParams, (exclude, vec![TemplatePartRevisionId(1), TemplatePartRevisionId(2)])))]
#[case::include(generate!(TemplatePartRevisionListParams, (include, vec![TemplatePartRevisionId(1)])))]
#[case::offset(generate!(TemplatePartRevisionListParams, (offset, Some(5))))]
#[case::order(generate!(TemplatePartRevisionListParams, (order, Some(WpApiParamOrder::Asc))))]
#[case::orderby(generate!(TemplatePartRevisionListParams, (orderby, Some(WpApiParamTemplatePartRevisionsOrderBy::Slug))))]
pub fn list_cases(#[case] params: TemplatePartRevisionListParams) {}

mod filter {
    use super::*;

    wp_api::generate_sparse_template_part_revision_field_with_edit_context_test_cases!();
    wp_api::generate_sparse_template_part_revision_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_template_part_revision_field_with_view_context_test_cases!();

    #[apply(sparse_template_part_revision_field_with_edit_context_test_cases)]
    #[case(&[SparseTemplatePartRevisionFieldWithEditContext::Id, SparseTemplatePartRevisionFieldWithEditContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_edit_context(
        #[case] fields: &[SparseTemplatePartRevisionFieldWithEditContext],
    ) {
        api_client()
            .template_part_revisions()
            .filter_retrieve_with_edit_context(&template_part_id(), &revision_id(), fields)
            .await
            .assert_response()
            .data
            .assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_template_part_revision_field_with_embed_context_test_cases)]
    #[case(&[SparseTemplatePartRevisionFieldWithEmbedContext::Id, SparseTemplatePartRevisionFieldWithEmbedContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_embed_context(
        #[case] fields: &[SparseTemplatePartRevisionFieldWithEmbedContext],
    ) {
        api_client()
            .template_part_revisions()
            .filter_retrieve_with_embed_context(&template_part_id(), &revision_id(), fields)
            .await
            .assert_response()
            .data
            .assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_template_part_revision_field_with_view_context_test_cases)]
    #[case(&[SparseTemplatePartRevisionFieldWithViewContext::Id, SparseTemplatePartRevisionFieldWithViewContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_view_context(
        #[case] fields: &[SparseTemplatePartRevisionFieldWithViewContext],
    ) {
        api_client()
            .template_part_revisions()
            .filter_retrieve_with_view_context(&template_part_id(), &revision_id(), fields)
            .await
            .assert_response()
            .data
            .assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_template_part_revision_field_with_edit_context_test_cases)]
    #[case(&[SparseTemplatePartRevisionFieldWithEditContext::Id, SparseTemplatePartRevisionFieldWithEditContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_edit_context(
        #[case] fields: &[SparseTemplatePartRevisionFieldWithEditContext],
        #[values(
            TemplatePartRevisionListParams::default(),
            generate!(TemplatePartRevisionListParams, (exclude, vec![TemplatePartRevisionId(2), TemplatePartRevisionId(3)])),
            generate!(TemplatePartRevisionListParams, (search, Some("foo".to_string())))
        )]
        params: TemplatePartRevisionListParams,
    ) {
        api_client()
            .template_part_revisions()
            .filter_list_with_edit_context(&template_part_id(), &params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|revision| {
                revision.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_template_part_revision_field_with_embed_context_test_cases)]
    #[case(&[SparseTemplatePartRevisionFieldWithEmbedContext::Id, SparseTemplatePartRevisionFieldWithEmbedContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_embed_context(
        #[case] fields: &[SparseTemplatePartRevisionFieldWithEmbedContext],
        #[values(
            TemplatePartRevisionListParams::default(),
            generate!(TemplatePartRevisionListParams, (exclude, vec![TemplatePartRevisionId(2), TemplatePartRevisionId(3)])),
            generate!(TemplatePartRevisionListParams, (search, Some("foo".to_string())))
        )]
        params: TemplatePartRevisionListParams,
    ) {
        api_client()
            .template_part_revisions()
            .filter_list_with_embed_context(&template_part_id(), &params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|revision| {
                revision.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_template_part_revision_field_with_view_context_test_cases)]
    #[case(&[SparseTemplatePartRevisionFieldWithViewContext::Id, SparseTemplatePartRevisionFieldWithViewContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_view_context(
        #[case] fields: &[SparseTemplatePartRevisionFieldWithViewContext],
        #[values(
            TemplatePartRevisionListParams::default(),
            generate!(TemplatePartRevisionListParams, (exclude, vec![TemplatePartRevisionId(2), TemplatePartRevisionId(3)])),
            generate!(TemplatePartRevisionListParams, (search, Some("foo".to_string())))
        )]
        params: TemplatePartRevisionListParams,
    ) {
        api_client()
            .template_part_revisions()
            .filter_list_with_view_context(&template_part_id(), &params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|revision| {
                revision.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }
}

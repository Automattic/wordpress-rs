use wp_api::{
    template_revisions::{
        SparseTemplateRevisionFieldWithEditContext, SparseTemplateRevisionFieldWithEmbedContext,
        SparseTemplateRevisionFieldWithViewContext, TemplateRevisionId, TemplateRevisionListParams,
        WpApiParamTemplateRevisionsOrderBy,
    },
    templates::TemplateId,
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_edit_context(#[case] params: TemplateRevisionListParams) {
    api_client()
        .template_revisions()
        .list_with_edit_context(&template_id(), &params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_embed_context(#[case] params: TemplateRevisionListParams) {
    api_client()
        .template_revisions()
        .list_with_embed_context(&template_id(), &params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_view_context(#[case] params: TemplateRevisionListParams) {
    api_client()
        .template_revisions()
        .list_with_view_context(&template_id(), &params)
        .await
        .assert_response();
}

fn template_id() -> TemplateId {
    TemplateId(
        TestCredentials::instance()
            .integration_test_custom_template_id
            .to_string(),
    )
}

#[template]
#[rstest]
#[case::default(TemplateRevisionListParams::default())]
#[case::page(generate!(TemplateRevisionListParams, (page, Some(1))))]
#[case::per_page(generate!(TemplateRevisionListParams, (per_page, Some(3))))]
#[case::search(generate!(TemplateRevisionListParams, (search, Some("foo".to_string()))))]
#[case::exclude(generate!(TemplateRevisionListParams, (exclude, vec![TemplateRevisionId(1), TemplateRevisionId(2)])))]
#[case::include(generate!(TemplateRevisionListParams, (include, vec![TemplateRevisionId(1)])))]
#[case::offset(generate!(TemplateRevisionListParams, (offset, Some(5))))]
#[case::order(generate!(TemplateRevisionListParams, (order, Some(WpApiParamOrder::Asc))))]
#[case::orderby(generate!(TemplateRevisionListParams, (orderby, Some(WpApiParamTemplateRevisionsOrderBy::Slug))))]
pub fn list_cases(#[case] params: TemplateRevisionListParams) {}

mod filter {
    use super::*;

    wp_api::generate_sparse_template_revision_field_with_edit_context_test_cases!();
    wp_api::generate_sparse_template_revision_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_template_revision_field_with_view_context_test_cases!();

    #[apply(sparse_template_revision_field_with_edit_context_test_cases)]
    #[case(&[SparseTemplateRevisionFieldWithEditContext::Id, SparseTemplateRevisionFieldWithEditContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_edit_context(
        #[case] fields: &[SparseTemplateRevisionFieldWithEditContext],
        #[values(
            TemplateRevisionListParams::default(),
            generate!(TemplateRevisionListParams, (exclude, vec![TemplateRevisionId(2), TemplateRevisionId(3)])),
            generate!(TemplateRevisionListParams, (search, Some("foo".to_string())))
        )]
        params: TemplateRevisionListParams,
    ) {
        api_client()
            .template_revisions()
            .filter_list_with_edit_context(&template_id(), &params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|revision| {
                revision.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_template_revision_field_with_embed_context_test_cases)]
    #[case(&[SparseTemplateRevisionFieldWithEmbedContext::Id, SparseTemplateRevisionFieldWithEmbedContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_embed_context(
        #[case] fields: &[SparseTemplateRevisionFieldWithEmbedContext],
        #[values(
            TemplateRevisionListParams::default(),
            generate!(TemplateRevisionListParams, (exclude, vec![TemplateRevisionId(2), TemplateRevisionId(3)])),
            generate!(TemplateRevisionListParams, (search, Some("foo".to_string())))
        )]
        params: TemplateRevisionListParams,
    ) {
        api_client()
            .template_revisions()
            .filter_list_with_embed_context(&template_id(), &params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|revision| {
                revision.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_template_revision_field_with_view_context_test_cases)]
    #[case(&[SparseTemplateRevisionFieldWithViewContext::Id, SparseTemplateRevisionFieldWithViewContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_view_context(
        #[case] fields: &[SparseTemplateRevisionFieldWithViewContext],
        #[values(
            TemplateRevisionListParams::default(),
            generate!(TemplateRevisionListParams, (exclude, vec![TemplateRevisionId(2), TemplateRevisionId(3)])),
            generate!(TemplateRevisionListParams, (search, Some("foo".to_string())))
        )]
        params: TemplateRevisionListParams,
    ) {
        api_client()
            .template_revisions()
            .filter_list_with_view_context(&template_id(), &params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|revision| {
                revision.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }
}

use wp_api::{
    navigation_revisions::{
        NavigationRevisionId, NavigationRevisionListParams,
        SparseNavigationRevisionFieldWithEditContext,
        SparseNavigationRevisionFieldWithEmbedContext,
        SparseNavigationRevisionFieldWithViewContext, WpApiParamNavigationRevisionsOrderBy,
    },
    navigations::NavigationId,
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_edit_context(#[case] params: NavigationRevisionListParams) {
    api_client()
        .navigation_revisions()
        .list_with_edit_context(&navigation_id(), &params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_embed_context(#[case] params: NavigationRevisionListParams) {
    api_client()
        .navigation_revisions()
        .list_with_embed_context(&navigation_id(), &params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_view_context(#[case] params: NavigationRevisionListParams) {
    api_client()
        .navigation_revisions()
        .list_with_view_context(&navigation_id(), &params)
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_edit_context() {
    api_client()
        .navigation_revisions()
        .retrieve_with_edit_context(&navigation_id(), &revision_id_for_navigation_id())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_embed_context() {
    api_client()
        .navigation_revisions()
        .retrieve_with_embed_context(&navigation_id(), &revision_id_for_navigation_id())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_view_context() {
    api_client()
        .navigation_revisions()
        .retrieve_with_view_context(&navigation_id(), &revision_id_for_navigation_id())
        .await
        .assert_response();
}

fn navigation_id() -> NavigationId {
    NavigationId(TestCredentials::instance().navigation_id)
}

fn revision_id_for_navigation_id() -> NavigationRevisionId {
    NavigationRevisionId(TestCredentials::instance().revision_id_for_navigation_id)
}

#[template]
#[rstest]
#[case::default(NavigationRevisionListParams::default())]
#[case::page(generate!(NavigationRevisionListParams, (page, Some(1))))]
#[case::per_page(generate!(NavigationRevisionListParams, (per_page, Some(3))))]
#[case::search(generate!(NavigationRevisionListParams, (search, Some("foo".to_string()))))]
#[case::exclude(generate!(NavigationRevisionListParams, (exclude, vec![NavigationRevisionId(1), NavigationRevisionId(2)])))]
#[case::include(generate!(NavigationRevisionListParams, (include, vec![NavigationRevisionId(1)])))]
#[case::offset(generate!(NavigationRevisionListParams, (offset, Some(5))))]
#[case::order(generate!(NavigationRevisionListParams, (order, Some(WpApiParamOrder::Asc))))]
#[case::orderby(generate!(NavigationRevisionListParams, (orderby, Some(WpApiParamNavigationRevisionsOrderBy::Slug))))]
fn list_cases(#[case] params: NavigationRevisionListParams) {}

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
        #[values(
            NavigationRevisionListParams::default(),
            generate!(NavigationRevisionListParams, (exclude, vec![NavigationRevisionId(2), NavigationRevisionId(3)])),
            generate!(NavigationRevisionListParams, (search, Some("foo".to_string())))
        )]
        params: NavigationRevisionListParams,
    ) {
        api_client()
            .navigation_revisions()
            .filter_list_with_edit_context(&navigation_id(), &params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|post| {
                post.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_navigation_revision_field_with_embed_context_test_cases)]
    #[case(&[SparseNavigationRevisionFieldWithEmbedContext::Id, SparseNavigationRevisionFieldWithEmbedContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_embed_context(
        #[case] fields: &[SparseNavigationRevisionFieldWithEmbedContext],
        #[values(
            NavigationRevisionListParams::default(),
            generate!(NavigationRevisionListParams, (exclude, vec![NavigationRevisionId(2), NavigationRevisionId(3)])),
            generate!(NavigationRevisionListParams, (search, Some("foo".to_string())))
        )]
        params: NavigationRevisionListParams,
    ) {
        api_client()
            .navigation_revisions()
            .filter_list_with_embed_context(&navigation_id(), &params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|post| {
                post.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_navigation_revision_field_with_view_context_test_cases)]
    #[case(&[SparseNavigationRevisionFieldWithViewContext::Id, SparseNavigationRevisionFieldWithViewContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_view_context(
        #[case] fields: &[SparseNavigationRevisionFieldWithViewContext],
        #[values(
            NavigationRevisionListParams::default(),
            generate!(NavigationRevisionListParams, (exclude, vec![NavigationRevisionId(2), NavigationRevisionId(3)])),
            generate!(NavigationRevisionListParams, (search, Some("foo".to_string())))
        )]
        params: NavigationRevisionListParams,
    ) {
        api_client()
            .navigation_revisions()
            .filter_list_with_view_context(&navigation_id(), &params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|post| {
                post.assert_that_instance_fields_nullability_match_provided_fields(fields)
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
            .navigation_revisions()
            .filter_retrieve_with_edit_context(
                &navigation_id(),
                &revision_id_for_navigation_id(),
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
            .navigation_revisions()
            .filter_retrieve_with_embed_context(
                &navigation_id(),
                &revision_id_for_navigation_id(),
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
            .navigation_revisions()
            .filter_retrieve_with_view_context(
                &navigation_id(),
                &revision_id_for_navigation_id(),
                fields,
            )
            .await
            .assert_response()
            .data
            .assert_that_instance_fields_nullability_match_provided_fields(fields);
    }
}

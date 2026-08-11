use wp_api::date::WpDateString;
use wp_api::navigations::{
    NavigationId, NavigationListParams, NavigationRetrieveParams, NavigationStatus,
    SparseNavigationFieldWithEditContext, SparseNavigationFieldWithEmbedContext,
    SparseNavigationFieldWithViewContext, WpApiParamNavigationsOrderBy,
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_edit_context(#[case] params: NavigationListParams) {
    api_client()
        .navigations()
        .list_with_edit_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_embed_context(#[case] params: NavigationListParams) {
    api_client()
        .navigations()
        .list_with_embed_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_view_context(#[case] params: NavigationListParams) {
    api_client()
        .navigations()
        .list_with_view_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_edit_context() {
    api_client()
        .navigations()
        .retrieve_with_edit_context(&navigation_id(), &NavigationRetrieveParams::default())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_embed_context() {
    api_client()
        .navigations()
        .retrieve_with_embed_context(&navigation_id(), &NavigationRetrieveParams::default())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_view_context() {
    api_client()
        .navigations()
        .retrieve_with_view_context(&navigation_id(), &NavigationRetrieveParams::default())
        .await
        .assert_response();
}

#[tokio::test]
#[rstest]
#[parallel]
#[case(NavigationListParams { per_page: Some(1), ..Default::default() })]
#[case(NavigationListParams { per_page: Some(1), order: Some(WpApiParamOrder::Desc), ..Default::default() })]
#[case(NavigationListParams { per_page: Some(1), order_by: Some(WpApiParamNavigationsOrderBy::Modified), ..Default::default() })]
async fn paginate_list_navigations_with_edit_context(#[case] params: NavigationListParams) {
    let first_page_response = api_client()
        .navigations()
        .list_with_edit_context(&params)
        .await
        .assert_response();
    assert!(!first_page_response.data.is_empty());
    let next_page_params = first_page_response.next_page_params.unwrap();
    let next_page_response = api_client()
        .navigations()
        .list_with_edit_context(&next_page_params)
        .await
        .assert_response();
    assert!(!next_page_response.data.is_empty());
    let prev_page_params = next_page_response.prev_page_params.unwrap();
    let prev_page_response = api_client()
        .navigations()
        .list_with_edit_context(&prev_page_params)
        .await
        .assert_response();
    assert!(!prev_page_response.data.is_empty());
}

fn navigation_id() -> NavigationId {
    NavigationId(TestCredentials::instance().navigation_id)
}

#[template]
#[rstest]
#[case::default(NavigationListParams::default())]
#[case::page(generate!(NavigationListParams, (page, Some(1))))]
#[case::per_page(generate!(NavigationListParams, (per_page, Some(3))))]
#[case::search(generate!(NavigationListParams, (search, Some("foo".to_string()))))]
#[case::after(generate!(NavigationListParams, (after, Some(WpDateString("2020-08-14T17:00:00".to_string())))))]
#[case::modified_after(generate!(NavigationListParams, (modified_after, Some(WpDateString("2024-01-14T17:00:00".to_string())))))]
#[case::before(generate!(NavigationListParams, (before, Some(WpDateString("2023-08-14T17:00:00".to_string())))))]
#[case::modified_before(generate!(NavigationListParams, (modified_before, Some(WpDateString("2024-01-14T17:00:00".to_string())))))]
#[case::exclude(generate!(NavigationListParams, (exclude, vec![NavigationId(1), NavigationId(2)])))]
#[case::include(generate!(NavigationListParams, (include, vec![NavigationId(1)])))]
#[case::offset(generate!(NavigationListParams, (offset, Some(2))))]
#[case::order(generate!(NavigationListParams, (order, Some(WpApiParamOrder::Asc))))]
#[case::orderby(generate!(NavigationListParams, (order_by, Some(WpApiParamNavigationsOrderBy::Id))))]
#[case::search_columns(generate!(NavigationListParams, (search_columns, vec!["post_content".to_string(), "post_excerpt".to_string()])))]
#[case::slug(generate!(NavigationListParams, (slug, vec!["foo".to_string(), "bar".to_string()])))]
#[case::status(generate!(NavigationListParams, (status, vec![NavigationStatus::Publish, NavigationStatus::Draft])))]
fn list_cases(#[case] params: NavigationListParams) {}

mod filter {
    use super::*;

    wp_api::generate_sparse_navigation_field_with_edit_context_test_cases!();
    wp_api::generate_sparse_navigation_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_navigation_field_with_view_context_test_cases!();

    #[apply(sparse_navigation_field_with_edit_context_test_cases)]
    #[case(&[SparseNavigationFieldWithEditContext::Id, SparseNavigationFieldWithEditContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_navigations_with_edit_context(
        #[case] fields: &[SparseNavigationFieldWithEditContext],
        #[values(
            NavigationListParams::default(),
            generate!(NavigationListParams, (status, vec![NavigationStatus::Draft, NavigationStatus::Publish])),
            generate!(NavigationListParams, (search, Some("foo".to_string())))
        )]
        params: NavigationListParams,
    ) {
        api_client()
            .navigations()
            .filter_list_with_edit_context(&params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|navigation| {
                navigation.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_navigation_field_with_edit_context_test_cases)]
    #[case(&[SparseNavigationFieldWithEditContext::Id, SparseNavigationFieldWithEditContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_navigations_with_edit_context(
        #[case] fields: &[SparseNavigationFieldWithEditContext],
    ) {
        let navigation = api_client()
            .navigations()
            .filter_retrieve_with_edit_context(
                &navigation_id(),
                &NavigationRetrieveParams::default(),
                fields,
            )
            .await
            .assert_response()
            .data;
        navigation.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }

    #[apply(sparse_navigation_field_with_embed_context_test_cases)]
    #[case(&[SparseNavigationFieldWithEmbedContext::Id, SparseNavigationFieldWithEmbedContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_navigations_with_embed_context(
        #[case] fields: &[SparseNavigationFieldWithEmbedContext],
        #[values(
            NavigationListParams::default(),
            generate!(NavigationListParams, (status, vec![NavigationStatus::Draft, NavigationStatus::Publish])),
            generate!(NavigationListParams, (search, Some("foo".to_string())))
        )]
        params: NavigationListParams,
    ) {
        api_client()
            .navigations()
            .filter_list_with_embed_context(&params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|navigation| {
                navigation.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_navigation_field_with_embed_context_test_cases)]
    #[case(&[SparseNavigationFieldWithEmbedContext::Id, SparseNavigationFieldWithEmbedContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_navigations_with_embed_context(
        #[case] fields: &[SparseNavigationFieldWithEmbedContext],
    ) {
        let navigation = api_client()
            .navigations()
            .filter_retrieve_with_embed_context(
                &navigation_id(),
                &NavigationRetrieveParams::default(),
                fields,
            )
            .await
            .assert_response()
            .data;
        navigation.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }

    #[apply(sparse_navigation_field_with_view_context_test_cases)]
    #[case(&[SparseNavigationFieldWithViewContext::Id, SparseNavigationFieldWithViewContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_navigations_with_view_context(
        #[case] fields: &[SparseNavigationFieldWithViewContext],
        #[values(
            NavigationListParams::default(),
            generate!(NavigationListParams, (status, vec![NavigationStatus::Draft, NavigationStatus::Publish])),
            generate!(NavigationListParams, (search, Some("foo".to_string())))
        )]
        params: NavigationListParams,
    ) {
        api_client()
            .navigations()
            .filter_list_with_view_context(&params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|navigation| {
                navigation.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_navigation_field_with_view_context_test_cases)]
    #[case(&[SparseNavigationFieldWithViewContext::Id, SparseNavigationFieldWithViewContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_navigations_with_view_context(
        #[case] fields: &[SparseNavigationFieldWithViewContext],
    ) {
        let navigation = api_client()
            .navigations()
            .filter_retrieve_with_view_context(
                &navigation_id(),
                &NavigationRetrieveParams::default(),
                fields,
            )
            .await
            .assert_response()
            .data;
        navigation.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }
}

use rstest::*;
use rstest_reuse::{self, apply, template};
use serial_test::parallel;
use wp_api::generate;
use wp_api::search_results::{
    SearchListParams, SearchResultSubtype, SearchResultType,
    SparseSearchResultFieldWithEmbedContext, SparseSearchResultFieldWithViewContext,
};
use wp_api_integration_tests::{AssertResponse, api_client};

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_embed_context(#[case] params: SearchListParams) {
    api_client()
        .search()
        .list_with_embed_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_view_context(#[case] params: SearchListParams) {
    api_client()
        .search()
        .list_with_view_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[rstest]
#[parallel]
#[case(SearchListParams { per_page: Some(1), ..Default::default() })]
#[case(SearchListParams { per_page: Some(1), object_type: Some(SearchResultType::Post), ..Default::default() })]
#[case(SearchListParams { per_page: Some(1), object_type: Some(SearchResultType::Term), object_subtype: Some(SearchResultSubtype::Category), ..Default::default() })]
async fn paginate_list_search_with_view_context(#[case] params: SearchListParams) {
    let first_page_response = api_client()
        .search()
        .list_with_view_context(&params)
        .await
        .assert_response();
    assert!(!first_page_response.data.is_empty());
    let next_page_params = first_page_response.next_page_params.unwrap();
    let next_page_response = api_client()
        .search()
        .list_with_view_context(&next_page_params)
        .await
        .assert_response();
    assert!(!next_page_response.data.is_empty());
    let prev_page_params = next_page_response.prev_page_params.unwrap();
    let prev_page_response = api_client()
        .search()
        .list_with_view_context(&prev_page_params)
        .await
        .assert_response();
    assert!(!prev_page_response.data.is_empty());
}

#[template]
#[rstest]
#[case::default(SearchListParams::default())]
#[case(generate!(SearchListParams, (page, Some(2))))]
#[case(generate!(SearchListParams, (per_page, Some(2))))]
#[case(generate!(SearchListParams, (search, Some("foo".to_string()))))]
#[case(generate!(SearchListParams, (object_type, Some(SearchResultType::Post))))]
#[case(generate!(SearchListParams, (object_type, Some(SearchResultType::Term))))]
#[case(generate!(SearchListParams, (object_type, Some(SearchResultType::PostFormat))))]
#[case(generate!(SearchListParams, (object_subtype, Some(SearchResultSubtype::Post))))]
#[case(generate!(SearchListParams, (object_subtype, Some(SearchResultSubtype::Page))))]
#[case(generate!(SearchListParams, (object_subtype, Some(SearchResultSubtype::Category))))]
#[case(generate!(SearchListParams, (object_subtype, Some(SearchResultSubtype::PostTag))))]
#[case(generate!(SearchListParams, (exclude, vec![1, 2])))]
#[case(generate!(SearchListParams, (include, vec![1, 2])))]
pub fn list_cases(#[case] params: SearchListParams) {}

mod filter {
    use super::*;

    wp_api::generate_sparse_search_result_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_search_result_field_with_view_context_test_cases!();

    #[apply(sparse_search_result_field_with_embed_context_test_cases)]
    #[case(&[SparseSearchResultFieldWithEmbedContext::Id, SparseSearchResultFieldWithEmbedContext::Title])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_embed_context(
        #[case] fields: &[SparseSearchResultFieldWithEmbedContext],
        #[values(
            SearchListParams::default(),
            generate!(SearchListParams, (page, Some(2))),
            generate!(SearchListParams, (search, Some("foo".to_string())))
        )]
        params: SearchListParams,
    ) {
        api_client()
            .search()
            .filter_list_with_embed_context(&params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|search_result| {
                search_result.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_search_result_field_with_view_context_test_cases)]
    #[case(&[SparseSearchResultFieldWithViewContext::Id, SparseSearchResultFieldWithViewContext::ObjectType])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_view_context(
        #[case] fields: &[SparseSearchResultFieldWithViewContext],
        #[values(
            SearchListParams::default(),
            generate!(SearchListParams, (page, Some(2))),
            generate!(SearchListParams, (search, Some("foo".to_string())))
        )]
        params: SearchListParams,
    ) {
        api_client()
            .search()
            .filter_list_with_view_context(&params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|search_result| {
                search_result.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }
}

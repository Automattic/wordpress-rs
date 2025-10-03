use wp_api::{
    posts::{
        PostListParams, PostRetrieveParams, PostStatus, SparseAnyPostFieldWithEditContext,
        SparseAnyPostFieldWithEmbedContext, SparseAnyPostFieldWithViewContext,
        WpApiParamPostsOrderBy, WpApiParamPostsSearchColumn,
    },
    request::endpoint::posts_endpoint::PostEndpointType,
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_edit_context(#[case] params: PostListParams) {
    api_client()
        .posts()
        .list_with_edit_context(&PostEndpointType::Navigation, &params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_embed_context(#[case] params: PostListParams) {
    api_client()
        .posts()
        .list_with_embed_context(&PostEndpointType::Navigation, &params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_view_context(#[case] params: PostListParams) {
    api_client()
        .posts()
        .list_with_view_context(&PostEndpointType::Navigation, &params)
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_edit_context() {
    api_client()
        .posts()
        .retrieve_with_edit_context(
            &PostEndpointType::Navigation,
            &navigation_id(),
            &PostRetrieveParams::default(),
        )
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_embed_context() {
    api_client()
        .posts()
        .retrieve_with_embed_context(
            &PostEndpointType::Navigation,
            &navigation_id(),
            &PostRetrieveParams::default(),
        )
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_view_context() {
    api_client()
        .posts()
        .retrieve_with_view_context(
            &PostEndpointType::Navigation,
            &navigation_id(),
            &PostRetrieveParams::default(),
        )
        .await
        .assert_response();
}

#[tokio::test]
#[rstest]
#[parallel]
#[case(PostListParams { per_page: Some(1), ..Default::default() })]
#[case(PostListParams { per_page: Some(1), order: Some(WpApiParamOrder::Desc), ..Default::default() })]
#[case(PostListParams { per_page: Some(1), orderby: Some(WpApiParamPostsOrderBy::Modified), ..Default::default() })]
async fn paginate_list_navigations_with_edit_context(#[case] params: PostListParams) {
    let first_page_response = api_client()
        .posts()
        .list_with_edit_context(&PostEndpointType::Navigation, &params)
        .await
        .assert_response();
    assert!(!first_page_response.data.is_empty());
    let next_page_params = first_page_response.next_page_params.unwrap();
    let next_page_response = api_client()
        .posts()
        .list_with_edit_context(&PostEndpointType::Navigation, &next_page_params)
        .await
        .assert_response();
    assert!(!next_page_response.data.is_empty());
    let prev_page_params = next_page_response.prev_page_params.unwrap();
    let prev_page_response = api_client()
        .posts()
        .list_with_edit_context(&PostEndpointType::Navigation, &prev_page_params)
        .await
        .assert_response();
    assert!(!prev_page_response.data.is_empty());
}

fn navigation_id() -> PostId {
    PostId(TestCredentials::instance().navigation_id)
}

#[template]
#[rstest]
#[case::default(PostListParams::default())]
#[case::page(generate!(PostListParams, (page, Some(1))))]
#[case::per_page(generate!(PostListParams, (per_page, Some(3))))]
#[case::search(generate!(PostListParams, (search, Some("foo".to_string()))))]
#[case::after(generate!(PostListParams, (after, Some(unwrapped_wp_gmt_date_time("2020-08-14T17:00:00+0200")))))]
#[case::modified_after(generate!(PostListParams, (modified_after, Some(unwrapped_wp_gmt_date_time("2024-01-14T17:00:00+0200")))))]
#[case::before(generate!(PostListParams, (before, Some(unwrapped_wp_gmt_date_time("2023-08-14T17:00:00+0000")))))]
#[case::modified_before(generate!(PostListParams, (modified_before, Some(unwrapped_wp_gmt_date_time("2024-01-14T17:00:00+0000")))))]
#[case::exclude(generate!(PostListParams, (exclude, vec![PostId(1), PostId(2)])))]
#[case::include(generate!(PostListParams, (include, vec![PostId(1)])))]
#[case::offset(generate!(PostListParams, (offset, Some(2))))]
#[case::order(generate!(PostListParams, (order, Some(WpApiParamOrder::Asc))))]
#[case::orderby(generate!(PostListParams, (orderby, Some(WpApiParamPostsOrderBy::Id))))]
#[case::search_columns(generate!(PostListParams, (search_columns, vec![WpApiParamPostsSearchColumn::PostContent, WpApiParamPostsSearchColumn::PostExcerpt])))]
#[case::slug(generate!(PostListParams, (slug, vec!["foo".to_string(), "bar".to_string()])))]
#[case::status(generate!(PostListParams, (status, vec![PostStatus::Publish, PostStatus::Draft])))]
fn list_cases(#[case] params: PostListParams) {}

mod filter {
    use super::*;

    wp_api::generate_sparse_any_post_field_with_edit_context_test_cases!();
    wp_api::generate_sparse_any_post_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_any_post_field_with_view_context_test_cases!();

    #[apply(sparse_any_post_field_with_edit_context_test_cases)]
    #[case(&[SparseAnyPostFieldWithEditContext::Id, SparseAnyPostFieldWithEditContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_navigations_with_edit_context(
        #[case] fields: &[SparseAnyPostFieldWithEditContext],
        #[values(
            PostListParams::default(),
            generate!(PostListParams, (status, vec![PostStatus::Draft, PostStatus::Publish])),
            generate!(PostListParams, (search, Some("foo".to_string())))
        )]
        params: PostListParams,
    ) {
        api_client()
            .posts()
            .filter_list_with_edit_context(&PostEndpointType::Navigation, &params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|navigation| {
                navigation.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_any_post_field_with_edit_context_test_cases)]
    #[case(&[SparseAnyPostFieldWithEditContext::Id, SparseAnyPostFieldWithEditContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_navigations_with_edit_context(
        #[case] fields: &[SparseAnyPostFieldWithEditContext],
    ) {
        let navigation = api_client()
            .posts()
            .filter_retrieve_with_edit_context(
                &PostEndpointType::Navigation,
                &navigation_id(),
                &PostRetrieveParams::default(),
                fields,
            )
            .await
            .assert_response()
            .data;
        navigation.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }

    #[apply(sparse_any_post_field_with_embed_context_test_cases)]
    #[case(&[SparseAnyPostFieldWithEmbedContext::Id, SparseAnyPostFieldWithEmbedContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_navigations_with_embed_context(
        #[case] fields: &[SparseAnyPostFieldWithEmbedContext],
        #[values(
            PostListParams::default(),
            generate!(PostListParams, (status, vec![PostStatus::Draft, PostStatus::Publish])),
            generate!(PostListParams, (search, Some("foo".to_string())))
        )]
        params: PostListParams,
    ) {
        api_client()
            .posts()
            .filter_list_with_embed_context(&PostEndpointType::Navigation, &params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|navigation| {
                navigation.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_any_post_field_with_embed_context_test_cases)]
    #[case(&[SparseAnyPostFieldWithEmbedContext::Id, SparseAnyPostFieldWithEmbedContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_navigations_with_embed_context(
        #[case] fields: &[SparseAnyPostFieldWithEmbedContext],
    ) {
        let navigation = api_client()
            .posts()
            .filter_retrieve_with_embed_context(
                &PostEndpointType::Navigation,
                &navigation_id(),
                &PostRetrieveParams::default(),
                fields,
            )
            .await
            .assert_response()
            .data;
        navigation.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }

    #[apply(sparse_any_post_field_with_view_context_test_cases)]
    #[case(&[SparseAnyPostFieldWithViewContext::Id, SparseAnyPostFieldWithViewContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_navigations_with_view_context(
        #[case] fields: &[SparseAnyPostFieldWithViewContext],
        #[values(
            PostListParams::default(),
            generate!(PostListParams, (status, vec![PostStatus::Draft, PostStatus::Publish])),
            generate!(PostListParams, (search, Some("foo".to_string())))
        )]
        params: PostListParams,
    ) {
        api_client()
            .posts()
            .filter_list_with_view_context(&PostEndpointType::Navigation, &params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|navigation| {
                navigation.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_any_post_field_with_view_context_test_cases)]
    #[case(&[SparseAnyPostFieldWithViewContext::Id, SparseAnyPostFieldWithViewContext::Slug])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_navigations_with_view_context(
        #[case] fields: &[SparseAnyPostFieldWithViewContext],
    ) {
        let navigation = api_client()
            .posts()
            .filter_retrieve_with_view_context(
                &PostEndpointType::Navigation,
                &navigation_id(),
                &PostRetrieveParams::default(),
                fields,
            )
            .await
            .assert_response()
            .data;
        navigation.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }
}

use wp_api::{
    posts::{
        PostId, PostListParams, PostRetrieveParams, PostStatus, SparseAnyPostFieldWithEditContext,
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
        .list_with_edit_context(&PostEndpointType::Pages, &params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_embed_context(#[case] params: PostListParams) {
    api_client()
        .posts()
        .list_with_embed_context(&PostEndpointType::Pages, &params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_view_context(#[case] params: PostListParams) {
    api_client()
        .posts()
        .list_with_view_context(&PostEndpointType::Pages, &params)
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_edit_context() {
    let test_credentials = TestCredentials::instance();
    api_client()
        .posts()
        .retrieve_with_edit_context(
            &PostEndpointType::Pages,
            &PostId(test_credentials.first_page_id),
            &PostRetrieveParams::default(),
        )
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_embed_context(#[case] params: PostRetrieveParams) {
    let test_credentials = TestCredentials::instance();
    api_client()
        .posts()
        .retrieve_with_embed_context(
            &PostEndpointType::Pages,
            &PostId(test_credentials.first_page_id),
            &PostRetrieveParams::default(),
        )
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_view_context(#[case] params: PostRetrieveParams) {
    let test_credentials = TestCredentials::instance();
    api_client()
        .posts()
        .retrieve_with_view_context(
            &PostEndpointType::Pages,
            &PostId(test_credentials.first_page_id),
            &PostRetrieveParams::default(),
        )
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_password_protected_with_edit_context() {
    let test_credentials = TestCredentials::instance();
    let page = api_client()
        .posts()
        .retrieve_with_edit_context(
            &PostEndpointType::Pages,
            &PostId(test_credentials.password_protected_page_id),
            &PostRetrieveParams {
                password: Some(
                    test_credentials
                        .password_protected_page_password
                        .to_string(),
                ),
            },
        )
        .await
        .assert_response()
        .data;
    assert_eq!(
        page.title.map(|t| t.rendered),
        Some(test_credentials.password_protected_page_title.to_string()),
    );
}

#[tokio::test]
#[parallel]
async fn retrieve_password_protected_with_embed_context() {
    let test_credentials = TestCredentials::instance();
    let page = api_client()
        .posts()
        .retrieve_with_embed_context(
            &PostEndpointType::Pages,
            &PostId(test_credentials.password_protected_page_id),
            &PostRetrieveParams {
                password: Some(
                    test_credentials
                        .password_protected_page_password
                        .to_string(),
                ),
            },
        )
        .await
        .assert_response()
        .data;
    assert_eq!(
        page.title.map(|t| t.rendered),
        Some(test_credentials.password_protected_page_title.to_string())
    );
}

#[tokio::test]
#[parallel]
async fn retrieve_password_protected_with_view_context() {
    let test_credentials = TestCredentials::instance();
    let page = api_client()
        .posts()
        .retrieve_with_view_context(
            &PostEndpointType::Pages,
            &PostId(test_credentials.password_protected_page_id),
            &PostRetrieveParams {
                password: Some(
                    test_credentials
                        .password_protected_page_password
                        .to_string(),
                ),
            },
        )
        .await
        .assert_response()
        .data;
    assert_eq!(
        page.title.map(|t| t.rendered),
        Some(test_credentials.password_protected_page_title.to_string())
    );
}

#[tokio::test]
#[rstest]
#[parallel]
#[case(PostListParams { per_page: Some(1), ..Default::default() })]
#[case(PostListParams { per_page: Some(1), order: Some(WpApiParamOrder::Desc), ..Default::default() })]
#[case(PostListParams { per_page: Some(1), orderby: Some(WpApiParamPostsOrderBy::Modified), ..Default::default() })]
async fn paginate_list_pages_with_edit_context(#[case] params: PostListParams) {
    let first_page_response = api_client()
        .posts()
        .list_with_edit_context(&PostEndpointType::Pages, &params)
        .await
        .assert_response();
    assert!(!first_page_response.data.is_empty());
    let next_page_params = first_page_response.next_page_params.unwrap();
    let next_page_response = api_client()
        .posts()
        .list_with_edit_context(&PostEndpointType::Pages, &next_page_params)
        .await
        .assert_response();
    assert!(!next_page_response.data.is_empty());
    let prev_page_params = next_page_response.prev_page_params.unwrap();
    let prev_page_response = api_client()
        .posts()
        .list_with_edit_context(&PostEndpointType::Pages, &prev_page_params)
        .await
        .assert_response();
    assert!(!prev_page_response.data.is_empty());
}

#[template]
#[rstest]
#[case::default(PostListParams::default())]
#[case::page(generate!(PostListParams, (page, Some(1))))]
#[case::per_page(generate!(PostListParams, (per_page, Some(3))))]
#[case::search(generate!(PostListParams, (search, Some("foo".to_string()))))]
#[case::after(generate!(PostListParams, (after, Some(unwrapped_wp_gmt_date_time("2020-08-14T17:00:00+02:00")))))]
#[case::modified_after(generate!(PostListParams, (modified_after, Some(unwrapped_wp_gmt_date_time("2024-01-14T17:00:00+02:00")))))]
#[case::author(generate!(PostListParams, (author, vec![FIRST_USER_ID, SECOND_USER_ID])))]
#[case::author_exclude(generate!(PostListParams, (author_exclude, vec![SECOND_USER_ID])))]
#[case::before(generate!(PostListParams, (before, Some(unwrapped_wp_gmt_date_time("2023-08-14T17:00:00+00:00")))))]
#[case::modified_before(generate!(PostListParams, (modified_before, Some(unwrapped_wp_gmt_date_time("2024-01-14T17:00:00+00:00")))))]
#[case::exclude(generate!(PostListParams, (exclude, vec![PostId(1), PostId(2)])))]
#[case::include(generate!(PostListParams, (include, vec![PostId(1)])))]
#[case::offset(generate!(PostListParams, (offset, Some(2))))]
#[case::order(generate!(PostListParams, (order, Some(WpApiParamOrder::Asc))))]
#[case::orderby(generate!(PostListParams, (orderby, Some(WpApiParamPostsOrderBy::Id))))]
#[case::search_columns(generate!(PostListParams, (search_columns, vec![WpApiParamPostsSearchColumn::PostContent, WpApiParamPostsSearchColumn::PostExcerpt])))]
#[case::slug(generate!(PostListParams, (slug, vec!["foo".to_string(), "bar".to_string()])))]
#[case::status(generate!(PostListParams, (status, vec![PostStatus::Publish, PostStatus::Pending])))]
#[case::parent(generate!(PostListParams, (parent, Some(PostId(1)))))]
#[case::parent_exclude(generate!(PostListParams, (parent_exclude, vec![PostId(1), PostId(2)])))]
#[case::menu_order(generate!(PostListParams, (menu_order, Some(1))))]
#[case::orderby_menu_order(generate!(PostListParams, (orderby, Some(WpApiParamPostsOrderBy::MenuOrder))))]
#[case::orderby_parent(generate!(PostListParams, (orderby, Some(WpApiParamPostsOrderBy::Parent))))]
fn list_cases(#[case] params: PostListParams) {}

mod filter {
    use super::*;

    wp_api::generate_sparse_any_post_field_with_edit_context_test_cases!();
    wp_api::generate_sparse_any_post_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_any_post_field_with_view_context_test_cases!();

    #[apply(sparse_any_post_field_with_edit_context_test_cases)]
    #[case(&[SparseAnyPostFieldWithEditContext::Id, SparseAnyPostFieldWithEditContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_pages_with_edit_context(
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
            .filter_list_with_edit_context(&PostEndpointType::Pages, &params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|page| {
                page.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_any_post_field_with_edit_context_test_cases)]
    #[case(&[SparseAnyPostFieldWithEditContext::Id, SparseAnyPostFieldWithEditContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_pages_with_edit_context(
        #[case] fields: &[SparseAnyPostFieldWithEditContext],
    ) {
        let test_credentials = TestCredentials::instance();
        let page = api_client()
            .posts()
            .filter_retrieve_with_edit_context(
                &PostEndpointType::Pages,
                &PostId(test_credentials.first_page_id),
                &PostRetrieveParams::default(),
                fields,
            )
            .await
            .assert_response()
            .data;
        page.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }

    #[apply(sparse_any_post_field_with_embed_context_test_cases)]
    #[case(&[SparseAnyPostFieldWithEmbedContext::Id, SparseAnyPostFieldWithEmbedContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_pages_with_embed_context(
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
            .filter_list_with_embed_context(&PostEndpointType::Pages, &params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|page| {
                page.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_any_post_field_with_embed_context_test_cases)]
    #[case(&[SparseAnyPostFieldWithEmbedContext::Id, SparseAnyPostFieldWithEmbedContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_pages_with_embed_context(
        #[case] fields: &[SparseAnyPostFieldWithEmbedContext],
    ) {
        let test_credentials = TestCredentials::instance();
        let page = api_client()
            .posts()
            .filter_retrieve_with_embed_context(
                &PostEndpointType::Pages,
                &PostId(test_credentials.first_page_id),
                &PostRetrieveParams::default(),
                fields,
            )
            .await
            .assert_response()
            .data;
        page.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }

    #[apply(sparse_any_post_field_with_view_context_test_cases)]
    #[case(&[SparseAnyPostFieldWithViewContext::Id, SparseAnyPostFieldWithViewContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_pages_with_view_context(
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
            .filter_list_with_view_context(&PostEndpointType::Pages, &params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|page| {
                page.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_any_post_field_with_view_context_test_cases)]
    #[case(&[SparseAnyPostFieldWithViewContext::Id, SparseAnyPostFieldWithViewContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_pages_with_view_context(
        #[case] fields: &[SparseAnyPostFieldWithViewContext],
    ) {
        let test_credentials = TestCredentials::instance();
        let page = api_client()
            .posts()
            .filter_retrieve_with_view_context(
                &PostEndpointType::Pages,
                &PostId(test_credentials.first_page_id),
                &PostRetrieveParams::default(),
                fields,
            )
            .await
            .assert_response()
            .data;
        page.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }
}

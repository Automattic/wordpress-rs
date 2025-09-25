use wp_api::{
    categories::CategoryId,
    posts::{
        PostId, PostListParams, PostRetrieveParams, PostStatus, SparseAnyPostFieldWithEditContext,
        SparseAnyPostFieldWithEmbedContext, SparseAnyPostFieldWithViewContext,
        WpApiParamPostsOrderBy, WpApiParamPostsSearchColumn, WpApiParamPostsTaxRelation,
    },
    request::endpoint::posts_endpoint::PostEndpointType,
    tags::TagId,
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn list_with_edit_context_number_of_pages() {
    let p = api_client()
        .posts()
        .list_with_edit_context(&PostEndpointType::Posts, &PostListParams::default())
        .await
        .assert_response();
    assert_eq!(p.header_map.wp_total(), Some(57));
    assert_eq!(p.header_map.wp_total_pages(), Some(6));
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_edit_context(#[case] params: PostListParams) {
    api_client()
        .posts()
        .list_with_edit_context(&PostEndpointType::Posts, &params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_embed_context(#[case] params: PostListParams) {
    api_client()
        .posts()
        .list_with_embed_context(&PostEndpointType::Posts, &params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_view_context(#[case] params: PostListParams) {
    api_client()
        .posts()
        .list_with_view_context(&PostEndpointType::Posts, &params)
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_edit_context() {
    api_client()
        .posts()
        .retrieve_with_edit_context(
            &PostEndpointType::Posts,
            &FIRST_POST_ID,
            &PostRetrieveParams::default(),
        )
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_embed_context(#[case] params: PostRetrieveParams) {
    api_client()
        .posts()
        .retrieve_with_embed_context(
            &PostEndpointType::Posts,
            &FIRST_POST_ID,
            &PostRetrieveParams::default(),
        )
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_view_context(#[case] params: PostRetrieveParams) {
    api_client()
        .posts()
        .retrieve_with_view_context(
            &PostEndpointType::Posts,
            &FIRST_POST_ID,
            &PostRetrieveParams::default(),
        )
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_password_protected_with_edit_context() {
    let test_credentials = TestCredentials::instance();
    let post = api_client()
        .posts()
        .retrieve_with_edit_context(
            &PostEndpointType::Posts,
            &PostId(test_credentials.password_protected_post_id),
            &PostRetrieveParams {
                password: Some(
                    test_credentials
                        .password_protected_post_password
                        .to_string(),
                ),
            },
        )
        .await
        .assert_response()
        .data;
    assert_eq!(
        post.title.rendered,
        test_credentials.password_protected_post_title
    );
}

#[tokio::test]
#[parallel]
async fn retrieve_password_protected_with_embed_context() {
    let test_credentials = TestCredentials::instance();
    let post = api_client()
        .posts()
        .retrieve_with_embed_context(
            &PostEndpointType::Posts,
            &PostId(test_credentials.password_protected_post_id),
            &PostRetrieveParams {
                password: Some(
                    test_credentials
                        .password_protected_post_password
                        .to_string(),
                ),
            },
        )
        .await
        .assert_response()
        .data;
    assert_eq!(
        post.title.rendered,
        test_credentials.password_protected_post_title
    );
}

#[tokio::test]
#[parallel]
async fn retrieve_password_protected_with_view_context() {
    let test_credentials = TestCredentials::instance();
    let post = api_client()
        .posts()
        .retrieve_with_view_context(
            &PostEndpointType::Posts,
            &PostId(test_credentials.password_protected_post_id),
            &PostRetrieveParams {
                password: Some(
                    test_credentials
                        .password_protected_post_password
                        .to_string(),
                ),
            },
        )
        .await
        .assert_response()
        .data;
    assert_eq!(
        post.title.rendered,
        test_credentials.password_protected_post_title
    );
}

#[tokio::test]
#[parallel]
async fn ensure_date_gmt_is_parsed_correctly() {
    let test_credentials = TestCredentials::instance();
    let post = api_client()
        .posts()
        .retrieve_with_edit_context(
            &PostEndpointType::Posts,
            &FIRST_POST_ID,
            &PostRetrieveParams::default(),
        )
        .await
        .assert_response()
        .data;
    assert_eq!(
        post.date_gmt
            .0
            .format(TestCredentials::date_format())
            .to_string(),
        test_credentials.first_post_date_gmt
    );
}

#[tokio::test]
#[rstest]
#[parallel]
#[case(PostListParams { per_page: Some(1), ..Default::default() })]
#[case(PostListParams { per_page: Some(1), order: Some(WpApiParamOrder::Desc), ..Default::default() })]
#[case(PostListParams { per_page: Some(1), orderby: Some(WpApiParamPostsOrderBy::Modified), ..Default::default() })]
async fn paginate_list_posts_with_edit_context(#[case] params: PostListParams) {
    let first_page_response = api_client()
        .posts()
        .list_with_edit_context(&PostEndpointType::Posts, &params)
        .await
        .assert_response();
    assert!(!first_page_response.data.is_empty());
    let next_page_params = first_page_response.next_page_params.unwrap();
    let next_page_response = api_client()
        .posts()
        .list_with_edit_context(&PostEndpointType::Posts, &next_page_params)
        .await
        .assert_response();
    assert!(!next_page_response.data.is_empty());
    let prev_page_params = next_page_response.prev_page_params.unwrap();
    let prev_page_response = api_client()
        .posts()
        .list_with_edit_context(&PostEndpointType::Posts, &prev_page_params)
        .await
        .assert_response();
    assert!(!prev_page_response.data.is_empty());
}

#[tokio::test]
#[rstest]
#[parallel]
#[case(PostEndpointType::Posts)]
#[case(PostEndpointType::Pages)]
// This test ensures that we can list & parse the given post type with default params
async fn list_with_post_endpoint_type_using_default_params(
    #[case] post_endpoint_type: PostEndpointType,
) {
    api_client()
        .posts()
        .list_with_edit_context(&post_endpoint_type, &PostListParams::default())
        .await
        .assert_response();
}

#[template]
#[rstest]
#[case::default(PostListParams::default())]
#[case::page(generate!(PostListParams, (page, Some(1))))]
#[case::per_page(generate!(PostListParams, (per_page, Some(3))))]
#[case::search(generate!(PostListParams, (search, Some("foo".to_string()))))]
#[case::after(generate!(PostListParams, (after, Some(unwrapped_wp_gmt_date_time("2020-08-14T17:00:00+0200")))))]
#[case::modified_after(generate!(PostListParams, (modified_after, Some(unwrapped_wp_gmt_date_time("2024-01-14T17:00:00+0200")))))]
#[case::author(generate!(PostListParams, (author, vec![FIRST_USER_ID, SECOND_USER_ID])))]
#[case::author_exclude(generate!(PostListParams, (author_exclude, vec![SECOND_USER_ID])))]
#[case::before(generate!(PostListParams, (before, Some(unwrapped_wp_gmt_date_time("2023-08-14T17:00:00+0000")))))]
#[case::modified_before(generate!(PostListParams, (modified_before, Some(unwrapped_wp_gmt_date_time("2024-01-14T17:00:00+0000")))))]
#[case::exclude(generate!(PostListParams, (exclude, vec![PostId(1), PostId(2)])))]
#[case::include(generate!(PostListParams, (include, vec![PostId(1)])))]
#[case::offset(generate!(PostListParams, (offset, Some(2))))]
#[case::order(generate!(PostListParams, (order, Some(WpApiParamOrder::Asc))))]
#[case::orderby(generate!(PostListParams, (orderby, Some(WpApiParamPostsOrderBy::Id))))]
#[case::search_columns(generate!(PostListParams, (search_columns, vec![WpApiParamPostsSearchColumn::PostContent, WpApiParamPostsSearchColumn::PostExcerpt])))]
#[case::slug(generate!(PostListParams, (slug, vec!["foo".to_string(), "bar".to_string()])))]
#[case::status(generate!(PostListParams, (status, vec![PostStatus::Publish, PostStatus::Pending])))]
#[case::tax_relation(generate!(PostListParams, (tax_relation, Some(WpApiParamPostsTaxRelation::And))))]
#[case::categories(generate!(PostListParams, (categories, vec![CategoryId(1)])))]
#[case::categories_exclude(generate!(PostListParams, (categories_exclude, vec![CategoryId(1)])))]
#[case::tags(generate!(PostListParams, (tags, vec![TagId(1)])))]
#[case::tags_exclude(generate!(PostListParams, (tags_exclude, vec![TagId(1)])))]
#[case::sticky(generate!(PostListParams, (sticky, Some(true))))]
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
    async fn filter_posts_with_edit_context(
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
            .filter_list_with_edit_context(&PostEndpointType::Posts, &params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|post| {
                post.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_any_post_field_with_edit_context_test_cases)]
    #[case(&[SparseAnyPostFieldWithEditContext::Id, SparseAnyPostFieldWithEditContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_posts_with_edit_context(
        #[case] fields: &[SparseAnyPostFieldWithEditContext],
    ) {
        let post = api_client()
            .posts()
            .filter_retrieve_with_edit_context(
                &PostEndpointType::Posts,
                &FIRST_POST_ID,
                &PostRetrieveParams::default(),
                fields,
            )
            .await
            .assert_response()
            .data;
        post.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }

    #[apply(sparse_any_post_field_with_embed_context_test_cases)]
    #[case(&[SparseAnyPostFieldWithEmbedContext::Id, SparseAnyPostFieldWithEmbedContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_posts_with_embed_context(
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
            .filter_list_with_embed_context(&PostEndpointType::Posts, &params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|post| {
                post.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_any_post_field_with_embed_context_test_cases)]
    #[case(&[SparseAnyPostFieldWithEmbedContext::Id, SparseAnyPostFieldWithEmbedContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_posts_with_embed_context(
        #[case] fields: &[SparseAnyPostFieldWithEmbedContext],
    ) {
        let post = api_client()
            .posts()
            .filter_retrieve_with_embed_context(
                &PostEndpointType::Posts,
                &FIRST_POST_ID,
                &PostRetrieveParams::default(),
                fields,
            )
            .await
            .assert_response()
            .data;
        post.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }

    #[apply(sparse_any_post_field_with_view_context_test_cases)]
    #[case(&[SparseAnyPostFieldWithViewContext::Id, SparseAnyPostFieldWithViewContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_posts_with_view_context(
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
            .filter_list_with_view_context(&PostEndpointType::Posts, &params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|post| {
                post.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_any_post_field_with_view_context_test_cases)]
    #[case(&[SparseAnyPostFieldWithViewContext::Id, SparseAnyPostFieldWithViewContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_posts_with_view_context(
        #[case] fields: &[SparseAnyPostFieldWithViewContext],
    ) {
        let post = api_client()
            .posts()
            .filter_retrieve_with_view_context(
                &PostEndpointType::Posts,
                &FIRST_POST_ID,
                &PostRetrieveParams::default(),
                fields,
            )
            .await
            .assert_response()
            .data;
        post.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }
}

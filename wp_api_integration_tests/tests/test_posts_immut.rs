use rstest::*;
use rstest_reuse::{self, apply, template};
use serial_test::parallel;
use wp_api::posts::{
    CategoryId, PostId, PostListParams, PostRetrieveParams, PostStatus,
    SparsePostFieldWithEditContext, SparsePostFieldWithEmbedContext,
    SparsePostFieldWithViewContext, TagId, WpApiParamPostsOrderBy, WpApiParamPostsSearchColumn,
    WpApiParamPostsTaxRelation,
};
use wp_api::{generate, WpApiParamOrder};
use wp_api_integration_tests::{
    api_client, AssertResponse, TestCredentials, FIRST_POST_ID, FIRST_USER_ID, SECOND_USER_ID,
};

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_edit_context(#[case] params: PostListParams) {
    api_client()
        .posts()
        .list_with_edit_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_embed_context(#[case] params: PostListParams) {
    api_client()
        .posts()
        .list_with_embed_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_view_context(#[case] params: PostListParams) {
    api_client()
        .posts()
        .list_with_view_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_edit_context() {
    api_client()
        .posts()
        .retrieve_with_edit_context(&FIRST_POST_ID, &PostRetrieveParams::default())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_embed_context(#[case] params: PostRetrieveParams) {
    api_client()
        .posts()
        .retrieve_with_embed_context(&FIRST_POST_ID, &PostRetrieveParams::default())
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_view_context(#[case] params: PostRetrieveParams) {
    api_client()
        .posts()
        .retrieve_with_view_context(&FIRST_POST_ID, &PostRetrieveParams::default())
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
        .assert_response();
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
        .assert_response();
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
        .assert_response();
    assert_eq!(
        post.title.rendered,
        test_credentials.password_protected_post_title
    );
}

#[template]
#[rstest]
#[case::default(PostListParams::default())]
#[case::page(generate!(PostListParams, (page, Some(1))))]
#[case::per_page(generate!(PostListParams, (per_page, Some(3))))]
#[case::search(generate!(PostListParams, (search, Some("foo".to_string()))))]
#[case::after(generate!(PostListParams, (after, Some("2020-08-14 17:00:00.000".to_string()))))]
#[case::modified_after(generate!(PostListParams, (modified_after, Some("2024-01-14 17:00:00.000".to_string()))))]
#[case::author(generate!(PostListParams, (author, vec![FIRST_USER_ID, SECOND_USER_ID])))]
#[case::author_exclude(generate!(PostListParams, (author_exclude, vec![SECOND_USER_ID])))]
#[case::before(generate!(PostListParams, (before, Some("2023-08-14 17:00:00.000".to_string()))))]
#[case::modified_before(generate!(PostListParams, (modified_before, Some("2024-01-14 17:00:00.000".to_string()))))]
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
pub fn list_cases(#[case] params: PostListParams) {}

mod filter {
    use super::*;

    wp_api::generate_sparse_post_field_with_edit_context_test_cases!();
    wp_api::generate_sparse_post_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_post_field_with_view_context_test_cases!();

    #[apply(sparse_post_field_with_edit_context_test_cases)]
    #[case(&[SparsePostFieldWithEditContext::Id, SparsePostFieldWithEditContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_posts_with_edit_context(
        #[case] fields: &[SparsePostFieldWithEditContext],
        #[values(
            PostListParams::default(),
            generate!(PostListParams, (status, vec![PostStatus::Draft, PostStatus::Publish])),
            generate!(PostListParams, (search, Some("foo".to_string())))
        )]
        params: PostListParams,
    ) {
        api_client()
            .posts()
            .filter_list_with_edit_context(&params, fields)
            .await
            .assert_response()
            .iter()
            .for_each(|post| {
                post.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_post_field_with_edit_context_test_cases)]
    #[case(&[SparsePostFieldWithEditContext::Id, SparsePostFieldWithEditContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_posts_with_edit_context(
        #[case] fields: &[SparsePostFieldWithEditContext],
    ) {
        let post = api_client()
            .posts()
            .filter_retrieve_with_edit_context(
                &FIRST_POST_ID,
                &PostRetrieveParams::default(),
                fields,
            )
            .await
            .assert_response();
        post.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }

    #[apply(sparse_post_field_with_embed_context_test_cases)]
    #[case(&[SparsePostFieldWithEmbedContext::Id, SparsePostFieldWithEmbedContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_posts_with_embed_context(
        #[case] fields: &[SparsePostFieldWithEmbedContext],
        #[values(
            PostListParams::default(),
            generate!(PostListParams, (status, vec![PostStatus::Draft, PostStatus::Publish])),
            generate!(PostListParams, (search, Some("foo".to_string())))
        )]
        params: PostListParams,
    ) {
        api_client()
            .posts()
            .filter_list_with_embed_context(&params, fields)
            .await
            .assert_response()
            .iter()
            .for_each(|post| {
                post.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_post_field_with_embed_context_test_cases)]
    #[case(&[SparsePostFieldWithEmbedContext::Id, SparsePostFieldWithEmbedContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_posts_with_embed_context(
        #[case] fields: &[SparsePostFieldWithEmbedContext],
    ) {
        let post = api_client()
            .posts()
            .filter_retrieve_with_embed_context(
                &FIRST_POST_ID,
                &PostRetrieveParams::default(),
                fields,
            )
            .await
            .assert_response();
        post.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }

    #[apply(sparse_post_field_with_view_context_test_cases)]
    #[case(&[SparsePostFieldWithViewContext::Id, SparsePostFieldWithViewContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_posts_with_view_context(
        #[case] fields: &[SparsePostFieldWithViewContext],
        #[values(
            PostListParams::default(),
            generate!(PostListParams, (status, vec![PostStatus::Draft, PostStatus::Publish])),
            generate!(PostListParams, (search, Some("foo".to_string())))
        )]
        params: PostListParams,
    ) {
        api_client()
            .posts()
            .filter_list_with_view_context(&params, fields)
            .await
            .assert_response()
            .iter()
            .for_each(|post| {
                post.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_post_field_with_view_context_test_cases)]
    #[case(&[SparsePostFieldWithViewContext::Id, SparsePostFieldWithViewContext::Author])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_posts_with_view_context(
        #[case] fields: &[SparsePostFieldWithViewContext],
    ) {
        let post = api_client()
            .posts()
            .filter_retrieve_with_view_context(
                &FIRST_POST_ID,
                &PostRetrieveParams::default(),
                fields,
            )
            .await
            .assert_response();
        post.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }
}

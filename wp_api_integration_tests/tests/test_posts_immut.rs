use rstest::*;
use rstest_reuse::{self, apply, template};
use serial_test::parallel;
use wp_api::posts::{
    CategoryId, PostId, PostListParams, PostStatus, TagId, WpApiParamPostsOrderBy,
    WpApiParamPostsSearchColumn, WpApiParamPostsTaxRelation,
};
use wp_api::{generate, WpApiParamOrder};
use wp_api_integration_tests::{api_client, AssertResponse, FIRST_USER_ID, SECOND_USER_ID};

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

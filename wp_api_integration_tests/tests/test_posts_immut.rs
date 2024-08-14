use rstest::*;
use rstest_reuse::{self, apply, template};
use serial_test::parallel;
use wp_api::generate;
use wp_api::posts::{PostId, PostListParams};
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
pub fn list_cases(#[case] params: PostListParams) {}

use rstest::*;
use rstest_reuse::{self, apply, template};
use serial_test::parallel;
use wp_api::{
    WpApiParamOrder, generate,
    post_revisions::{PostRevisionId, PostRevisionListParams, WpApiParamPostRevisionsOrderBy},
    posts::PostId,
};
use wp_api_integration_tests::{AssertResponse, TestCredentials, api_client};

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_edit_context(#[case] params: PostRevisionListParams) {
    api_client()
        .post_revisions()
        .list_with_edit_context(&revisioned_post_id(), &params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_embed_context(#[case] params: PostRevisionListParams) {
    api_client()
        .post_revisions()
        .list_with_embed_context(&revisioned_post_id(), &params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_view_context(#[case] params: PostRevisionListParams) {
    api_client()
        .post_revisions()
        .list_with_view_context(&revisioned_post_id(), &params)
        .await
        .assert_response();
}

fn revisioned_post_id() -> PostId {
    PostId(TestCredentials::instance().revisioned_post_id)
}

#[template]
#[rstest]
#[case::default(PostRevisionListParams::default())]
#[case::page(generate!(PostRevisionListParams, (page, Some(1))))]
#[case::per_page(generate!(PostRevisionListParams, (per_page, Some(3))))]
#[case::search(generate!(PostRevisionListParams, (search, Some("foo".to_string()))))]
#[case::exclude(generate!(PostRevisionListParams, (exclude, vec![PostRevisionId(1), PostRevisionId(2)])))]
#[case::include(generate!(PostRevisionListParams, (include, vec![PostRevisionId(1)])))]
// TODO: Increase the offset after updating the test site setup to create multiple revisions
#[case::offset(generate!(PostRevisionListParams, (offset, Some(0))))]
#[case::order(generate!(PostRevisionListParams, (order, Some(WpApiParamOrder::Asc))))]
#[case::orderby(generate!(PostRevisionListParams, (orderby, Some(WpApiParamPostRevisionsOrderBy::Slug))))]
fn list_cases(#[case] params: PostRevisionListParams) {}

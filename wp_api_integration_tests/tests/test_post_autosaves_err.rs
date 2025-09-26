use wp_api::{
    post_revisions::PostRevisionId,
    posts::{PostCreateParams, PostId},
    request::endpoint::posts_endpoint::PostEndpointType,
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn list_err_post_invalid_parent() {
    api_client()
        .autosaves()
        .list_with_edit_context(&PostEndpointType::Posts, &PostId(99999999))
        .await
        .assert_wp_error(WpErrorCode::PostInvalidParent)
}

#[tokio::test]
#[parallel]
async fn retrieve_err_post_invalid_parent() {
    api_client()
        .autosaves()
        .retrieve_with_edit_context(
            &PostEndpointType::Posts,
            &PostId(99999999),
            &PostRevisionId(1),
        )
        .await
        .assert_wp_error(WpErrorCode::PostInvalidParent)
}

#[tokio::test]
#[parallel]
async fn retrieve_err_post_no_autosave() {
    api_client()
        .autosaves()
        .retrieve_with_edit_context(
            &PostEndpointType::Posts,
            &PostId(TestCredentials::instance().revisioned_post_id),
            &PostRevisionId(1),
        )
        .await
        .assert_wp_error(WpErrorCode::PostNoAutosave)
}

#[tokio::test]
#[parallel]
async fn create_err_post_invalid_id() {
    api_client()
        .autosaves()
        .create(
            &PostEndpointType::Posts,
            &PostId(99999999),
            &PostCreateParams::default(),
        )
        .await
        .assert_wp_error(WpErrorCode::PostInvalidId)
}

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
        .list_with_edit_context(&PostEndpointType::Pages, &PostId(99999999))
        .await
        .assert_wp_error(WpErrorCode::PostInvalidParent)
}

#[tokio::test]
#[parallel]
async fn list_err_cannot_read_as_subscriber() {
    api_client_as_subscriber()
        .autosaves()
        .list_with_edit_context(&PostEndpointType::Pages, &autosaved_page_id())
        .await
        .assert_wp_error(WpErrorCode::CannotRead)
}

#[tokio::test]
#[parallel]
async fn retrieve_err_post_invalid_parent() {
    api_client()
        .autosaves()
        .retrieve_with_edit_context(
            &PostEndpointType::Pages,
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
            &PostEndpointType::Pages,
            &PostId(TestCredentials::instance().revisioned_page_id),
            &PostRevisionId(1),
        )
        .await
        .assert_wp_error(WpErrorCode::PostNoAutosave)
}

#[tokio::test]
#[parallel]
async fn retrieve_err_cannot_read_as_subscriber() {
    api_client_as_subscriber()
        .autosaves()
        .retrieve_with_edit_context(
            &PostEndpointType::Pages,
            &autosaved_page_id(),
            &autosave_id_for_autosaved_page_id(),
        )
        .await
        .assert_wp_error(WpErrorCode::CannotRead)
}

#[tokio::test]
#[parallel]
async fn create_err_post_invalid_id() {
    api_client()
        .autosaves()
        .create(
            &PostEndpointType::Pages,
            &PostId(99999999),
            &PostCreateParams::default(),
        )
        .await
        .assert_wp_error(WpErrorCode::PostInvalidId)
}

#[tokio::test]
#[parallel]
async fn create_err_cannot_edit_as_subscriber() {
    api_client_as_subscriber()
        .autosaves()
        .create(
            &PostEndpointType::Pages,
            &autosaved_page_id(),
            &PostCreateParams::default(),
        )
        .await
        .assert_wp_error(WpErrorCode::CannotEdit)
}

fn autosaved_page_id() -> PostId {
    PostId(TestCredentials::instance().autosaved_page_id)
}

fn autosave_id_for_autosaved_page_id() -> PostRevisionId {
    PostRevisionId(TestCredentials::instance().autosave_id_for_autosaved_page_id)
}

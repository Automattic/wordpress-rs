use wp_api::{
    navigation_revisions::NavigationRevisionId,
    navigations::{NavigationCreateParams, NavigationId},
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn list_err_post_invalid_parent() {
    api_client()
        .navigation_autosaves()
        .list_with_edit_context(&NavigationId(99999999))
        .await
        .assert_wp_error(WpErrorCode::PostInvalidParent)
}

#[tokio::test]
#[parallel]
async fn list_err_cannot_read_as_subscriber() {
    api_client_as_subscriber()
        .navigation_autosaves()
        .list_with_edit_context(&autosaved_navigation_id())
        .await
        .assert_wp_error(WpErrorCode::CannotRead)
}

#[tokio::test]
#[parallel]
async fn retrieve_err_post_invalid_parent() {
    api_client()
        .navigation_autosaves()
        .retrieve_with_edit_context(&NavigationId(99999999), &NavigationRevisionId(1))
        .await
        .assert_wp_error(WpErrorCode::PostInvalidParent)
}

#[tokio::test]
#[parallel]
async fn retrieve_err_post_no_autosave() {
    api_client()
        .navigation_autosaves()
        .retrieve_with_edit_context(
            &NavigationId(TestCredentials::instance().navigation_id),
            &NavigationRevisionId(1),
        )
        .await
        .assert_wp_error(WpErrorCode::PostNoAutosave)
}

#[tokio::test]
#[parallel]
async fn retrieve_err_cannot_read_as_subscriber() {
    api_client_as_subscriber()
        .navigation_autosaves()
        .retrieve_with_edit_context(
            &autosaved_navigation_id(),
            &autosave_id_for_autosaved_navigation_id(),
        )
        .await
        .assert_wp_error(WpErrorCode::CannotRead)
}

#[tokio::test]
#[parallel]
async fn create_err_post_invalid_id() {
    api_client()
        .navigation_autosaves()
        .create(&NavigationId(99999999), &NavigationCreateParams::default())
        .await
        .assert_wp_error(WpErrorCode::PostInvalidId)
}

#[tokio::test]
#[parallel]
async fn create_err_cannot_edit_as_subscriber() {
    api_client_as_subscriber()
        .navigation_autosaves()
        .create(
            &autosaved_navigation_id(),
            &NavigationCreateParams::default(),
        )
        .await
        .assert_wp_error(WpErrorCode::CannotEdit)
}

fn autosaved_navigation_id() -> NavigationId {
    NavigationId(TestCredentials::instance().autosaved_navigation_id)
}

fn autosave_id_for_autosaved_navigation_id() -> NavigationRevisionId {
    NavigationRevisionId(TestCredentials::instance().autosave_id_for_autosaved_navigation_id)
}

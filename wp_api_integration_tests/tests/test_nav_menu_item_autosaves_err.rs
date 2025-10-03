use wp_api::{
    nav_menu_item_revisions::{NavMenuItemRevisionCreateParams, NavMenuItemRevisionId},
    nav_menu_items::NavMenuItemId,
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn list_err_cannot_read() {
    api_client_as_subscriber()
        .nav_menu_item_autosaves()
        .list_with_edit_context(&NavMenuItemId(TestCredentials::instance().nav_menu_item_id))
        .await
        .assert_wp_error(WpErrorCode::CannotRead)
}

#[tokio::test]
#[parallel]
async fn list_err_post_invalid_parent() {
    api_client()
        .nav_menu_item_autosaves()
        .list_with_edit_context(&NavMenuItemId(99999999))
        .await
        .assert_wp_error(WpErrorCode::PostInvalidParent)
}

#[tokio::test]
#[parallel]
async fn retrieve_err_cannot_read() {
    api_client_as_subscriber()
        .nav_menu_item_autosaves()
        .retrieve_with_edit_context(
            &NavMenuItemId(TestCredentials::instance().nav_menu_item_id),
            &NavMenuItemRevisionId(TestCredentials::instance().autosave_id_for_nav_menu_item_id),
        )
        .await
        .assert_wp_error(WpErrorCode::CannotRead)
}

#[tokio::test]
#[parallel]
async fn retrieve_err_post_invalid_parent() {
    api_client()
        .nav_menu_item_autosaves()
        .retrieve_with_edit_context(&NavMenuItemId(99999999), &NavMenuItemRevisionId(1))
        .await
        .assert_wp_error(WpErrorCode::PostInvalidParent)
}

#[tokio::test]
#[parallel]
async fn retrieve_err_post_no_autosave() {
    api_client()
        .nav_menu_item_autosaves()
        .retrieve_with_edit_context(&NavMenuItemId(1747), &NavMenuItemRevisionId(1))
        .await
        .assert_wp_error(WpErrorCode::PostNoAutosave)
}

#[tokio::test]
#[parallel]
async fn create_err_cannot_edit() {
    api_client_as_subscriber()
        .nav_menu_item_autosaves()
        .create(
            &NavMenuItemId(TestCredentials::instance().nav_menu_item_id),
            &NavMenuItemRevisionCreateParams::default(),
        )
        .await
        .assert_wp_error(WpErrorCode::CannotEdit)
}

#[tokio::test]
#[parallel]
async fn create_err_post_invalid_id() {
    api_client()
        .nav_menu_item_autosaves()
        .create(
            &NavMenuItemId(99999999),
            &NavMenuItemRevisionCreateParams::default(),
        )
        .await
        .assert_wp_error(WpErrorCode::PostInvalidId)
}

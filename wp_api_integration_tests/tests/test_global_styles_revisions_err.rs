use integration_test_credentials::TestCredentials;
use wp_api::{
    global_styles::GlobalStylesId,
    global_styles_revisions::{GlobalStylesRevisionId, GlobalStylesRevisionListParams},
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn list_err_post_invalid_parent() {
    api_client()
        .global_styles_revisions()
        .list_with_edit_context(
            &GlobalStylesId(99999999),
            &GlobalStylesRevisionListParams::default(),
        )
        .await
        .assert_wp_error(WpErrorCode::PostInvalidParent)
}

#[tokio::test]
#[parallel]
async fn list_err_revision_invalid_offset_number() {
    api_client()
        .global_styles_revisions()
        .list_with_edit_context(
            &global_styles_id(),
            &GlobalStylesRevisionListParams {
                offset: Some(99999999),
                ..Default::default()
            },
        )
        .await
        .assert_wp_error(WpErrorCode::RevisionInvalidOffsetNumber)
}

#[tokio::test]
#[parallel]
async fn list_err_revision_invalid_page_number() {
    api_client()
        .global_styles_revisions()
        .list_with_edit_context(
            &global_styles_id(),
            &GlobalStylesRevisionListParams {
                page: Some(99999999),
                ..Default::default()
            },
        )
        .await
        .assert_wp_error(WpErrorCode::RevisionInvalidPageNumber)
}

#[tokio::test]
#[parallel]
async fn list_err_cannot_read_as_subscriber() {
    api_client_as_subscriber()
        .global_styles_revisions()
        .list_with_edit_context(
            &global_styles_id(),
            &GlobalStylesRevisionListParams::default(),
        )
        .await
        .assert_wp_error(WpErrorCode::CannotRead)
}

#[tokio::test]
#[parallel]
async fn retrieve_err_post_invalid_parent() {
    api_client()
        .global_styles_revisions()
        .retrieve_with_edit_context(&GlobalStylesId(99999999), &revision_id())
        .await
        .assert_wp_error(WpErrorCode::PostInvalidParent)
}

#[tokio::test]
#[parallel]
async fn retrieve_err_post_invalid_id() {
    api_client()
        .global_styles_revisions()
        .retrieve_with_edit_context(&global_styles_id(), &GlobalStylesRevisionId(99999999))
        .await
        .assert_wp_error(WpErrorCode::PostInvalidId)
}

#[tokio::test]
#[parallel]
async fn retrieve_err_cannot_read_as_subscriber() {
    api_client_as_subscriber()
        .global_styles_revisions()
        .retrieve_with_edit_context(&global_styles_id(), &revision_id())
        .await
        .assert_wp_error(WpErrorCode::CannotRead)
}

fn global_styles_id() -> GlobalStylesId {
    GlobalStylesId(TestCredentials::instance().global_styles_id)
}

fn revision_id() -> GlobalStylesRevisionId {
    GlobalStylesRevisionId(TestCredentials::instance().revision_id_for_global_styles_id)
}

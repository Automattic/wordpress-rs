use wp_api::{
    block_revisions::BlockRevisionId,
    blocks::{BlockCreateParams, BlockId},
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn list_err_post_invalid_parent() {
    api_client()
        .block_autosaves()
        .list_with_edit_context(&BlockId(99999999))
        .await
        .assert_wp_error(WpErrorCode::PostInvalidParent)
}

#[tokio::test]
#[parallel]
async fn list_err_cannot_read_as_subscriber() {
    api_client_as_subscriber()
        .block_autosaves()
        .list_with_edit_context(&autosaved_block_id())
        .await
        .assert_wp_error(WpErrorCode::CannotRead)
}

#[tokio::test]
#[parallel]
async fn retrieve_err_post_invalid_parent() {
    api_client()
        .block_autosaves()
        .retrieve_with_edit_context(&BlockId(99999999), &BlockRevisionId(1))
        .await
        .assert_wp_error(WpErrorCode::PostInvalidParent)
}

#[tokio::test]
#[parallel]
async fn retrieve_err_post_no_autosave() {
    api_client()
        .block_autosaves()
        .retrieve_with_edit_context(
            &BlockId(TestCredentials::instance().block_id),
            &BlockRevisionId(1),
        )
        .await
        .assert_wp_error(WpErrorCode::PostNoAutosave)
}

#[tokio::test]
#[parallel]
async fn retrieve_err_cannot_read_as_subscriber() {
    api_client_as_subscriber()
        .block_autosaves()
        .retrieve_with_edit_context(&autosaved_block_id(), &autosave_id_for_autosaved_block_id())
        .await
        .assert_wp_error(WpErrorCode::CannotRead)
}

#[tokio::test]
#[parallel]
async fn create_err_post_invalid_id() {
    api_client()
        .block_autosaves()
        .create(&BlockId(99999999), &BlockCreateParams::default())
        .await
        .assert_wp_error(WpErrorCode::PostInvalidId)
}

#[tokio::test]
#[parallel]
async fn create_err_cannot_edit_as_subscriber() {
    api_client_as_subscriber()
        .block_autosaves()
        .create(&autosaved_block_id(), &BlockCreateParams::default())
        .await
        .assert_wp_error(WpErrorCode::CannotEdit)
}

fn autosaved_block_id() -> BlockId {
    BlockId(TestCredentials::instance().autosaved_block_id)
}

fn autosave_id_for_autosaved_block_id() -> BlockRevisionId {
    BlockRevisionId(TestCredentials::instance().autosave_id_for_autosaved_block_id)
}

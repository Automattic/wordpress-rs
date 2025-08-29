use wp_api::{post_revisions::{PostRevisionListParams, PostRevisionId}, posts::PostId};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn list_err_post_invalid_parent() {
    api_client()
        .post_revisions()
        .list_with_edit_context(&PostId(99999999), &PostRevisionListParams::default())
        .await
        .assert_wp_error(WpErrorCode::PostInvalidParent)
}

#[tokio::test]
#[parallel]
async fn list_err_revision_invalid_offset_number() {
    api_client()
        .post_revisions()
        .list_with_edit_context(
            &revisioned_post_id(),
            &PostRevisionListParams {
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
        .post_revisions()
        .list_with_edit_context(
            &revisioned_post_id(),
            &PostRevisionListParams {
                page: Some(99999999),
                ..Default::default()
            },
        )
        .await
        .assert_wp_error(WpErrorCode::RevisionInvalidPageNumber)
}

#[tokio::test]
#[parallel]
async fn retrieve_err_post_invalid_parent() {
    api_client()
        .post_revisions()
        .retrieve_with_edit_context(&PostId(99999999), &valid_revision_id())
        .await
        .assert_wp_error(WpErrorCode::PostInvalidParent)
}

#[tokio::test]
#[parallel]
async fn retrieve_err_post_invalid_id() {
    api_client()
        .post_revisions()
        .retrieve_with_edit_context(&revisioned_post_id(), &PostRevisionId(99999999))
        .await
        .assert_wp_error(WpErrorCode::PostInvalidId)
}

#[tokio::test]
#[parallel]
async fn delete_err_post_invalid_parent() {
    api_client()
        .post_revisions()
        .delete(&PostId(99999999), &valid_revision_id())
        .await
        .assert_wp_error(WpErrorCode::PostInvalidParent)
}

#[tokio::test]
#[parallel]
async fn delete_err_post_invalid_id() {
    api_client()
        .post_revisions()
        .delete(&revisioned_post_id(), &PostRevisionId(99999999))
        .await
        .assert_wp_error(WpErrorCode::PostInvalidId)
}

fn revisioned_post_id() -> PostId {
    PostId(TestCredentials::instance().revisioned_post_id)
}

fn valid_revision_id() -> PostRevisionId {
    PostRevisionId(TestCredentials::instance().revision_id_for_revisioned_post_id)
}

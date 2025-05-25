use serial_test::parallel;
use wp_api::{WpErrorCode, post_revisions::PostRevisionListParams, posts::PostId};
use wp_api_integration_tests::{AssertWpError, TestCredentials, api_client};

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

fn revisioned_post_id() -> PostId {
    PostId(TestCredentials::instance().revisioned_post_id)
}

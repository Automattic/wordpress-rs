use wp_api_integration_tests::prelude::*;

#[rstest]
#[tokio::test]
#[parallel]
async fn list_post_statuses_err_forbidden_context() {
    api_client_as_subscriber()
        .post_statuses()
        .list_with_edit_context()
        .await
        .assert_wp_error(WpErrorCode::CannotView);
}

#[rstest]
#[tokio::test]
#[parallel]
async fn retrieve_post_status_err_status_invalid() {
    api_client()
        .post_statuses()
        .retrieve_with_view_context(&"non_existent_status".into())
        .await
        .assert_wp_error(WpErrorCode::StatusInvalid);
}

#[rstest]
#[tokio::test]
#[parallel]
async fn retrieve_post_status_err_forbidden_context() {
    api_client_as_subscriber()
        .post_statuses()
        .retrieve_with_edit_context(&"private".into())
        .await
        .assert_wp_error(WpErrorCode::CannotReadStatus);
}

#[tokio::test]
#[rstest]
#[parallel]
#[case("auto-draft")]
#[case("inherit")]
async fn retrieve_post_status_err_cannot_read_status(#[case] status_slug: &str) {
    api_client()
        .post_statuses()
        .retrieve_with_view_context(&status_slug.into())
        .await
        .assert_wp_error(WpErrorCode::CannotReadStatus);
}

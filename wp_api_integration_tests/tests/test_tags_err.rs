use serial_test::parallel;
use wp_api::{
    WpErrorCode,
    tags::{TagCreateParams, TagListParams, TagUpdateParams},
};
use wp_api_integration_tests::{
    AssertWpError, POST_ID_INVALID, TAG_ID_100, TAG_ID_INVALID, api_client,
    api_client_as_subscriber,
};

#[tokio::test]
#[parallel]
async fn create_err_cannot_create() {
    api_client_as_subscriber()
        .tags()
        .create(&TagCreateParams {
            name: "foo".to_string(),
            description: None,
            slug: None,
        })
        .await
        .assert_wp_error(WpErrorCode::CannotCreate);
}

#[tokio::test]
#[parallel]
async fn delete_err_cannot_delete() {
    api_client_as_subscriber()
        .tags()
        .delete(&TAG_ID_100)
        .await
        .assert_wp_error(WpErrorCode::CannotDelete);
}

#[tokio::test]
#[parallel]
async fn list_err_forbidden_context() {
    api_client_as_subscriber()
        .tags()
        .list_with_edit_context(&TagListParams::default())
        .await
        .assert_wp_error(WpErrorCode::ForbiddenContext);
}

#[tokio::test]
#[parallel]
async fn list_err_post_invalid_id() {
    api_client()
        .tags()
        .list_with_edit_context(&TagListParams {
            post: Some(POST_ID_INVALID),
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::PostInvalidId);
}

#[tokio::test]
#[parallel]
async fn retrieve_err_term_invalid() {
    api_client()
        .tags()
        .retrieve_with_edit_context(&TAG_ID_INVALID)
        .await
        .assert_wp_error(WpErrorCode::TermInvalid);
}

#[tokio::test]
#[parallel]
async fn update_err_cannot_update() {
    api_client_as_subscriber()
        .tags()
        .update(&TAG_ID_100, &TagUpdateParams::default())
        .await
        .assert_wp_error(WpErrorCode::CannotUpdate);
}

use serial_test::parallel;
use wp_api::{
    posts::{PostCreateParams, PostRetrieveParams},
    WpErrorCode,
};
use wp_api_integration_tests::{api_client, AssertWpError, PASSWORD_PROTECTED_POST_ID};

#[tokio::test]
#[parallel]
async fn create_post_err() {
    api_client()
        .posts()
        .create(&PostCreateParams::default())
        .await
        .assert_wp_error(WpErrorCode::EmptyContent)
}

#[tokio::test]
#[parallel]
async fn retrieve_password_protected_post_err_wrong_password() {
    api_client()
        .posts()
        .retrieve_with_view_context(
            &PASSWORD_PROTECTED_POST_ID,
            &PostRetrieveParams {
                password: Some("wrong_password".to_string()),
            },
        )
        .await
        .assert_wp_error(WpErrorCode::PostIncorrectPassword);
}

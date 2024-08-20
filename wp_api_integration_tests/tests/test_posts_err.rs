use serial_test::parallel;
use wp_api::{posts::PostRetrieveParams, WpErrorCode};
use wp_api_integration_tests::{api_client, AssertWpError, PASSWORD_PROTECTED_POST_ID};

#[tokio::test]
#[parallel]
async fn retrieve_password_protected_err_() {
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

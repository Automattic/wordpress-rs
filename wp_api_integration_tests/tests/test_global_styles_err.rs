use integration_test_credentials::TestCredentials;
use wp_api::global_styles::GlobalStylesId;
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn retrieve_err_forbidden_context_as_subscriber() {
    api_client_as_subscriber()
        .global_styles()
        .retrieve_with_edit_context(&global_styles_id())
        .await
        .assert_wp_error(WpErrorCode::ForbiddenContext)
}

fn global_styles_id() -> GlobalStylesId {
    GlobalStylesId(TestCredentials::instance().global_styles_id)
}

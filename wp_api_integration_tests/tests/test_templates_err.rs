use serial_test::parallel;
use wp_api::{
    WpErrorCode,
    templates::{TemplateId, TemplateUpdateParams},
};
use wp_api_integration_tests::{AssertWpError, TEMPLATE_TWENTY_TWENTY_FOUR_SINGLE, api_client};

#[tokio::test]
#[parallel]
async fn delete_template_err_invalid_template() {
    api_client()
        .templates()
        .delete(&TemplateId(TEMPLATE_TWENTY_TWENTY_FOUR_SINGLE.to_string()))
        .await
        .assert_wp_error(WpErrorCode::InvalidTemplate)
}

#[tokio::test]
#[parallel]
async fn delete_template_err_template_not_found() {
    api_client()
        .templates()
        .delete(&TemplateId("foo".to_string()))
        .await
        .assert_wp_error(WpErrorCode::TemplateNotFound)
}

#[tokio::test]
#[parallel]
async fn update_template_err_template_not_found() {
    api_client()
        .templates()
        .update(
            &TemplateId("foo".to_string()),
            &TemplateUpdateParams::default(),
        )
        .await
        .assert_wp_error(WpErrorCode::TemplateNotFound)
}

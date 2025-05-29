use wp_api::templates::{TemplateCreateParams, TemplateId, TemplateUpdateParams};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn create_template_err_empty_content() {
    // Creating a template requires `title` or `content`
    api_client()
        .templates()
        .create(&TemplateCreateParams::new("foo".to_string()))
        .await
        .assert_wp_error(WpErrorCode::EmptyContent)
}

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

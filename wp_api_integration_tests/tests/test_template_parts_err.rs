use wp_api::template_parts::{TemplatePartCreateParams, TemplatePartId, TemplatePartUpdateParams};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn create_template_part_err_empty_content() {
    // Creating a template part requires `title` or `content`
    api_client()
        .template_parts()
        .create(&TemplatePartCreateParams::new("foo".to_string()))
        .await
        .assert_wp_error(WpErrorCode::EmptyContent)
}

#[tokio::test]
#[parallel]
async fn delete_template_part_err_invalid_template() {
    api_client()
        .template_parts()
        .delete(&TemplatePartId(
            TEMPLATE_PART_TWENTY_TWENTY_FOUR_HEADER.to_string(),
        ))
        .await
        .assert_wp_error(WpErrorCode::InvalidTemplate)
}

#[tokio::test]
#[parallel]
async fn delete_template_part_err_template_not_found() {
    api_client()
        .template_parts()
        .delete(&TemplatePartId("foo".to_string()))
        .await
        .assert_wp_error(WpErrorCode::TemplateNotFound)
}

#[tokio::test]
#[parallel]
async fn update_template_part_err_cannot_manage_templates() {
    api_client_as_subscriber()
        .template_parts()
        .update(
            &TemplatePartId(
                TestCredentials::instance()
                    .integration_test_custom_template_part_id
                    .to_string(),
            ),
            &TemplatePartUpdateParams::default(),
        )
        .await
        .assert_wp_error(WpErrorCode::CannotManageTemplates)
}

#[tokio::test]
#[parallel]
async fn update_template_part_err_invalid_author() {
    api_client()
        .template_parts()
        .update(
            &TemplatePartId(
                TestCredentials::instance()
                    .integration_test_custom_template_part_id
                    .to_string(),
            ),
            &TemplatePartUpdateParams {
                author: Some(UserId(99999999)),
                ..Default::default()
            },
        )
        .await
        .assert_wp_error(WpErrorCode::InvalidAuthor);
}

#[tokio::test]
#[parallel]
async fn update_template_part_err_template_not_found() {
    api_client()
        .template_parts()
        .update(
            &TemplatePartId("foo".to_string()),
            &TemplatePartUpdateParams::default(),
        )
        .await
        .assert_wp_error(WpErrorCode::TemplateNotFound)
}

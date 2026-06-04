use wp_api::{
    template_part_autosaves::TemplatePartAutosaveId,
    template_parts::{TemplatePartCreateParams, TemplatePartId},
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn list_err_post_invalid_parent() {
    api_client()
        .template_part_autosaves()
        .list_with_edit_context(&TemplatePartId("foo".to_string()))
        .await
        .assert_wp_error(WpErrorCode::PostInvalidParent)
}

#[tokio::test]
#[parallel]
async fn list_err_cannot_read_as_subscriber() {
    api_client_as_subscriber()
        .template_part_autosaves()
        .list_with_edit_context(&autosaved_template_part_id())
        .await
        .assert_wp_error(WpErrorCode::CannotRead)
}

#[tokio::test]
#[parallel]
async fn retrieve_err_post_invalid_parent() {
    api_client()
        .template_part_autosaves()
        .retrieve_with_edit_context(
            &TemplatePartId("foo".to_string()),
            &TemplatePartAutosaveId(1),
        )
        .await
        .assert_wp_error(WpErrorCode::PostInvalidParent)
}

#[tokio::test]
#[parallel]
async fn retrieve_err_post_no_autosave() {
    // Use the custom template part which has no autosave set up
    api_client()
        .template_part_autosaves()
        .retrieve_with_edit_context(
            &TemplatePartId(
                TestCredentials::instance()
                    .integration_test_custom_template_part_id
                    .to_string(),
            ),
            &TemplatePartAutosaveId(1),
        )
        .await
        .assert_wp_error(WpErrorCode::PostNoAutosave)
}

#[tokio::test]
#[parallel]
async fn retrieve_err_cannot_read_as_subscriber() {
    api_client_as_subscriber()
        .template_part_autosaves()
        .retrieve_with_edit_context(&autosaved_template_part_id(), &autosave_id())
        .await
        .assert_wp_error(WpErrorCode::CannotRead)
}

#[tokio::test]
#[parallel]
async fn create_err_post_invalid_parent() {
    api_client()
        .template_part_autosaves()
        .create(
            &TemplatePartId("foo".to_string()),
            &TemplatePartCreateParams::new("test".to_string()),
        )
        .await
        .assert_wp_error(WpErrorCode::PostInvalidParent)
}

#[tokio::test]
#[parallel]
async fn create_err_cannot_manage_templates_as_subscriber() {
    api_client_as_subscriber()
        .template_part_autosaves()
        .create(
            &autosaved_template_part_id(),
            &TemplatePartCreateParams::new("test".to_string()),
        )
        .await
        .assert_wp_error(WpErrorCode::CannotManageTemplates)
}

fn autosaved_template_part_id() -> TemplatePartId {
    TemplatePartId(
        TestCredentials::instance()
            .autosaved_template_part_id
            .to_string(),
    )
}

fn autosave_id() -> TemplatePartAutosaveId {
    TemplatePartAutosaveId(TestCredentials::instance().autosave_id_for_autosaved_template_part)
}

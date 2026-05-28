use wp_api::{
    template_autosaves::TemplateAutosaveId,
    templates::{TemplateCreateParams, TemplateId},
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn list_err_template_not_found() {
    api_client()
        .template_autosaves()
        .list_with_edit_context(&TemplateId("foo".to_string()))
        .await
        .assert_wp_error(WpErrorCode::PostInvalidParent)
}

#[tokio::test]
#[parallel]
async fn list_err_cannot_read_as_subscriber() {
    api_client_as_subscriber()
        .template_autosaves()
        .list_with_edit_context(&autosaved_template_id())
        .await
        .assert_wp_error(WpErrorCode::CannotRead)
}

#[tokio::test]
#[parallel]
async fn retrieve_err_template_not_found() {
    api_client()
        .template_autosaves()
        .retrieve_with_edit_context(&TemplateId("foo".to_string()), &TemplateAutosaveId(1))
        .await
        .assert_wp_error(WpErrorCode::PostInvalidParent)
}

#[tokio::test]
#[parallel]
async fn retrieve_err_post_no_autosave() {
    // Use the custom template which has no autosave set up
    api_client()
        .template_autosaves()
        .retrieve_with_edit_context(
            &TemplateId(
                TestCredentials::instance()
                    .integration_test_custom_template_id
                    .to_string(),
            ),
            &TemplateAutosaveId(1),
        )
        .await
        .assert_wp_error(WpErrorCode::PostNoAutosave)
}

#[tokio::test]
#[parallel]
async fn retrieve_err_cannot_read_as_subscriber() {
    api_client_as_subscriber()
        .template_autosaves()
        .retrieve_with_edit_context(&autosaved_template_id(), &autosave_id())
        .await
        .assert_wp_error(WpErrorCode::CannotRead)
}

#[tokio::test]
#[parallel]
async fn create_err_template_not_found() {
    api_client()
        .template_autosaves()
        .create(
            &TemplateId("foo".to_string()),
            &TemplateCreateParams::new("test".to_string()),
        )
        .await
        .assert_wp_error(WpErrorCode::PostInvalidParent)
}

#[tokio::test]
#[parallel]
async fn create_err_cannot_manage_templates_as_subscriber() {
    api_client_as_subscriber()
        .template_autosaves()
        .create(
            &autosaved_template_id(),
            &TemplateCreateParams::new("test".to_string()),
        )
        .await
        .assert_wp_error(WpErrorCode::CannotManageTemplates)
}

fn autosaved_template_id() -> TemplateId {
    TemplateId(
        TestCredentials::instance()
            .autosaved_template_id
            .to_string(),
    )
}

fn autosave_id() -> TemplateAutosaveId {
    TemplateAutosaveId(TestCredentials::instance().autosave_id_for_autosaved_template)
}

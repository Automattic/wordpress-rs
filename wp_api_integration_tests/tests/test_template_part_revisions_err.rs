use wp_api::{template_part_revisions::TemplatePartRevisionId, template_parts::TemplatePartId};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn list_err_post_invalid_parent() {
    api_client()
        .template_part_revisions()
        .list_with_edit_context(&TemplatePartId("foo".to_string()), &Default::default())
        .await
        .assert_wp_error(WpErrorCode::PostInvalidParent)
}

#[tokio::test]
#[parallel]
async fn list_err_cannot_read_as_subscriber() {
    api_client_as_subscriber()
        .template_part_revisions()
        .list_with_edit_context(&template_part_id(), &Default::default())
        .await
        .assert_wp_error(WpErrorCode::CannotRead)
}

#[tokio::test]
#[parallel]
async fn retrieve_err_post_invalid_parent() {
    api_client()
        .template_part_revisions()
        .retrieve_with_edit_context(
            &TemplatePartId("foo".to_string()),
            &TemplatePartRevisionId(1),
        )
        .await
        .assert_wp_error(WpErrorCode::PostInvalidParent)
}

#[tokio::test]
#[parallel]
async fn retrieve_err_cannot_read_as_subscriber() {
    api_client_as_subscriber()
        .template_part_revisions()
        .retrieve_with_edit_context(&template_part_id(), &revision_id())
        .await
        .assert_wp_error(WpErrorCode::CannotRead)
}

#[tokio::test]
#[parallel]
async fn delete_err_post_invalid_parent() {
    api_client()
        .template_part_revisions()
        .delete(
            &TemplatePartId("foo".to_string()),
            &TemplatePartRevisionId(1),
        )
        .await
        .assert_wp_error(WpErrorCode::PostInvalidParent)
}

#[tokio::test]
#[parallel]
async fn delete_err_cannot_delete_as_subscriber() {
    api_client_as_subscriber()
        .template_part_revisions()
        .delete(&template_part_id(), &revision_id())
        .await
        .assert_wp_error(WpErrorCode::CannotDelete)
}

fn template_part_id() -> TemplatePartId {
    TemplatePartId(
        TestCredentials::instance()
            .integration_test_custom_template_part_id
            .to_string(),
    )
}

fn revision_id() -> TemplatePartRevisionId {
    TemplatePartRevisionId(TestCredentials::instance().revision_id_for_custom_template_part)
}

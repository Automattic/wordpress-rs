use wp_api::{template_parts::TemplatePartId, templates::TemplateStatus};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[serial]
async fn delete_template_part() {
    let response = api_client()
        .template_parts()
        .delete(&TemplatePartId(
            TestCredentials::instance()
                .integration_test_custom_template_part_id
                .to_string(),
        ))
        .await
        .assert_response();
    assert!(response.data.deleted);

    RestoreServer::db().await;
}

#[tokio::test]
#[serial]
async fn trash_template_part() {
    let response = api_client()
        .template_parts()
        .trash(&TemplatePartId(
            TestCredentials::instance()
                .integration_test_custom_template_part_id
                .to_string(),
        ))
        .await
        .assert_response();
    assert_eq!(response.data.status, TemplateStatus::Trash);

    RestoreServer::db().await;
}

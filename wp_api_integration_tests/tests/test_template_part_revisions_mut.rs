use wp_api::{template_part_revisions::TemplatePartRevisionId, template_parts::TemplatePartId};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[serial]
async fn delete_template_part_revision() {
    let response = api_client()
        .template_part_revisions()
        .delete(
            &TemplatePartId(
                TestCredentials::instance()
                    .integration_test_custom_template_part_id
                    .to_string(),
            ),
            &TemplatePartRevisionId(
                TestCredentials::instance().revision_id_for_custom_template_part,
            ),
        )
        .await
        .assert_response();
    assert!(response.data.deleted);

    RestoreServer::db().await;
}

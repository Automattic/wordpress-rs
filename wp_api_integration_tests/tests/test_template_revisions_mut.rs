use wp_api::{template_revisions::TemplateRevisionId, templates::TemplateId};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[serial]
async fn delete_template_revision() {
    let response = api_client()
        .template_revisions()
        .delete(
            &TemplateId(
                TestCredentials::instance()
                    .integration_test_custom_template_id
                    .to_string(),
            ),
            &TemplateRevisionId(TestCredentials::instance().revision_id_for_custom_template),
        )
        .await
        .assert_response();
    assert!(response.data.deleted);

    RestoreServer::db().await;
}

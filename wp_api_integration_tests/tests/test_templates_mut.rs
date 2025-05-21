use serial_test::serial;
use wp_api::templates::TemplateId;
use wp_api_integration_tests::{TestCredentials, api_client, backend::RestoreServer};

#[tokio::test]
#[serial]
async fn delete_template() {
    let template_delete_response = api_client()
        .templates()
        .delete(&TemplateId(
            TestCredentials::instance()
                .integration_test_custom_template_id
                .to_string(),
        ))
        .await;
    assert!(
        template_delete_response.is_ok(),
        "{:#?}",
        template_delete_response
    );
    assert!(template_delete_response.unwrap().data.deleted);

    RestoreServer::db().await;
}

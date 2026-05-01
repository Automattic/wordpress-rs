use wp_api::templates::{TemplateCreateParams, TemplateId};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
async fn create_autosave() {
    let content = "Test autosave template content".to_string();
    let mut params = TemplateCreateParams::new("autosave_test".to_string());
    params.content = Some(content);

    api_client()
        .template_autosaves()
        .create(&autosaved_template_id(), &params)
        .await
        .assert_response();
}

fn autosaved_template_id() -> TemplateId {
    TemplateId(
        TestCredentials::instance()
            .autosaved_template_id
            .to_string(),
    )
}

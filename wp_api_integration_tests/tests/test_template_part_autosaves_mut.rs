use wp_api::{template_parts::TemplatePartCreateParams, template_parts::TemplatePartId};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
async fn create_autosave() {
    let mut params = TemplatePartCreateParams::new("autosave_test".to_string());
    params.content = Some("Test autosave template part content".to_string());

    api_client()
        .template_part_autosaves()
        .create(&autosaved_template_part_id(), &params)
        .await
        .assert_response();
}

fn autosaved_template_part_id() -> TemplatePartId {
    TemplatePartId(
        TestCredentials::instance()
            .autosaved_template_part_id
            .to_string(),
    )
}

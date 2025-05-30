use macro_helper::generate_update_test;
use wp_api::templates::{
    SparseTemplateContent, SparseTemplateContentWrapper, SparseTemplateTitle,
    SparseTemplateTitleWrapper, TemplateCreateParams, TemplateId, TemplateStatus,
    TemplateUpdateParams, TemplateWithEditContext,
};
use wp_api_integration_tests::prelude::*;

const TEST_SLUG: &str = "foo_template_slug";
const TEST_TITLE: &str = "foo template title";

#[tokio::test]
#[serial]
async fn create_template_with_slug_and_content() {
    let content = "foo template content";
    let mut params = TemplateCreateParams::new(TEST_SLUG.to_string());
    params.content = Some(content.to_string());
    test_create_template(&params, |created_template| {
        assert_slug(&created_template);
        assert_eq!(
            created_template.content,
            SparseTemplateContentWrapper::Object(SparseTemplateContent {
                raw: Some(content.to_string()),
                rendered: None,
                protected: None,
                block_version: None
            })
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn create_template_with_slug_and_title() {
    let mut params = TemplateCreateParams::new(TEST_SLUG.to_string());
    params.title = Some(TEST_TITLE.to_string());
    test_create_template(&params, |created_template| {
        assert_slug(&created_template);
        assert_title(&created_template);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn create_template_with_slug_title_and_theme() {
    let theme = "foo template theme";
    let mut params = TemplateCreateParams::new(TEST_SLUG.to_string());
    params.title = Some(TEST_TITLE.to_string());
    params.theme = Some(theme.to_string());
    test_create_template(&params, |created_template| {
        assert_slug(&created_template);
        assert_title(&created_template);
        assert_eq!(created_template.theme, theme);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn create_template_with_slug_title_and_author() {
    let mut params = TemplateCreateParams::new(TEST_SLUG.to_string());
    params.title = Some(TEST_TITLE.to_string());
    params.author = Some(SECOND_USER_ID);
    test_create_template(&params, |created_template| {
        assert_title(&created_template);
        assert_eq!(created_template.author, SECOND_USER_ID);
    })
    .await;
}

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

#[tokio::test]
#[serial]
async fn trash_template() {
    let template_trash_response = api_client()
        .templates()
        .trash(&TemplateId(
            TestCredentials::instance()
                .integration_test_custom_template_id
                .to_string(),
        ))
        .await;
    assert!(
        template_trash_response.is_ok(),
        "{:#?}",
        template_trash_response
    );
    assert_eq!(
        template_trash_response.unwrap().data.status,
        TemplateStatus::Trash
    );

    RestoreServer::db().await;
}

generate_update_test!(
    update_content,
    content,
    "new_content".to_string(),
    |updated_template| {
        assert_eq!(
            updated_template.content,
            SparseTemplateContentWrapper::Object(SparseTemplateContent {
                raw: Some("new_content".to_string()),
                rendered: None,
                protected: None,
                block_version: Some(0)
            })
        );
    }
);
generate_update_test!(
    update_title,
    title,
    TEST_TITLE.to_string(),
    |updated_template| {
        assert_title(&updated_template);
    }
);
generate_update_test!(
    update_description,
    description,
    "new_description".to_string(),
    |updated_template| {
        assert_eq!(updated_template.description, "new_description");
    }
);
generate_update_test!(update_author, author, SECOND_USER_ID, |updated_template| {
    assert_eq!(updated_template.author, SECOND_USER_ID);
});

async fn test_update_template<F>(params: &TemplateUpdateParams, assert: F)
where
    F: Fn(TemplateWithEditContext),
{
    let response = api_client()
        .templates()
        .update(
            &TemplateId(
                TestCredentials::instance()
                    .integration_test_custom_template_id
                    .to_string(),
            ),
            params,
        )
        .await
        .assert_response();
    assert(response.data);
    RestoreServer::db().await;
}

async fn test_create_template<F>(params: &TemplateCreateParams, assert: F)
where
    F: Fn(TemplateWithEditContext),
{
    let response = api_client()
        .templates()
        .create(params)
        .await
        .assert_response();
    assert(response.data);
    RestoreServer::db().await;
}

fn assert_slug(template: &TemplateWithEditContext) {
    assert_eq!(template.slug, TEST_SLUG);
}

fn assert_title(template: &TemplateWithEditContext) {
    assert_eq!(
        template.title,
        SparseTemplateTitleWrapper::Object(SparseTemplateTitle {
            raw: Some(TEST_TITLE.to_string()),
            rendered: Some(TEST_TITLE.to_string())
        })
    );
}

mod macro_helper {
    macro_rules! generate_update_test {
        ($ident:ident, $field:ident, $new_value:expr, $assertion:expr) => {
            paste::paste! {
                #[tokio::test]
                #[serial]
                async fn $ident() {
                    let updated_value = $new_value;
                    test_update_template(
                        &TemplateUpdateParams {
                            $field: Some(updated_value),
                            ..Default::default()
                        }, $assertion)
                    .await;
                }
            }
        };
    }

    pub(super) use generate_update_test;
}

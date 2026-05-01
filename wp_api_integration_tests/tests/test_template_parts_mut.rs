use macro_helper::generate_update_test;
use wp_api::{
    template_parts::{
        TemplatePartCreateParams, TemplatePartId, TemplatePartUpdateParams,
        TemplatePartWithEditContext,
    },
    templates::{
        SparseTemplateContent, SparseTemplateContentWrapper, SparseTemplateTitle,
        SparseTemplateTitleWrapper, TemplateStatus,
    },
};
use wp_api_integration_tests::prelude::*;

const TEST_SLUG: &str = "foo_template_part_slug";
const TEST_TITLE: &str = "foo template part title";

#[tokio::test]
#[serial]
async fn create_template_part_with_slug_and_content() {
    let content = "foo template part content";
    let mut params = TemplatePartCreateParams::new(TEST_SLUG.to_string());
    params.content = Some(content.to_string());
    test_create_template_part(&params, |created| {
        assert_slug(&created);
        assert_eq!(
            created.content,
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
async fn create_template_part_with_slug_and_title() {
    let mut params = TemplatePartCreateParams::new(TEST_SLUG.to_string());
    params.title = Some(TEST_TITLE.to_string());
    test_create_template_part(&params, |created| {
        assert_slug(&created);
        assert_title(&created);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn create_template_part_with_slug_title_and_theme() {
    let theme = "foo template part theme";
    let mut params = TemplatePartCreateParams::new(TEST_SLUG.to_string());
    params.title = Some(TEST_TITLE.to_string());
    params.theme = Some(theme.to_string());
    test_create_template_part(&params, |created| {
        assert_slug(&created);
        assert_title(&created);
        assert_eq!(created.theme, Some(theme.to_string()));
    })
    .await;
}

#[tokio::test]
#[serial]
async fn create_template_part_with_slug_title_and_author() {
    let mut params = TemplatePartCreateParams::new(TEST_SLUG.to_string());
    params.title = Some(TEST_TITLE.to_string());
    params.author = Some(SECOND_USER_ID);
    test_create_template_part(&params, |created| {
        assert_title(&created);
        assert_eq!(created.author, SECOND_USER_ID);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn create_template_part_with_slug_content_and_area() {
    let mut params = TemplatePartCreateParams::new(TEST_SLUG.to_string());
    params.content = Some("foo template part content".to_string());
    params.area = Some("header".to_string());
    test_create_template_part(&params, |created| {
        assert_slug(&created);
        assert_eq!(created.area, "header");
    })
    .await;
}

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

generate_update_test!(
    update_content,
    content,
    "new_content".to_string(),
    |updated| {
        assert_eq!(
            updated.content,
            SparseTemplateContentWrapper::Object(SparseTemplateContent {
                raw: Some("new_content".to_string()),
                rendered: None,
                protected: None,
                block_version: Some(0)
            })
        );
    }
);
generate_update_test!(update_title, title, TEST_TITLE.to_string(), |updated| {
    assert_title(&updated);
});
generate_update_test!(
    update_description,
    description,
    "new_description".to_string(),
    |updated| {
        assert_eq!(updated.description, "new_description");
    }
);
generate_update_test!(update_author, author, SECOND_USER_ID, |updated| {
    assert_eq!(updated.author, SECOND_USER_ID);
});
generate_update_test!(update_area, area, "footer".to_string(), |updated| {
    assert_eq!(updated.area, "footer");
});

async fn test_update_template_part<F>(params: &TemplatePartUpdateParams, assert: F)
where
    F: Fn(TemplatePartWithEditContext),
{
    let response = api_client()
        .template_parts()
        .update(
            &TemplatePartId(
                TestCredentials::instance()
                    .integration_test_custom_template_part_id
                    .to_string(),
            ),
            params,
        )
        .await
        .assert_response();
    assert(response.data);
    RestoreServer::db().await;
}

async fn test_create_template_part<F>(params: &TemplatePartCreateParams, assert: F)
where
    F: Fn(TemplatePartWithEditContext),
{
    let response = api_client()
        .template_parts()
        .create(params)
        .await
        .assert_response();
    assert(response.data);
    RestoreServer::db().await;
}

fn assert_slug(template_part: &TemplatePartWithEditContext) {
    assert_eq!(template_part.slug, TEST_SLUG);
}

fn assert_title(template_part: &TemplatePartWithEditContext) {
    assert_eq!(
        template_part.title,
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
                    test_update_template_part(
                        &TemplatePartUpdateParams {
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

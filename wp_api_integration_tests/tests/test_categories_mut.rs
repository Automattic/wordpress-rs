use macro_helper::generate_update_test;
use serial_test::serial;
use wp_api::categories::{CategoryCreateParams, CategoryUpdateParams, CategoryWithEditContext};
use wp_api_integration_tests::backend::{Backend, RestoreServer};
use wp_api_integration_tests::{AssertResponse, CATEGORY_ID_48, CATEGORY_ID_59, api_client};
use wp_cli::WpCliCategory;

#[tokio::test]
#[serial]
async fn create_category_with_just_name() {
    test_create_category(
        &CategoryCreateParams {
            name: "foo".to_string(),
            description: None,
            slug: None,
            parent: None,
        },
        |created_category, category_from_wp_cli| {
            assert_eq!(created_category.name, "foo");
            assert_eq!(category_from_wp_cli.name, "foo");
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn create_category_with_name_and_description() {
    test_create_category(
        &CategoryCreateParams {
            name: "foo".to_string(),
            description: Some("bar".to_string()),
            slug: None,
            parent: None,
        },
        |created_category, category_from_wp_cli| {
            assert_eq!(created_category.name, "foo");
            assert_eq!(created_category.description, "bar");
            assert_eq!(category_from_wp_cli.description, "bar");
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn create_category_with_name_and_slug() {
    test_create_category(
        &CategoryCreateParams {
            name: "foo".to_string(),
            description: None,
            slug: Some("bar".to_string()),
            parent: None,
        },
        |created_category, category_from_wp_cli| {
            assert_eq!(created_category.name, "foo");
            assert_eq!(created_category.slug, "bar");
            assert_eq!(category_from_wp_cli.slug, "bar");
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn create_category_with_name_and_parent() {
    test_create_category(
        &CategoryCreateParams {
            name: "foo".to_string(),
            description: None,
            slug: None,
            parent: Some(CATEGORY_ID_48),
        },
        |created_category, category_from_wp_cli| {
            assert_eq!(created_category.name, "foo");
            assert_eq!(created_category.parent, CATEGORY_ID_48);
            assert_eq!(category_from_wp_cli.parent, CATEGORY_ID_48.0);
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn create_category_with_name_description_and_slug() {
    test_create_category(
        &CategoryCreateParams {
            name: "foo".to_string(),
            description: Some("bar".to_string()),
            slug: Some("quox".to_string()),
            parent: None,
        },
        |created_category, category_from_wp_cli| {
            assert_eq!(created_category.name, "foo");
            assert_eq!(created_category.description, "bar");
            assert_eq!(category_from_wp_cli.description, "bar");
            assert_eq!(created_category.slug, "quox");
            assert_eq!(category_from_wp_cli.slug, "quox");
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn delete_category() {
    // Delete the category using the API and ensure it's successful
    let category_delete_response = api_client().categories().delete(&CATEGORY_ID_59).await;
    assert!(
        category_delete_response.is_ok(),
        "{:#?}",
        category_delete_response
    );
    assert!(category_delete_response.unwrap().data.deleted);

    // Assert that the category was deleted
    assert!(
        !Backend::categories()
            .await
            .into_iter()
            .any(|u| u.id == CATEGORY_ID_59.0),
        "Category wasn't deleted"
    );

    RestoreServer::db().await;
}

generate_update_test!(
    update_description,
    description,
    "new_description".to_string(),
    |updated_category, updated_category_from_wp_cli| {
        assert_eq!(updated_category.description, "new_description");
        assert_eq!(updated_category_from_wp_cli.description, "new_description");
    }
);

generate_update_test!(
    update_name,
    name,
    "new_name".to_string(),
    |updated_category, updated_category_from_wp_cli| {
        assert_eq!(updated_category.name, "new_name");
        assert_eq!(updated_category_from_wp_cli.name, "new_name");
    }
);

generate_update_test!(
    update_slug,
    slug,
    "new_slug".to_string(),
    |updated_category, updated_category_from_wp_cli| {
        assert_eq!(updated_category.slug, "new_slug");
        assert_eq!(updated_category_from_wp_cli.slug, "new_slug");
    }
);

generate_update_test!(
    update_parent,
    parent,
    CATEGORY_ID_48,
    |updated_category, updated_category_from_wp_cli| {
        assert_eq!(updated_category.parent, CATEGORY_ID_48);
        assert_eq!(updated_category_from_wp_cli.parent, CATEGORY_ID_48.0);
    }
);

async fn test_create_category<F>(params: &CategoryCreateParams, assert: F)
where
    F: Fn(CategoryWithEditContext, WpCliCategory),
{
    let created_category = api_client()
        .categories()
        .create(params)
        .await
        .assert_response()
        .data;
    let created_category_from_wp_cli = Backend::category(&created_category.id).await;
    assert(created_category, created_category_from_wp_cli);
    RestoreServer::db().await;
}

async fn test_update_category<F>(params: &CategoryUpdateParams, assert: F)
where
    F: Fn(CategoryWithEditContext, WpCliCategory),
{
    let updated_category = api_client()
        .categories()
        .update(&CATEGORY_ID_59, params)
        .await
        .assert_response()
        .data;
    let updated_category_from_wp_cli = Backend::category(&CATEGORY_ID_59).await;
    assert(updated_category, updated_category_from_wp_cli);
    RestoreServer::db().await;
}

mod macro_helper {
    macro_rules! generate_update_test {
        ($ident:ident, $field:ident, $new_value:expr, $assertion:expr) => {
            paste::paste! {
                #[tokio::test]
                #[serial]
                async fn $ident() {
                    let updated_value = $new_value;
                    test_update_category(
                        &CategoryUpdateParams {
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

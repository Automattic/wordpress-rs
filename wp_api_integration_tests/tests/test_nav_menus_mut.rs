use macro_helper::generate_update_test;
use wp_api::nav_menus::{NavMenuCreateParams, NavMenuUpdateParams, NavMenuWithEditContext};
use wp_api_integration_tests::prelude::*;

const TEST_MENU_NAME: &str = "Test Nav Menu";
const TEST_MENU_DESCRIPTION: &str = "Test navigation menu description";
const TEST_MENU_SLUG: &str = "test-nav-menu";

#[tokio::test]
#[serial]
async fn create_nav_menu_with_name_only() {
    test_create_nav_menu(
        &NavMenuCreateParams {
            name: TEST_MENU_NAME.to_string(),
            ..Default::default()
        },
        |created_menu| {
            assert_eq!(created_menu.name, TEST_MENU_NAME);
            assert!(created_menu.id.0 > 0);
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn create_nav_menu_with_name_and_description() {
    test_create_nav_menu(
        &NavMenuCreateParams {
            name: TEST_MENU_NAME.to_string(),
            description: Some(TEST_MENU_DESCRIPTION.to_string()),
            ..Default::default()
        },
        |created_menu| {
            assert_eq!(created_menu.name, TEST_MENU_NAME);
            assert_eq!(created_menu.description, TEST_MENU_DESCRIPTION);
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn create_nav_menu_with_name_and_slug() {
    test_create_nav_menu(
        &NavMenuCreateParams {
            name: TEST_MENU_NAME.to_string(),
            slug: Some(TEST_MENU_SLUG.to_string()),
            ..Default::default()
        },
        |created_menu| {
            assert_eq!(created_menu.name, TEST_MENU_NAME);
            assert_eq!(created_menu.slug, TEST_MENU_SLUG);
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn create_nav_menu_with_auto_add_enabled() {
    test_create_nav_menu(
        &NavMenuCreateParams {
            name: TEST_MENU_NAME.to_string(),
            auto_add: Some(true),
            ..Default::default()
        },
        |created_menu| {
            assert_eq!(created_menu.name, TEST_MENU_NAME);
            assert!(created_menu.auto_add);
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn create_nav_menu_with_all_fields() {
    test_create_nav_menu(
        &NavMenuCreateParams {
            name: TEST_MENU_NAME.to_string(),
            description: Some(TEST_MENU_DESCRIPTION.to_string()),
            slug: Some(TEST_MENU_SLUG.to_string()),
            auto_add: Some(true),
            locations: Some(vec![]),
        },
        |created_menu| {
            assert_eq!(created_menu.name, TEST_MENU_NAME);
            assert_eq!(created_menu.description, TEST_MENU_DESCRIPTION);
            assert_eq!(created_menu.slug, TEST_MENU_SLUG);
            assert!(created_menu.auto_add);
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn delete_nav_menu() {
    let delete_response = api_client()
        .nav_menus()
        .delete(&NAV_MENU_ID_179)
        .await
        .assert_response();

    assert!(delete_response.data.deleted);
    assert_eq!(delete_response.data.previous.id, NAV_MENU_ID_179);

    RestoreServer::db().await;
}

generate_update_test!(
    update_name,
    name,
    "Updated Menu Name".to_string(),
    |updated_menu| {
        assert_eq!(updated_menu.name, "Updated Menu Name");
    }
);

generate_update_test!(
    update_description,
    description,
    "Updated menu description".to_string(),
    |updated_menu| {
        assert_eq!(updated_menu.description, "Updated menu description");
    }
);

generate_update_test!(update_auto_add_enabled, auto_add, true, |updated_menu| {
    assert!(updated_menu.auto_add);
});

generate_update_test!(update_auto_add_disabled, auto_add, false, |updated_menu| {
    assert!(!updated_menu.auto_add);
});

async fn test_create_nav_menu<F>(params: &NavMenuCreateParams, assert: F)
where
    F: Fn(NavMenuWithEditContext),
{
    let response = api_client()
        .nav_menus()
        .create(params)
        .await
        .assert_response();
    assert(response.data);
    RestoreServer::db().await;
}

async fn test_update_nav_menu<F>(params: &NavMenuUpdateParams, assert: F)
where
    F: Fn(NavMenuWithEditContext),
{
    let response = api_client()
        .nav_menus()
        .update(&NAV_MENU_ID_179, params)
        .await
        .assert_response();
    assert(response.data);
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
                    test_update_nav_menu(
                        &NavMenuUpdateParams {
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

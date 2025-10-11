use macro_helper::{generate_update_nav_menu_item_status_test, generate_update_test};
use wp_api::nav_menu_items::{
    NavMenuItemCreateParams, NavMenuItemId, NavMenuItemStatus, NavMenuItemType,
    NavMenuItemUpdateParams, NavMenuItemWithEditContext,
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[serial]
async fn create_nav_menu_item_with_just_title() {
    test_create_nav_menu_item(
        &NavMenuItemCreateParams {
            title: Some("Test Menu Item".to_string()),
            nav_menu_item_type: Some(NavMenuItemType::Custom),
            url: Some("https://example.com".to_string()),
            menus: Some(NAV_MENU_ID_179),
            ..Default::default()
        },
        |created_item| {
            assert_eq!(created_item.title.raw, Some("Test Menu Item".to_string()));
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn create_nav_menu_item_with_title_and_url() {
    test_create_nav_menu_item(
        &NavMenuItemCreateParams {
            title: Some("Test Menu Item".to_string()),
            nav_menu_item_type: Some(NavMenuItemType::Custom),
            url: Some("https://example.com/test".to_string()),
            menus: Some(NAV_MENU_ID_179),
            ..Default::default()
        },
        |created_item| {
            assert_eq!(created_item.title.raw, Some("Test Menu Item".to_string()));
            assert_eq!(created_item.url, "https://example.com/test");
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn create_nav_menu_item_with_description() {
    test_create_nav_menu_item(
        &NavMenuItemCreateParams {
            title: Some("Test Menu Item".to_string()),
            nav_menu_item_type: Some(NavMenuItemType::Custom),
            url: Some("https://example.com".to_string()),
            description: Some("Test description".to_string()),
            menus: Some(NAV_MENU_ID_179),
            ..Default::default()
        },
        |created_item| {
            assert_eq!(created_item.description, "Test description");
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn create_nav_menu_item_with_classes() {
    test_create_nav_menu_item(
        &NavMenuItemCreateParams {
            title: Some("Test Menu Item".to_string()),
            nav_menu_item_type: Some(NavMenuItemType::Custom),
            url: Some("https://example.com".to_string()),
            classes: vec!["class1".to_string(), "class2".to_string()],
            menus: Some(NAV_MENU_ID_179),
            ..Default::default()
        },
        |created_item| {
            assert_eq!(
                created_item.classes,
                vec!["class1".to_string(), "class2".to_string()]
            );
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn create_nav_menu_item_with_target_blank() {
    test_create_nav_menu_item(
        &NavMenuItemCreateParams {
            title: Some("Test Menu Item".to_string()),
            nav_menu_item_type: Some(NavMenuItemType::Custom),
            url: Some("https://example.com".to_string()),
            target: Some("_blank".to_string()),
            menus: Some(NAV_MENU_ID_179),
            ..Default::default()
        },
        |created_item| {
            assert_eq!(created_item.target, "_blank");
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn create_nav_menu_item_with_attr_title() {
    test_create_nav_menu_item(
        &NavMenuItemCreateParams {
            title: Some("Test Menu Item".to_string()),
            nav_menu_item_type: Some(NavMenuItemType::Custom),
            url: Some("https://example.com".to_string()),
            attr_title: Some("Title attribute".to_string()),
            menus: Some(NAV_MENU_ID_179),
            ..Default::default()
        },
        |created_item| {
            assert_eq!(created_item.attr_title, "Title attribute");
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn create_nav_menu_item_with_xfn() {
    test_create_nav_menu_item(
        &NavMenuItemCreateParams {
            title: Some("Test Menu Item".to_string()),
            nav_menu_item_type: Some(NavMenuItemType::Custom),
            url: Some("https://example.com".to_string()),
            xfn: vec!["friend".to_string(), "colleague".to_string()],
            menus: Some(NAV_MENU_ID_179),
            ..Default::default()
        },
        |created_item| {
            assert_eq!(
                created_item.xfn,
                vec!["friend".to_string(), "colleague".to_string()]
            );
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn create_nav_menu_item_with_menu_order() {
    test_create_nav_menu_item(
        &NavMenuItemCreateParams {
            title: Some("Test Menu Item".to_string()),
            nav_menu_item_type: Some(NavMenuItemType::Custom),
            url: Some("https://example.com".to_string()),
            menu_order: Some(5),
            menus: Some(NAV_MENU_ID_179),
            ..Default::default()
        },
        |created_item| {
            assert_eq!(created_item.menu_order, 5);
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn delete_nav_menu_item() {
    let nav_menu_item_id = NavMenuItemId(TestCredentials::instance().nav_menu_item_id);
    let delete_response = api_client()
        .nav_menu_items()
        .delete(&nav_menu_item_id)
        .await;
    assert!(delete_response.is_ok(), "{delete_response:#?}");
    assert!(delete_response.unwrap().data.deleted);

    RestoreServer::db().await;
}

generate_update_test!(
    update_title,
    title,
    "Updated Title".to_string(),
    |updated_item| {
        assert_eq!(updated_item.title.raw, Some("Updated Title".to_string()));
    }
);

generate_update_test!(
    update_url,
    url,
    "https://updated.example.com".to_string(),
    |updated_item| {
        assert_eq!(updated_item.url, "https://updated.example.com");
    }
);

generate_update_test!(
    update_description,
    description,
    "Updated description".to_string(),
    |updated_item| {
        assert_eq!(updated_item.description, "Updated description");
    }
);

generate_update_test!(
    update_attr_title,
    attr_title,
    "Updated attr title".to_string(),
    |updated_item| {
        assert_eq!(updated_item.attr_title, "Updated attr title");
    }
);

generate_update_test!(
    update_target,
    target,
    "_blank".to_string(),
    |updated_item| {
        assert_eq!(updated_item.target, "_blank");
    }
);

generate_update_test!(update_menu_order, menu_order, 10i64, |updated_item| {
    assert_eq!(updated_item.menu_order, 10);
});

#[tokio::test]
#[serial]
async fn update_classes() {
    let updated_value = vec!["new-class1".to_string(), "new-class2".to_string()];
    test_update_nav_menu_item(
        &NavMenuItemUpdateParams {
            classes: updated_value.clone(),
            ..Default::default()
        },
        |updated_item| {
            assert_eq!(updated_item.classes, updated_value.clone());
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn update_xfn() {
    let updated_value = vec!["met".to_string(), "co-worker".to_string()];
    test_update_nav_menu_item(
        &NavMenuItemUpdateParams {
            xfn: updated_value.clone(),
            ..Default::default()
        },
        |updated_item| {
            assert_eq!(updated_item.xfn, updated_value.clone());
        },
    )
    .await;
}

generate_update_nav_menu_item_status_test!(Draft);
generate_update_nav_menu_item_status_test!(Publish);

async fn test_create_nav_menu_item<F>(params: &NavMenuItemCreateParams, assert: F)
where
    F: Fn(NavMenuItemWithEditContext),
{
    let created_item = api_client()
        .nav_menu_items()
        .create(params)
        .await
        .assert_response()
        .data;
    assert(created_item);
    RestoreServer::db().await;
}

async fn test_update_nav_menu_item<F>(params: &NavMenuItemUpdateParams, assert: F)
where
    F: Fn(NavMenuItemWithEditContext),
{
    let updated_item = api_client()
        .nav_menu_items()
        .update(
            &NavMenuItemId(TestCredentials::instance().nav_menu_item_id),
            params,
        )
        .await
        .assert_response()
        .data;
    assert(updated_item);
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
                    test_update_nav_menu_item(
                        &NavMenuItemUpdateParams {
                            $field: Some(updated_value),
                            ..Default::default()
                        }, $assertion)
                    .await;
                }
            }
        };
    }

    macro_rules! generate_update_nav_menu_item_status_test {
        ($status:ident) => {
            paste::paste! {
                #[tokio::test]
                #[serial]
                async fn [<update_nav_menu_item_status_to_ $status:lower>]() {
                    test_update_nav_menu_item(
                        &NavMenuItemUpdateParams {
                            status: Some(NavMenuItemStatus::$status),
                            ..Default::default()
                        },
                        |updated_item| {
                            assert_eq!(updated_item.status, NavMenuItemStatus::$status);
                        }
                    ).await;
                }
            }
        };
    }

    pub(super) use generate_update_nav_menu_item_status_test;
    pub(super) use generate_update_test;
}

use wp_api::nav_menu_items::{
    NavMenuItemCreateParams, NavMenuItemId, NavMenuItemType, NavMenuItemUpdateParams,
};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn list_nav_menu_items_err_cannot_view() {
    api_client_as_subscriber()
        .nav_menu_items()
        .list_with_view_context(&Default::default())
        .await
        .assert_wp_error(WpErrorCode::CannotView);
}

#[tokio::test]
#[parallel]
async fn retrieve_nav_menu_item_err_cannot_view() {
    api_client_as_subscriber()
        .nav_menu_items()
        .retrieve_with_view_context(&NavMenuItemId(TestCredentials::instance().nav_menu_item_id))
        .await
        .assert_wp_error(WpErrorCode::CannotView);
}

#[tokio::test]
#[parallel]
async fn list_nav_menu_items_err_forbidden_context() {
    api_client_with_auth_provider(WpAuthenticationProvider::none().into())
        .nav_menu_items()
        .list_with_edit_context(&Default::default())
        .await
        .assert_wp_error(WpErrorCode::ForbiddenContext);
}

#[tokio::test]
#[parallel]
async fn retrieve_nav_menu_item_err_forbidden_context() {
    api_client_with_auth_provider(WpAuthenticationProvider::none().into())
        .nav_menu_items()
        .retrieve_with_edit_context(&NavMenuItemId(TestCredentials::instance().nav_menu_item_id))
        .await
        .assert_wp_error(WpErrorCode::ForbiddenContext);
}

#[tokio::test]
#[parallel]
async fn retrieve_nav_menu_item_err_post_invalid_id() {
    api_client()
        .nav_menu_items()
        .retrieve_with_edit_context(&NavMenuItemId(99999999))
        .await
        .assert_wp_error(WpErrorCode::PostInvalidId);
}

#[tokio::test]
#[parallel]
async fn create_nav_menu_item_err_cannot_create() {
    api_client_as_subscriber()
        .nav_menu_items()
        .create(&NavMenuItemCreateParams {
            title: Some("Test Item".to_string()),
            menus: Some(NAV_MENU_ID_179),
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::CannotCreate);
}

#[tokio::test]
#[parallel]
async fn create_nav_menu_item_err_title_required() {
    api_client()
        .nav_menu_items()
        .create(&NavMenuItemCreateParams {
            nav_menu_item_type: Some(NavMenuItemType::Custom),
            menus: Some(NAV_MENU_ID_179),
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::TitleRequired);
}

#[tokio::test]
#[parallel]
async fn create_nav_menu_item_err_url_required() {
    api_client()
        .nav_menu_items()
        .create(&NavMenuItemCreateParams {
            title: Some("Test Item".to_string()),
            nav_menu_item_type: Some(NavMenuItemType::Custom),
            menus: Some(NAV_MENU_ID_179),
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::UrlRequired);
}

#[tokio::test]
#[parallel]
async fn create_nav_menu_item_err_invalid_param() {
    api_client()
        .nav_menu_items()
        .create(&NavMenuItemCreateParams {
            title: Some("Test Item".to_string()),
            nav_menu_item_type: Some(NavMenuItemType::Custom),
            url: Some("javascript:alert('xss')".to_string()),
            menus: Some(NAV_MENU_ID_179),
            ..Default::default()
        })
        .await
        // Returns `WpErrorCode::InvalidParam` instead of `WpErrorCode::InvalidUrl`
        .assert_wp_error(WpErrorCode::InvalidParam);
}

#[tokio::test]
#[parallel]
async fn create_nav_menu_item_err_term_invalid_id() {
    api_client()
        .nav_menu_items()
        .create(&NavMenuItemCreateParams {
            nav_menu_item_type: Some(NavMenuItemType::Taxonomy),
            object_id: Some(99999999),
            menus: Some(NAV_MENU_ID_179),
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::TermInvalidId);
}

#[tokio::test]
#[parallel]
async fn create_nav_menu_item_err_post_invalid_id() {
    api_client()
        .nav_menu_items()
        .create(&NavMenuItemCreateParams {
            nav_menu_item_type: Some(NavMenuItemType::PostType),
            object_id: Some(99999999),
            menus: Some(NAV_MENU_ID_179),
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::PostInvalidId);
}

#[tokio::test]
#[parallel]
async fn create_nav_menu_item_err_post_invalid_type() {
    api_client()
        .nav_menu_items()
        .create(&NavMenuItemCreateParams {
            nav_menu_item_type: Some(NavMenuItemType::PostTypeArchive),
            object: Some("invalid_post_type_that_does_not_exist".to_string()),
            menus: Some(NAV_MENU_ID_179),
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::PostInvalidType);
}

#[tokio::test]
#[parallel]
async fn update_nav_menu_item_err_cannot_edit() {
    api_client_as_subscriber()
        .nav_menu_items()
        .update(
            &NavMenuItemId(TestCredentials::instance().nav_menu_item_id),
            &NavMenuItemUpdateParams {
                title: Some("Updated Title".to_string()),
                ..Default::default()
            },
        )
        .await
        .assert_wp_error(WpErrorCode::CannotEdit);
}

#[tokio::test]
#[parallel]
async fn update_nav_menu_item_err_post_invalid_id() {
    api_client()
        .nav_menu_items()
        .update(
            &NavMenuItemId(99999999),
            &NavMenuItemUpdateParams {
                title: Some("Updated Title".to_string()),
                ..Default::default()
            },
        )
        .await
        .assert_wp_error(WpErrorCode::PostInvalidId);
}

#[tokio::test]
#[parallel]
async fn delete_nav_menu_item_err_cannot_delete() {
    api_client_as_subscriber()
        .nav_menu_items()
        .delete(&NavMenuItemId(TestCredentials::instance().nav_menu_item_id))
        .await
        .assert_wp_error(WpErrorCode::CannotDelete);
}

#[tokio::test]
#[parallel]
async fn delete_nav_menu_item_err_post_invalid_id() {
    api_client()
        .nav_menu_items()
        .delete(&NavMenuItemId(99999999))
        .await
        .assert_wp_error(WpErrorCode::PostInvalidId);
}

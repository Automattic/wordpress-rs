use wp_api::nav_menus::{NavMenuCreateParams, NavMenuId, NavMenuListParams, NavMenuUpdateParams};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn create_nav_menu_err_cannot_create() {
    api_client_as_subscriber()
        .nav_menus()
        .create(&NavMenuCreateParams {
            name: "Test Menu".to_string(),
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::CannotCreate);
}

// #[tokio::test]
// #[parallel]
// async fn create_nav_menu_err_menu_exists() {
//     api_client()
//         .nav_menus()
//         .create(&NavMenuCreateParams {
//             name: "Main Menu".to_string(),
//             ..Default::default()
//         })
//         .await
//         .assert_wp_error(WpErrorCode::MenuExists);
// }

#[tokio::test]
#[parallel]
async fn create_nav_menu_err_invalid_menu_location() {
    api_client()
        .nav_menus()
        .create(&NavMenuCreateParams {
            name: "Test Menu Invalid Location".to_string(),
            locations: Some(vec!["invalid_location_that_does_not_exist".to_string()]),
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::InvalidParam);
}

#[tokio::test]
#[parallel]
async fn retrieve_nav_menu_err_term_invalid() {
    api_client()
        .nav_menus()
        .retrieve_with_edit_context(&NavMenuId(99999999))
        .await
        .assert_wp_error(WpErrorCode::TermInvalid);
}

#[tokio::test]
#[parallel]
async fn update_nav_menu_err_cannot_update() {
    api_client_as_subscriber()
        .nav_menus()
        .update(
            &NAV_MENU_ID_179,
            &NavMenuUpdateParams {
                name: Some("Updated Menu Name".to_string()),
                ..Default::default()
            },
        )
        .await
        .assert_wp_error(WpErrorCode::CannotUpdate);
}

#[tokio::test]
#[parallel]
async fn update_nav_menu_err_term_invalid() {
    api_client()
        .nav_menus()
        .update(
            &NavMenuId(99999999),
            &NavMenuUpdateParams {
                name: Some("Updated Menu".to_string()),
                ..Default::default()
            },
        )
        .await
        .assert_wp_error(WpErrorCode::TermInvalid);
}

#[tokio::test]
#[parallel]
async fn update_nav_menu_err_invalid_menu_location() {
    api_client()
        .nav_menus()
        .update(
            &NAV_MENU_ID_179,
            &NavMenuUpdateParams {
                locations: Some(vec!["invalid_location_that_does_not_exist".to_string()]),
                ..Default::default()
            },
        )
        .await
        .assert_wp_error(WpErrorCode::InvalidParam);
}

#[tokio::test]
#[parallel]
async fn delete_nav_menu_err_cannot_delete() {
    api_client_as_subscriber()
        .nav_menus()
        .delete(&NAV_MENU_ID_179)
        .await
        .assert_wp_error(WpErrorCode::CannotDelete);
}

#[tokio::test]
#[parallel]
async fn delete_nav_menu_err_term_invalid() {
    api_client()
        .nav_menus()
        .delete(&NavMenuId(99999999))
        .await
        .assert_wp_error(WpErrorCode::TermInvalid);
}

#[tokio::test]
#[parallel]
async fn list_nav_menus_err_forbidden_context() {
    api_client_with_auth_provider(WpAuthenticationProvider::none().into())
        .nav_menus()
        .list_with_edit_context(&NavMenuListParams::default())
        .await
        .assert_wp_error(WpErrorCode::ForbiddenContext);
}

#[tokio::test]
#[parallel]
async fn retrieve_nav_menu_err_forbidden_context() {
    api_client_with_auth_provider(WpAuthenticationProvider::none().into())
        .nav_menus()
        .retrieve_with_edit_context(&NAV_MENU_ID_179)
        .await
        .assert_wp_error(WpErrorCode::ForbiddenContext);
}

#[tokio::test]
#[parallel]
async fn list_nav_menus_with_post_err_forbidden_context() {
    api_client()
        .nav_menus()
        .list_with_edit_context(&NavMenuListParams {
            post: Some(FIRST_POST_ID),
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::ForbiddenContext);
}

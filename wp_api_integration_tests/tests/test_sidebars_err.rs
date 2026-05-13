use wp_api::sidebars::SidebarId;
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn list_sidebars_err_cannot_manage_widgets() {
    api_client_as_subscriber()
        .sidebars()
        .list_with_edit_context()
        .await
        .assert_wp_error(WpErrorCode::CannotManageWidgets);
}

#[tokio::test]
#[parallel]
async fn retrieve_sidebar_err_cannot_manage_widgets() {
    api_client_as_subscriber()
        .sidebars()
        .retrieve_with_edit_context(&SidebarId("wp_inactive_widgets".to_string()))
        .await
        .assert_wp_error(WpErrorCode::CannotManageWidgets);
}

#[tokio::test]
#[parallel]
async fn retrieve_sidebar_err_not_found() {
    api_client()
        .sidebars()
        .retrieve_with_edit_context(&SidebarId(
            "nonexistent_sidebar_that_does_not_exist".to_string(),
        ))
        .await
        .assert_wp_error(WpErrorCode::SidebarNotFound);
}

#[tokio::test]
#[parallel]
async fn update_sidebar_err_cannot_manage_widgets() {
    api_client_as_subscriber()
        .sidebars()
        .update(
            &SidebarId("wp_inactive_widgets".to_string()),
            &wp_api::sidebars::SidebarUpdateParams::default(),
        )
        .await
        .assert_wp_error(WpErrorCode::CannotManageWidgets);
}

use serial_test::parallel;
use wp_api::{
    themes::{ThemeListParams, ThemeStatus},
    WpErrorCode,
};
use wp_api_integration_tests::{
    api_client, api_client_as_subscriber, AssertWpError, THEME_TWENTY_TWENTY_FIVE,
};

#[tokio::test]
#[parallel]
async fn list_err_cannot_view_active_theme() {
    api_client_as_subscriber()
        .themes()
        .list_with_edit_context(&ThemeListParams {
            status: Some(ThemeStatus::Active),
        })
        .await
        .assert_wp_error(WpErrorCode::CannotViewActiveTheme);
}

#[tokio::test]
#[parallel]
async fn list_err_cannot_view_themes() {
    api_client_as_subscriber()
        .themes()
        .list_with_edit_context(&ThemeListParams::default())
        .await
        .assert_wp_error(WpErrorCode::CannotViewThemes);
}

#[tokio::test]
#[parallel]
async fn retrieve_err_cannot_view_themes() {
    api_client_as_subscriber()
        .themes()
        .retrieve_with_edit_context(&THEME_TWENTY_TWENTY_FIVE.into())
        .await
        .assert_wp_error(WpErrorCode::CannotViewThemes);
}

#[tokio::test]
#[parallel]
async fn retrieve_err_theme_not_found() {
    api_client()
        .themes()
        .retrieve_with_edit_context(&"invalid_stylesheet".into())
        .await
        .assert_wp_error(WpErrorCode::ThemeNotFound);
}

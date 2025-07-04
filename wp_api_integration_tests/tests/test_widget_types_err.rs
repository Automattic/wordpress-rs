use wp_api::widget_types::WidgetTypeId;
use wp_api_integration_tests::{WIDGET_TYPE_TEXT, prelude::*};

#[tokio::test]
#[parallel]
async fn list_widget_types_err_cannot_manage_widgets() {
    api_client_with_auth_provider(WpAuthenticationProvider::none().into())
        .widget_types()
        .list_with_edit_context()
        .await
        .assert_wp_error(WpErrorCode::CannotManageWidgets);
}

#[tokio::test]
#[parallel]
async fn list_widget_types_err_subscriber_cannot_manage_widgets() {
    api_client_as_subscriber()
        .widget_types()
        .list_with_edit_context()
        .await
        .assert_wp_error(WpErrorCode::CannotManageWidgets);
}

#[tokio::test]
#[parallel]
async fn retrieve_widget_type_err_cannot_manage_widgets() {
    api_client_with_auth_provider(WpAuthenticationProvider::none().into())
        .widget_types()
        .retrieve_with_edit_context(&WidgetTypeId(WIDGET_TYPE_TEXT.to_string()))
        .await
        .assert_wp_error(WpErrorCode::CannotManageWidgets);
}

#[tokio::test]
#[parallel]
async fn retrieve_widget_type_err_invalid_widget_type() {
    api_client()
        .widget_types()
        .retrieve_with_edit_context(&WidgetTypeId("nonexistent-widget-type".to_string()))
        .await
        .assert_wp_error(WpErrorCode::WidgetTypeInvalid);
}

#[tokio::test]
#[parallel]
async fn retrieve_widget_type_err_invalid_widget_type_embed_context() {
    api_client()
        .widget_types()
        .retrieve_with_embed_context(&WidgetTypeId("invalid-widget-123".to_string()))
        .await
        .assert_wp_error(WpErrorCode::WidgetTypeInvalid);
}

#[tokio::test]
#[parallel]
async fn retrieve_widget_type_err_invalid_widget_type_view_context() {
    api_client()
        .widget_types()
        .retrieve_with_view_context(&WidgetTypeId("another-invalid-widget".to_string()))
        .await
        .assert_wp_error(WpErrorCode::WidgetTypeInvalid);
}

#[tokio::test]
#[parallel]
async fn retrieve_widget_type_err_subscriber_cannot_manage_widgets() {
    api_client_as_subscriber()
        .widget_types()
        .retrieve_with_view_context(&WidgetTypeId(WIDGET_TYPE_TEXT.to_string()))
        .await
        .assert_wp_error(WpErrorCode::CannotManageWidgets);
}

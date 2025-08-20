use wp_api::widget_types::WidgetTypeId;
use wp_api::widgets::{
    WidgetCreateParams, WidgetId, WidgetInstanceParams, WidgetListParams, WidgetUpdateParams,
};
use wp_api_integration_tests::{WIDGET_INACTIVE_WIDGETS_SIDEBAR, WIDGET_TYPE_TEXT, prelude::*};

#[tokio::test]
#[parallel]
async fn create_widget_err_cannot_manage_widgets() {
    api_client_as_subscriber()
        .widgets()
        .create(&valid_widget_create_params())
        .await
        .assert_wp_error(WpErrorCode::CannotManageWidgets);
}

#[tokio::test]
#[parallel]
async fn create_widget_err_invalid_widget() {
    // Test creating a widget with a non-existent widget type.
    // The WordPress controller validates that the provided `id_base` corresponds to a registered
    // widget type.
    let params = WidgetCreateParams {
        id_base: WidgetTypeId("non_existent_widget_type".to_string()),
        sidebar: WIDGET_INACTIVE_WIDGETS_SIDEBAR.to_string(),
        instance: None,
        form_data: None,
    };
    api_client()
        .widgets()
        .create(&params)
        .await
        .assert_wp_error(WpErrorCode::InvalidWidget);
}

#[tokio::test]
#[parallel]
async fn create_widget_err_invalid_widget_malformed_encoded_instance() {
    let instance = WidgetInstanceParams::Encoded {
        encoded: "foo".to_string(),
        hash: "bar".to_string(),
    };
    let params = WidgetCreateParams {
        id_base: WidgetTypeId(WIDGET_TYPE_TEXT.to_string()),
        sidebar: WIDGET_INACTIVE_WIDGETS_SIDEBAR.to_string(),
        instance: Some(instance),
        form_data: None,
    };
    api_client()
        .widgets()
        .create(&params)
        .await
        .assert_wp_error(WpErrorCode::InvalidWidget);
}

#[tokio::test]
#[parallel]
async fn delete_widget_err_cannot_manage_widgets() {
    // The WordPress widgets controller requires 'edit_theme_options' capability which
    // subscribers don't have.
    api_client_as_subscriber()
        .widgets()
        .delete(&WidgetId("foo".to_string()))
        .await
        .assert_wp_error(WpErrorCode::CannotManageWidgets);
}

#[tokio::test]
#[parallel]
async fn delete_widget_err_widget_not_found() {
    api_client()
        .widgets()
        .delete(&WidgetId("non-existent-widget-99999".to_string()))
        .await
        .assert_wp_error(WpErrorCode::WidgetNotFound);
}

#[tokio::test]
#[parallel]
async fn list_widgets_err_cannot_manage_widgets() {
    api_client_with_auth_provider(WpAuthenticationProvider::none().into())
        .widgets()
        .list_with_edit_context(&WidgetListParams::default())
        .await
        .assert_wp_error(WpErrorCode::CannotManageWidgets);
}

#[tokio::test]
#[parallel]
async fn retrieve_widget_err_widget_not_found() {
    api_client()
        .widgets()
        .retrieve_with_edit_context(&WidgetId("non-existent-widget-99999".to_string()))
        .await
        .assert_wp_error(WpErrorCode::WidgetNotFound);
}

#[tokio::test]
#[parallel]
async fn update_widget_err_cannot_manage_widgets() {
    // The WordPress widgets controller requires 'edit_theme_options' capability which
    // subscribers don't have.
    api_client_as_subscriber()
        .widgets()
        .update(&WidgetId("foo".to_string()), &WidgetUpdateParams::default())
        .await
        .assert_wp_error(WpErrorCode::CannotManageWidgets);
}

#[tokio::test]
#[parallel]
async fn update_widget_err_invalid_widget() {
    api_client()
        .widgets()
        .update(
            &WidgetId("non-existent-widget-99999".to_string()),
            &WidgetUpdateParams::default(),
        )
        .await
        .assert_wp_error(WpErrorCode::InvalidWidget);
}

// Helpers

fn valid_widget_create_params() -> WidgetCreateParams {
    WidgetCreateParams {
        id_base: WidgetTypeId("text".to_string()),
        sidebar: WIDGET_INACTIVE_WIDGETS_SIDEBAR.to_string(),
        instance: None,
        form_data: None,
    }
}

use wp_api::{
    sidebars::{SidebarId, SidebarUpdateParams},
    widgets::WidgetId,
};
use wp_api_integration_tests::{WIDGET_ID_BLOCK_2, prelude::*};

#[tokio::test]
#[serial]
async fn update_sidebar_widgets() {
    let sidebar_id = &SidebarId("wp_inactive_widgets".to_string());
    let widget_id = WidgetId(WIDGET_ID_BLOCK_2.to_string());
    let params = SidebarUpdateParams {
        widgets: vec![widget_id.clone()],
    };
    let response = api_client()
        .sidebars()
        .update(sidebar_id, &params)
        .await
        .assert_response();
    assert!(response.data.widgets.contains(&widget_id));

    RestoreServer::db().await;
}

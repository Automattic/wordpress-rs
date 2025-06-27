use std::collections::HashMap;
use wp_api::{
    JsonValue,
    widget_types::WidgetTypeId,
    widgets::{WidgetCreateParams, WidgetId, WidgetInstanceCreateParams, WidgetWithEditContext},
};
use wp_api_integration_tests::{WIDGET_ID_BLOCK_2, prelude::*};

const TEST_SIDEBAR: &str = "wp_inactive_widgets";
const TEST_WIDGET_TYPE: &str = "text";

#[tokio::test]
#[serial]
async fn create_widget() {
    let params = WidgetCreateParams {
        id_base: WidgetTypeId(TEST_WIDGET_TYPE.to_string()),
        sidebar: TEST_SIDEBAR.to_string(),
        instance: None,
        form_data: None,
    };
    test_create_widget(&params, |created_widget| {
        assert_eq!(
            created_widget.id_base,
            WidgetTypeId(TEST_WIDGET_TYPE.to_string())
        );
        assert_eq!(created_widget.sidebar, TEST_SIDEBAR);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn create_widget_with_raw_instance() {
    let mut raw = HashMap::new();
    raw.insert(
        "title".to_string(),
        JsonValue::String("foo_title".to_string()),
    );
    let instance = WidgetInstanceCreateParams::Raw { raw };
    let params = WidgetCreateParams {
        id_base: WidgetTypeId(TEST_WIDGET_TYPE.to_string()),
        sidebar: TEST_SIDEBAR.to_string(),
        instance: Some(instance),
        form_data: None,
    };
    test_create_widget(&params, |created_widget| {
        assert_eq!(
            created_widget.id_base,
            WidgetTypeId(TEST_WIDGET_TYPE.to_string())
        );
        assert_eq!(created_widget.sidebar, TEST_SIDEBAR);
        assert_eq!(
            created_widget
                .instance
                .raw
                .expect("raw instance expected for this test")
                .get("title"),
            Some(&JsonValue::String("foo_title".to_string()))
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn create_widget_with_encoded() {
    let encoded =
        "YTozOntzOjU6InRpdGxlIjtzOjM6ImZvbyI7czo0OiJ0ZXh0IjtzOjM6ImJhciI7czo2OiJmaWx0ZXIiO2I6MDt9";
    let instance = WidgetInstanceCreateParams::Encoded {
        encoded: encoded.to_string(),
        hash: "617122067917f65822b0f1db144ac92b".to_string(),
    };
    let params = WidgetCreateParams {
        id_base: WidgetTypeId(TEST_WIDGET_TYPE.to_string()),
        sidebar: TEST_SIDEBAR.to_string(),
        instance: Some(instance),
        form_data: None,
    };
    test_create_widget(&params, |created_widget| {
        assert_eq!(
            created_widget.id_base,
            WidgetTypeId(TEST_WIDGET_TYPE.to_string())
        );
        assert_eq!(created_widget.sidebar, TEST_SIDEBAR);
        assert_eq!(created_widget.instance.encoded, Some(encoded.to_string()));
    })
    .await;
}

#[tokio::test]
#[serial]
async fn delete_widget() {
    let widget_delete_response = api_client()
        .widgets()
        .delete(&WidgetId(WIDGET_ID_BLOCK_2.to_string()))
        .await;
    assert!(
        widget_delete_response.is_ok(),
        "{:#?}",
        widget_delete_response
    );
    assert!(widget_delete_response.unwrap().data.deleted);

    RestoreServer::db().await;
}

async fn test_create_widget<F>(params: &WidgetCreateParams, assert: F)
where
    F: Fn(WidgetWithEditContext),
{
    let response = api_client()
        .widgets()
        .create(params)
        .await
        .assert_response();
    assert(response.data);
    RestoreServer::db().await;
}

// async fn test_update_widget<F>(widget_id: &WidgetId, params: &WidgetUpdateParams, assert: F)
// where
//     F: Fn(WidgetWithEditContext),
// {
//     let response = api_client()
//         .widgets()
//         .update(widget_id, params)
//         .await
//         .assert_response();
//     assert(response.data);
//     RestoreServer::db().await;
// }

use std::collections::HashMap;
use wp_api::{
    JsonValue,
    widget_types::WidgetTypeId,
    widgets::{WidgetCreateParams, WidgetInstanceCreateParams, WidgetWithEditContext},
};
use wp_api_integration_tests::prelude::*;

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

// #[tokio::test]
// #[serial]
// async fn delete_widget() {
//     // First create a widget to delete
//     let mut instance = HashMap::new();
//     instance.insert("title".to_string(), "Widget to Delete".to_string());
//
//     let create_params = WidgetCreateParams {
//         id: None,
//         id_base: Some(TEST_ID_BASE.to_string()),
//         sidebar: TEST_SIDEBAR.to_string(),
//         instance: Some(instance),
//         form_data: None,
//     };
//
//     let created_response = api_client()
//         .widgets()
//         .create(&create_params)
//         .await
//         .assert_response();
//
//     let widget_id = &created_response.data.id;
//
//     // Now delete it
//     let delete_response = api_client().widgets().delete(widget_id).await;
//
//     assert!(delete_response.is_ok(), "{:#?}", delete_response);
//     let delete_data = delete_response.unwrap().data;
//     assert!(delete_data.deleted);
//     assert_eq!(delete_data.previous.id, *widget_id);
//
//     RestoreServer::db().await;
// }
//
// #[tokio::test]
// #[serial]
// async fn trash_widget() {
//     // First create a widget to trash
//     let mut instance = HashMap::new();
//     instance.insert("title".to_string(), "Widget to Trash".to_string());
//
//     let create_params = WidgetCreateParams {
//         id: None,
//         id_base: Some(TEST_ID_BASE.to_string()),
//         sidebar: TEST_SIDEBAR.to_string(),
//         instance: Some(instance),
//         form_data: None,
//     };
//
//     let created_response = api_client()
//         .widgets()
//         .create(&create_params)
//         .await
//         .assert_response();
//
//     let widget_id = &created_response.data.id;
//
//     // Now trash it (move to inactive widgets)
//     let trash_response = api_client().widgets().trash(widget_id).await;
//
//     assert!(trash_response.is_ok(), "{:#?}", trash_response);
//     let trashed_widget = trash_response.unwrap().data;
//     assert_eq!(trashed_widget.id, *widget_id);
//     // When trashing, widgets are typically moved to inactive widgets sidebar
//     assert_eq!(trashed_widget.sidebar, "wp_inactive_widgets");
//
//     RestoreServer::db().await;
// }
//
// generate_update_test!(
//     update_sidebar,
//     sidebar,
//     "sidebar-1".to_string(),
//     |updated_widget| {
//         assert_eq!(updated_widget.sidebar, "sidebar-1");
//     }
// );
//
// generate_update_test!(
//     update_instance_title,
//     instance,
//     {
//         let mut instance = HashMap::new();
//         instance.insert("title".to_string(), "Updated Title".to_string());
//         instance.insert("text".to_string(), "Updated content".to_string());
//         instance
//     },
//     |updated_widget| {
//         assert!(updated_widget.instance.is_some());
//         if let Some(widget_instance) = &updated_widget.instance {
//             assert!(widget_instance.raw.is_some());
//             if let Some(raw) = &widget_instance.raw {
//                 assert_eq!(raw.get("title"), Some(&"Updated Title".to_string()));
//                 assert_eq!(raw.get("text"), Some(&"Updated content".to_string()));
//             }
//         }
//     }
// );
//
// generate_update_test!(
//     update_with_form_data,
//     form_data,
//     "widget-text[2][title]=Form+Updated+Title&widget-text[2][text]=Form+updated+content"
//         .to_string(),
//     |updated_widget| {
//         // The form data should have been processed and reflected in the instance
//         assert!(updated_widget.instance.is_some());
//     }
// );

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

// async fn test_update_existing_widget<F>(params: &WidgetUpdateParams, assert: F)
// where
//     F: Fn(WidgetWithEditContext),
// {
//     // Create a widget first
//     let mut instance = HashMap::new();
//     instance.insert("title".to_string(), "Initial Title".to_string());
//     instance.insert("text".to_string(), "Initial content".to_string());
//
//     let create_params = WidgetCreateParams {
//         id: None,
//         id_base: Some(TEST_ID_BASE.to_string()),
//         sidebar: TEST_SIDEBAR.to_string(),
//         instance: Some(instance),
//         form_data: None,
//     };
//
//     let created_response = api_client()
//         .widgets()
//         .create(&create_params)
//         .await
//         .assert_response();
//
//     let widget_id = &created_response.data.id;
//
//     // Now update it
//     test_update_widget(widget_id, params, assert).await;
// }

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

// mod macro_helper {
//     macro_rules! generate_update_test {
//         ($ident:ident, $field:ident, $new_value:expr, $assertion:expr) => {
//             paste::paste! {
//                 #[tokio::test]
//                 #[serial]
//                 async fn $ident() {
//                     let updated_value = $new_value;
//                     test_update_existing_widget(
//                         &WidgetUpdateParams {
//                             $field: Some(updated_value),
//                             ..Default::default()
//                         }, $assertion)
//                     .await;
//                 }
//             }
//         };
//     }
//
//     pub(super) use generate_update_test;
// }

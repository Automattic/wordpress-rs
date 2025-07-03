use macro_helper::generate_update_test;
use std::collections::HashMap;
use wp_api::{
    JsonValue,
    widget_types::WidgetTypeId,
    widgets::{
        WidgetCreateParams, WidgetId, WidgetInstanceParams, WidgetUpdateParams,
        WidgetWithEditContext,
    },
};
use wp_api_integration_tests::{
    WIDGET_ID_BLOCK_2, WIDGET_INACTIVE_WIDGETS_SIDEBAR, WIDGET_TYPE_TEXT, prelude::*,
};

#[tokio::test]
#[serial]
async fn create_widget() {
    let params = WidgetCreateParams {
        id_base: WidgetTypeId(WIDGET_TYPE_TEXT.to_string()),
        sidebar: WIDGET_INACTIVE_WIDGETS_SIDEBAR.to_string(),
        instance: None,
        form_data: None,
    };
    test_create_widget(&params, |created_widget| {
        assert_eq!(
            created_widget.id_base,
            WidgetTypeId(WIDGET_TYPE_TEXT.to_string())
        );
        assert_eq!(created_widget.sidebar, WIDGET_INACTIVE_WIDGETS_SIDEBAR);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn create_widget_with_raw_instance() {
    let new_title = "foo_title";
    let mut raw = HashMap::new();
    raw.insert(
        "title".to_string(),
        JsonValue::String(new_title.to_string()),
    );
    let instance = WidgetInstanceParams::Raw { raw };
    let params = WidgetCreateParams {
        id_base: WidgetTypeId(WIDGET_TYPE_TEXT.to_string()),
        sidebar: WIDGET_INACTIVE_WIDGETS_SIDEBAR.to_string(),
        instance: Some(instance),
        form_data: None,
    };
    test_create_widget(&params, |created_widget| {
        assert_eq!(
            created_widget.id_base,
            WidgetTypeId(WIDGET_TYPE_TEXT.to_string())
        );
        assert_eq!(created_widget.sidebar, WIDGET_INACTIVE_WIDGETS_SIDEBAR);
        assert_eq!(
            created_widget
                .instance
                .raw
                .expect("raw instance expected for this test")
                .get("title"),
            Some(&JsonValue::String(new_title.to_string()))
        );
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

generate_update_test!(
    update_widget_sidebar,
    sidebar,
    "foo".to_string(),
    |updated_widget| {
        assert_eq!(updated_widget.sidebar, "foo".to_string());
    }
);

#[tokio::test]
#[serial]
async fn update_widget_instance() {
    let new_content = "foo_content";
    let mut raw = HashMap::new();
    raw.insert(
        "content".to_string(),
        JsonValue::String(new_content.to_string()),
    );
    let instance = WidgetInstanceParams::Raw { raw };
    test_update_widget(
        &WidgetUpdateParams {
            instance: Some(instance),
            ..Default::default()
        },
        |updated_widget| {
            assert_eq!(
                updated_widget
                    .instance
                    .raw
                    .expect("raw instance expected for this test")
                    .get("content"),
                Some(&JsonValue::String(new_content.to_string()))
            );
        },
    )
    .await;
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

async fn test_update_widget<F>(params: &WidgetUpdateParams, assert: F)
where
    F: Fn(WidgetWithEditContext),
{
    let response = api_client()
        .widgets()
        .update(&WidgetId(WIDGET_ID_BLOCK_2.to_string()), params)
        .await
        .assert_response();
    assert(response.data);
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
                    test_update_widget(
                        &WidgetUpdateParams {
                            $field: Some(updated_value),
                            ..Default::default()
                        }, $assertion)
                    .await;
                }
            }
        };
    }

    pub(super) use generate_update_test;
}

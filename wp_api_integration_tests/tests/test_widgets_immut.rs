use wp_api::widgets::{
    SparseWidgetFieldWithEditContext, SparseWidgetFieldWithEmbedContext,
    SparseWidgetFieldWithViewContext, WidgetId, WidgetListParams,
};
use wp_api_integration_tests::{WIDGET_ID_BLOCK_2, prelude::*};

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_edit_context(#[case] params: WidgetListParams) {
    api_client()
        .widgets()
        .list_with_edit_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_embed_context(#[case] params: WidgetListParams) {
    api_client()
        .widgets()
        .list_with_embed_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[apply(list_cases)]
#[parallel]
async fn list_with_view_context(#[case] params: WidgetListParams) {
    api_client()
        .widgets()
        .list_with_view_context(&params)
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_edit_context() {
    api_client()
        .widgets()
        .retrieve_with_edit_context(&WidgetId(WIDGET_ID_BLOCK_2.to_string()))
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_embed_context() {
    api_client()
        .widgets()
        .retrieve_with_embed_context(&WidgetId(WIDGET_ID_BLOCK_2.to_string()))
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_view_context() {
    api_client()
        .widgets()
        .retrieve_with_view_context(&WidgetId(WIDGET_ID_BLOCK_2.to_string()))
        .await
        .assert_response();
}

#[template]
#[rstest]
#[case::default(WidgetListParams::default())]
#[case::sidebar_wp_inactive_widgets(generate!(WidgetListParams, (sidebar, Some("wp_inactive_widgets".to_string()))))]
pub fn list_cases(#[case] params: WidgetListParams) {}

mod filter {
    use super::*;

    wp_api::generate_sparse_widget_field_with_edit_context_test_cases!();
    wp_api::generate_sparse_widget_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_widget_field_with_view_context_test_cases!();

    #[apply(sparse_widget_field_with_edit_context_test_cases)]
    #[case(&[SparseWidgetFieldWithEditContext::Id, SparseWidgetFieldWithEditContext::IdBase])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_edit_context(
        #[case] fields: &[SparseWidgetFieldWithEditContext],
        #[values(
            WidgetListParams::default(),
            generate!(WidgetListParams, (sidebar, Some("wp_inactive_widgets".to_string()))),
        )]
        params: WidgetListParams,
    ) {
        api_client()
            .widgets()
            .filter_list_with_edit_context(&params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|widget| {
                widget.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_widget_field_with_edit_context_test_cases)]
    #[case(&[SparseWidgetFieldWithEditContext::Id, SparseWidgetFieldWithEditContext::IdBase])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_edit_context(
        #[case] fields: &[SparseWidgetFieldWithEditContext],
    ) {
        let widget = api_client()
            .widgets()
            .filter_retrieve_with_edit_context(&WidgetId(WIDGET_ID_BLOCK_2.to_string()), fields)
            .await
            .assert_response()
            .data;
        widget.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }

    #[apply(sparse_widget_field_with_embed_context_test_cases)]
    #[case(&[SparseWidgetFieldWithEmbedContext::Id, SparseWidgetFieldWithEmbedContext::IdBase])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_embed_context(
        #[case] fields: &[SparseWidgetFieldWithEmbedContext],
        #[values(
            WidgetListParams::default(),
            generate!(WidgetListParams, (sidebar, Some("wp_inactive_widgets".to_string()))),
        )]
        params: WidgetListParams,
    ) {
        api_client()
            .widgets()
            .filter_list_with_embed_context(&params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|widget| {
                widget.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_widget_field_with_embed_context_test_cases)]
    #[case(&[SparseWidgetFieldWithEmbedContext::Id, SparseWidgetFieldWithEmbedContext::IdBase])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_embed_context(
        #[case] fields: &[SparseWidgetFieldWithEmbedContext],
    ) {
        let widget = api_client()
            .widgets()
            .filter_retrieve_with_embed_context(&WidgetId(WIDGET_ID_BLOCK_2.to_string()), fields)
            .await
            .assert_response()
            .data;
        widget.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }

    #[apply(sparse_widget_field_with_view_context_test_cases)]
    #[case(&[SparseWidgetFieldWithViewContext::Id, SparseWidgetFieldWithViewContext::IdBase])]
    #[tokio::test]
    #[parallel]
    async fn filter_list_with_view_context(
        #[case] fields: &[SparseWidgetFieldWithViewContext],
        #[values(
            WidgetListParams::default(),
            generate!(WidgetListParams, (sidebar, Some("wp_inactive_widgets".to_string()))),
        )]
        params: WidgetListParams,
    ) {
        api_client()
            .widgets()
            .filter_list_with_view_context(&params, fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|widget| {
                widget.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_widget_field_with_view_context_test_cases)]
    #[case(&[SparseWidgetFieldWithViewContext::Id, SparseWidgetFieldWithViewContext::IdBase])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_with_view_context(
        #[case] fields: &[SparseWidgetFieldWithViewContext],
    ) {
        let widget = api_client()
            .widgets()
            .filter_retrieve_with_view_context(&WidgetId(WIDGET_ID_BLOCK_2.to_string()), fields)
            .await
            .assert_response()
            .data;
        widget.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }
}

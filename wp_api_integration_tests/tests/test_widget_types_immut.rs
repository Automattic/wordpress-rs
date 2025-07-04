use wp_api::widget_types::{
    SparseWidgetTypeFieldWithEditContext, SparseWidgetTypeFieldWithEmbedContext,
    SparseWidgetTypeFieldWithViewContext, WidgetTypeId,
};
use wp_api_integration_tests::{WIDGET_TYPE_TEXT, prelude::*};

#[tokio::test]
#[parallel]
async fn list_with_edit_context() {
    let response = api_client()
        .widget_types()
        .list_with_edit_context()
        .await
        .assert_response();
    assert!(!response.data.is_empty());
}

#[tokio::test]
#[parallel]
async fn list_with_embed_context() {
    api_client()
        .widget_types()
        .list_with_embed_context()
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn list_with_view_context() {
    api_client()
        .widget_types()
        .list_with_view_context()
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_edit_context() {
    api_client()
        .widget_types()
        .retrieve_with_edit_context(&WidgetTypeId(WIDGET_TYPE_TEXT.to_string()))
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_embed_context() {
    api_client()
        .widget_types()
        .retrieve_with_embed_context(&WidgetTypeId(WIDGET_TYPE_TEXT.to_string()))
        .await
        .assert_response();
}

#[tokio::test]
#[parallel]
async fn retrieve_with_view_context() {
    api_client()
        .widget_types()
        .retrieve_with_view_context(&WidgetTypeId(WIDGET_TYPE_TEXT.to_string()))
        .await
        .assert_response();
}

mod filter {
    use super::*;

    wp_api::generate_sparse_widget_type_field_with_edit_context_test_cases!();
    wp_api::generate_sparse_widget_type_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_widget_type_field_with_view_context_test_cases!();

    #[apply(sparse_widget_type_field_with_edit_context_test_cases)]
    #[case(&[SparseWidgetTypeFieldWithEditContext::Id, SparseWidgetTypeFieldWithEditContext::Name])]
    #[tokio::test]
    #[parallel]
    async fn filter_widget_types_with_edit_context(
        #[case] fields: &[SparseWidgetTypeFieldWithEditContext],
    ) {
        api_client()
            .widget_types()
            .filter_list_with_edit_context(fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|widget_type| {
                widget_type.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_widget_type_field_with_edit_context_test_cases)]
    #[case(&[SparseWidgetTypeFieldWithEditContext::Id, SparseWidgetTypeFieldWithEditContext::Name])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_widget_types_with_edit_context(
        #[case] fields: &[SparseWidgetTypeFieldWithEditContext],
    ) {
        let widget_type = api_client()
            .widget_types()
            .filter_retrieve_with_edit_context(&WidgetTypeId(WIDGET_TYPE_TEXT.to_string()), fields)
            .await
            .assert_response()
            .data;
        widget_type.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }

    #[apply(sparse_widget_type_field_with_embed_context_test_cases)]
    #[case(&[SparseWidgetTypeFieldWithEmbedContext::Id, SparseWidgetTypeFieldWithEmbedContext::Name])]
    #[tokio::test]
    #[parallel]
    async fn filter_widget_types_with_embed_context(
        #[case] fields: &[SparseWidgetTypeFieldWithEmbedContext],
    ) {
        api_client()
            .widget_types()
            .filter_list_with_embed_context(fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|widget_type| {
                widget_type.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_widget_type_field_with_embed_context_test_cases)]
    #[case(&[SparseWidgetTypeFieldWithEmbedContext::Id, SparseWidgetTypeFieldWithEmbedContext::Name])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_widget_types_with_embed_context(
        #[case] fields: &[SparseWidgetTypeFieldWithEmbedContext],
    ) {
        let widget_type = api_client()
            .widget_types()
            .filter_retrieve_with_embed_context(&WidgetTypeId(WIDGET_TYPE_TEXT.to_string()), fields)
            .await
            .assert_response()
            .data;
        widget_type.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }

    #[apply(sparse_widget_type_field_with_view_context_test_cases)]
    #[case(&[SparseWidgetTypeFieldWithViewContext::Id, SparseWidgetTypeFieldWithViewContext::Name])]
    #[tokio::test]
    #[parallel]
    async fn filter_widget_types_with_view_context(
        #[case] fields: &[SparseWidgetTypeFieldWithViewContext],
    ) {
        api_client()
            .widget_types()
            .filter_list_with_view_context(fields)
            .await
            .assert_response()
            .data
            .iter()
            .for_each(|widget_type| {
                widget_type.assert_that_instance_fields_nullability_match_provided_fields(fields)
            });
    }

    #[apply(sparse_widget_type_field_with_view_context_test_cases)]
    #[case(&[SparseWidgetTypeFieldWithViewContext::Id, SparseWidgetTypeFieldWithViewContext::Name])]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_widget_types_with_view_context(
        #[case] fields: &[SparseWidgetTypeFieldWithViewContext],
    ) {
        let widget_type = api_client()
            .widget_types()
            .filter_retrieve_with_view_context(&WidgetTypeId(WIDGET_TYPE_TEXT.to_string()), fields)
            .await
            .assert_response()
            .data;
        widget_type.assert_that_instance_fields_nullability_match_provided_fields(fields)
    }
}

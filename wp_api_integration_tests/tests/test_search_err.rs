use serial_test::parallel;
use wp_api::{
    WpErrorCode,
    search_results::{SearchListParams, SearchResultSubtype, SearchResultType},
};
use wp_api_integration_tests::{AssertWpError, api_client};

#[tokio::test]
#[parallel]
async fn list_err_object_type_invalid_param() {
    api_client()
        .search()
        .list_with_view_context(&SearchListParams {
            object_type: Some(SearchResultType::Custom("foo".to_string())),
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::InvalidParam);
}

#[tokio::test]
#[parallel]
async fn list_err_object_subtype_invalid_param() {
    api_client()
        .search()
        .list_with_view_context(&SearchListParams {
            object_subtype: Some(SearchResultSubtype::Custom("foo".to_string())),
            ..Default::default()
        })
        .await
        .assert_wp_error(WpErrorCode::InvalidParam);
}

use serial_test::parallel;
use wp_api::{
    taxonomies::{TaxonomyListParams, TaxonomyType},
    WpErrorCode,
};
use wp_api_integration_tests::{api_client_as_subscriber, AssertWpError};

#[tokio::test]
#[parallel]
async fn list_err_cannot_view() {
    api_client_as_subscriber()
        .taxonomies()
        .list_with_edit_context(&TaxonomyListParams::default())
        .await
        .assert_wp_error(WpErrorCode::CannotView);
}

#[tokio::test]
#[parallel]
async fn retrieve_err_forbidden_context() {
    api_client_as_subscriber()
        .taxonomies()
        .retrieve_with_edit_context(&TaxonomyType::PostTag)
        .await
        .assert_wp_error(WpErrorCode::ForbiddenContext);
}

#[tokio::test]
#[parallel]
async fn retrieve_err_taxonomy_invalid() {
    api_client_as_subscriber()
        .taxonomies()
        .retrieve_with_edit_context(&TaxonomyType::Custom("foo".to_string()))
        .await
        .assert_wp_error(WpErrorCode::TaxonomyInvalid);
}

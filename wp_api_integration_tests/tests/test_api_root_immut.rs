use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn test_fetching_api_root() {
    let response = api_client().api_root().get().await.assert_response();

    assert_eq!(response.data.url, "http://localhost");
}

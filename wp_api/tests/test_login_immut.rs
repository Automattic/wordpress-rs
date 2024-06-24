use integration_test_common::{AssertResponse, AsyncWpNetworking};
use std::sync::Arc;

pub mod integration_test_common;

#[tokio::test]
async fn test_login_for_localhost() {
    let site_url = "http://localhost";
    let executor = Arc::new(AsyncWpNetworking::default());
    let wp_rest_api_urls = wp_api::login::find_api_urls(site_url, executor)
        .await
        .assert_response();
    dbg!("wp_rest_api_urls: {}", wp_rest_api_urls);
}

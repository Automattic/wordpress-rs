use integration_test_common::{AssertResponse, AsyncWpNetworking};
use std::sync::Arc;

pub mod integration_test_common;

#[tokio::test]
async fn test_login_flow() {
    let site_url = "http://localhost".to_string();
    let executor = Arc::new(AsyncWpNetworking::default());

    let r = wp_api::login::find_api_urls(site_url, executor)
        .await
        .assert_response();
    dbg!("r: {}", r);
}

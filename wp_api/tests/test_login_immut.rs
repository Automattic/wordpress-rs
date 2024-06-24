use integration_test_common::{AssertResponse, AsyncWpNetworking};
use rstest::rstest;
use std::sync::Arc;

pub mod integration_test_common;

#[rstest]
#[case("http://localhost")]
#[case("http://localhost/wp-json")]
#[case("http://localhost/wp-admin.php")]
#[case("https://orchestremetropolitain.com/wp-json")]
#[case("orchestremetropolitain.com/wp-json")]
// TODO: Theses cases should work, but they don't yet
//#[case("localhost")]
//#[case("https://orchestremetropolitain.com")]
#[tokio::test]
async fn test_login_flow(#[case] site_url: &str) {
    let executor = Arc::new(AsyncWpNetworking::default());
    let wp_rest_api_urls = wp_api::login::find_api_urls(site_url, executor)
        .await
        .assert_response();
    dbg!("wp_rest_api_urls: {}", wp_rest_api_urls);
}

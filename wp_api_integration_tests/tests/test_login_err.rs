use rstest::rstest;
use serial_test::parallel;
use std::sync::Arc;
use wp_api::login::{UrlDiscoveryError, WpLoginClient};
use wp_api_integration_tests::AsyncWpNetworking;

#[rstest]
#[case("http://optional-https.wpmt.co")] // Fails because it's `http`
#[tokio::test]
#[parallel]
async fn test_login_flow_err_url_discovery_failed(#[case] site_url: &str) {
    let client = WpLoginClient::new(Arc::new(AsyncWpNetworking::default()));
    let err = client
        .api_discovery(site_url.to_string())
        .await
        .unwrap_err();
    assert!(matches!(err, UrlDiscoveryError::UrlDiscoveryFailed { .. }));
}

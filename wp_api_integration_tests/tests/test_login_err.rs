use rstest::rstest;
use serial_test::parallel;
use std::sync::Arc;
use wp_api::login::login_client::WpLoginClient;
use wp_api::login::url_discovery::{AutoDiscoveryAttemptFailure, AutoDiscoveryAttemptType};
use wp_api_integration_tests::AsyncWpNetworking;

#[rstest]
#[case("http://optional-https.wpmt.co")] // Fails because it's `http`
#[tokio::test]
#[parallel]
async fn test_login_flow_err_parse_api_details(#[case] site_url: &str) {
    let client = WpLoginClient::new(Arc::new(AsyncWpNetworking::default()));
    let mut result = client.api_discovery(site_url.to_string()).await;
    let original_attempt_error = result
        .attempts
        .remove(&AutoDiscoveryAttemptType::Original)
        .unwrap()
        .result
        .unwrap_err();
    println!();
    assert!(
        matches!(
            original_attempt_error,
            AutoDiscoveryAttemptFailure::ParseApiDetails { .. }
        ),
        "{:#?}",
        original_attempt_error
    );
}

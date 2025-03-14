use rstest::rstest;
use serial_test::parallel;
use std::sync::Arc;
use wp_api::{
    login::{login_client::WpLoginClient, url_discovery::AutoDiscoveryAttemptType},
    middleware::WpApiMiddlewarePipeline,
    reqwest_request_executor::ReqwestRequestExecutor,
};

#[rstest]
#[case("http://jalib923knblakis9ba92q3nbaslkes.nope")]
#[tokio::test]
#[parallel]
async fn test_login_flow_err_network_error(#[case] site_url: &str) {
    let client = WpLoginClient::new(
        Arc::new(ReqwestRequestExecutor::new(true)),
        Arc::new(WpApiMiddlewarePipeline::default()),
    );
    let mut result = client.api_discovery(site_url.to_string()).await;
    let original_attempt_error = result
        .attempts
        .remove(&AutoDiscoveryAttemptType::UserInput)
        .unwrap()
        .api_discovery_result
        .unwrap_err();
    assert!(
        original_attempt_error.is_network_error(),
        "{:#?}",
        original_attempt_error
    );
}

#[rstest]
#[case("https://wordfence.wpmt.co")]
#[tokio::test]
#[parallel]
async fn application_passwords_not_supported(#[case] site_url: &str) {
    let client = WpLoginClient::new(
        Arc::new(ReqwestRequestExecutor::new(true)),
        Arc::new(WpApiMiddlewarePipeline::default()),
    );
    let mut result = client.api_discovery(site_url.to_string()).await;
    let original_attempt_error = result
        .attempts
        .remove(&AutoDiscoveryAttemptType::UserInput)
        .unwrap()
        .api_discovery_result
        .unwrap_err();
    assert_eq!(
        original_attempt_error.is_application_passwords_disabled(),
        Some(true),
        "{:#?}",
        original_attempt_error
    );
}

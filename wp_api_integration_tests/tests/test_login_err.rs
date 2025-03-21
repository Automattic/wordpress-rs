use rstest::rstest;
use serial_test::parallel;
use std::sync::Arc;
use wp_api::{
    RequestExecutionError, RequestExecutionErrorReason,
    login::{
        login_client::WpLoginClient,
        url_discovery::{
            AutoDiscoveryAttemptFailure, AutoDiscoveryAttemptType, FindApiRootFailure,
        },
    },
    middleware::{ApiDiscoveryAuthenticationMiddleware, WpApiMiddleware, WpApiMiddlewarePipeline},
    reqwest_request_executor::ReqwestRequestExecutor,
};

#[rstest]
#[case("http://jalib923knblakis9ba92q3nbaslkes.nope")]
#[tokio::test]
#[parallel]
async fn test_login_flow_err_network_error(#[case] site_url: &str) {
    let original_attempt_error = login_flow_err_helper(site_url, vec![]).await;
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
async fn test_login_flow_err_application_passwords_not_supported(#[case] site_url: &str) {
    let original_attempt_error = login_flow_err_helper(site_url, vec![]).await;
    assert_eq!(
        original_attempt_error.is_application_passwords_disabled(),
        Some(true),
        "{:#?}",
        original_attempt_error
    );
}

#[rstest]
#[case("https://basic-auth.wpmt.co")]
#[tokio::test]
#[parallel]
async fn test_login_flow_err_http_authentication_required_error(#[case] site_url: &str) {
    let original_attempt_error = login_flow_err_helper(site_url, vec![]).await;
    assert!(matches!(
        original_attempt_error,
        AutoDiscoveryAttemptFailure::FindApiRoot {
            find_api_root_failure: FindApiRootFailure::FetchHomepage {
                error: RequestExecutionError::RequestExecutionFailed {
                    reason: RequestExecutionErrorReason::HttpAuthenticationRequiredError { .. },
                    ..
                },
            },
            ..
        }
    ));
}

#[rstest]
#[case("https://basic-auth.wpmt.co")]
#[tokio::test]
#[parallel]
async fn test_login_flow_err_http_authentication_rejected_error(#[case] site_url: &str) {
    let original_attempt_error = login_flow_err_helper(
        site_url,
        vec![Arc::new(ApiDiscoveryAuthenticationMiddleware::new(
            "invalid".to_string(),
            "invalid".to_string(),
        ))],
    )
    .await;
    assert!(matches!(
        original_attempt_error,
        AutoDiscoveryAttemptFailure::FindApiRoot {
            find_api_root_failure: FindApiRootFailure::FetchHomepage {
                error: RequestExecutionError::RequestExecutionFailed {
                    reason: RequestExecutionErrorReason::HttpAuthenticationRejectedError { .. },
                    ..
                },
            },
            ..
        }
    ));
}

#[rstest]
#[case("https://www.beeper.com/")]
#[tokio::test]
#[parallel]
async fn test_login_flow_err_not_a_wordpress_site(#[case] site_url: &str) {
    let err = login_flow_err_helper(site_url, vec![]).await;
    assert!(
        matches!(
            err,
            AutoDiscoveryAttemptFailure::FindApiRoot {
                find_api_root_failure: FindApiRootFailure::ProbablyNotAWordPressSite { .. },
                ..
            }
        ),
        "{:#?}",
        err
    );
}

async fn login_flow_err_helper(
    site_url: &str,
    middlewares: Vec<Arc<dyn WpApiMiddleware>>,
) -> AutoDiscoveryAttemptFailure {
    WpLoginClient::new(
        Arc::new(ReqwestRequestExecutor::new(true)),
        Arc::new(WpApiMiddlewarePipeline { middlewares }),
    )
    .api_discovery(site_url.to_string())
    .await
    .attempts
    .remove(&AutoDiscoveryAttemptType::UserInput)
    .unwrap()
    .api_discovery_result
    .unwrap_err()
}

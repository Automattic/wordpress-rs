use std::sync::Arc;
use wp_api::{
    login::{
        login_client::WpLoginClient,
        url_discovery::{
            ApplicationPasswordsNotSupportedReason, AutoDiscoveryAttemptFailure,
            FetchAndParseApiRootFailure,
        },
    },
    middleware::WpApiMiddlewarePipeline,
    reqwest_request_executor::ReqwestRequestExecutor,
};

/// Spec Example 8
///
/// Tests that a WordPress site with WordFence installed (which disables application passwords)
/// is correctly detected. Requires the wp-env test server to be running (`make wp-env-wordfence-start`).
#[tokio::test]
async fn login_spec_8_site_with_application_passwords_disabled_by_wordfence() {
    let error = login_err("http://localhost:4100")
        .await
        .to_fetch_and_parse_api_root_failure();
    if let FetchAndParseApiRootFailure::ApplicationPasswordsNotSupported {
        reason:
            Some(ApplicationPasswordsNotSupportedReason::ApplicationPasswordBlockedByPlugin {
                ref plugin,
            }),
        ..
    } = error
    {
        assert_eq!(plugin.name, "Wordfence");
    } else {
        panic!(
            "Expected ApplicationPasswordsNotSupportedReason::ApplicationPasswordBlockedByPlugin, got: {error:?}"
        );
    }
}

trait AutoDiscoveryAttemptFailureExtension {
    fn to_fetch_and_parse_api_root_failure(self) -> FetchAndParseApiRootFailure;
}

impl AutoDiscoveryAttemptFailureExtension for AutoDiscoveryAttemptFailure {
    fn to_fetch_and_parse_api_root_failure(self) -> FetchAndParseApiRootFailure {
        if let AutoDiscoveryAttemptFailure::FetchAndParseApiRoot {
            fetch_and_parse_api_root_failure,
            ..
        } = self
        {
            fetch_and_parse_api_root_failure
        } else {
            panic!("Expected AutoDiscoveryAttemptFailure::FetchAndParseApiRoot, got: {self:?}");
        }
    }
}

async fn login_err(site_url: &str) -> AutoDiscoveryAttemptFailure {
    let client = WpLoginClient::new(
        Arc::new(ReqwestRequestExecutor::default()),
        Arc::new(WpApiMiddlewarePipeline {
            middlewares: vec![],
        }),
    );
    client
        .api_discovery(site_url.to_string(), None)
        .await
        .combined_result()
        .expect_err("Expected api discovery to fail")
        .clone()
}

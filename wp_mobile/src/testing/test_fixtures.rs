//! Test fixtures for wp_mobile tests
//!
//! This module provides reusable test utilities such as mock API clients,
//! database setup helpers, and other common test fixtures.

use async_trait::async_trait;
use rstest::*;
use std::sync::Arc;
use wp_api::prelude::*;
use wp_api::request::{RequestContext, WpMultipartFormRequest};

#[derive(Debug)]
pub struct MockExecutor {
    execute_fn: fn(Arc<WpNetworkRequest>) -> Result<WpNetworkResponse, RequestExecutionError>,
}

impl MockExecutor {
    pub fn with_execute_fn(
        execute_fn: fn(Arc<WpNetworkRequest>) -> Result<WpNetworkResponse, RequestExecutionError>,
    ) -> Self {
        Self { execute_fn }
    }
}

#[async_trait]
impl RequestExecutor for MockExecutor {
    async fn execute(
        &self,
        request: Arc<WpNetworkRequest>,
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        (self.execute_fn)(request)
    }

    async fn upload(
        &self,
        _request: Arc<WpMultipartFormRequest>,
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        unimplemented!("upload not implemented in MockExecutor")
    }

    async fn sleep(&self, _: u64) {}

    fn cancel(&self, _: Arc<RequestContext>) {}
}

/// Empty app notifier that does nothing
#[derive(Debug)]
pub struct EmptyAppNotifier;

#[async_trait]
impl WpAppNotifier for EmptyAppNotifier {
    async fn requested_with_invalid_authentication(&self, _request_url: String) {
        // no-op
    }
}

/// rstest fixture providing a mock WpApiClient for testing
///
/// Since most service layer tests don't actually use the client (they read from cache),
/// this returns a minimal mock that panics if a network request is attempted.
/// This helps catch unexpected network calls in unit tests.
///
/// # Example
///
/// ```rust
/// #[rstest]
/// fn test_something(mock_api_client: Arc<WpApiClient>) {
///     // Use api_client in your test
/// }
/// ```
#[fixture]
pub fn mock_api_client() -> Arc<WpApiClient> {
    let mock_executor = Arc::new(MockExecutor::with_execute_fn(|_| {
        panic!("Network request should not be made in this test")
    }));

    let api_root_url =
        Arc::new(ParsedUrl::parse("https://test.local/wp-json").expect("Failed to parse test URL"));

    WpApiClient::new(
        Arc::new(WpOrgSiteApiUrlResolver::new(api_root_url)),
        WpApiClientDelegate {
            auth_provider: Arc::new(WpAuthenticationProvider::none()),
            request_executor: mock_executor,
            middleware_pipeline: Arc::new(WpApiMiddlewarePipeline::default()),
            app_notifier: Arc::new(EmptyAppNotifier),
        },
    )
    .into()
}

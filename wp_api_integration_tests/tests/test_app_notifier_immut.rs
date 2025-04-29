use async_trait::async_trait;
use serial_test::parallel;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use wp_api::{
    WpApiClient, WpApiClientDelegate, WpAppNotifier, WpErrorCode, auth::WpAuthenticationProvider,
    middleware::WpApiMiddlewarePipeline, reqwest_request_executor::ReqwestRequestExecutor,
    users::UserListParams,
};
use wp_api_integration_tests::{AssertWpError, TestCredentials, test_site_url};

#[tokio::test]
#[parallel]
async fn test_notification_unauthorized_request() {
    let notifier: Arc<FooAppNotifier> = FooAppNotifier::new(|| true).into();
    api_client_as_unauthenticated_with_notifier(notifier.clone())
        .users()
        // Edit context requires authentication
        .list_with_edit_context(&UserListParams::default())
        .await
        .assert_wp_error(WpErrorCode::ForbiddenContext);
    assert!(
        notifier.assertion.load(Ordering::Relaxed),
        "Failed to notify with `unauthorized_request`"
    );
}

fn api_client_as_unauthenticated_with_notifier(app_notifier: Arc<FooAppNotifier>) -> WpApiClient {
    WpApiClient::new(
        test_site_url(),
        WpApiClientDelegate {
            auth_provider: Arc::new(WpAuthenticationProvider::static_with_username_and_password(
                TestCredentials::instance().admin_username.to_string(),
                "invalid".to_string(),
            )),
            request_executor: Arc::new(ReqwestRequestExecutor::default()),
            middleware_pipeline: Arc::new(WpApiMiddlewarePipeline::default()),
            app_notifier,
        },
    )
}

#[derive(Debug)]
pub struct FooAppNotifier {
    unauthorized_request_fn: fn() -> bool,
    assertion: AtomicBool,
}

impl FooAppNotifier {
    pub fn new(unauthorized_request_fn: fn() -> bool) -> Self {
        Self {
            unauthorized_request_fn,
            assertion: AtomicBool::new(false),
        }
    }
}

#[async_trait]
impl WpAppNotifier for FooAppNotifier {
    async fn authentication_becomes_invalid(&self) {
        let result = (self.unauthorized_request_fn)();
        self.assertion.store(result, Ordering::Relaxed);
    }
}

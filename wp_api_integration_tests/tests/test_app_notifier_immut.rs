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
async fn test_notification_requested_with_invalid_authentication() {
    let notifier: Arc<FooAppNotifier> = FooAppNotifier::new(|| true).into();
    api_client_as_unauthenticated_with_notifier(notifier.clone())
        .users()
        // Edit context requires authentication
        .list_with_edit_context(&UserListParams::default())
        .await
        .assert_wp_error(WpErrorCode::ForbiddenContext);
    assert!(
        notifier.assertion.load(Ordering::Relaxed),
        "Failed to notify with `requested_with_invalid_authentication`"
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
    requested_with_invalid_authentication_fn: fn() -> bool,
    assertion: AtomicBool,
}

impl FooAppNotifier {
    pub fn new(requested_with_invalid_authentication_fn: fn() -> bool) -> Self {
        Self {
            requested_with_invalid_authentication_fn,
            assertion: AtomicBool::new(false),
        }
    }
}

#[async_trait]
impl WpAppNotifier for FooAppNotifier {
    async fn requested_with_invalid_authentication(&self) {
        let result = (self.requested_with_invalid_authentication_fn)();
        self.assertion.store(result, Ordering::Relaxed);
    }
}

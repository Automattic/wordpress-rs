use async_trait::async_trait;
use serial_test::parallel;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use wp_api::{
    WpApiClient, WpAppNotification, WpAppNotifier, WpAuthentication, WpErrorCode,
    middleware::WpApiMiddlewarePipeline, reqwest_request_executor::ReqwestRequestExecutor,
    users::UserListParams,
};
use wp_api_integration_tests::{AssertWpError, test_site_url};

#[tokio::test]
#[parallel]
async fn test_notification_unauthorized_request() {
    let notifier: Arc<FooAppNotifier> =
        FooAppNotifier::new(|notification| notification == WpAppNotification::UnauthorizedRequest)
            .into();
    api_client_as_unauthenticated_with_notifier(notifier.clone())
        .users()
        // Edit context requires authentication
        .list_with_edit_context(&UserListParams::default())
        .await
        .assert_wp_error(WpErrorCode::ForbiddenContext);
    assert!(
        notifier.assertion.load(Ordering::Relaxed),
        "Failed to notify with `WpAppNotification::UnauthorizedRequest`"
    );
}

fn api_client_as_unauthenticated_with_notifier(notifier: Arc<FooAppNotifier>) -> WpApiClient {
    WpApiClient::new(
        test_site_url(),
        WpAuthentication::None,
        Arc::new(ReqwestRequestExecutor::default()),
        notifier,
        Arc::new(WpApiMiddlewarePipeline::default()),
    )
}

#[derive(Debug)]
pub struct FooAppNotifier {
    notify_fn: fn(WpAppNotification) -> bool,
    assertion: AtomicBool,
}

impl FooAppNotifier {
    pub fn new(notify_fn: fn(WpAppNotification) -> bool) -> Self {
        Self {
            notify_fn,
            assertion: AtomicBool::new(false),
        }
    }
}

#[async_trait]
impl WpAppNotifier for FooAppNotifier {
    async fn notify(&self, notification: WpAppNotification) {
        let result = (self.notify_fn)(notification);
        self.assertion.store(result, Ordering::Relaxed);
    }
}

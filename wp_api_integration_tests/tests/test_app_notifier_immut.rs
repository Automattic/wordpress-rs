use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};
use wp_api::{request::RequestContext, users::UserListParams};
use wp_api_integration_tests::prelude::*;

#[tokio::test]
#[parallel]
async fn test_requested_with_invalid_authentication_for_forbidden_context() {
    let notifier: Arc<FooAppNotifier> = FooAppNotifier::new(|| true).into();
    api_client(
        Arc::new(ReqwestRequestExecutor::default()),
        notifier.clone(),
    )
    .users()
    // Edit context requires authentication
    .list_with_edit_context(&UserListParams::default())
    .await
    .assert_wp_error(WpErrorCode::ForbiddenContext);
    assert!(
        notifier.has_triggered_notification.load(Ordering::Relaxed),
        "Failed to notify with `requested_with_invalid_authentication`"
    );
}

#[tokio::test]
#[parallel]
async fn test_requested_with_invalid_authentication_for_unauthorized_error() {
    let notifier: Arc<FooAppNotifier> = FooAppNotifier::new(|| true).into();
    let executor = Arc::new(TrackedRequestExecutor::default());
    api_client(executor.clone(), notifier.clone())
        .users()
        .retrieve_me_with_edit_context()
        .await
        .assert_wp_error(WpErrorCode::Unauthorized);
    assert!(
        notifier.has_triggered_notification.load(Ordering::Relaxed),
        "Failed to notify with `requested_with_invalid_authentication`"
    );
    assert!(
        !executor
            .requested_urls
            .lock()
            .unwrap()
            .iter()
            .any(|u| u.contains("introspect")),
        "If the initial request returns `WpErrorCode::Unauthorized`, we shouldn't make a request to `/application-passwords/introspect` endpoint"
    );
}

fn api_client(
    request_executor: Arc<dyn RequestExecutor>,
    app_notifier: Arc<FooAppNotifier>,
) -> WpApiClient {
    WpApiClient::new(
        test_site_api_url_resolver(),
        WpApiClientDelegate {
            auth_provider: Arc::new(WpAuthenticationProvider::static_with_username_and_password(
                TestCredentials::instance().admin_username.to_string(),
                "invalid".to_string(),
            )),
            request_executor,
            middleware_pipeline: Arc::new(WpApiMiddlewarePipeline::default()),
            app_notifier,
        },
    )
}

#[derive(Debug)]
pub struct FooAppNotifier {
    requested_with_invalid_authentication_fn: fn() -> bool,
    has_triggered_notification: AtomicBool,
}

impl FooAppNotifier {
    pub fn new(requested_with_invalid_authentication_fn: fn() -> bool) -> Self {
        Self {
            requested_with_invalid_authentication_fn,
            has_triggered_notification: AtomicBool::new(false),
        }
    }
}

#[async_trait]
impl WpAppNotifier for FooAppNotifier {
    async fn requested_with_invalid_authentication(&self, _request_url: String) {
        let result = (self.requested_with_invalid_authentication_fn)();
        self.has_triggered_notification
            .store(result, Ordering::Relaxed);
    }
}

#[derive(Debug)]
pub struct TrackedRequestExecutor {
    executor: ReqwestRequestExecutor,
    requested_urls: Arc<Mutex<Vec<String>>>,
}

impl TrackedRequestExecutor {
    pub fn new() -> Self {
        Self {
            executor: ReqwestRequestExecutor::default(),
            requested_urls: Arc::new(Mutex::new(vec![])),
        }
    }
}

impl Default for TrackedRequestExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RequestExecutor for TrackedRequestExecutor {
    async fn execute(
        &self,
        request: Arc<WpNetworkRequest>,
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        self.requested_urls
            .lock()
            .unwrap()
            .push(request.url().0.clone());
        self.executor.execute(request).await
    }

    async fn upload_media(
        &self,
        media_upload_request: Arc<MediaUploadRequest>,
    ) -> Result<WpNetworkResponse, MediaUploadRequestExecutionError> {
        self.upload_media(media_upload_request).await
    }

    async fn sleep(&self, _: u64) {}

    fn cancel(&self, _: Arc<RequestContext>) {}
}

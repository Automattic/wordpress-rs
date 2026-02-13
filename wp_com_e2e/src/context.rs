use async_trait::async_trait;
use integration_test_credentials::WpComTestCredentials;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;
use wp_api::{
    prelude::*,
    wp_com::{WpComSiteId, client::WpComApiClient},
};
use wp_mobile::service::WpService;
use wp_mobile_cache::WpApiCache;

pub struct TestContext {
    pub client: WpComApiClient,
    pub service: WpService,
    pub token: String,
    pub runtime: Runtime,
}

#[derive(Debug)]
struct EmptyAppNotifier;

#[async_trait]
impl WpAppNotifier for EmptyAppNotifier {
    async fn requested_with_invalid_authentication(&self, _request_url: String) {
        // no-op
    }
}

impl TestContext {
    pub fn new(token: String) -> Self {
        let runtime = Runtime::new().expect("Failed to create Tokio runtime");

        let delegate = WpApiClientDelegate {
            auth_provider: WpAuthenticationProvider::static_with_auth(WpAuthentication::Bearer {
                token: token.clone(),
            })
            .into(),
            request_executor: Arc::new(ReqwestRequestExecutor::new(false, Duration::from_secs(60))),
            middleware_pipeline: Arc::new(WpApiMiddlewarePipeline::default()),
            app_notifier: Arc::new(EmptyAppNotifier),
        };

        let client = WpComApiClient::new(delegate.clone());

        let site_id = WpComTestCredentials::instance().site_id;
        // Use a test site on the test account used in CI.
        let site_id = if site_id == 0 { 229889220 } else { site_id };

        let cache = Arc::new(WpApiCache::new(None).expect("Failed to create in-memory cache"));
        cache
            .perform_migrations()
            .expect("Migrations should succeed");

        let service = WpService::new_wordpress_com(WpComSiteId(site_id), delegate, cache.clone())
            .expect("Failed to create WpService");

        Self {
            client,
            service,
            token,
            runtime,
        }
    }
}

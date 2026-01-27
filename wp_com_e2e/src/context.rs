use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;
use wp_api::{prelude::*, wp_com::client::WpComApiClient};

pub struct TestContext {
    pub client: WpComApiClient,
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

        let client = WpComApiClient::new(delegate);

        Self {
            client,
            token,
            runtime,
        }
    }
}

use std::sync::Arc;

use async_trait::async_trait;
use jetpack_api::{
    request::{JetpackNetworkResponse, JetpackRequestExecutor},
    JetpackRequestExecutionError,
};
use wp_api::request::WpNetworkRequest;
use wp_api_integration_tests::AsyncWpNetworking;

#[derive(Debug, Default)]
pub struct AsyncJpNetworking {
    inner: AsyncWpNetworking,
}

#[async_trait]
impl JetpackRequestExecutor for AsyncJpNetworking {
    async fn execute(
        &self,
        request: Arc<WpNetworkRequest>,
    ) -> Result<JetpackNetworkResponse, JetpackRequestExecutionError> {
        self.inner
            .async_request(request)
            .await
            .map_err(|err| JetpackRequestExecutionError::RequestExecutionFailed {
                status_code: err.status().map(|s| s.as_u16()),
                reason: err.to_string(),
            })
            .map(JetpackNetworkResponse::from)
    }
}

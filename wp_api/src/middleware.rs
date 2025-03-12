use crate::{
    request::{RequestExecutor, WpNetworkRequest, WpNetworkResponse},
    RequestExecutionError, RequestExecutionErrorReason,
};
use std::{fmt::Debug, sync::Arc, time::Duration};

#[derive(Debug, Default, uniffi::Object)]
pub struct WpApiMiddlewarePipeline {
    middlewares: Vec<Arc<dyn WpApiMiddleware>>,
}

#[uniffi::export]
impl WpApiMiddlewarePipeline {
    #[uniffi::constructor]
    fn new(middlewares: Vec<Arc<dyn WpApiMiddleware>>) -> Self {
        Self { middlewares }
    }

    pub async fn process(
        &self,
        request_executor: Arc<dyn RequestExecutor>,
        response: WpNetworkResponse,
        request: Arc<WpNetworkRequest>,
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        let mut response = response;

        for middleware in &self.middlewares {
            response = middleware
                .process(request_executor.clone(), response, request.clone())
                .await?;
        }

        Ok(response)
    }
}

#[derive(Debug, Default, uniffi::Object)]
struct WpApiMiddlewarePipelineBuilder {
    middlewares: Vec<Arc<dyn WpApiMiddleware>>,
}

#[uniffi::export]
impl WpApiMiddlewarePipelineBuilder {
    #[uniffi::constructor]
    fn new() -> Self {
        Self::default()
    }

    fn add_middleware(&self, middleware: Arc<dyn WpApiMiddleware>) -> Self {
        let mut new_middlewares = self.middlewares.clone();
        new_middlewares.push(middleware.clone());
        WpApiMiddlewarePipelineBuilder {
            middlewares: new_middlewares,
        }
    }

    fn build(&self) -> WpApiMiddlewarePipeline {
        println!("Building middleware pipeline");
        WpApiMiddlewarePipeline::new(<Vec<Arc<dyn WpApiMiddleware>> as Clone>::clone(
            &self.middlewares,
        ))
    }
}

#[uniffi::export(with_foreign)]
#[async_trait::async_trait]
pub trait WpApiMiddleware: Send + Sync + Debug {
    async fn process(
        &self,
        request_executor: Arc<dyn RequestExecutor>,
        response: WpNetworkResponse,
        request: Arc<WpNetworkRequest>,
    ) -> Result<WpNetworkResponse, RequestExecutionError>;
}

/// A trait for types that perform HTTP requests. Types that implement this trait
/// have middleware that can modify the HTTP request and response.
#[async_trait::async_trait]
pub trait PerformsRequests {
    fn get_middleware_pipeline(&self) -> Arc<WpApiMiddlewarePipeline>;
    fn get_request_executor(&self) -> Arc<dyn RequestExecutor>;

    async fn perform(
        &self,
        request: Arc<WpNetworkRequest>,
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        let pipeline = &self.get_middleware_pipeline();
        let response = self.get_request_executor().execute(request.clone()).await?;

        pipeline
            .process(
                self.get_request_executor().clone(),
                response,
                request.clone(),
            )
            .await
    }
}

// MARK: - RetryAfterMiddleware

#[derive(Debug, uniffi::Object)]
struct RetryAfterMiddleware {
    max_retries: u8,
    max_retry_wait_seconds: u64,
}

#[uniffi::export]
impl RetryAfterMiddleware {
    #[uniffi::constructor]
    fn new(max_retries: u8, max_retry_wait_seconds: u64) -> Self {
        println!("Creating retry middleware");
        Self {
            max_retries,
            max_retry_wait_seconds,
        }
    }
}

#[uniffi::export]
#[async_trait::async_trait]
impl WpApiMiddleware for RetryAfterMiddleware {
    async fn process(
        &self,
        request_executor: Arc<dyn RequestExecutor>,
        response: WpNetworkResponse,
        request: Arc<WpNetworkRequest>,
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        let mut response = response;

        if !response.is_rate_limit_exceeded() {
            // If the status code is not `429`, there is nothing to do
            return Ok(response);
        }
        if request.retry_count >= self.max_retries {
            return Err(RequestExecutionError::RequestExecutionFailed {
                status_code: Some(response.status_code),
                redirects: None,
                reason: RequestExecutionErrorReason::MisconfiguredRateLimitError {},
            });
        }
        if let Some(retry_after) = response.get_retry_after() {
            request_executor
                .sleep(
                    // If the server sends some super-long value, we don't want to wait that long
                    Duration::from_secs(std::cmp::min(retry_after, self.max_retry_wait_seconds))
                        .as_millis() as u64,
                )
                .await;
            let new_request = Arc::new(request.clone_with_incremented_retry_count());
            response = request_executor.execute(new_request.clone()).await?;
            self.process(request_executor, response, new_request).await
        } else {
            // We have no idea how long to wait so we shouldn't try
            Ok(response)
        }
    }
}

// MARK: - ApiDiscoveryAuthenticationMiddleware

#[derive(Debug, uniffi::Object)]
struct ApiDiscoveryAuthenticationMiddleware {
    username: String,
    password: String,
}

#[uniffi::export]
impl ApiDiscoveryAuthenticationMiddleware {
    #[uniffi::constructor]
    fn new(username: String, password: String) -> Self {
        println!("Creating HTTP authentication middleware");
        Self { username, password }
    }
}

#[uniffi::export]
#[async_trait::async_trait]
impl WpApiMiddleware for ApiDiscoveryAuthenticationMiddleware {
    async fn process(
        &self,
        request_executor: Arc<dyn RequestExecutor>,
        response: WpNetworkResponse,
        request: Arc<WpNetworkRequest>,
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        if response.request_header_map.has_http_authentication() {
            // Request was already authenticated
            return Ok(response);
        }

        if !response.is_http_authentication_required() {
            return Ok(response);
        }

        request_executor
            .execute(
                request
                    .adding_http_authentication(&self.username, &self.password)
                    .into(),
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod retry_after {
        use super::*;
        use crate::{
            request::{
                endpoint::{media_endpoint::MediaUploadRequest, WpEndpointUrl},
                WpNetworkHeaderMap,
            },
            MediaUploadRequestExecutionError,
        };
        use async_trait::async_trait;
        use http::HeaderMap;
        use std::sync::atomic::{AtomicBool, Ordering};

        // This executor will return `429` for the first request and `200` afterwards
        #[derive(Debug)]
        struct FooExecutor {
            first_request: AtomicBool,
        }

        #[async_trait]
        impl RequestExecutor for FooExecutor {
            async fn execute(
                &self,
                _: Arc<WpNetworkRequest>,
            ) -> Result<WpNetworkResponse, RequestExecutionError> {
                if self.first_request.load(Ordering::Relaxed) {
                    println!("First mock request; returning 429..");
                    self.first_request.store(false, Ordering::Relaxed);
                    Ok(rate_limit_exceeded_response())
                } else {
                    println!("Second mock request; returning 200..");
                    Ok(WpNetworkResponse {
                        body: vec![],
                        status_code: 200,
                        response_header_map: WpNetworkHeaderMap::default().into(),
                        request_url: WpEndpointUrl("http://example.com".to_string()),
                        request_header_map: Arc::new(WpNetworkHeaderMap::default()),
                    })
                }
            }

            async fn upload_media(
                &self,
                _: Arc<MediaUploadRequest>,
            ) -> Result<WpNetworkResponse, MediaUploadRequestExecutionError> {
                Err(MediaUploadRequestExecutionError::RequestExecutionFailed {
                    status_code: None,
                    redirects: None,
                    reason: RequestExecutionErrorReason::GenericError {
                        error_message: "test condition".to_string(),
                    },
                })
            }

            async fn sleep(&self, _: u64) {}
        }

        #[tokio::test]
        async fn test_retry_after_middleware_success() {
            // Since the executor returns `429` for the first request, we need to retry twice
            let result = execute_retry_after_middleware(2).await;
            assert!(result.is_ok());
            assert_eq!(result.unwrap().status_code, 200);
        }

        #[tokio::test]
        async fn test_retry_after_middleware_failure() {
            // Since the executor returns `429` for the first request, we can retry once
            let result = execute_retry_after_middleware(1).await;
            assert!(matches!(
                result,
                Err(RequestExecutionError::RequestExecutionFailed {
                    status_code: Some(429),
                    redirects: None,
                    reason: RequestExecutionErrorReason::MisconfiguredRateLimitError {},
                })
            ));
        }

        async fn execute_retry_after_middleware(
            max_retries: u8,
        ) -> Result<WpNetworkResponse, RequestExecutionError> {
            let foo_executor = FooExecutor {
                first_request: AtomicBool::new(true),
            };
            let retry_middleware = RetryAfterMiddleware::new(max_retries, 10);
            retry_middleware
                .process(
                    Arc::new(foo_executor),
                    rate_limit_exceeded_response(),
                    WpNetworkRequest::get(WpEndpointUrl("unused".to_string())).into(),
                )
                .await
        }

        fn rate_limit_exceeded_response() -> WpNetworkResponse {
            let mut map = HeaderMap::new();
            map.insert(
                http::header::RETRY_AFTER,
                http::header::HeaderValue::from_static("1"),
            );
            WpNetworkResponse {
                body: vec![],
                status_code: 429,
                response_header_map: Arc::new(map.into()),
                request_url: WpEndpointUrl("http://example.com".to_string()),
                request_header_map: Arc::new(WpNetworkHeaderMap::default()),
            }
        }
    }
}

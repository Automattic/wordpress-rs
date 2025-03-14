use crate::{
    request::{RequestExecutor, WpNetworkRequest, WpNetworkResponse},
    RequestExecutionError, RequestExecutionErrorReason,
};
use std::{fmt::Debug, sync::Arc, time::Duration};

#[derive(Debug, Default, uniffi::Object)]
pub struct WpApiMiddlewarePipeline {
    pub middlewares: Vec<Arc<dyn WpApiMiddleware>>,
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

        let response = pipeline
            .process(
                self.get_request_executor().clone(),
                response,
                request.clone(),
            )
            .await?;

        // TODO: `WpApiError::try_parse` also handles this case. Neither of these seem like the
        // correct way to handle these errors, because neither implementation is "central" enough.
        // Since there isn't a great place to handle this at the moment, and it's not clear whether
        // we'll include middleware in `WpApiClient`, we are including this logic here for now.
        if let Some(reason) = RequestExecutionErrorReason::try_from_response(&response) {
            return Err(RequestExecutionError::RequestExecutionFailed {
                status_code: Some(response.status_code),
                redirects: None,
                reason,
            });
        }

        Ok(response)
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
pub struct ApiDiscoveryAuthenticationMiddleware {
    username: String,
    password: String,
}

#[uniffi::export]
impl ApiDiscoveryAuthenticationMiddleware {
    #[uniffi::constructor]
    pub fn new(username: String, password: String) -> Self {
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

    mod api_discovery_authentication_middleware {
        use crate::{
            request::{
                endpoint::{media_endpoint::MediaUploadRequest, WpEndpointUrl},
                WpNetworkHeaderMap,
            },
            MediaUploadRequestExecutionError,
        };

        use super::*;
        use async_trait::async_trait;
        use http::HeaderMap;

        #[derive(Debug)]
        struct FooExecutor {
            execute_fn:
                fn(Arc<WpNetworkRequest>) -> Result<WpNetworkResponse, RequestExecutionError>,
        }

        #[async_trait]
        impl RequestExecutor for FooExecutor {
            async fn execute(
                &self,
                request: Arc<WpNetworkRequest>,
            ) -> Result<WpNetworkResponse, RequestExecutionError> {
                (self.execute_fn)(request)
            }

            async fn upload_media(
                &self,
                _: Arc<MediaUploadRequest>,
            ) -> Result<WpNetworkResponse, MediaUploadRequestExecutionError> {
                Err(MediaUploadRequestExecutionError::RequestExecutionFailed {
                    status_code: None,
                    redirects: None,
                    reason: RequestExecutionErrorReason::GenericError {
                        error_message: "upload_media is not used".to_string(),
                    },
                })
            }

            async fn sleep(&self, _: u64) {}
        }

        #[tokio::test]
        async fn test_api_discovery_authentication_middleware_request_has_http_authentication() {
            // This test covers the case where the initial request has authentication header and
            // the initial response is 403. This should be no-op by the middleware because the
            // initial request has the authentication header.

            let executor = FooExecutor {
                execute_fn: |_| panic!("execute function shouldn't be called for this test"),
            };
            let result =
                execute_api_discovery_authentication_middleware(executor.into(), true, 403).await;
            assert!(
                result.is_ok(),
                "Since the initial request is authenticated, middleware should be no-op"
            );
        }

        #[tokio::test]
        async fn test_api_discovery_authentication_middleware_response_is_http_authentication_required_false(
        ) {
            // This test covers the case where the initial request doesn't have authentication
            // header but the initial response is not 401 or 403. This should be no-op by the
            // middleware because the initial response is not 401 or 403.

            let executor = FooExecutor {
                execute_fn: |_| panic!("execute function shouldn't be called for this test"),
            };
            let result =
                execute_api_discovery_authentication_middleware(executor.into(), false, 301).await;
            assert!(
                result.is_ok(),
                "Since the initial response is not 401 or 403, middleware should be no-op"
            );
        }

        #[tokio::test]
        async fn test_api_discovery_authentication_middleware_retries_with_authentication_headers()
        {
            // This test covers the case where the initial request doesn't have authentication
            // header and the initial response is 401 or 403. This should result in middleware
            // making a second request by adding the authentication header.

            let executor = FooExecutor {
                execute_fn: |request| {
                    assert_eq!(request.retry_count, 1);
                    Ok(WpNetworkResponse {
                        body: vec![],
                        status_code: 204,
                        response_header_map: Arc::new(WpNetworkHeaderMap::default()),
                        request_url: WpEndpointUrl("http://example.com".to_string()),
                        request_header_map: Arc::new(WpNetworkHeaderMap::default()),
                    })
                },
            };
            let response =
                execute_api_discovery_authentication_middleware(executor.into(), false, 403)
                    .await
                    .expect("");
            assert_eq!(response.status_code, 204);
        }

        async fn execute_api_discovery_authentication_middleware(
            request_executor: Arc<FooExecutor>,
            initial_request_has_authorization_header: bool,
            initial_response_status_code: u16,
        ) -> Result<WpNetworkResponse, RequestExecutionError> {
            let middleware =
                ApiDiscoveryAuthenticationMiddleware::new("foo".to_string(), "bar".to_string());
            let mut map = HeaderMap::new();
            if initial_request_has_authorization_header {
                map.insert(
                    http::header::AUTHORIZATION,
                    http::header::HeaderValue::from_static("any_value"),
                );
            }
            middleware
                .process(
                    request_executor,
                    WpNetworkResponse {
                        body: vec![],
                        status_code: initial_response_status_code,
                        response_header_map: Arc::new(WpNetworkHeaderMap::default()),
                        request_url: WpEndpointUrl("http://example.com".to_string()),
                        request_header_map: Arc::new(map.into()),
                    },
                    WpNetworkRequest::get(WpEndpointUrl("unused".to_string())).into(),
                )
                .await
        }
    }

    mod retry_after_middleware {
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
                        error_message: "upload_media is not used".to_string(),
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

use crate::request::HttpAuthMethodParsingError;
use crate::RequestExecutionError;
use crate::WpApiError;
use crate::{
    request::{RequestExecutor, WpNetworkRequest, WpNetworkResponse},
    RequestExecutionErrorReason,
};
use std::fmt::Debug;
use std::future::Future;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[derive(Debug, uniffi::Object)]
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
    ) -> Result<WpNetworkResponse, WpApiError> {
        let mut response = response;

        for middleware in self.middlewares.iter() {
            let result = middleware
                .process(request_executor.clone(), response, request.clone())
                .await;

            match result {
                Ok(_response) => {
                    response = _response;
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }

        // This hard-coded middleware is used to detect if the server requires HTTP authentication
        // and if it does (and none was provided), it will return an error.
        response = HttpAuthenticationDetectionMiddleware::new()
            .process(request_executor.clone(), response, request.clone())
            .await?;

        Ok(response)
    }
}

impl Default for WpApiMiddlewarePipeline {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

#[uniffi::export]
fn default_middleware_pipeline() -> WpApiMiddlewarePipeline {
    WpApiMiddlewarePipeline::default()
}

#[derive(Debug, uniffi::Object)]
struct WpApiMiddlewarePipelineBuilder {
    middlewares: Vec<Arc<dyn WpApiMiddleware>>,
}

#[uniffi::export]
impl WpApiMiddlewarePipelineBuilder {
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
    ) -> Result<WpNetworkResponse, WpApiError>;
}

/// A trait for types that perform HTTP requests. Types that implement this trait
/// have middleware that can modify the HTTP request and response.
pub trait PerformsRequests {
    fn get_middleware_pipeline(&self) -> Arc<WpApiMiddlewarePipeline>;
    fn get_request_executor(&self) -> Arc<dyn RequestExecutor>;

    fn perform(
        &self,
        request: Arc<WpNetworkRequest>,
    ) -> impl std::future::Future<Output = Result<WpNetworkResponse, RequestExecutionError>> + Send
    where
        Self: Sync,
    {
        async move {
            let pipeline = &self.get_middleware_pipeline();
            let response = self.get_request_executor().execute(request.clone()).await?;

            pipeline
                .process(
                    self.get_request_executor().clone(),
                    response,
                    request.clone(),
                )
                .await
                .map_err(|e| match e {
                    WpApiError::RequestExecutionFailed {
                        status_code,
                        redirects,
                        reason,
                    } => RequestExecutionError::RequestExecutionFailed {
                        status_code,
                        redirects,
                        reason,
                    },
                    _ => RequestExecutionError::RequestExecutionFailed {
                        status_code: None,
                        redirects: None,
                        reason: RequestExecutionErrorReason::GenericError {
                            error_message: e.to_string(),
                        },
                    },
                })
        }
    }
}

// MARK: - RetryMiddleware

#[derive(Debug, uniffi::Object)]
struct RetryMiddleware {
    max_retries: u32,
    max_retry_wait_seconds: u64,
}

#[uniffi::export]
impl RetryMiddleware {
    #[uniffi::constructor]
    fn new(max_retries: u32, max_retry_wait_seconds: u64) -> Self {
        println!("Creating retry middleware");
        Self {
            max_retries,
            max_retry_wait_seconds,
        }
    }
}

#[uniffi::export]
#[async_trait::async_trait]
impl WpApiMiddleware for RetryMiddleware {
    async fn process(
        &self,
        request_executor: Arc<dyn RequestExecutor>,
        response: WpNetworkResponse,
        request: Arc<WpNetworkRequest>,
    ) -> Result<WpNetworkResponse, WpApiError> {
        let mut retry_count = 0;
        let mut response = response;

        loop {
            if retry_count >= self.max_retries {
                return Err(WpApiError::RequestExecutionFailed {
                    status_code: Some(response.status_code),
                    redirects: None,
                    reason: RequestExecutionErrorReason::MisconfiguredRateLimitError {},
                });
            }

            if response.is_rate_limit_exceeded() && self.max_retries != 0 {
                let retry_after = response.get_retry_after();

                if let Some(mut retry_after) = retry_after {
                    // If the server sends some super-long value, we don't want to wait that long
                    if retry_after > self.max_retry_wait_seconds {
                        retry_after = self.max_retry_wait_seconds;
                        async_sleep(Duration::from_secs(retry_after)).await;
                    }

                    // task::sleep(Duration::from_secs(retry_after)).await;
                    response = request_executor.execute(request.clone()).await?;
                } else {
                    return Ok(response); // It's not ok, but we'll let the layer above handle that – we have no idea how long to wait so we shouldn't try
                }
                retry_count += 1;
            }
        }
    }
}

fn async_sleep(duration: std::time::Duration) -> impl Future<Output = ()> {
    let (tx, rx) = futures::channel::oneshot::channel::<i32>();

    thread::spawn(move || {
        thread::sleep(duration);
        let _ = tx.send(0); // ignore error because it is perfectly ok when the receiver is dropped
    });

    async move {
        rx.await.unwrap();
    }
}

// MARK: - HttpAuthenticationMiddleware

#[derive(Debug, uniffi::Object)]
struct HttpAuthenticationMiddleware {
    username: String,
    password: String,
}

#[uniffi::export]
impl HttpAuthenticationMiddleware {
    #[uniffi::constructor]
    fn new(username: String, password: String) -> Self {
        println!("Creating HTTP authentication middleware");
        Self { username, password }
    }
}

#[uniffi::export]
#[async_trait::async_trait]
impl WpApiMiddleware for HttpAuthenticationMiddleware {
    async fn process(
        &self,
        request_executor: Arc<dyn RequestExecutor>,
        response: WpNetworkResponse,
        request: Arc<WpNetworkRequest>,
    ) -> Result<WpNetworkResponse, WpApiError> {
        if !response.is_http_authentication_required() {
            return Ok(response);
        }

        let new_request = request.adding_http_authentication(&self.username, &self.password);
        let original_url = new_request.url();

        let response = request_executor.execute(new_request.into()).await?;

        if response.is_http_authentication_required() {
            let reason = match response.get_http_auth_method() {
                Ok(maybe_method) => match maybe_method {
                    Some(method) => RequestExecutionErrorReason::HttpAuthenticationRejectedError {
                        hostname: original_url.into(),
                        method: Some(method),
                    },
                    None => RequestExecutionErrorReason::MisconfiguredHttpAuthenticationError {
                        issue: HttpAuthMethodParsingError::Unknown,
                    },
                },
                Err(e) => {
                    RequestExecutionErrorReason::MisconfiguredHttpAuthenticationError { issue: e }
                }
            };

            return Err(WpApiError::RequestExecutionFailed {
                status_code: Some(response.status_code),
                redirects: None,
                reason,
            });
        }

        Ok(response)
    }
}

// MARK: - HttpAuthenticationDetectionMiddleware

#[derive(Debug, uniffi::Object)]
struct HttpAuthenticationDetectionMiddleware {}

#[uniffi::export]
impl HttpAuthenticationDetectionMiddleware {
    #[uniffi::constructor]
    fn new() -> Self {
        Self {}
    }
}

#[uniffi::export]
#[async_trait::async_trait]
impl WpApiMiddleware for HttpAuthenticationDetectionMiddleware {
    async fn process(
        &self,
        _request_executor: Arc<dyn RequestExecutor>,
        response: WpNetworkResponse,
        request: Arc<WpNetworkRequest>,
    ) -> Result<WpNetworkResponse, WpApiError> {
        if !response.is_http_authentication_required() || request.has_http_authentication() {
            return Ok(response);
        }

        let reason = match response.get_http_auth_method() {
            Ok(maybe_method) => match maybe_method {
                Some(method) => RequestExecutionErrorReason::HttpAuthenticationRequiredError {
                    hostname: request.url().into(),
                    method: Some(method),
                },
                None => RequestExecutionErrorReason::MisconfiguredHttpAuthenticationError {
                    issue: HttpAuthMethodParsingError::Unknown,
                },
            },
            Err(e) => {
                RequestExecutionErrorReason::MisconfiguredHttpAuthenticationError { issue: e }
            }
        };

        Err(WpApiError::RequestExecutionFailed {
            status_code: Some(response.status_code),
            redirects: None,
            reason,
        })
    }
}

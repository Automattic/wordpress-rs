//! A request executor that plays back a script, for tests of services that use the network.

use crate::testing::EmptyAppNotifier;
use async_trait::async_trait;
use futures::channel::oneshot;
use serde_json::Value;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use wp_api::prelude::*;
use wp_api::request::{NetworkRequestAccessor, RequestContext, WpMultipartFormRequest};

pub(crate) enum Step {
    Respond(u32, Value),
    /// Waits for the test to release it, so the test can act while the request is in flight.
    Hold(oneshot::Receiver<()>, u32, Value),
    /// The device is offline, so the request never left.
    Offline,
    /// The request timed out, so the server may have applied it.
    TimedOut,
    /// The host couldn't be reached, though the device is online: it's gone, or it refused.
    Unreachable,
}

#[derive(Debug, Clone)]
pub(crate) struct Sent {
    pub(crate) method: RequestMethod,
    pub(crate) url: String,
    pub(crate) body: Option<Value>,
    /// The request's `Authorization` header.
    pub(crate) authorization: Option<String>,
    /// The request's `User-Agent` header, which only a request that names its own has: an
    /// executor adds the library's to the others.
    pub(crate) user_agent: Option<String>,
}

/// Plays back a script of responses, and records every request.
#[derive(Debug, Default)]
pub(crate) struct ScriptedExecutor {
    steps: Mutex<VecDeque<StepBox>>,
    sent: Mutex<Vec<Sent>>,
}

pub(crate) struct StepBox(Step);

impl std::fmt::Debug for StepBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Step")
    }
}

impl ScriptedExecutor {
    pub(crate) fn push(&self, step: Step) {
        self.steps.lock().unwrap().push_back(StepBox(step));
    }

    pub(crate) fn sent(&self) -> Vec<Sent> {
        self.sent.lock().unwrap().clone()
    }
}

#[async_trait]
impl RequestExecutor for ScriptedExecutor {
    async fn execute(
        &self,
        request: Arc<WpNetworkRequest>,
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        let headers = request.header_map().to_header_map();
        let header = |name: &str| {
            headers
                .get(name)
                .and_then(|value| value.to_str().ok())
                .map(str::to_string)
        };
        self.sent.lock().unwrap().push(Sent {
            method: request.method(),
            url: request.url().0,
            body: request
                .body_as_string()
                .map(|body| serde_json::from_str(&body).unwrap()),
            authorization: header("authorization"),
            user_agent: header("user-agent"),
        });

        let step = self.steps.lock().unwrap().pop_front();
        let (status_code, body) = match step.expect("Unexpected request").0 {
            Step::Respond(status_code, body) => (status_code, body),
            Step::Hold(release, status_code, body) => {
                release.await.expect("The test dropped the release");
                (status_code, body)
            }
            Step::Offline => {
                return Err(RequestExecutionError::RequestExecutionFailed {
                    status_code: None,
                    redirects: None,
                    reason: RequestExecutionErrorReason::DeviceIsOfflineError {
                        error_message: "The Internet connection appears to be offline.".to_string(),
                    },
                    request_url: request.url().0,
                    request_method: request.method(),
                });
            }
            Step::TimedOut => {
                return Err(RequestExecutionError::RequestExecutionFailed {
                    status_code: None,
                    redirects: None,
                    reason: RequestExecutionErrorReason::HttpTimeoutError,
                    request_url: request.url().0,
                    request_method: request.method(),
                });
            }
            Step::Unreachable => {
                return Err(RequestExecutionError::RequestExecutionFailed {
                    status_code: None,
                    redirects: None,
                    reason: RequestExecutionErrorReason::ConnectionError {
                        reason: "Could not connect to the server.".to_string(),
                    },
                    request_url: request.url().0,
                    request_method: request.method(),
                });
            }
        };

        Ok(WpNetworkResponse {
            body: body.to_string().into_bytes(),
            status_code,
            response_header_map: Default::default(),
            request_url: request.url(),
            request_method: request.method(),
            request_header_map: Default::default(),
        })
    }

    async fn upload(
        &self,
        _request: Arc<WpMultipartFormRequest>,
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        unimplemented!("Nothing scripted uploads")
    }

    async fn sleep(&self, _: u64) {}

    fn cancel(&self, _: Arc<RequestContext>) {}
}

pub(crate) fn delegate(executor: Arc<dyn RequestExecutor>) -> WpApiClientDelegate {
    WpApiClientDelegate {
        auth_provider: Arc::new(WpAuthenticationProvider::none()),
        request_executor: executor,
        middleware_pipeline: Arc::new(WpApiMiddlewarePipeline::default()),
        app_notifier: Arc::new(EmptyAppNotifier),
        language_provider: None,
    }
}

use crate::{
    api_error::{InvalidSslErrorReason, RequestExecutionError, RequestExecutionErrorReason},
    request::{
        NetworkRequestAccessor, RequestContext, RequestExecutor, RequestMethod,
        WpMultipartFormField, WpMultipartFormRequest, WpNetworkHeaderMap, WpNetworkRequest,
        WpNetworkResponse, user_agent,
    },
};
use async_trait::async_trait;
use h2::Error as Http2Error;
use http::{HeaderMap, HeaderValue};
use hyper::Error as HyperError;
use reqwest::multipart::Part;
use rustls::{CertificateError, Error as TlsError};
use std::{error::Error, sync::Arc, time::Duration};

const DEFAULT_TIMEOUT: u64 = 10;

#[derive(Debug)]
pub struct ReqwestRequestExecutor {
    client: reqwest::Client,
}

impl Default for ReqwestRequestExecutor {
    fn default() -> Self {
        Self::new_with_default_timeout(false)
    }
}

impl ReqwestRequestExecutor {
    pub fn new(danger_accept_invalid_certs: bool, timeout: Duration) -> Self {
        Self {
            client: reqwest::Client::builder()
                .danger_accept_invalid_certs(danger_accept_invalid_certs)
                .timeout(timeout)
                .tls_backend_rustls()
                .build()
                .expect("We should be able to build the reqwest client with this configuration"),
        }
    }

    pub fn new_with_default_timeout(danger_accept_invalid_certs: bool) -> Self {
        Self::new(
            danger_accept_invalid_certs,
            Duration::from_secs(DEFAULT_TIMEOUT),
        )
    }

    pub fn new_with_cookie_store() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT))
                .tls_backend_rustls()
                .cookie_store(true)
                .build()
                .expect("We should be able to build the reqwest client with this configuration"),
        }
    }
}

impl ReqwestRequestExecutor {
    pub async fn async_request(
        &self,
        wp_request: Arc<WpNetworkRequest>,
    ) -> Result<WpNetworkResponse, reqwest::Error> {
        let mut request_header_map = wp_request.header_map().to_header_map();
        request_header_map.insert(
            http::header::USER_AGENT,
            user_agent::reqwest::user_agent_for_reqwest_request_executor(),
        );
        let mut request = self
            .client
            .request(
                Self::request_method(wp_request.method()),
                wp_request.url().0.as_str(),
            )
            .headers(request_header_map);
        if let Some(body) = wp_request.body() {
            request = request.body(body.contents());
        }
        let mut response = request.send().await?;
        let response_header_map = std::mem::take(response.headers_mut());

        let status = response.status();
        let body = response.bytes().await?;

        Ok(WpNetworkResponse {
            status_code: status.as_u16() as u32,
            body: body.to_vec(),
            response_header_map: Arc::new(WpNetworkHeaderMap::new(response_header_map)),
            request_url: wp_request.url(),
            request_method: wp_request.method(),
            request_header_map: wp_request.header_map(),
        })
    }

    pub fn request_method(method: RequestMethod) -> http::Method {
        match method {
            RequestMethod::GET => reqwest::Method::GET,
            RequestMethod::POST => reqwest::Method::POST,
            RequestMethod::PUT => reqwest::Method::PUT,
            RequestMethod::DELETE => reqwest::Method::DELETE,
            RequestMethod::HEAD => reqwest::Method::HEAD,
        }
    }
}

#[async_trait]
impl RequestExecutor for ReqwestRequestExecutor {
    async fn execute(
        &self,
        request: Arc<WpNetworkRequest>,
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        let url = request.url().0.clone();
        let method = request.method();
        self.async_request(request)
            .await
            .map_err(|e| request_execution_error_from_reqwest(e, url, method))
    }

    async fn upload(
        &self,
        upload_request: Arc<WpMultipartFormRequest>,
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        let request = self
            .client
            .request(
                Self::request_method(upload_request.method()),
                upload_request.url().0.as_str(),
            )
            .headers(upload_request.header_map().to_header_map());
        let mut form = reqwest::multipart::Form::new();

        for field in upload_request.form() {
            match field {
                WpMultipartFormField::Text { name, value } => {
                    form = form.text(name, value);
                }
                WpMultipartFormField::File { name, file } => {
                    let file_path = file.file_path;
                    let mut file_header_map = HeaderMap::new();
                    if let Some(mime_type) = &file.mime_type {
                        file_header_map.insert(
                            http::header::CONTENT_TYPE,
                            HeaderValue::from_str(mime_type).unwrap(),
                        );
                    }
                    let part = Part::file(file_path)
                        .await
                        .unwrap()
                        .headers(file_header_map);
                    form = form.part(name, part);
                }
            }
        }

        let url = upload_request.url().0.clone();
        let method = upload_request.method();
        let request = request.multipart(form);
        let mut response = request
            .send()
            .await
            .map_err(|e| request_execution_error_from_reqwest(e, url, method))?;

        let header_map = std::mem::take(response.headers_mut());
        Ok(WpNetworkResponse {
            status_code: response.status().as_u16() as u32,
            body: response.bytes().await.unwrap().to_vec(),
            response_header_map: Arc::new(WpNetworkHeaderMap::new(header_map)),
            request_url: upload_request.url(),
            request_method: upload_request.method(),
            request_header_map: upload_request.header_map(),
        })
    }

    async fn sleep(&self, millis: u64) {
        tokio::time::sleep(std::time::Duration::from_millis(millis)).await;
    }

    fn cancel(&self, _context: Arc<RequestContext>) {
        // No-op for reqwest
    }
}

fn request_execution_error_from_reqwest(
    error: reqwest::Error,
    request_url: String,
    request_method: RequestMethod,
) -> RequestExecutionError {
    let status_code = error.status().map(|s| s.as_u16() as u32);
    let reason = if error.is_timeout() {
        RequestExecutionErrorReason::HttpTimeoutError
    } else if let Some(tls_error) = error.as_tls_error() {
        tls_error.into()
    } else if let Some(io_error) = error.as_io_error() {
        match io_error.kind() {
            std::io::ErrorKind::UnexpectedEof => RequestExecutionErrorReason::HttpError {
                reason: "The server terminated the connection unexpectedly".to_string(),
            },
            _ => RequestExecutionErrorReason::HttpError {
                reason: io_error.to_string(),
            },
        }
    } else if let Some(hyper_error) = error.as_hyper_error() {
        hyper_error.into()
    } else if error.is_connect() {
        // DNS resolution failures surface here: reqwest performs name resolution as
        // part of establishing the connection, so a failed lookup is a connect error.
        RequestExecutionErrorReason::NonExistentSiteError {
            error_message: Some(error.to_string()),
            suggested_action: None,
        }
    } else {
        RequestExecutionErrorReason::GenericError {
            error_message: error.to_string(),
        }
    };

    RequestExecutionError::RequestExecutionFailed {
        status_code,
        redirects: None,
        reason,
        request_url,
        request_method,
    }
}

/// Converts all HTTPS errors to a RequestExecutionErrorReason
impl From<&TlsError> for RequestExecutionErrorReason {
    fn from(error: &TlsError) -> Self {
        match error {
            TlsError::InvalidCertificate(CertificateError::NotValidForNameContext {
                expected,
                presented,
            }) => RequestExecutionErrorReason::InvalidSslError {
                reason: InvalidSslErrorReason::CertificateNotValidForName {
                    hostname: expected.to_str().to_string(),
                    presented_hostnames: presented.to_vec(),
                },
            },
            _ => RequestExecutionErrorReason::GenericError {
                error_message: error.to_string(),
            },
        }
    }
}

/// Converts from the Hyper frameworks underlying errors to a RequestExecutionErrorReason
impl From<&HyperError> for RequestExecutionErrorReason {
    fn from(error: &HyperError) -> Self {
        if let Some(http2_error) = error.find::<Http2Error>() {
            if let Some(reason) = http2_error.reason() {
                return RequestExecutionErrorReason::HttpError {
                    reason: reason.description().to_string(),
                };
            } else {
                return RequestExecutionErrorReason::GenericError {
                    error_message: http2_error.to_string(),
                };
            }
        }

        if error.is_closed() {
            return RequestExecutionErrorReason::GenericError {
                error_message: error.to_string(),
            };
        }

        if error.is_incomplete_message() {
            // The server terminated the connection unexpectedly
            return RequestExecutionErrorReason::GenericError {
                error_message: error.to_string(),
            };
        }

        RequestExecutionErrorReason::GenericError {
            error_message: error.to_string(),
        }
    }
}

trait ExaminableError {
    fn as_io_error(&self) -> Option<&std::io::Error>;
    fn as_tls_error(&self) -> Option<&TlsError>;
    fn as_hyper_error(&self) -> Option<&HyperError>;
}

impl ExaminableError for reqwest::Error {
    fn as_io_error(&self) -> Option<&std::io::Error> {
        self.find::<std::io::Error>()
    }

    fn as_tls_error(&self) -> Option<&TlsError> {
        if let Some(error) = self.as_io_error() {
            let Some(inner_error) = error.get_ref() else {
                println!("No inner error found for {:?}", self.url());
                return None;
            };

            let Some(io_error) = inner_error.downcast_ref::<std::io::Error>()?.get_ref() else {
                println!("No inner error found for {:?}", self.url());
                return None;
            };

            let Some(tls_error) = io_error.downcast_ref::<TlsError>() else {
                println!("No inner error found for {:?}", self.url());
                return None;
            };

            return Some(tls_error);
        }

        None
    }

    fn as_hyper_error(&self) -> Option<&HyperError> {
        self.find::<HyperError>()
    }
}

// It's probably possible to have a single implementation for all of these, but we can do that later
trait FindsError {
    fn find<E: Error + 'static>(&self) -> Option<&E>;
}

impl FindsError for reqwest::Error {
    fn find<E: Error + 'static>(&self) -> Option<&E> {
        find_error::<E>(self)
    }
}
impl FindsError for HyperError {
    fn find<E: Error + 'static>(&self) -> Option<&E> {
        find_error::<E>(self)
    }
}
impl FindsError for Http2Error {
    fn find<E: Error + 'static>(&self) -> Option<&E> {
        find_error::<E>(self)
    }
}

// From https://github.com/hyperium/hyper-util/blob/master/src/error.rs
pub(crate) fn find_error<'a, E: Error + 'static>(top: &'a (dyn Error + 'static)) -> Option<&'a E> {
    let mut err = Some(top);
    while let Some(src) = err {
        if src.is::<E>() {
            return src.downcast_ref();
        }
        err = src.source();
    }
    None
}

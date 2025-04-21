use crate::{
    MediaUploadRequestExecutionError, RequestExecutionError, RequestExecutionErrorReason,
    api_error::InvalidSslErrorReason,
    request::{
        RequestExecutor, RequestMethod, WpNetworkHeaderMap, WpNetworkRequest, WpNetworkResponse,
        endpoint::media_endpoint::MediaUploadRequest, user_agent,
    },
};
use async_trait::async_trait;
use h2::Error as Http2Error;
use hickory_resolver::error::ResolveError;
use http::{HeaderMap, HeaderValue};
use hyper::Error as HyperError;
use reqwest::multipart::Part;
use rustls::{CertificateError, Error as TlsError};
use std::{error::Error, sync::Arc, time::Duration};

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
    pub fn new(danger_accept_invalid_certs: bool, timeout: Option<Duration>) -> Self {
        Self {
            client: reqwest::Client::builder()
                .danger_accept_invalid_certs(danger_accept_invalid_certs)
                .timeout(timeout.unwrap_or(Duration::from_secs(10)))
                .use_rustls_tls()
                .build()
                .expect("We should be able to build the reqwest client with this configuration"),
        }
    }

    pub fn new_with_timeout(danger_accept_invalid_certs: bool, timeout: Duration) -> Self {
        Self::new(danger_accept_invalid_certs, Some(timeout))
    }

    pub fn new_with_default_timeout(danger_accept_invalid_certs: bool) -> Self {
        Self::new(danger_accept_invalid_certs, None)
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
            status_code: status.as_u16(),
            body: body.to_vec(),
            response_header_map: Arc::new(WpNetworkHeaderMap::new(response_header_map)),
            request_url: wp_request.url(),
            request_header_map: wp_request.header_map(),
        })
    }

    pub async fn upload_media_request(
        &self,
        media_upload_request: Arc<MediaUploadRequest>,
    ) -> Result<WpNetworkResponse, reqwest::Error> {
        let request = self
            .client
            .request(
                Self::request_method(media_upload_request.method()),
                media_upload_request.url().0.as_str(),
            )
            .headers(media_upload_request.header_map().to_header_map());
        let file_path = media_upload_request.file_path();
        let mut file_header_map = HeaderMap::new();
        file_header_map.insert(
            http::header::CONTENT_TYPE,
            HeaderValue::from_str(&media_upload_request.file_content_type()).unwrap(),
        );
        let mut form = reqwest::multipart::Form::new().part(
            "file",
            Part::file(file_path)
                .await
                .unwrap()
                .headers(file_header_map),
        );
        for (k, v) in media_upload_request.media_params() {
            form = form.text(k, v)
        }

        let request = request.multipart(form);
        let mut response = request.send().await?;

        let header_map = std::mem::take(response.headers_mut());
        Ok(WpNetworkResponse {
            status_code: response.status().as_u16(),
            body: response.bytes().await.unwrap().to_vec(),
            response_header_map: Arc::new(WpNetworkHeaderMap::new(header_map)),
            request_url: media_upload_request.url(),
            request_header_map: media_upload_request.header_map(),
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
        self.async_request(request).await.map_err(|e| e.into())
    }

    async fn upload_media(
        &self,
        media_upload_request: Arc<MediaUploadRequest>,
    ) -> Result<WpNetworkResponse, MediaUploadRequestExecutionError> {
        self.upload_media_request(media_upload_request)
            .await
            .map_err(
                |err| MediaUploadRequestExecutionError::RequestExecutionFailed {
                    status_code: err.status().map(|s| s.as_u16()),
                    redirects: None,
                    reason: RequestExecutionErrorReason::GenericError {
                        error_message: err.to_string(),
                    },
                },
            )
    }

    async fn sleep(&self, millis: u64) {
        tokio::time::sleep(std::time::Duration::from_millis(millis)).await;
    }
}

impl From<reqwest::Error> for RequestExecutionError {
    fn from(error: reqwest::Error) -> Self {
        let status_code = error.status().map(|s| s.as_u16());
        if error.is_timeout() {
            return RequestExecutionError::RequestExecutionFailed {
                status_code,
                redirects: None,
                reason: RequestExecutionErrorReason::HttpTimeoutError,
            };
        }

        if let Some(tls_error) = error.as_tls_error() {
            return RequestExecutionError::RequestExecutionFailed {
                status_code,
                redirects: None,
                reason: tls_error.into(),
            };
        }

        if let Some(io_error) = error.as_io_error() {
            match io_error.kind() {
                std::io::ErrorKind::ConnectionRefused => {
                    return RequestExecutionError::RequestExecutionFailed {
                        status_code,
                        redirects: None,
                        reason: RequestExecutionErrorReason::NonExistentSiteError {
                            error_message: Some("Connection refused".to_string()),
                            suggested_action: None,
                        },
                    };
                }
                std::io::ErrorKind::UnexpectedEof => {
                    // Server terminated the connection unexpectedly
                    return RequestExecutionError::RequestExecutionFailed {
                        status_code,
                        redirects: None,
                        reason: RequestExecutionErrorReason::NonExistentSiteError {
                            error_message: Some(
                                "The server terminated the connection unexpectedly".to_string(),
                            ),
                            suggested_action: None,
                        },
                    };
                }
                _ => {
                    return RequestExecutionError::RequestExecutionFailed {
                        status_code,
                        redirects: None,
                        reason: RequestExecutionErrorReason::GenericError {
                            error_message: error.to_string(),
                        },
                    };
                }
            }
        }

        if let Some(hyper_error) = error.as_hyper_error() {
            return RequestExecutionError::RequestExecutionFailed {
                status_code,
                redirects: None,
                reason: hyper_error.into(),
            };
        }

        if let Some(dns_error) = error.as_dns_error() {
            return RequestExecutionError::RequestExecutionFailed {
                status_code,
                redirects: None,
                reason: dns_error.into(),
            };
        }

        RequestExecutionError::RequestExecutionFailed {
            status_code,
            redirects: None,
            reason: RequestExecutionErrorReason::GenericError {
                error_message: error.to_string(),
            },
        }
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

/// Converts all DNS errors to a RequestExecutionErrorReason
impl From<&ResolveError> for RequestExecutionErrorReason {
    fn from(error: &ResolveError) -> Self {
        // Future improvement: We could probably detect when the domain is valid, but
        // there's no DNS record for the provided hostname

        RequestExecutionErrorReason::NonExistentSiteError {
            error_message: Some(error.to_string()),
            suggested_action: None,
        }
    }
}

/// Converts from the Hyper frameworks underlying errors to a RequestExecutionErrorReason
impl From<&HyperError> for RequestExecutionErrorReason {
    fn from(error: &HyperError) -> Self {
        if let Some(http2_error) = error.find::<Http2Error>() {
            // TODO: We can probably handle more cases here, such as:
            // - Connection reset

            return RequestExecutionErrorReason::GenericError {
                error_message: http2_error.to_string(),
            };
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
    fn as_dns_error(&self) -> Option<&ResolveError>;
    fn as_tls_error(&self) -> Option<&TlsError>;
    fn as_hyper_error(&self) -> Option<&HyperError>;
}

impl ExaminableError for reqwest::Error {
    fn as_io_error(&self) -> Option<&std::io::Error> {
        self.find::<std::io::Error>()
    }

    fn as_dns_error(&self) -> Option<&ResolveError> {
        self.find::<ResolveError>()
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

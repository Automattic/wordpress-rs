use crate::{
    MediaUploadRequestExecutionError, RequestExecutionError, RequestExecutionErrorReason,
    request::{
        RequestExecutor, RequestMethod, WpNetworkHeaderMap, WpNetworkRequest, WpNetworkResponse,
        endpoint::media_endpoint::MediaUploadRequest,
    },
};
use async_trait::async_trait;
use http::{HeaderMap, HeaderValue};
use reqwest::multipart::Part;
use std::{sync::Arc, time::Duration};
use std::error::Error;

use hickory_resolver::ResolveError;


#[derive(Debug)]
pub struct ReqwestRequestExecutor {
    client: reqwest::Client,
}

impl ReqwestRequestExecutor {
    pub fn new(danger_accept_invalid_certs: bool, timeout: Option<Duration>) -> Self {
        Self {
            client: reqwest::Client::builder()
                .danger_accept_invalid_certs(danger_accept_invalid_certs)
                .timeout(timeout.unwrap_or(Duration::from_secs(10)))
                .use_rustls_tls()
                .tls_info(true)
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
        let mut request = self
            .client
            .request(
                Self::request_method(wp_request.method()),
                wp_request.url().0.as_str(),
            )
            .headers(wp_request.header_map().to_header_map());
        if let Some(body) = wp_request.body() {
            request = request.body(body.contents());
        }
        let mut response = request.send().await?;
        
        let header_map = std::mem::take(response.headers_mut());

        let status = response.status();
        let body = response.bytes().await?;

        Ok(WpNetworkResponse {
            status_code: status.as_u16(),
            body: body.to_vec(),
            response_header_map: Arc::new(WpNetworkHeaderMap::new(header_map)),
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
        let hostname = request.url().0.as_str().to_string();
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

        if error.is_timeout() {
            return RequestExecutionError::RequestExecutionFailed {
                status_code: error.status().map(|s| s.as_u16()),
                redirects: None,
                reason: RequestExecutionErrorReason::HttpTimeoutError {
                    hostname: "".to_string(),
                },
            }
        }

        if error.is_an_internal_connect_error() {
            println!("Internal connect error: {:?}", error);
            if let Some(error_kind) = error.get_connect_error_kind() {
                match error_kind {
                    std::io::ErrorKind::ConnectionRefused => {
                        return RequestExecutionError::RequestExecutionFailed {
                            status_code: None,
                            redirects: None,
                            reason: RequestExecutionErrorReason::NonExistentSiteError {
                                error_message: Some("Connection refused".to_string()),
                                suggested_action: None,
                            },
                        };
                    }
                    std::io::ErrorKind::HostUnreachable => {
                        return RequestExecutionError::RequestExecutionFailed {
                            status_code: None,
                            redirects: None,
                            reason: RequestExecutionErrorReason::NonExistentSiteError {
                                error_message: Some("Host unreachable".to_string()),
                                suggested_action: None,
                            },
                        };
                    },
                    std::io::ErrorKind::NetworkUnreachable => {
                        return RequestExecutionError::RequestExecutionFailed {
                            status_code: None,
                            redirects: None,
                            reason: RequestExecutionErrorReason::DeviceIsOfflineError {
                                error_message: "Network unreachable".to_string(),
                            },
                        };
                    }
                    _ => {}
                }
            }
        }

        if error.is_dns_error() {
            return error.as_dns_error().unwrap().into();
        }

        println!("Error: {:?}", error);
        println!("Error: {:?}", error.source());
        todo!();

        // RequestExecutionError::RequestExecutionFailed {
        //     status_code: None,
        //     redirects: None,
        //     reason: RequestExecutionErrorReason::GenericError {
        //         error_message: error.to_string(),
        //     },
        // }
    }
}

impl From<ResolveError> for RequestExecutionError {
    fn from(error: ResolveError) -> Self {
        println!("ResolveError: {:?}", error);

        RequestExecutionError::RequestExecutionFailed {
            status_code: None,
            redirects: None,
            reason: RequestExecutionErrorReason::NonExistentSiteError {
                error_message: None,
                suggested_action: None
            }
        }
    }
}
trait ExaminableError {
    fn is_an_internal_connect_error(&self) -> bool;
    fn is_dns_error(&self) -> bool;
    fn as_dns_error(&self) -> Option<ResolveError>;
    fn get_connect_error_kind(&self) -> Option<std::io::ErrorKind>;
}

impl ExaminableError for reqwest::Error {
    fn is_an_internal_connect_error(&self) -> bool {
        self.source()
            .expect("There should be a source")
            .is::<hyper_util::client::legacy::Error>()
    }

    fn is_dns_error(&self) -> bool {
        self.source()
            .expect("There should be a source")
            .is::<ResolveError>()
    }

    fn as_dns_error(&self) -> Option<ResolveError> {
        self.source()
            .expect("There should be a source")
            .downcast_ref::<ResolveError>().cloned()
    }

    fn get_connect_error_kind(&self) -> Option<std::io::ErrorKind> {
        if let Some(error) = self
            .source()
            .unwrap()
            .downcast_ref::<hyper_util::client::legacy::Error>()
        {
            if let Some(source) = error.source() {
                if let Some(os_error) = source.source() {
                    if let Some(io_error) = os_error.downcast_ref::<std::io::Error>() {
                        return Some(io_error.kind());
                    }
                }
            }
        }

        None
    }
}

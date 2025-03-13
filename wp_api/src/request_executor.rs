use crate::api_error::{MediaUploadRequestExecutionError, RequestExecutionError};
use crate::request::endpoint::media_endpoint::MediaUploadRequest;
use crate::request::{
    RequestExecutor, RequestMethod, WpNetworkHeaderMap, WpNetworkRequest, WpNetworkResponse,
};
use crate::RequestExecutionErrorReason;
use std::error::Error;
use std::sync::Arc;

#[derive(Debug, Default)]

pub struct WpRequestExecutor {}

#[async_trait::async_trait]
impl RequestExecutor for WpRequestExecutor {
    async fn execute(
        &self,
        request: Arc<WpNetworkRequest>,
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        let response = reqwest::Client::new()
            .request(request.method().into(), request.url().url())
            .headers(request.header_map().to_header_map())
            .body(request.body_as_string().unwrap_or_default())
            .send()
            .await?;

        // println!("Response: {:?}", response.extensions().len());
        // println!("HTTP Info: {:?}", response.extensions().get::<HttpInfo>());

        let status_code = response.status().into();
        let header_map = Arc::new(response.headers().into());
        let body = response.bytes().await?.to_vec();

        Ok(WpNetworkResponse {
            body,
            status_code,
            header_map,
        })
    }

    async fn upload_media(
        &self,
        _media_upload_request: Arc<MediaUploadRequest>,
    ) -> Result<WpNetworkResponse, MediaUploadRequestExecutionError> {
        todo!()
    }
}

impl From<RequestMethod> for reqwest::Method {
    fn from(method: RequestMethod) -> Self {
        match method {
            RequestMethod::GET => reqwest::Method::GET,
            RequestMethod::POST => reqwest::Method::POST,
            RequestMethod::PUT => reqwest::Method::PUT,
            RequestMethod::DELETE => reqwest::Method::DELETE,
            RequestMethod::HEAD => reqwest::Method::HEAD,
        }
    }
}

impl From<&http::HeaderMap> for WpNetworkHeaderMap {
    fn from(header_map: &http::HeaderMap) -> Self {
        WpNetworkHeaderMap::new(header_map.clone())
    }
}

impl From<reqwest::Error> for RequestExecutionError {
    fn from(error: reqwest::Error) -> Self {
        // println!("Error: {:?}", error);
        // println!("Error: {:?}", error.source());

        if error.is_an_internal_connect_error() {
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
                    _ => {}
                }
            }
        }

        RequestExecutionError::RequestExecutionFailed {
            status_code: None,
            redirects: None,
            reason: RequestExecutionErrorReason::GenericError {
                error_message: error.to_string(),
            },
        }
    }
}

trait ExaminableError {
    fn is_an_internal_connect_error(&self) -> bool;
    fn get_connect_error_kind(&self) -> Option<std::io::ErrorKind>;
}

impl ExaminableError for reqwest::Error {
    fn is_an_internal_connect_error(&self) -> bool {
        self.source()
            .expect("There should be a source")
            .is::<hyper_util::client::legacy::Error>()
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

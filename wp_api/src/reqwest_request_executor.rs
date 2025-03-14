use crate::{
    request::{
        endpoint::media_endpoint::MediaUploadRequest, RequestExecutor, RequestMethod,
        WpNetworkHeaderMap, WpNetworkRequest, WpNetworkResponse,
    },
    MediaUploadRequestExecutionError, RequestExecutionError, RequestExecutionErrorReason,
};
use async_trait::async_trait;
use http::{HeaderMap, HeaderValue};
use reqwest::multipart::Part;
use std::sync::Arc;

#[derive(Debug)]
pub struct ReqwestRequestExecutor {
    client: reqwest::Client,
}

impl ReqwestRequestExecutor {
    pub fn new(danger_accept_invalid_certs: bool) -> Self {
        Self {
            client: reqwest::Client::builder()
                .danger_accept_invalid_certs(danger_accept_invalid_certs)
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
        Ok(WpNetworkResponse {
            status_code: response.status().as_u16(),
            body: response.bytes().await.unwrap().to_vec(),
            header_map: Arc::new(WpNetworkHeaderMap::new(header_map)),
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
            header_map: Arc::new(WpNetworkHeaderMap::new(header_map)),
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
        self.async_request(request).await.map_err(|err| {
            RequestExecutionError::RequestExecutionFailed {
                status_code: err.status().map(|s| s.as_u16()),
                redirects: None,
                reason: RequestExecutionErrorReason::GenericError {
                    error_message: err.to_string(),
                },
            }
        })
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
}

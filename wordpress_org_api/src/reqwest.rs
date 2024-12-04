use std::sync::Arc;

use reqwest::Client as ReqwestClient;
use wp_api::{
    request::{
        endpoint::media_endpoint::MediaUploadRequest, RequestExecutor, RequestMethod,
        WpNetworkHeaderMap, WpNetworkRequest, WpNetworkResponse,
    },
    MediaUploadRequestExecutionError, RequestExecutionError,
};

#[derive(Debug)]
pub struct ReqwestExecutor {
    client: ReqwestClient,
}

impl ReqwestExecutor {
    fn request_method(method: RequestMethod) -> http::Method {
        match method {
            RequestMethod::GET => reqwest::Method::GET,
            RequestMethod::POST => reqwest::Method::POST,
            RequestMethod::PUT => reqwest::Method::PUT,
            RequestMethod::DELETE => reqwest::Method::DELETE,
            RequestMethod::HEAD => reqwest::Method::HEAD,
        }
    }
}

#[async_trait::async_trait]
impl RequestExecutor for ReqwestExecutor {
    async fn execute(
        &self,
        wp_request: Arc<WpNetworkRequest>,
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        let mut request = self
            .client
            .request(
                Self::request_method(wp_request.method()),
                wp_request.url().0.as_str(),
            )
            .headers(wp_request.header_map().as_header_map());
        if let Some(body) = wp_request.body() {
            request = request.body(body.contents());
        }
        let mut response =
            request
                .send()
                .await
                .map_err(|err| RequestExecutionError::RequestExecutionFailed {
                    status_code: err.status().map(|s| s.as_u16()),
                    reason: err.to_string(),
                })?;

        let header_map = std::mem::take(response.headers_mut());
        Ok(WpNetworkResponse {
            status_code: response.status().as_u16(),
            body: response.bytes().await.unwrap().to_vec(),
            header_map: Arc::new(WpNetworkHeaderMap::new(header_map)),
        })
    }

    async fn upload_media(
        &self,
        _: Arc<MediaUploadRequest>,
    ) -> Result<WpNetworkResponse, MediaUploadRequestExecutionError> {
        unimplemented!("upload_media is not implemented for sending requests to api.wordpress.org")
    }
}

impl From<reqwest::Client> for crate::Client {
    fn from(client: ReqwestClient) -> crate::Client {
        let executor = ReqwestExecutor { client };
        crate::Client {
            request_executor: Arc::new(executor),
        }
    }
}

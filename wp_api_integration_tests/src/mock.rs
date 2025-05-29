use async_trait::async_trait;
use std::sync::Arc;
use wp_api::{prelude::*, request::endpoint::media_endpoint::MediaUploadRequest};

#[derive(Debug)]
pub struct MockExecutor {
    execute_fn: fn(Arc<WpNetworkRequest>) -> Result<WpNetworkResponse, RequestExecutionError>,
    upload_media_fn:
        fn(Arc<MediaUploadRequest>) -> Result<WpNetworkResponse, MediaUploadRequestExecutionError>,
}

impl MockExecutor {
    pub fn with_execute_fn(
        execute_fn: fn(Arc<WpNetworkRequest>) -> Result<WpNetworkResponse, RequestExecutionError>,
    ) -> Self {
        Self {
            execute_fn,
            upload_media_fn: |_: Arc<MediaUploadRequest>| {
                panic!("Upload media is not implemented for `MockExecutor`")
            },
        }
    }
}

#[async_trait]
impl RequestExecutor for MockExecutor {
    async fn execute(
        &self,
        request: Arc<WpNetworkRequest>,
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        (self.execute_fn)(request)
    }

    async fn upload_media(
        &self,
        media_upload_request: Arc<MediaUploadRequest>,
    ) -> Result<WpNetworkResponse, MediaUploadRequestExecutionError> {
        (self.upload_media_fn)(media_upload_request)
    }

    async fn sleep(&self, _: u64) {}
}

pub mod response_helpers {
    use http::{HeaderMap, header::HeaderValue};
    use std::{fs, path::PathBuf, sync::Arc};
    use wp_api::request::{WpNetworkHeaderMap, WpNetworkResponse, endpoint::WpEndpointUrl};

    pub fn with_api_root(url: &str) -> WpNetworkResponse {
        let mut map = HeaderMap::new();
        let link_header_value = format!("<{url}>; rel=\"https://api.w.org/\"");
        map.insert(
            http::header::LINK,
            HeaderValue::from_str(&link_header_value).expect("Failed to create Link header"),
        );
        WpNetworkResponse {
            body: vec![],
            status_code: 200,
            response_header_map: Arc::new(map.into()),
            request_url: WpEndpointUrl("".to_string()),
            request_header_map: WpNetworkHeaderMap::default().into(),
        }
    }

    pub fn json_response_from_integration_test_responses(file_name: &str) -> WpNetworkResponse {
        let mut json_file_path = std::path::PathBuf::from(env!("CARGO_WORKSPACE_DIR"));
        json_file_path.push("test-data");
        json_file_path.push("integration-test-responses");
        json_file_path.push(file_name);
        json_response_from_path(&json_file_path)
    }

    pub fn json_response_from_path(json_file_path: &PathBuf) -> WpNetworkResponse {
        let json = fs::read_to_string(json_file_path).unwrap_or_else(|_| {
            panic!(
                "Should have been able to read the json file at: '{:#?}'",
                json_file_path
            )
        });
        let mut map = HeaderMap::new();
        map.insert(
            http::header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        WpNetworkResponse {
            body: json.as_bytes().to_vec(),
            status_code: 200,
            response_header_map: Arc::new(map.into()),
            request_url: WpEndpointUrl("".to_string()),
            request_header_map: WpNetworkHeaderMap::default().into(),
        }
    }

    pub fn retry_response(delay: usize) -> WpNetworkResponse {
        let mut map = HeaderMap::new();
        map.insert(
            http::header::RETRY_AFTER,
            HeaderValue::from_str(format!("{delay}").as_str())
                .expect("Failed to create Retry-After header"),
        );
        WpNetworkResponse {
            body: vec![],
            status_code: 429,
            response_header_map: Arc::new(map.into()),
            request_url: WpEndpointUrl("".to_string()),
            request_header_map: WpNetworkHeaderMap::default().into(),
        }
    }

    pub fn empty_response(status_code: u16) -> WpNetworkResponse {
        WpNetworkResponse {
            body: vec![],
            status_code,
            response_header_map: WpNetworkHeaderMap::default().into(),
            request_url: WpEndpointUrl("".to_string()),
            request_header_map: WpNetworkHeaderMap::default().into(),
        }
    }
}

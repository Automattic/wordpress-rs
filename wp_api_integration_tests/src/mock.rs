use async_trait::async_trait;
use std::sync::Arc;
use wp_api::{
    prelude::*,
    request::{RequestContext, WpMultipartFormRequest},
};

#[derive(Debug)]
pub struct MockExecutor {
    execute_fn: fn(Arc<WpNetworkRequest>) -> Result<WpNetworkResponse, RequestExecutionError>,
}

impl MockExecutor {
    pub fn with_execute_fn(
        execute_fn: fn(Arc<WpNetworkRequest>) -> Result<WpNetworkResponse, RequestExecutionError>,
    ) -> Self {
        Self { execute_fn }
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

    async fn upload(
        &self,
        _request: Arc<WpMultipartFormRequest>,
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        unimplemented!()
    }

    async fn sleep(&self, _: u64) {}

    fn cancel(&self, _: Arc<RequestContext>) {}
}

pub mod response_helpers {
    use http::{HeaderMap, header::HeaderValue};
    use std::{fs, path::PathBuf, sync::Arc};
    use wp_api::request::{
        RequestMethod, WpNetworkHeaderMap, WpNetworkResponse, endpoint::WpEndpointUrl,
    };

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
            request_method: RequestMethod::GET,
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

    pub fn json_response_from_login_mocks(file_name: &str) -> WpNetworkResponse {
        let mut json_file_path = std::path::PathBuf::from(env!("CARGO_WORKSPACE_DIR"));
        json_file_path.push("test-data");
        json_file_path.push("login-mocks");
        json_file_path.push(file_name);
        json_response_from_path(&json_file_path)
    }

    pub fn json_response_from_path(json_file_path: &PathBuf) -> WpNetworkResponse {
        let json = fs::read_to_string(json_file_path).unwrap_or_else(|_| {
            panic!("Should have been able to read the json file at: '{json_file_path:#?}'")
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
            request_method: RequestMethod::GET,
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
            request_method: RequestMethod::GET,
            request_header_map: WpNetworkHeaderMap::default().into(),
        }
    }

    pub fn empty_response(status_code: u16) -> WpNetworkResponse {
        WpNetworkResponse {
            body: vec![],
            status_code,
            response_header_map: WpNetworkHeaderMap::default().into(),
            request_url: WpEndpointUrl("".to_string()),
            request_method: RequestMethod::GET,
            request_header_map: WpNetworkHeaderMap::default().into(),
        }
    }

    pub fn html_response_from_login_mocks(file_name: &str) -> WpNetworkResponse {
        let mut file_path = std::path::PathBuf::from(env!("CARGO_WORKSPACE_DIR"));
        file_path.push("test-data");
        file_path.push("login-mocks");
        file_path.push(file_name);
        let html = fs::read_to_string(&file_path).unwrap_or_else(|_| {
            panic!("Should have been able to read the file at: '{file_path:#?}'")
        });
        let mut map = HeaderMap::new();
        map.insert(
            http::header::CONTENT_TYPE,
            HeaderValue::from_static("text/html; charset=UTF-8"),
        );
        WpNetworkResponse {
            body: html.as_bytes().to_vec(),
            status_code: 200,
            response_header_map: Arc::new(map.into()),
            request_url: WpEndpointUrl("".to_string()),
            request_header_map: WpNetworkHeaderMap::default().into(),
        }
    }

    pub fn response_with_status_and_headers(
        status_code: u16,
        headers: HeaderMap,
    ) -> WpNetworkResponse {
        WpNetworkResponse {
            body: vec![],
            status_code,
            response_header_map: Arc::new(headers.into()),
            request_url: WpEndpointUrl("".to_string()),
            request_header_map: WpNetworkHeaderMap::default().into(),
        }
    }
}

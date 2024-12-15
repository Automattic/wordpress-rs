use crate::{
    request::{endpoint::WpEndpointUrl, RequestExecutor, WpNetworkRequest, WpNetworkResponse},
    RequestExecutionError,
};
use serde::de::DeserializeOwned;
use std::{result::Result, sync::Arc};
use url::Url;

use super::plugin_directory::{PluginInformation, QueryPluginResponse};

#[derive(Debug, uniffi::Object)]
pub struct WordPressOrgApiClient {
    pub(crate) request_executor: Arc<dyn RequestExecutor>,
}

#[uniffi::export]
impl WordPressOrgApiClient {
    #[uniffi::constructor]
    pub fn new(request_executor: Arc<dyn RequestExecutor>) -> Self {
        Self { request_executor }
    }

    pub async fn plugin_information(
        &self,
        slug: &str,
    ) -> Result<PluginInformation, WordPressOrgApiClientError> {
        self.execute(self.plugin_information_request(slug)).await
    }

    pub async fn browse_plugins(
        &self,
        category: Option<WordPressOrgApiPluginDirectoryCategory>,
        page: u64,
        page_size: u64,
    ) -> Result<QueryPluginResponse, WordPressOrgApiClientError> {
        let request = self.query_plugins_request(page, page_size, |url| match category {
            Some(category) => {
                let mut url = url;
                url.query_pairs_mut()
                    .append_pair("browse", category.as_str());
                url
            }
            None => url,
        });
        self.execute(request).await
    }

    pub async fn search_plugins(
        &self,
        search: String,
        page: u64,
        page_size: u64,
    ) -> Result<QueryPluginResponse, WordPressOrgApiClientError> {
        let request = self.query_plugins_request(page, page_size, |url| {
            let mut url = url;
            url.query_pairs_mut().append_pair("search", &search);
            url
        });
        self.execute(request).await
    }
}

impl WordPressOrgApiClient {
    fn plugin_information_request(&self, slug: &str) -> WpNetworkRequest {
        let mut url = Self::plugin_info_api_url();
        url.query_pairs_mut()
            .append_pair("action", "plugin_information")
            .append_pair("fields", "icons")
            .append_pair("slug", slug);
        WpNetworkRequest::get(WpEndpointUrl(url.to_string()))
    }

    fn query_plugins_request<F>(
        &self,
        page: u64,
        page_size: u64,
        url_builder: F,
    ) -> WpNetworkRequest
    where
        F: FnOnce(Url) -> Url,
    {
        let mut url = Self::plugin_info_api_url();
        url.query_pairs_mut()
            .append_pair("action", "query_plugins")
            .append_pair("page", &page.to_string())
            .append_pair("per_page", &page_size.to_string());
        let url = url_builder(url);
        WpNetworkRequest::get(WpEndpointUrl(url.to_string()))
    }

    async fn execute<T>(&self, request: WpNetworkRequest) -> Result<T, WordPressOrgApiClientError>
    where
        T: DeserializeOwned,
    {
        let response = self.request_executor.execute(Arc::new(request)).await?;
        Self::parse(response)
    }
}

impl WordPressOrgApiClient {
    fn plugin_info_api_url() -> Url {
        Url::parse("https://api.wordpress.org/plugins/info/1.2/").expect("The URL is valid")
    }

    fn parse<T>(response: WpNetworkResponse) -> Result<T, WordPressOrgApiClientError>
    where
        T: DeserializeOwned,
    {
        match response.status_code {
            200 => serde_json::from_slice(&response.body).map_err(|e| {
                WordPressOrgApiClientError::ResponseParsingError {
                    reason: format!("Failed to parse response body as JSON: {}", e),
                    response: String::from_utf8_lossy(&response.body).to_string(),
                }
            }),
            _ => Err(WordPressOrgApiClientError::UnexpectedStatusCodeError {
                status_code: response.status_code,
                response: String::from_utf8_lossy(&response.body).to_string(),
            }),
        }
    }
}

#[derive(Debug, PartialEq, Eq, uniffi::Enum)]
pub enum WordPressOrgApiPluginDirectoryCategory {
    New,
    Popular,
    Updated,
    TopRated,
}

impl WordPressOrgApiPluginDirectoryCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            WordPressOrgApiPluginDirectoryCategory::New => "new",
            WordPressOrgApiPluginDirectoryCategory::Popular => "popular",
            WordPressOrgApiPluginDirectoryCategory::Updated => "updated",
            WordPressOrgApiPluginDirectoryCategory::TopRated => "top-rated",
        }
    }
}

#[derive(Debug, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum WordPressOrgApiClientError {
    #[error(
        "Request execution failed!\nStatus Code: '{:?}'.\nResponse: '{}'",
        status_code,
        reason
    )]
    RequestExecutionFailed {
        status_code: Option<u16>,
        reason: String,
    },
    #[error("Error while parsing. \nReason: {}\nResponse: {}", reason, response)]
    ResponseParsingError { reason: String, response: String },
    #[error(
        "Received a response with an unexpected status code. \nStatus code: {}\nResponse: {}",
        status_code,
        response
    )]
    UnexpectedStatusCodeError { status_code: u16, response: String },
}

impl From<RequestExecutionError> for WordPressOrgApiClientError {
    fn from(e: RequestExecutionError) -> Self {
        match e {
            RequestExecutionError::RequestExecutionFailed {
                status_code,
                reason,
            } => WordPressOrgApiClientError::RequestExecutionFailed {
                status_code,
                reason,
            },
        }
    }
}

#[cfg(test)]
mod tests {

    use futures::lock::Mutex;

    use super::*;
    use crate::{
        request::{endpoint::media_endpoint::MediaUploadRequest, RequestExecutor},
        MediaUploadRequestExecutionError,
    };
    use std::sync::Arc;

    #[derive(Debug)]
    struct MockRequestExecutor {
        requests: Mutex<Vec<Arc<WpNetworkRequest>>>,
    }

    impl MockRequestExecutor {
        fn new() -> Self {
            Self {
                requests: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait::async_trait]
    impl RequestExecutor for MockRequestExecutor {
        async fn execute(
            &self,
            _wp_request: Arc<WpNetworkRequest>,
        ) -> std::result::Result<WpNetworkResponse, RequestExecutionError> {
            self.requests.lock().await.push(_wp_request);

            Err(RequestExecutionError::RequestExecutionFailed {
                status_code: None,
                reason: "Mocked request executor".to_string(),
            })
        }

        async fn upload_media(
            &self,
            media_upload_request: Arc<MediaUploadRequest>,
        ) -> std::result::Result<WpNetworkResponse, MediaUploadRequestExecutionError> {
            unimplemented!(
                "upload_media is not implemented for sending requests to api.wordpress.org"
            )
        }
    }

    #[tokio::test]
    async fn test_plugin_info_requests_include_icons() {
        let request_executor = Arc::new(MockRequestExecutor::new());
        let client = WordPressOrgApiClient::new(request_executor.clone());
        let _ = client.plugin_information("akismet").await;

        let requests = request_executor.requests.lock().await;
        assert!(requests.len() == 1);

        let request = &requests[0];
        assert!(request.url.0.contains("fields=icons"));
    }

    #[tokio::test]
    async fn test_search_does_not_include_pagination() {
        let request_executor = Arc::new(MockRequestExecutor::new());
        let client = WordPressOrgApiClient::new(request_executor.clone());
        let _ = client.search_plugins("akismet".to_string(), 3, 24).await;

        let requests = request_executor.requests.lock().await;
        assert!(requests.len() == 1);

        let request = &requests[0];

        // The 'request[x]' parameters do not work for the search endpoint.
        // The 'page' and 'per_page' parameters do.
        assert!(!request.url.0.contains("request[page]"));
        assert!(!request.url.0.contains("request[per_page]"));
        assert!(request.url.0.contains("page=3"));
        assert!(request.url.0.contains("per_page=24"));
    }
}

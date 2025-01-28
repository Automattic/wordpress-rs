use crate::{
    api_error::RequestExecutionErrorReason,
    request::{endpoint::WpEndpointUrl, RequestExecutor, WpNetworkRequest, WpNetworkResponse},
    wordpress_org::update_check::UpdateCheckRequest,
    ParsedUrl, PluginWithViewContext, PluginWpOrgDirectorySlug, RequestExecutionError,
};
use serde::de::DeserializeOwned;
use std::{result::Result, sync::Arc};
use url::Url;

use super::plugin_directory::{PluginInformation, QueryPluginResponse};
use super::update_check::UpdateCheckResponse;

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
        slug: &PluginWpOrgDirectorySlug,
    ) -> Result<PluginInformation, WordPressOrgApiClientError> {
        self.execute(Self::plugin_information_request(slug)).await
    }

    pub async fn browse_plugins(
        &self,
        category: Option<WordPressOrgApiPluginDirectoryCategory>,
        page: u64,
        page_size: u64,
    ) -> Result<QueryPluginResponse, WordPressOrgApiClientError> {
        let request = Self::browse_plugins_request(category, page, page_size);
        self.execute(request).await
    }

    pub async fn search_plugins(
        &self,
        search: String,
        page: u64,
        page_size: u64,
    ) -> Result<QueryPluginResponse, WordPressOrgApiClientError> {
        let request = Self::search_plugins_request(search, page, page_size);
        self.execute(request).await
    }

    pub async fn check_plugin_updates(
        &self,
        wordpress_core_version: String,
        site_url: Arc<ParsedUrl>,
        plugins: Vec<PluginWithViewContext>,
    ) -> Result<UpdateCheckResponse, WordPressOrgApiClientError> {
        let site_url = Arc::unwrap_or_clone(site_url);
        match UpdateCheckRequest::new(wordpress_core_version, site_url, plugins).try_into() {
            Ok(request) => self.execute(request).await,
            Err(e) => Err(WordPressOrgApiClientError::RequestEncodingError {
                reason: e.to_string(),
            }),
        }
    }
}

impl WordPressOrgApiClient {
    pub fn browse_plugins_request(
        category: Option<WordPressOrgApiPluginDirectoryCategory>,
        page: u64,
        page_size: u64,
    ) -> WpNetworkRequest {
        Self::query_plugins_request(page, page_size, |url| match category {
            Some(category) => {
                let mut url = url;
                url.query_pairs_mut()
                    .append_pair("browse", category.as_str());
                url
            }
            None => url,
        })
    }

    pub fn search_plugins_request(search: String, page: u64, page_size: u64) -> WpNetworkRequest {
        Self::query_plugins_request(page, page_size, |url| {
            let mut url = url;
            url.query_pairs_mut().append_pair("search", &search);
            url
        })
    }

    fn plugin_information_request(slug: &PluginWpOrgDirectorySlug) -> WpNetworkRequest {
        let mut url = Self::plugin_info_api_url();
        url.query_pairs_mut()
            .append_pair("action", "plugin_information")
            .append_pair("fields", "icons")
            .append_pair("slug", slug.as_str());
        WpNetworkRequest::get(WpEndpointUrl(url.to_string()))
    }

    fn query_plugins_request<F>(page: u64, page_size: u64, url_builder: F) -> WpNetworkRequest
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

    fn plugin_info_api_url() -> Url {
        Url::parse("https://api.wordpress.org/plugins/info/1.2/").expect("The URL is valid")
    }

    async fn execute<T>(&self, request: WpNetworkRequest) -> Result<T, WordPressOrgApiClientError>
    where
        T: DeserializeOwned,
    {
        let response = self.request_executor.execute(Arc::new(request)).await?;
        Self::parse(response)
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
    #[error("Failed to encode request. Reason: {}", reason)]
    RequestEncodingError { reason: String },
    #[error(
        "Request execution failed!\nStatus Code: '{:?}'\nRedirects: '{:#?}'\nReason: '{:#?}'",
        status_code,
        redirects,
        reason
    )]
    RequestExecutionFailed {
        status_code: Option<u16>,
        redirects: Option<Vec<String>>,
        reason: RequestExecutionErrorReason,
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
                redirects,
                reason,
            } => WordPressOrgApiClientError::RequestExecutionFailed {
                status_code,
                redirects,
                reason,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_info_requests_include_icons() {
        let request = WordPressOrgApiClient::plugin_information_request(&("akismet".into()));
        assert!(request.url.0.contains("fields=icons"));
    }

    #[test]
    fn test_search_does_not_include_pagination() {
        let request = WordPressOrgApiClient::search_plugins_request("akismet".to_string(), 3, 24);

        // The 'request[x]' parameters do not work for the search endpoint.
        // The 'page' and 'per_page' parameters do.
        assert!(!request.url.0.contains("request[page]"));
        assert!(!request.url.0.contains("request[per_page]"));
        assert!(request.url.0.contains("page=3"));
        assert!(request.url.0.contains("per_page=24"));
    }
}

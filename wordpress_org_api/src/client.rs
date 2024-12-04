use serde::de::DeserializeOwned;
use std::sync::Arc;
use url::Url;
use wp_api::{
    request::{endpoint::WpEndpointUrl, RequestExecutor, WpNetworkRequest, WpNetworkResponse},
    RequestExecutionError,
};

use crate::plugin_directory::PluginInformation;

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

    pub async fn plugin_information(&self, slug: &str) -> crate::Result<PluginInformation> {
        let mut url = Self::plugin_info_api_url();
        url.query_pairs_mut()
            .append_pair("action", "plugin_information")
            .append_pair("fields", "icons")
            .append_pair("slug", slug);
        let request = WpNetworkRequest::get(WpEndpointUrl(url.to_string()));
        let response = self.request_executor.execute(Arc::new(request)).await?;
        Self::parse(response)
    }
}

impl WordPressOrgApiClient {
    fn plugin_info_api_url() -> Url {
        Url::parse("https://api.wordpress.org/plugins/info/1.2/").expect("The URL is valid")
    }

    fn parse<T>(response: WpNetworkResponse) -> crate::Result<T>
    where
        T: DeserializeOwned,
    {
        match response.status_code {
            200 => {
                serde_json::from_slice(&response.body).map_err(|e| WordPressOrgApiClientError::ResponseParsingError {
                    reason: format!("Failed to parse response body as JSON: {}", e),
                    response: String::from_utf8_lossy(&response.body).to_string(),
                })
            }
            _ => Err(WordPressOrgApiClientError::UnexpectedStatusCodeError {
                status_code: response.status_code,
                response: String::from_utf8_lossy(&response.body).to_string(),
            }),
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

uniffi::setup_scaffolding!();

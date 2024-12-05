use serde::de::DeserializeOwned;
use std::fmt::Debug;
use std::sync::Arc;
use url::Url;
use wp_api::request::{endpoint::WpEndpointUrl, WpNetworkRequest, WpNetworkResponse};

use crate::plugin_directory::PluginInformation;

#[uniffi::export(with_foreign)]
#[async_trait::async_trait]
pub trait WordPressOrgApiRequestExecutor: Send + Sync + Debug {
    async fn execute(
        &self,
        request: Arc<WpNetworkRequest>,
    ) -> Result<WordPressOrgApiNetworkResponse, WordPressOrgApiRequestExecutionError>;
}

#[derive(Debug, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum WordPressOrgApiRequestExecutionError {
    #[error(
        "Request execution failed!\nStatus Code: '{:?}'.\nResponse: '{}'",
        status_code,
        reason
    )]
    RequestExecutionFailed {
        status_code: Option<u16>,
        reason: String,
    },
}

#[derive(Debug, Default, uniffi::Object)]
pub struct DummyObject;

#[uniffi::export]
impl DummyObject {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, uniffi::Record)]
pub struct WordPressOrgApiNetworkResponse {
    pub inner: WpNetworkResponse,
    // Putting a uniffi reference type here to prevent uniffi-rs from generating
    // a `Equtable` and `Hashable` implementation in the Swift binding file.
    // They wouldn't be able to compile, because `WpNetworkResponse` doesn't implement
    // `Equtable` and `Hashable`.
    dummy: Arc<DummyObject>,
}

impl From<WpNetworkResponse> for WordPressOrgApiNetworkResponse {
    fn from(inner: WpNetworkResponse) -> Self {
        Self { inner, dummy: DummyObject.into() }
    }
}

#[derive(Debug, uniffi::Object)]
pub struct WordPressOrgApiClient {
    pub(crate) request_executor: Arc<dyn WordPressOrgApiRequestExecutor>,
}

#[uniffi::export]
impl WordPressOrgApiClient {
    #[uniffi::constructor]
    pub fn new(request_executor: Arc<dyn WordPressOrgApiRequestExecutor>) -> Self {
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
        Self::parse(response.inner)
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

impl From<WordPressOrgApiRequestExecutionError> for WordPressOrgApiClientError {
    fn from(e: WordPressOrgApiRequestExecutionError) -> Self {
        match e {
            WordPressOrgApiRequestExecutionError::RequestExecutionFailed {
                status_code,
                reason,
            } => WordPressOrgApiClientError::RequestExecutionFailed {
                status_code,
                reason,
            },
        }
    }
}

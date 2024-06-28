use std::collections::HashMap;
use std::str;
use std::sync::Arc;

use crate::request::endpoint::WpEndpointUrl;
use crate::request::{RequestExecutor, RequestMethod, WpNetworkRequest, WpNetworkResponse};

use super::url_discovery::{
    self, FetchApiDetailsError, FetchApiRootUrlError, ParsedUrl, StateInitial, UrlDiscoveryResult,
    UrlDiscoveryState,
};

const API_ROOT_LINK_HEADER: &str = "https://api.w.org/";

#[derive(Debug, uniffi::Object)]
struct UniffiWpLoginClient {
    inner: Arc<WpLoginClient>,
}

#[uniffi::export]
impl UniffiWpLoginClient {
    #[uniffi::constructor]
    fn new(request_executor: Arc<dyn RequestExecutor>) -> Self {
        Self {
            inner: WpLoginClient::new(request_executor).into(),
        }
    }

    async fn api_discovery(&self, site_url: &str) -> UrlDiscoveryResult {
        self.inner.api_discovery(site_url).await
    }
}

#[derive(Debug)]
pub struct WpLoginClient {
    request_executor: Arc<dyn RequestExecutor>,
}

impl WpLoginClient {
    pub fn new(request_executor: Arc<dyn RequestExecutor>) -> Self {
        Self { request_executor }
    }

    pub async fn api_discovery(&self, site_url: &str) -> UrlDiscoveryResult {
        let attempts = futures::future::join_all(
            url_discovery::find_attempts(site_url)
                .iter()
                .map(|s| async { self.attempt_api_discovery(s).await }),
        )
        .await;
        let successful_attempt = attempts
            .iter()
            .find(|a| matches!(a, UrlDiscoveryState::FetchedApiDetails { .. }));
        if let Some(UrlDiscoveryState::FetchedApiDetails {
            site_url,
            api_details,
            api_root_url,
        }) = successful_attempt
        {
            UrlDiscoveryResult::Success {
                site_url: site_url.clone(),
                api_details: api_details.clone(),
                api_root_url: api_root_url.clone(),
                attempts,
            }
        } else {
            UrlDiscoveryResult::Failure { attempts }
        }
    }

    async fn attempt_api_discovery(&self, site_url: &str) -> UrlDiscoveryState {
        let initial_state = StateInitial::new(site_url);
        let parsed_url_state = match initial_state.parse() {
            Ok(s) => s,
            Err(e) => {
                return UrlDiscoveryState::FailedToParseSiteUrl {
                    site_url: site_url.to_string(),
                    error: e,
                }
            }
        };
        let api_root_response = match self.fetch_api_root_url(&parsed_url_state.site_url).await {
            Ok(response) => response,
            Err(e) => {
                return UrlDiscoveryState::FailedToFetchApiRootUrl {
                    site_url: parsed_url_state.site_url,
                    error: e,
                }
            }
        };
        let fetched_api_root_url = match parsed_url_state.parse_api_root_response(api_root_response)
        {
            Ok(s) => s,
            Err(e) => return e,
        };
        let api_details_response = match self
            .fetch_wp_api_details(&fetched_api_root_url.api_root_url)
            .await
        {
            Ok(response) => response,
            Err(e) => {
                return UrlDiscoveryState::FailedToFetchApiDetails {
                    site_url: fetched_api_root_url.site_url,
                    api_root_url: fetched_api_root_url.api_root_url,
                    error: e,
                }
            }
        };
        match fetched_api_root_url.parse_api_details_response(api_details_response) {
            Ok(s) => s.into(),
            Err(e) => e,
        }
    }

    // Fetches the site's homepage with a HEAD request, then extracts the Link header pointing
    // to the WP.org API root
    async fn fetch_api_root_url(
        &self,
        parsed_site_url: &ParsedUrl,
    ) -> Result<WpNetworkResponse, FetchApiRootUrlError> {
        let api_root_request = WpNetworkRequest {
            method: RequestMethod::HEAD,
            url: WpEndpointUrl(parsed_site_url.url()),
            header_map: HashMap::new(),
            body: None,
        };
        self.request_executor
            .execute(api_root_request)
            .await
            .map_err(FetchApiRootUrlError::from)
    }

    async fn fetch_wp_api_details(
        &self,
        api_root_url: &ParsedUrl,
    ) -> Result<WpNetworkResponse, FetchApiDetailsError> {
        self.request_executor
            .execute(WpNetworkRequest {
                method: RequestMethod::GET,
                url: WpEndpointUrl(api_root_url.url()),
                header_map: HashMap::new(),
                body: None,
            })
            .await
            .map_err(FetchApiDetailsError::from)
    }
}

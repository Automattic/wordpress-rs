use std::collections::HashMap;
use std::str;
use std::sync::Arc;

use crate::request::endpoint::WpEndpointUrl;
use crate::request::{RequestExecutor, RequestMethod, WpNetworkRequest, WpNetworkResponse};

use super::url_discovery::{
    self, FetchApiDetailsError, FetchApiRootUrlError, ParsedUrl, StateInitial,
    UrlDiscoveryAttemptError, UrlDiscoveryAttemptSuccess, UrlDiscoveryError, UrlDiscoveryState,
    UrlDiscoverySuccess,
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

    async fn api_discovery(
        &self,
        site_url: String,
    ) -> Result<UrlDiscoverySuccess, UrlDiscoveryError> {
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

    pub async fn api_discovery(
        &self,
        site_url: String,
    ) -> Result<UrlDiscoverySuccess, UrlDiscoveryError> {
        let attempts = futures::future::join_all(
            url_discovery::construct_attempts(site_url)
                .iter()
                .map(|s| async { self.attempt_api_discovery(s).await }),
        )
        .await;
        let successful_attempt = attempts.iter().find_map(|a| {
            if let Ok(s) = a {
                Some((
                    s.site_url.clone(),
                    s.api_details.clone(),
                    s.api_root_url.clone(),
                ))
            } else {
                None
            }
        });

        let attempts = attempts
            .into_iter()
            .map(|a| match a {
                Ok(s) => UrlDiscoveryState::Success(s),
                Err(e) => UrlDiscoveryState::Failure(e),
            })
            .collect();
        if let Some(s) = successful_attempt {
            Ok(UrlDiscoverySuccess {
                site_url: s.0,
                api_details: s.1,
                api_root_url: s.2,
                attempts,
            })
        } else {
            Err(UrlDiscoveryError::UrlDiscoveryFailed { attempts })
        }
    }

    async fn attempt_api_discovery(
        &self,
        site_url: &str,
    ) -> Result<UrlDiscoveryAttemptSuccess, UrlDiscoveryAttemptError> {
        let initial_state = StateInitial::new(site_url);
        let parsed_url_state =
            initial_state
                .parse()
                .map_err(|e| UrlDiscoveryAttemptError::FailedToParseSiteUrl {
                    site_url: site_url.to_string(),
                    error: e,
                })?;
        let state_fetched_api_root_url = self
            .fetch_api_root_url(&parsed_url_state.site_url)
            .await
            .and_then(|r| parsed_url_state.parse_api_root_response(r))
            .map_err(|e| UrlDiscoveryAttemptError::FetchApiRootUrlFailed {
                site_url: parsed_url_state.site_url.clone().into(),
                error: e,
            })?;
        self.fetch_wp_api_details(&state_fetched_api_root_url.api_root_url)
            .await
            .map_err(|e| UrlDiscoveryAttemptError::FetchApiDetailsFailed {
                site_url: state_fetched_api_root_url.site_url.clone().into(),
                api_root_url: state_fetched_api_root_url.api_root_url.clone().into(),
                error: e,
            })
            .and_then(|r| state_fetched_api_root_url.parse_api_details_response(r))
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

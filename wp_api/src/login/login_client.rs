use super::{
    url_discovery::{
        self, AutoDiscoveryAttempt, AutoDiscoveryAttemptFailure, AutoDiscoveryAttemptResult,
        AutoDiscoveryAttemptSuccess, AutoDiscoveryResult, AutoDiscoveryUniffiResult,
        FindApiRootLinkHeaderFailure, FindApiRootLinkHeaderSuccess, IsWordPressSiteAttemptResult,
        IsWordPressSiteResult, IsWordPressSiteUniffiResult, ParseApiRootUrlError,
    },
    WpApiDetails,
};
use crate::{
    request::{
        endpoint::WpEndpointUrl, RequestExecutor, RequestMethod, WpNetworkHeaderMap,
        WpNetworkRequest, WpNetworkResponse,
    },
    ParsedUrl, RequestExecutionError,
};
use std::{str, sync::Arc};
use uuid::Uuid;

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

    async fn api_discovery(&self, site_url: String) -> AutoDiscoveryUniffiResult {
        self.inner.api_discovery(site_url).await.into()
    }

    async fn is_wordpress_site_discovery(&self, site_url: String) -> IsWordPressSiteUniffiResult {
        self.inner
            .is_wordpress_site_discovery(site_url)
            .await
            .into()
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

    pub async fn api_discovery(&self, site_url: String) -> AutoDiscoveryResult {
        let attempts = futures::future::join_all(
            url_discovery::construct_attempts(site_url)
                .into_iter()
                .map(|attempt| async { self.attempt_api_discovery(attempt).await }),
        )
        .await;
        AutoDiscoveryResult {
            attempts: attempts.into_iter().map(|r| (r.attempt_type, r)).collect(),
        }
    }

    pub async fn is_wordpress_site_discovery(&self, site_url: String) -> IsWordPressSiteResult {
        let attempts = futures::future::join_all(
            url_discovery::construct_attempts(site_url)
                .into_iter()
                .map(|attempt| async { self.attempt_is_wordpress_site(attempt).await }),
        )
        .await;
        IsWordPressSiteResult {
            attempts: attempts.into_iter().map(|r| (r.attempt_type, r)).collect(),
        }
    }

    async fn attempt_api_discovery(
        &self,
        attempt: AutoDiscoveryAttempt,
    ) -> AutoDiscoveryAttemptResult {
        let result = self
            .inner_attempt_api_discovery(attempt.attempt_site_url.as_str())
            .await;
        AutoDiscoveryAttemptResult {
            attempt_type: attempt.attempt_type,
            attempt_site_url: attempt.attempt_site_url,
            result,
        }
    }

    async fn inner_attempt_api_discovery(
        &self,
        attempt_site_url: &str,
    ) -> Result<AutoDiscoveryAttemptSuccess, AutoDiscoveryAttemptFailure> {
        let api_root_url_success = self.find_api_root_url(attempt_site_url).await?;
        let fetch_api_details_response = match self
            .fetch_wp_api_details(&api_root_url_success.api_root_url)
            .await
        {
            Ok(r) => r,
            Err(error) => {
                return Err(AutoDiscoveryAttemptFailure::FetchApiDetails {
                    parsed_site_url: api_root_url_success.parsed_site_url,
                    api_root_url: api_root_url_success.api_root_url,
                    error,
                })
            }
        };
        let api_details: WpApiDetails =
            match serde_json::from_slice::<WpApiDetails>(&fetch_api_details_response.body) {
                Ok(api_details) => api_details,
                Err(error) => {
                    return Err(AutoDiscoveryAttemptFailure::ParseApiDetails {
                        parsed_site_url: api_root_url_success.parsed_site_url,
                        api_root_url: api_root_url_success.api_root_url,
                        parsing_error_message: error.to_string(),
                    })
                }
            };

        Ok(AutoDiscoveryAttemptSuccess {
            parsed_site_url: api_root_url_success.parsed_site_url,
            api_root_url: api_root_url_success.api_root_url,
            api_details,
        })
    }

    async fn attempt_is_wordpress_site(
        &self,
        attempt: AutoDiscoveryAttempt,
    ) -> IsWordPressSiteAttemptResult {
        let api_link_header_result = self
            .find_api_root_url(attempt.attempt_site_url.as_str())
            .await;
        IsWordPressSiteAttemptResult {
            attempt_type: attempt.attempt_type,
            api_link_header_result,
        }
    }

    async fn find_api_root_url(
        &self,
        attempt_site_url: &str,
    ) -> Result<FindApiRootLinkHeaderSuccess, FindApiRootLinkHeaderFailure> {
        let parsed_site_url = ParsedUrl::parse(attempt_site_url)
            .map_err(|error| FindApiRootLinkHeaderFailure::ParseSiteUrl { error })?;
        let fetch_api_root_url_response = match self.fetch_api_root_url(&parsed_site_url).await {
            Ok(r) => r,
            Err(error) => {
                return Err(FindApiRootLinkHeaderFailure::FetchApiRootUrl {
                    parsed_site_url,
                    error,
                })
            }
        };
        let api_root_url =
            match self.parse_api_root_response(&parsed_site_url, fetch_api_root_url_response) {
                Ok(api_root_url) => api_root_url,
                Err(error) => {
                    return Err(FindApiRootLinkHeaderFailure::ParseApiRootUrl {
                        parsed_site_url,
                        error,
                    })
                }
            };
        Ok(FindApiRootLinkHeaderSuccess {
            parsed_site_url,
            api_root_url,
        })
    }

    fn parse_api_root_response(
        &self,
        site_url: &ParsedUrl,
        response: WpNetworkResponse,
    ) -> Result<ParsedUrl, ParseApiRootUrlError> {
        match response
            .get_link_header(API_ROOT_LINK_HEADER)
            .into_iter()
            .nth(0)
        {
            Some(url) => Ok(ParsedUrl::new(url)),
            None => Err(ParseApiRootUrlError::ApiRootLinkHeaderNotFound {
                header_map: response.header_map,
                status_code: response.status_code,
            }),
        }
    }

    // Fetches the site's homepage with a HEAD request, then extracts the Link header pointing
    // to the WP.org API root
    async fn fetch_api_root_url(
        &self,
        parsed_site_url: &ParsedUrl,
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        let api_root_request = WpNetworkRequest {
            uuid: Uuid::new_v4().into(),
            method: RequestMethod::HEAD,
            url: WpEndpointUrl(parsed_site_url.url()),
            header_map: WpNetworkHeaderMap::default().into(),
            body: None,
        };
        self.request_executor.execute(api_root_request.into()).await
    }

    async fn fetch_wp_api_details(
        &self,
        api_root_url: &ParsedUrl,
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        self.request_executor
            .execute(
                WpNetworkRequest {
                    uuid: Uuid::new_v4().into(),
                    method: RequestMethod::GET,
                    url: WpEndpointUrl(api_root_url.url()),
                    header_map: WpNetworkHeaderMap::default().into(),
                    body: None,
                }
                .into(),
            )
            .await
    }
}

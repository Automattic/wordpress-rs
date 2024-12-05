use std::str;
use std::sync::Arc;

use crate::request::endpoint::WpEndpointUrl;
use crate::request::{
    RequestExecutor, RequestMethod, WpNetworkHeaderMap, WpNetworkRequest, WpNetworkResponse,
};
use crate::ParsedUrl;

use super::url_discovery::{
    self, AutoDiscoveryAttempt, FetchApiDetailsError, FetchApiRootUrlError,
    UrlDiscoveryAttemptError, UrlDiscoveryAttemptSuccess, UrlDiscoveryError, UrlDiscoveryState,
    UrlDiscoverySuccess,
};
use super::WpApiDetails;

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
                    Arc::clone(&s.site_url),
                    Arc::clone(&s.api_details),
                    Arc::clone(&s.api_root_url),
                ))
            } else {
                None
            }
        });

        let attempts = attempts
            .into_iter()
            .map(|a| match a {
                Ok(s) => (s.site_url.url(), UrlDiscoveryState::Success(s)),
                Err(e) => (e.site_url(), UrlDiscoveryState::Failure(e)),
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
        attempt: &AutoDiscoveryAttempt,
    ) -> Result<UrlDiscoveryAttemptSuccess, UrlDiscoveryAttemptError> {
        let site_url = attempt.site_url.as_str();
        let parsed_site_url: Arc<ParsedUrl> = self.parse_attempt_url(site_url)?.into();
        let api_root_url: Arc<ParsedUrl> = self
            .fetch_api_root_url(&parsed_site_url)
            .await
            .and_then(|r| self.parse_api_root_response(&parsed_site_url, r))
            .map_err(|e| UrlDiscoveryAttemptError::FetchApiRootUrlFailed {
                site_url: Arc::clone(&parsed_site_url),
                error: e,
            })?
            .into();
        let api_details: Arc<WpApiDetails> = self
            .fetch_wp_api_details(Arc::clone(&api_root_url))
            .await
            .and_then(|api_details_response| self.parse_api_details_response(api_details_response))
            .map_err(|err| UrlDiscoveryAttemptError::FetchApiDetailsFailed {
                site_url: Arc::clone(&parsed_site_url),
                api_root_url: Arc::clone(&api_root_url),
                error: err,
            })?
            .into();
        Ok(UrlDiscoveryAttemptSuccess {
            site_url: Arc::clone(&parsed_site_url),
            api_root_url: Arc::clone(&api_root_url),
            api_details: Arc::clone(&api_details),
        })
    }

    fn parse_attempt_url(&self, site_url: &str) -> Result<ParsedUrl, UrlDiscoveryAttemptError> {
        ParsedUrl::parse(site_url).map_err(|e| UrlDiscoveryAttemptError::FailedToParseSiteUrl {
            site_url: site_url.to_string(),
            error: e,
        })
    }

    fn parse_api_root_response(
        &self,
        site_url: &ParsedUrl,
        response: WpNetworkResponse,
    ) -> Result<ParsedUrl, FetchApiRootUrlError> {
        match response
            .get_link_header(API_ROOT_LINK_HEADER)
            .into_iter()
            .nth(0)
        {
            Some(url) => Ok(ParsedUrl::new(url)),
            None => Err(FetchApiRootUrlError::ApiRootLinkHeaderNotFound {
                header_map: response.header_map,
                status_code: response.status_code,
            }),
        }
    }

    fn parse_api_details_response(
        &self,
        response: WpNetworkResponse,
    ) -> Result<WpApiDetails, FetchApiDetailsError> {
        serde_json::from_slice::<WpApiDetails>(&response.body).map_err(|err| {
            FetchApiDetailsError::ApiDetailsCouldntBeParsed {
                reason: err.to_string(),
                response: response.body_as_string(),
            }
        })
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
            header_map: WpNetworkHeaderMap::default().into(),
            body: None,
        };
        self.request_executor
            .execute(api_root_request.into())
            .await
            .map_err(FetchApiRootUrlError::from)
    }

    async fn fetch_wp_api_details(
        &self,
        api_root_url: Arc<ParsedUrl>,
    ) -> Result<WpNetworkResponse, FetchApiDetailsError> {
        self.request_executor
            .execute(
                WpNetworkRequest {
                    method: RequestMethod::GET,
                    url: WpEndpointUrl(api_root_url.url()),
                    header_map: WpNetworkHeaderMap::default().into(),
                    body: None,
                }
                .into(),
            )
            .await
            .map_err(FetchApiDetailsError::from)
    }
}

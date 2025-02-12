use super::{
    url_discovery::{
        self, AutoDiscoveryAttempt, AutoDiscoveryAttemptFailure, AutoDiscoveryAttemptResult,
        AutoDiscoveryAttemptSuccess, AutoDiscoveryResult, AutoDiscoveryUniffiResult,
        FetchWpJsonFailure, FetchWpJsonSuccess, FindApiRootLinkHeaderFailure,
        FindApiRootLinkHeaderSuccess, IsWordPressSiteAttemptResult, IsWordPressSiteParseHtmlResult,
        ParseApiRootUrlError, ParseHtmlFailure,
    },
    WpApiDetails,
};
use crate::{
    login::url_discovery::RootWpJson,
    request::{
        endpoint::WpEndpointUrl, RequestExecutor, RequestMethod, WpNetworkHeaderMap,
        WpNetworkRequest, WpNetworkResponse,
    },
    ParseUrlError, ParsedUrl, RequestExecutionError,
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

    async fn attempt_api_discovery(
        &self,
        attempt: AutoDiscoveryAttempt,
    ) -> AutoDiscoveryAttemptResult {
        let attempt_site_url = attempt.attempt_site_url.as_str();
        let parse_html_result = self
            .fetch_site(attempt_site_url)
            .await
            .map(|r| IsWordPressSiteParseHtmlResult::parse_response(&r.body_as_string()));
        let api_root_url_from_link_tag = parse_html_result
            .as_ref()
            .ok()
            .and_then(|h| h.api_root_url_from_link_tag.clone());
        let api_link_header_result = self
            .find_api_root_url(attempt_site_url, api_root_url_from_link_tag)
            .await;
        let discovery_result = self
            .inner_attempt_api_discovery(api_link_header_result.clone())
            .await;
        let is_wordpress_site = self
            .attempt_is_wordpress_site(attempt_site_url, api_link_header_result, parse_html_result)
            .await;
        AutoDiscoveryAttemptResult {
            attempt_type: attempt.attempt_type,
            attempt_site_url: attempt.attempt_site_url,
            api_discovery_result: discovery_result,
            is_wordpress_site,
        }
    }

    async fn inner_attempt_api_discovery(
        &self,
        api_link_header_result: Result<FindApiRootLinkHeaderSuccess, FindApiRootLinkHeaderFailure>,
    ) -> Result<AutoDiscoveryAttemptSuccess, AutoDiscoveryAttemptFailure> {
        let api_root_url_success = api_link_header_result?;
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
        attempt_site_url: &str,
        api_link_header_result: Result<FindApiRootLinkHeaderSuccess, FindApiRootLinkHeaderFailure>,
        parse_html_result: Result<IsWordPressSiteParseHtmlResult, ParseHtmlFailure>,
    ) -> IsWordPressSiteAttemptResult {
        let fetch_wp_json_result = self.fetch_wp_json(attempt_site_url).await;
        IsWordPressSiteAttemptResult {
            api_link_header_result,
            fetch_wp_json_result,
            parse_html_result,
        }
    }

    async fn find_api_root_url(
        &self,
        attempt_site_url: &str,
        api_root_url_from_link_tag: Option<ParsedUrl>,
    ) -> Result<FindApiRootLinkHeaderSuccess, FindApiRootLinkHeaderFailure> {
        let parsed_site_url = ParsedUrl::parse(attempt_site_url)
            .map_err(|error| FindApiRootLinkHeaderFailure::ParseSiteUrl { error })?;
        if let Some(api_root_url) = api_root_url_from_link_tag {
            Ok(FindApiRootLinkHeaderSuccess {
                parsed_site_url,
                api_root_url,
            })
        } else {
            self.fetch_and_parse_api_root_url_header(parsed_site_url)
                .await
        }
    }

    async fn fetch_and_parse_api_root_url_header(
        &self,
        parsed_site_url: ParsedUrl,
    ) -> Result<FindApiRootLinkHeaderSuccess, FindApiRootLinkHeaderFailure> {
        let fetch_api_root_url_response = match self.fetch_api_root_url(&parsed_site_url).await {
            Ok(r) => r,
            Err(error) => {
                return Err(FindApiRootLinkHeaderFailure::FetchApiRootUrl {
                    parsed_site_url,
                    error,
                })
            }
        };
        let api_root_url = match self.parse_api_root_response(fetch_api_root_url_response) {
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

    async fn fetch_wp_json(
        &self,
        attempt_site_url: &str,
    ) -> Result<FetchWpJsonSuccess, FetchWpJsonFailure> {
        let wp_json_url = {
            let mut wp_json_url = ParsedUrl::parse(attempt_site_url)
                .map_err(|error| FetchWpJsonFailure::ParseSiteUrl { error })?;
            wp_json_url
                .inner
                .path_segments_mut()
                .map_err(|_| FetchWpJsonFailure::ParseSiteUrl {
                    error: ParseUrlError::RelativeUrlWithCannotBeABaseBase,
                })?
                .push("wp-json");
            wp_json_url
        };
        let fetch_wp_json_response = match self
            .request_executor
            .execute(
                WpNetworkRequest {
                    uuid: Uuid::new_v4().into(),
                    method: RequestMethod::GET,
                    url: WpEndpointUrl(wp_json_url.url()),
                    header_map: WpNetworkHeaderMap::default().into(),
                    body: None,
                }
                .into(),
            )
            .await
        {
            Ok(r) => r,
            Err(error) => {
                return Err(FetchWpJsonFailure::FetchWpJson { wp_json_url, error });
            }
        };

        let root_wp_json = match serde_json::from_slice::<RootWpJson>(&fetch_wp_json_response.body)
        {
            Ok(r) => r,
            Err(_) => return Err(FetchWpJsonFailure::ParseWpJson { wp_json_url }),
        };
        Ok(FetchWpJsonSuccess {
            wp_json_url,
            root_wp_json,
        })
    }

    async fn fetch_site(
        &self,
        attempt_site_url: &str,
    ) -> Result<WpNetworkResponse, ParseHtmlFailure> {
        let site_url = ParsedUrl::parse(attempt_site_url)
            .map_err(|error| ParseHtmlFailure::ParseSiteUrl { error })?;
        self.request_executor
            .execute(
                WpNetworkRequest {
                    uuid: Uuid::new_v4().into(),
                    method: RequestMethod::GET,
                    url: WpEndpointUrl(site_url.url()),
                    header_map: WpNetworkHeaderMap::default().into(),
                    body: None,
                }
                .into(),
            )
            .await
            .map_err(|error| ParseHtmlFailure::FetchSite { error })
    }
}

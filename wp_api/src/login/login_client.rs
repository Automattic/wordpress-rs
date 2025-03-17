use super::{
    url_discovery::{
        self, ApplicationPasswordsNotSupportedReason, AutoDiscoveryAttempt,
        AutoDiscoveryAttemptFailure, AutoDiscoveryAttemptResult, AutoDiscoveryAttemptSuccess,
        AutoDiscoveryResult, AutoDiscoveryUniffiResult, FetchWpJsonFailure, FetchWpJsonSuccess,
        FindApiRootLinkHeaderFailure, FindApiRootLinkHeaderSuccess, IsWordPressSiteAttemptResult,
        IsWordPressSiteParseHtmlResult, ParseApiRootUrlError, ParseHtmlFailure,
        API_ROOT_LINK_HEADER,
    },
    WpApiDetails,
};
use crate::{
    login::url_discovery::RootWpJson,
    middleware::{PerformsRequests, WpApiMiddlewarePipeline},
    request::{
        endpoint::{WpEndpointUrl, WP_JSON_PATH_SEGMENTS},
        RequestExecutor, RequestMethod, WpNetworkHeaderMap, WpNetworkRequest, WpNetworkResponse,
    },
    ParseUrlError, ParsedUrl, RequestExecutionError,
};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, uniffi::Object)]
struct UniffiWpLoginClient {
    inner: Arc<WpLoginClient>,
}

#[uniffi::export]
impl UniffiWpLoginClient {
    #[uniffi::constructor]
    fn new(
        request_executor: Arc<dyn RequestExecutor>,
        middleware_pipeline: Arc<WpApiMiddlewarePipeline>,
    ) -> Self {
        Self {
            inner: WpLoginClient::new(request_executor, middleware_pipeline).into(),
        }
    }

    async fn api_discovery(&self, site_url: String) -> AutoDiscoveryUniffiResult {
        self.inner.api_discovery(site_url).await.into()
    }
}

#[derive(Debug)]
pub struct WpLoginClient {
    request_executor: Arc<dyn RequestExecutor>,
    middleware_pipeline: Arc<WpApiMiddlewarePipeline>,
}

impl WpLoginClient {
    pub fn new(
        request_executor: Arc<dyn RequestExecutor>,
        middleware_pipeline: Arc<WpApiMiddlewarePipeline>,
    ) -> Self {
        Self {
            request_executor,
            middleware_pipeline,
        }
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
        let parsed_site_url: Arc<ParsedUrl> = match ParsedUrl::parse(&attempt.attempt_site_url) {
            Ok(u) => u,
            Err(e) => {
                return AutoDiscoveryAttemptResult::from_parse_site_url_error(attempt, e);
            }
        }
        .into();
        let parse_html_result = self
            .fetch_site(Arc::clone(&parsed_site_url))
            .await
            .map(|r| IsWordPressSiteParseHtmlResult::parse_response(&r.body_as_string()));
        let api_root_url_from_link_tag = parse_html_result
            .as_ref()
            .ok()
            .and_then(|r| r.api_root_url_from_link_tag.as_ref())
            .map(Arc::clone);
        let fetch_wp_json_result = self.fetch_wp_json(Arc::clone(&parsed_site_url)).await;
        let api_link_header_result = self
            .find_api_root_url(
                Arc::clone(&parsed_site_url),
                api_root_url_from_link_tag,
                fetch_wp_json_result.clone(),
            )
            .await;
        let discovery_result = self
            .inner_attempt_api_discovery(api_link_header_result.clone())
            .await;
        let is_wordpress_site = IsWordPressSiteAttemptResult {
            api_link_header_result,
            fetch_wp_json_result,
            parse_html_result,
        };
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
            match WpApiDetails::try_from(fetch_api_details_response.body) {
                Ok(api_details) => api_details,
                Err(error) => {
                    return Err(AutoDiscoveryAttemptFailure::ParseApiDetails {
                        parsed_site_url: api_root_url_success.parsed_site_url,
                        api_root_url: api_root_url_success.api_root_url,
                        parsing_error_message: error.to_string(),
                    });
                }
            };

        if !api_details.has_application_passwords_authentication_url() {
            let reason = if api_details.has_application_password_blocking_plugin() {
                let plugins = api_details.application_password_blocking_plugins();

                if plugins.len() == 1 {
                    // If there's only one candidate, we can show more information in the error message
                    Some(ApplicationPasswordsNotSupportedReason::ApplicationPasswordBlockedByPlugin {
                        plugin: plugins.first().expect("Already verified there is one plugin").clone(),
                    })
                } else {
                    // If there's more than one, for now we'll just give a generic error
                    Some(ApplicationPasswordsNotSupportedReason::ApplicationPasswordBlockedByMultiplePlugins)
                }
            } else if !api_details.uses_https() {
                // Application Passwords are disabled for non-HTTPS sites by default
                if api_details.site_url_is_local_development_environment() {
                    Some(ApplicationPasswordsNotSupportedReason::SiteIsLocalDevelopmentEnvironment)
                } else {
                    Some(ApplicationPasswordsNotSupportedReason::ApplicationPasswordsDisabledForHttpSite)
                }
            } else {
                None
            };

            Err(
                AutoDiscoveryAttemptFailure::ApplicationPasswordsNotSupported {
                    parsed_site_url: api_root_url_success.parsed_site_url,
                    api_root_url: api_root_url_success.api_root_url,
                    api_details: api_details.into(),
                    reason,
                },
            )
        } else {
            Ok(AutoDiscoveryAttemptSuccess {
                parsed_site_url: api_root_url_success.parsed_site_url,
                api_root_url: api_root_url_success.api_root_url,
                api_details,
            })
        }
    }

    async fn find_api_root_url(
        &self,
        parsed_site_url: Arc<ParsedUrl>,
        api_root_url_from_link_tag: Option<Arc<ParsedUrl>>,
        fetch_wp_json_result: Result<FetchWpJsonSuccess, FetchWpJsonFailure>,
    ) -> Result<FindApiRootLinkHeaderSuccess, FindApiRootLinkHeaderFailure> {
        if let Some(api_root_url) = api_root_url_from_link_tag {
            return Ok(FindApiRootLinkHeaderSuccess {
                parsed_site_url,
                api_root_url,
            });
        }
        match self
            .fetch_and_parse_api_root_url_header(parsed_site_url.clone())
            .await
        {
            Ok(s) => Ok(s),
            Err(fetch_and_parse_api_root_url_header_err) => {
                // If we can't find the link header, but we were able to fetch `/wp-json` from the
                // attempt url, we assume that it's the correct api root url
                //
                // We don't immediately rely on this because there might be cases where `/wp-json`
                // exists, but its not the actual API root
                if let Ok(fetch_wp_json_success) = fetch_wp_json_result {
                    Ok(FindApiRootLinkHeaderSuccess {
                        parsed_site_url,
                        api_root_url: fetch_wp_json_success.wp_json_url,
                    })
                } else {
                    // If fetching `/wp-json` wasn't successful, we return the original error from
                    // fetching the api root link header
                    Err(fetch_and_parse_api_root_url_header_err)
                }
            }
        }
    }

    async fn fetch_and_parse_api_root_url_header(
        &self,
        parsed_site_url: Arc<ParsedUrl>,
    ) -> Result<FindApiRootLinkHeaderSuccess, FindApiRootLinkHeaderFailure> {
        let fetch_api_root_url_response =
            match self.fetch_api_root_url(Arc::clone(&parsed_site_url)).await {
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
        }
        .into();
        Ok(FindApiRootLinkHeaderSuccess {
            parsed_site_url,
            api_root_url,
        })
    }

    /// Parses the API root URL from the Link header
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
                header_map: response.response_header_map,
                status_code: response.status_code,
            }),
        }
    }

    // Fetches the site's homepage headers with a HEAD request
    async fn fetch_api_root_url(
        &self,
        parsed_site_url: Arc<ParsedUrl>,
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        let api_root_request = WpNetworkRequest {
            uuid: Uuid::new_v4().into(),
            retry_count: 0,
            method: RequestMethod::HEAD,
            url: WpEndpointUrl(parsed_site_url.url()),
            header_map: WpNetworkHeaderMap::default().into(),
            body: None,
        };
        self.perform(api_root_request.into()).await
    }

    async fn fetch_wp_api_details(
        &self,
        api_root_url: &ParsedUrl,
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        self.perform(
            WpNetworkRequest {
                uuid: Uuid::new_v4().into(),
                retry_count: 0,
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
        parsed_site_url: Arc<ParsedUrl>,
    ) -> Result<FetchWpJsonSuccess, FetchWpJsonFailure> {
        let wp_json_url: ParsedUrl = {
            let mut wp_json_url = parsed_site_url.inner.clone();
            wp_json_url
                .path_segments_mut()
                .map_err(|_| FetchWpJsonFailure::ParseSiteUrl {
                    error: ParseUrlError::RelativeUrlWithCannotBeABaseBase,
                })?
                .extend(WP_JSON_PATH_SEGMENTS);
            wp_json_url
        }
        .into();
        let fetch_wp_json_response = match self
            .perform(
                WpNetworkRequest {
                    uuid: Uuid::new_v4().into(),
                    retry_count: 0,
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
            wp_json_url: wp_json_url.into(),
            root_wp_json,
        })
    }

    async fn fetch_site(
        &self,
        parsed_site_url: Arc<ParsedUrl>,
    ) -> Result<WpNetworkResponse, ParseHtmlFailure> {
        self.perform(
            WpNetworkRequest {
                uuid: Uuid::new_v4().into(),
                retry_count: 0,
                method: RequestMethod::GET,
                url: WpEndpointUrl(parsed_site_url.url()),
                header_map: WpNetworkHeaderMap::default().into(),
                body: None,
            }
            .into(),
        )
        .await
        .map_err(|error| ParseHtmlFailure::FetchSite { error })
    }
}

impl PerformsRequests for WpLoginClient {
    fn get_middleware_pipeline(&self) -> Arc<WpApiMiddlewarePipeline> {
        self.middleware_pipeline.clone()
    }

    fn get_request_executor(&self) -> Arc<dyn RequestExecutor> {
        self.request_executor.clone()
    }
}

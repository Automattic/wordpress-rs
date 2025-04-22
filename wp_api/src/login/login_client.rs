use super::{
    WpApiDetails,
    url_discovery::{
        self, API_ROOT_LINK_HEADER, ApiRootUrl, ApplicationPasswordsNotSupportedReason,
        AutoDiscoveryAttempt, AutoDiscoveryAttemptFailure, AutoDiscoveryAttemptResult,
        AutoDiscoveryAttemptSuccess, AutoDiscoveryResult, FetchAndParseApiRootFailure,
        FindApiRootFailure, ParseHomepageResult, XmlrpcDisabledReason, XmlrpcDiscoveryError,
        extract_rsd_url, is_xmlrpc_response, parse_rsd_for_xmlrpc,
    },
};
use crate::{
    ParsedUrl, RequestExecutionError, WpError,
    middleware::{PerformsRequests, WpApiMiddlewarePipeline},
    request::{
        RequestExecutor, RequestMethod, ResponseBodyType, WpNetworkHeaderMap, WpNetworkRequest,
        WpNetworkRequestBody, WpNetworkResponse,
        endpoint::{WP_JSON_PATH_SEGMENTS, WpEndpointUrl},
    },
};
use itertools::Itertools;
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

    async fn api_discovery(
        &self,
        site_url: String,
    ) -> Result<AutoDiscoveryAttemptSuccess, AutoDiscoveryAttemptFailure> {
        self.inner
            .api_discovery(site_url)
            .await
            .combined_result()
            .cloned()
            .map_err(|e| e.clone())
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

        match self.find_api_root_url(Arc::clone(&parsed_site_url)).await {
            Ok(api_root_url) => AutoDiscoveryAttemptResult {
                attempt_type: attempt.attempt_type,
                attempt_site_url: attempt.attempt_site_url,
                api_discovery_result: self
                    .fetch_and_parse_api_root(Arc::clone(&parsed_site_url), &api_root_url)
                    .await
                    .map_err(|fetch_and_parse_api_root_failure| {
                        AutoDiscoveryAttemptFailure::from_fetch_and_parse_api_root_failure(
                            parsed_site_url,
                            api_root_url.0,
                            fetch_and_parse_api_root_failure,
                        )
                    }),
            },
            Err(find_api_root_failure) => {
                let root_wp_json_url: Arc<ParsedUrl> =
                    match Self::root_wp_json_url((*parsed_site_url).clone()) {
                        Some(u) => u,
                        None => {
                            return AutoDiscoveryAttemptResult {
                                attempt_type: attempt.attempt_type,
                                attempt_site_url: attempt.attempt_site_url,
                                api_discovery_result: Err(
                                    AutoDiscoveryAttemptFailure::from_find_api_root_failure(
                                        parsed_site_url,
                                        find_api_root_failure,
                                    ),
                                ),
                            };
                        }
                    }
                    .into();

                // If we can't find the api root, we try using the root `/wp-json` as a last resort
                match self
                    .fetch_and_parse_api_root(
                        Arc::clone(&parsed_site_url),
                        &ApiRootUrl(Arc::clone(&root_wp_json_url)),
                    )
                    .await
                {
                    Ok(api_discovery_success) => AutoDiscoveryAttemptResult {
                        attempt_type: attempt.attempt_type,
                        attempt_site_url: attempt.attempt_site_url,
                        api_discovery_result: Ok(api_discovery_success),
                    },
                    Err(fetch_and_parse_api_root_failure) => match fetch_and_parse_api_root_failure
                    {
                        FetchAndParseApiRootFailure::FetchApiRoot { .. }
                        | FetchAndParseApiRootFailure::ParseApiRoot { .. } => {
                            // If we fail to fetch or parse root `/wp-json`, we return the original
                            // find API root url failure
                            AutoDiscoveryAttemptResult {
                                attempt_type: attempt.attempt_type,
                                attempt_site_url: attempt.attempt_site_url,
                                api_discovery_result: Err(
                                    AutoDiscoveryAttemptFailure::from_find_api_root_failure(
                                        parsed_site_url,
                                        find_api_root_failure,
                                    ),
                                ),
                            }
                        }
                        _ => {
                            // If we successfully fetch the root `/wp-json`, but had another
                            // failure afterwards, we return that failure, because the API
                            // discovery has progressed further than the original find API root url
                            // failure
                            let err = Err(
                                AutoDiscoveryAttemptFailure::from_fetch_and_parse_api_root_failure(
                                    parsed_site_url,
                                    root_wp_json_url,
                                    fetch_and_parse_api_root_failure,
                                ),
                            );
                            AutoDiscoveryAttemptResult {
                                attempt_type: attempt.attempt_type,
                                attempt_site_url: attempt.attempt_site_url,
                                api_discovery_result: err,
                            }
                        }
                    },
                }
            }
        }
    }

    async fn fetch_and_parse_api_root(
        &self,
        parsed_site_url: Arc<ParsedUrl>,
        api_root_url: &ApiRootUrl,
    ) -> Result<AutoDiscoveryAttemptSuccess, FetchAndParseApiRootFailure> {
        let fetch_api_details_response = match self.fetch_api_root(api_root_url).await {
            Ok(r) => r,
            Err(error) => return Err(FetchAndParseApiRootFailure::FetchApiRoot { error }),
        };
        let api_details = Self::parse_api_root(&fetch_api_details_response)?;

        if let Some(application_passwords_authentication_url) =
            api_details.find_application_passwords_authentication_url()
        {
            let application_passwords_authentication_url =
                ParsedUrl::parse(application_passwords_authentication_url.as_str())
                    .expect(
                        "Application passwords url returned from the server should be a valid url",
                    )
                    .into();
            Ok(AutoDiscoveryAttemptSuccess {
                parsed_site_url,
                api_root_url: Arc::clone(&api_root_url.0),
                api_details: Arc::new(api_details),
                application_passwords_authentication_url,
            })
        } else {
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
                FetchAndParseApiRootFailure::ApplicationPasswordsNotSupported {
                    api_details: api_details.into(),
                    reason,
                },
            )
        }
    }

    async fn find_api_root_url(
        &self,
        parsed_site_url: Arc<ParsedUrl>,
    ) -> Result<ApiRootUrl, FindApiRootFailure> {
        let response = self
            .fetch_homepage(Arc::clone(&parsed_site_url))
            .await
            .map_err(|error| FindApiRootFailure::FetchHomepage { error })?;
        // First check if we can find and parse the api root from the link header
        if let Some(api_root_url) = self.parse_response_link_header_to_find_api_root(&response) {
            return Ok(ApiRootUrl(api_root_url.into()));
        }
        // If we can't find the api root in the link header, we parse the HTML page to look for it
        // in the link tags
        let parse_html_result = ParseHomepageResult::parse_response(&response.body_as_string());
        if let Some(api_root_url) = parse_html_result.api_root_url_from_link_tag {
            return Ok(ApiRootUrl(api_root_url));
        }

        if parse_html_result.does_look_like_a_wp_site() {
            Err(FindApiRootFailure::RestApiDisabled)
        } else {
            Err(FindApiRootFailure::ProbablyNotAWordPressSite)
        }
    }

    fn parse_response_link_header_to_find_api_root(
        &self,
        response: &WpNetworkResponse,
    ) -> Option<ParsedUrl> {
        response
            .get_link_header(API_ROOT_LINK_HEADER)
            .into_iter()
            .nth(0)
            .map(ParsedUrl::new)
    }

    async fn fetch_api_root(
        &self,
        api_root_url: &ApiRootUrl,
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        self.perform(
            WpNetworkRequest {
                uuid: Uuid::new_v4().into(),
                retry_count: 0,
                method: RequestMethod::GET,
                url: WpEndpointUrl(api_root_url.0.url()),
                header_map: WpNetworkHeaderMap::default().into(),
                body: None,
            }
            .into(),
        )
        .await
    }

    fn root_wp_json_url(parsed_site_url: ParsedUrl) -> Option<ParsedUrl> {
        let mut root_wp_json_url = parsed_site_url.inner;
        root_wp_json_url
            .path_segments_mut()
            .ok()?
            .extend(WP_JSON_PATH_SEGMENTS);
        Some(root_wp_json_url.into())
    }

    async fn fetch_homepage(
        &self,
        parsed_site_url: Arc<ParsedUrl>,
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
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
    }

    fn parse_api_root(
        fetch_api_details_response: &WpNetworkResponse,
    ) -> Result<WpApiDetails, FetchAndParseApiRootFailure> {
        WpApiDetails::try_from(fetch_api_details_response.body.as_slice()).map_err(|error| {
            if let Some(wp_error) = WpError::try_parse(&fetch_api_details_response.body) {
                FetchAndParseApiRootFailure::WpError {
                    error_code: wp_error.code,
                    error_message: wp_error.message,
                    status_code: fetch_api_details_response.status_code,
                }
            } else {
                let response_body = fetch_api_details_response.body_as_string();
                let response_body_type = ResponseBodyType::new(&response_body);
                FetchAndParseApiRootFailure::ParseApiRoot {
                    parsing_error_message: error.to_string(),
                    response_body,
                    response_body_type,
                }
            }
        })
    }

    pub async fn xmlrpc_discovery(
        &self,
        details: AutoDiscoveryAttemptSuccess,
    ) -> Result<ParsedUrl, XmlrpcDiscoveryError> {
        let mut candidates: Vec<ParsedUrl> = vec![];
        // Prioritize discovered XML-RPC URL if it's available from the site.
        if let Ok(url) = self.xmlrpc_from_rsd(&details.parsed_site_url).await {
            candidates.push(url);
        }
        // Fallback to the default XML-RPC URL.
        candidates.push(
            details
                .parsed_site_url
                .by_extending_and_splitting_by_forward_slash(["xmlrpc.php"])
                .into(),
        );
        candidates.dedup();

        let mut failures: Vec<XmlrpcDiscoveryError> = vec![];
        for candidate in candidates {
            match self
                .validate_xmlrpc_url(&candidate, &details.api_details)
                .await
            {
                Ok(_) => return Ok(candidate),
                Err(error) => {
                    failures.push(error);
                }
            }
        }

        Err(failures
            .into_iter()
            .sorted_by(|a, b| b.importance().cmp(&a.importance()))
            .next()
            .expect("There is at least one failure"))
    }

    async fn validate_xmlrpc_url(
        &self,
        url: &ParsedUrl,
        api_details: &WpApiDetails,
    ) -> Result<(), XmlrpcDiscoveryError> {
        let response = self.perform(
            WpNetworkRequest {
                uuid: Uuid::new_v4().into(),
                retry_count: 0,
                method: RequestMethod::POST,
                url: WpEndpointUrl(url.url()),
                header_map: WpNetworkHeaderMap::default().into(),
                body: Some(Arc::new(WpNetworkRequestBody::new(r#"<?xml version="1.0"?><methodCall><methodName>system.listMethods</methodName></methodCall>"#.as_bytes().to_vec()))),
            }
            .into(),
        )
        .await
        // It's very likely xml-rpc is blocked by the hosting provider (the request has not reached to WordPress),
        // if the site does not send any valid HTTP response.
        .map_err(|_| XmlrpcDiscoveryError::Disabled { reason: XmlrpcDisabledReason::ByHost })?;

        // 200 status code and a valid XML-RPC response indicates that XML-RPC is enabled.
        // All other responses indicate that XML-RPC is disabled.
        if response.status_code == 200 && is_xmlrpc_response(&response.body_as_string()) {
            return Ok(());
        }

        let mut plugins = api_details.xmlrpc_blocking_plugins();
        let reason = match plugins.len() {
            0 => XmlrpcDisabledReason::ByHost,
            1 => XmlrpcDisabledReason::ByPlugin {
                plugin: plugins.pop().expect("Already verified there is one plugin"),
            },
            _ => XmlrpcDisabledReason::ByMultiplePlugins,
        };
        Err(XmlrpcDiscoveryError::Disabled { reason })
    }

    async fn xmlrpc_from_rsd(
        &self,
        parsed_site_url: &ParsedUrl,
    ) -> Result<ParsedUrl, XmlrpcDiscoveryError> {
        let response = self
            .perform(
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
            .map_err(|error| XmlrpcDiscoveryError::FetchHomepage { error })?;

        let rsd_url = extract_rsd_url(&response.body_as_string())
            .ok_or(XmlrpcDiscoveryError::EndpointNotFound)?;

        let rsd_response = self
            .perform(
                WpNetworkRequest {
                    uuid: Uuid::new_v4().into(),
                    retry_count: 0,
                    method: RequestMethod::GET,
                    url: WpEndpointUrl(rsd_url),
                    header_map: WpNetworkHeaderMap::default().into(),
                    body: None,
                }
                .into(),
            )
            .await
            .map_err(|_| XmlrpcDiscoveryError::Disabled {
                reason: XmlrpcDisabledReason::ByHost,
            })?;

        parse_rsd_for_xmlrpc(&rsd_response.body_as_string())
            .ok_or(XmlrpcDiscoveryError::EndpointNotFound)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{WpErrorCode, unit_test_common::wp_network_response_from_json};

    #[test]
    fn test_parse_api_details_wp_error_rest_forbidden() {
        let json = r#"{
          "code": "rest_forbidden",
          "message": "REST API access is restricted."
        }"#;
        let response = wp_network_response_from_json(json, 403);
        let result = WpLoginClient::parse_api_root(&response);
        assert!(
            matches!(
                result,
                Err(FetchAndParseApiRootFailure::WpError {
                    error_code: WpErrorCode::Forbidden,
                    status_code: 403,
                    ..
                })
            ),
            "{:#?}",
            result
        );
    }
}

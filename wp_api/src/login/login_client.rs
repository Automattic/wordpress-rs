use super::{
    WpApiApplicationPasswordDetails, WpApiDetails,
    url_discovery::{
        self, API_ROOT_LINK_HEADER, ApiRootUrl, ApplicationPasswordsNotSupportedReason,
        AutoDiscoveryAttempt, AutoDiscoveryAttemptFailure, AutoDiscoveryAttemptResult,
        AutoDiscoveryAttemptSuccess, AutoDiscoveryResult, DiscoveredAuthenticationMechanism,
        FetchAndParseApiRootFailure, FindApiRootFailure, ParseApiRootFailureReason,
        ParseHomepageResult, XmlrpcDisabledReason, XmlrpcDiscoveryError, extract_rsd_url,
        is_xmlrpc_response, parse_rsd_for_xmlrpc,
    },
};
use crate::{
    api_error::{
        RequestExecutionError, RequestExecutionErrorReason, WpApiError, WpError, WpErrorCode,
    },
    auth::WpAuthenticationProvider,
    middleware::{PerformsRequests, WpApiMiddlewarePipeline},
    parsed_url::ParsedUrl,
    request::{
        RequestContext, RequestExecutor, RequestMethod, ResponseBodyType, WpNetworkHeaderMap,
        WpNetworkRequest, WpNetworkRequestBody, WpNetworkResponse,
        endpoint::{
            WP_JSON_PATH_SEGMENTS, WpEndpointUrl, WpOrgSiteApiUrlResolver,
            application_passwords_endpoint::{
                ApplicationPasswordsRequestBuilder,
                ApplicationPasswordsRequestRetrieveCurrentWithEditContextResponse,
            },
        },
    },
};
use itertools::Itertools;
use std::sync::Arc;
use uuid::Uuid;
use wp_localization::{MessageBundle, WpMessages, WpSupportsLocalization};
use wp_localization_macro::WpDeriveLocalizable;

#[derive(uniffi::Object)]
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
        context: Option<Arc<RequestContext>>,
    ) -> Result<AutoDiscoveryAttemptSuccess, AutoDiscoveryAttemptFailure> {
        self.inner
            .api_discovery(site_url, context)
            .await
            .combined_result()
            .cloned()
            .map_err(|e| e.clone())
    }

    /// See [`WpLoginClient::verify_issued_application_password`]: call it only with an
    /// application password the site issued moments ago.
    async fn verify_issued_application_password(
        &self,
        api_root_url: Arc<ParsedUrl>,
        credentials: WpApiApplicationPasswordDetails,
        context: Option<Arc<RequestContext>>,
    ) -> Result<(), VerifyIssuedApplicationPasswordError> {
        self.inner
            .verify_issued_application_password(api_root_url, credentials, context)
            .await
    }
}

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

    pub fn new_with_default_middleware_pipeline(
        request_executor: Arc<dyn RequestExecutor>,
    ) -> Self {
        Self::new(
            request_executor,
            Arc::new(WpApiMiddlewarePipeline::default()),
        )
    }

    pub async fn api_discovery(
        &self,
        site_url: String,
        context: Option<Arc<RequestContext>>,
    ) -> AutoDiscoveryResult {
        let attempts =
            futures::future::join_all(url_discovery::construct_attempts(site_url).into_iter().map(
                |attempt| async { self.attempt_api_discovery(attempt, context.clone()).await },
            ))
            .await;
        AutoDiscoveryResult {
            attempts: attempts.into_iter().map(|r| (r.attempt_type, r)).collect(),
        }
    }

    async fn attempt_api_discovery(
        &self,
        attempt: AutoDiscoveryAttempt,
        context: Option<Arc<RequestContext>>,
    ) -> AutoDiscoveryAttemptResult {
        let parsed_site_url: Arc<ParsedUrl> = match ParsedUrl::parse(&attempt.attempt_site_url) {
            Ok(u) => u,
            Err(e) => {
                return AutoDiscoveryAttemptResult::from_parse_site_url_error(attempt, e);
            }
        }
        .into();

        match self
            .find_api_root_url(Arc::clone(&parsed_site_url), context.clone())
            .await
        {
            Ok(api_root_url) => AutoDiscoveryAttemptResult {
                attempt_type: attempt.attempt_type,
                attempt_site_url: attempt.attempt_site_url,
                api_discovery_result: self
                    .fetch_and_parse_api_root(Arc::clone(&parsed_site_url), &api_root_url, context)
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
                        context,
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
        context: Option<Arc<RequestContext>>,
    ) -> Result<AutoDiscoveryAttemptSuccess, FetchAndParseApiRootFailure> {
        let fetch_api_details_response = match self.fetch_api_root(api_root_url, context).await {
            Ok(r) => r,
            Err(error) => return Err(FetchAndParseApiRootFailure::FetchApiRoot { error }),
        };
        let api_details = Self::parse_api_root(&fetch_api_details_response)?;

        // Try Application Passwords first (preferred for self-hosted sites)
        if let Some(application_passwords_authentication_url) =
            api_details.find_application_passwords_authentication_url()
        {
            let authentication_url =
                ParsedUrl::parse(application_passwords_authentication_url.as_str())
                    .expect(
                        "Application passwords url returned from the server should be a valid url",
                    )
                    .into();
            return Ok(AutoDiscoveryAttemptSuccess {
                parsed_site_url,
                api_root_url: Arc::clone(&api_root_url.0),
                api_details: Arc::new(api_details),
                authentication: DiscoveredAuthenticationMechanism::ApplicationPasswords {
                    authentication_url,
                },
            });
        }

        // Try OAuth2 (used by WordPress.com sites)
        if let Some(oauth2_endpoints) = api_details.find_oauth2_endpoints() {
            return Ok(AutoDiscoveryAttemptSuccess {
                parsed_site_url,
                api_root_url: Arc::clone(&api_root_url.0),
                api_details: Arc::new(api_details),
                authentication: DiscoveredAuthenticationMechanism::OAuth2 {
                    endpoints: oauth2_endpoints,
                },
            });
        }

        // Neither authentication mechanism is available
        let reason = if api_details.has_application_password_blocking_plugin() {
            let plugins = api_details.application_password_blocking_plugins();

            if plugins.len() == 1 {
                // If there's only one candidate, we can show more information in the error message
                Some(
                    ApplicationPasswordsNotSupportedReason::ApplicationPasswordBlockedByPlugin {
                        plugin: plugins
                            .first()
                            .expect("Already verified there is one plugin")
                            .clone(),
                    },
                )
            } else {
                // If there's more than one, for now we'll just give a generic error
                Some(ApplicationPasswordsNotSupportedReason::ApplicationPasswordBlockedByMultiplePlugins)
            }
        } else if !api_details.uses_https() {
            // Application Passwords are disabled for non-HTTPS sites by default
            if api_details.site_url_is_local_development_environment() {
                Some(ApplicationPasswordsNotSupportedReason::SiteIsLocalDevelopmentEnvironment)
            } else {
                Some(
                    ApplicationPasswordsNotSupportedReason::ApplicationPasswordsDisabledForHttpSite,
                )
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

    async fn find_api_root_url(
        &self,
        parsed_site_url: Arc<ParsedUrl>,
        context: Option<Arc<RequestContext>>,
    ) -> Result<ApiRootUrl, FindApiRootFailure> {
        let response = self
            .fetch_homepage(Arc::clone(&parsed_site_url), context)
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
        context: Option<Arc<RequestContext>>,
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
            context,
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
        context: Option<Arc<RequestContext>>,
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
            context,
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
                let reason = if let ResponseBodyType::MaybeHtml = response_body_type {
                    ParseApiRootFailureReason::from_maybe_html_response_body(response_body.as_str())
                } else {
                    None
                };
                FetchAndParseApiRootFailure::ParseApiRoot {
                    parsing_error_message: error.to_string(),
                    response_body,
                    response_body_type,
                    reason,
                }
            }
        })
    }

    /// Sends one authenticated request with an application password the site issued moments
    /// ago, to find out whether the site's server passes the `Authorization` header to
    /// WordPress.
    ///
    /// Call this only with a freshly issued password: right after the authorization flow
    /// returns it, or right after the site creates it. Some hosts answer a revoked or wrong
    /// password with the same `401 rest_not_logged_in` response, so the result is meaningless
    /// for any other password.
    pub async fn verify_issued_application_password(
        &self,
        api_root_url: Arc<ParsedUrl>,
        credentials: WpApiApplicationPasswordDetails,
        context: Option<Arc<RequestContext>>,
    ) -> Result<(), VerifyIssuedApplicationPasswordError> {
        let request = ApplicationPasswordsRequestBuilder::new(
            Arc::new(WpOrgSiteApiUrlResolver::new(api_root_url)),
            Arc::new(WpAuthenticationProvider::static_with_username_and_password(
                credentials.user_login,
                credentials.password,
            )),
            None,
        )
        .retrieve_current_with_edit_context();
        let response = self
            .perform(request.into(), context)
            .await
            .map_err(WpApiError::from)?;
        // Parse the body rather than only checking for an error response: a proxy that answers
        // 200 with an HTML page has not confirmed the password.
        let parsed: Result<
            ApplicationPasswordsRequestRetrieveCurrentWithEditContextResponse,
            WpApiError,
        > = response.parse();
        parsed?;
        Ok(())
    }

    pub async fn xmlrpc_discovery(
        &self,
        details: AutoDiscoveryAttemptSuccess,
        context: Option<Arc<RequestContext>>,
    ) -> Result<ParsedUrl, XmlrpcDiscoveryError> {
        let mut candidates: Vec<ParsedUrl> = vec![];
        // Prioritize discovered XML-RPC URL if it's available from the site.
        if let Ok(url) = self
            .xmlrpc_from_rsd(&details.parsed_site_url, context.clone())
            .await
        {
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
                .validate_xmlrpc_url(&candidate, &details.api_details, context.clone())
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
        context: Option<Arc<RequestContext>>,
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
            context,
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
        context: Option<Arc<RequestContext>>,
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
                context.clone(),
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
                context,
            )
            .await
            .map_err(|_| XmlrpcDiscoveryError::Disabled {
                reason: XmlrpcDisabledReason::ByHost,
            })?;

        parse_rsd_for_xmlrpc(&rsd_response.body_as_string())
            .ok_or(XmlrpcDiscoveryError::EndpointNotFound)
    }
}

/// The failure of [`WpLoginClient::verify_issued_application_password`].
#[derive(Debug, thiserror::Error, uniffi::Error, WpDeriveLocalizable)]
pub enum VerifyIssuedApplicationPasswordError {
    /// WordPress answered `401 rest_not_logged_in` to a request that carried the freshly
    /// issued password, so the `Authorization` header did not reach WordPress.
    AuthorizationHeaderBlocked { hostname: String, error: WpApiError },
    /// Any other failure. None of them is expected seconds after the password was issued,
    /// and none identifies the cause reliably enough to get its own variant.
    Other { error: WpApiError },
}

impl From<WpApiError> for VerifyIssuedApplicationPasswordError {
    fn from(error: WpApiError) -> Self {
        // Only a WordPress error proves the request reached WordPress. A non-WordPress 401 or
        // 403 may come from a rule that denies only application-password routes, while the
        // rest of the sign-in still works, so it is not treated as a blocked header.
        match &error {
            WpApiError::WpError {
                error_code: WpErrorCode::Unauthorized,
                status_code: 401,
                request_url,
                ..
            } => Self::AuthorizationHeaderBlocked {
                hostname: RequestExecutionErrorReason::hostname_of(request_url),
                error,
            },
            _ => Self::Other { error },
        }
    }
}

impl WpSupportsLocalization for VerifyIssuedApplicationPasswordError {
    fn message_bundle(&self) -> MessageBundle<'_> {
        match self {
            Self::AuthorizationHeaderBlocked { .. } => WpMessages::authorization_header_blocked(),
            Self::Other { error } => error.message_bundle(),
        }
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
    use crate::{request::WpMultipartFormRequest, unit_test_common::wp_network_response_from_json};
    use async_trait::async_trait;
    use rstest::*;
    use std::sync::Mutex;

    const INTROSPECT_URL: &str =
        "https://example.com/wp-json/wp/v2/users/me/application-passwords/introspect?context=edit";
    const HTML_BODY: &str = "<html><body><h1>Unauthorized</h1></body></html>";

    /// Answers every request with `response`, stamped with the request's URL, and records the
    /// requests it receives.
    struct StubExecutor {
        response: WpNetworkResponse,
        requests: Mutex<Vec<Arc<WpNetworkRequest>>>,
    }

    #[async_trait]
    impl RequestExecutor for StubExecutor {
        async fn execute(
            &self,
            request: Arc<WpNetworkRequest>,
        ) -> Result<WpNetworkResponse, RequestExecutionError> {
            self.requests.lock().unwrap().push(request.clone());
            Ok(WpNetworkResponse {
                body: self.response.body.clone(),
                status_code: self.response.status_code,
                response_header_map: self.response.response_header_map.clone(),
                request_url: request.url.clone(),
                request_method: request.method.clone(),
                request_header_map: request.header_map.clone(),
            })
        }

        async fn upload(
            &self,
            _request: Arc<WpMultipartFormRequest>,
        ) -> Result<WpNetworkResponse, RequestExecutionError> {
            unimplemented!()
        }

        async fn sleep(&self, _: u64) {}

        fn cancel(&self, _: Arc<RequestContext>) {}
    }

    async fn verify(
        response: WpNetworkResponse,
    ) -> (
        Result<(), VerifyIssuedApplicationPasswordError>,
        Vec<Arc<WpNetworkRequest>>,
    ) {
        let executor = Arc::new(StubExecutor {
            response,
            requests: Mutex::new(vec![]),
        });
        let client = WpLoginClient::new_with_default_middleware_pipeline(executor.clone());
        let result = client
            .verify_issued_application_password(
                ParsedUrl::parse("https://example.com/wp-json/")
                    .unwrap()
                    .into(),
                WpApiApplicationPasswordDetails {
                    site_url: "https://example.com".to_string(),
                    user_login: "demo".to_string(),
                    password: "abcd efgh".to_string(),
                },
                None,
            )
            .await;
        let requests = executor.requests.lock().unwrap().clone();
        (result, requests)
    }

    #[tokio::test]
    async fn test_verify_issued_application_password_succeeds() {
        let json = r#"{
          "uuid": "0b0c1a5e-8c6f-4d4b-9f3e-2f1d7c5b8a90",
          "app_id": "",
          "name": "App",
          "created": "2026-09-30T01:00:00",
          "last_used": null,
          "last_ip": null
        }"#;
        let (result, requests) = verify(wp_network_response_from_json(json, 200)).await;
        assert!(result.is_ok(), "{result:#?}");

        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        assert_eq!(request.method, RequestMethod::GET);
        assert_eq!(request.url.0, INTROSPECT_URL);
        // "demo:abcd efgh" in base64.
        assert_eq!(
            request
                .header_map
                .to_header_map()
                .get(http::header::AUTHORIZATION)
                .unwrap(),
            "Basic ZGVtbzphYmNkIGVmZ2g="
        );
    }

    #[tokio::test]
    async fn test_verify_issued_application_password_rest_not_logged_in_is_blocked() {
        let json = r#"{
          "code": "rest_not_logged_in",
          "message": "You are not currently logged in.",
          "data": { "status": 401 }
        }"#;
        let (result, _) = verify(wp_network_response_from_json(json, 401)).await;
        assert!(
            matches!(
                &result,
                Err(VerifyIssuedApplicationPasswordError::AuthorizationHeaderBlocked {
                    hostname,
                    error: WpApiError::WpError {
                        error_code: WpErrorCode::Unauthorized,
                        status_code: 401,
                        ..
                    },
                }) if hostname == "example.com"
            ),
            "{result:#?}"
        );
    }

    #[rstest]
    #[case::html_401(HTML_BODY, 401)]
    #[case::html_403(HTML_BODY, 403)]
    #[case::application_passwords_disabled(
        r#"{"code":"application_passwords_disabled","message":"Application passwords are not available.","data":{"status":401}}"#,
        401
    )]
    #[case::server_error("", 500)]
    #[case::html_200(HTML_BODY, 200)]
    #[case::malformed_json_200("{", 200)]
    #[tokio::test]
    async fn test_verify_issued_application_password_other_failures(
        #[case] body: &str,
        #[case] status_code: u32,
    ) {
        let (result, _) = verify(wp_network_response_from_json(body, status_code)).await;
        assert!(
            matches!(
                result,
                Err(VerifyIssuedApplicationPasswordError::Other { .. })
            ),
            "{result:#?}"
        );
    }

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
            "{result:#?}"
        );
    }
}

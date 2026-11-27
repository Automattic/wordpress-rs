use super::{
    WpApiApplicationPasswordDetails, WpApiDetails,
    url_discovery::{
        self, API_ROOT_LINK_HEADER, ApiRootUrl, ApplicationPasswordsNotSupportedReason,
        AutoDiscoveryAttempt, AutoDiscoveryAttemptFailure, AutoDiscoveryAttemptResult,
        AutoDiscoveryAttemptSuccess, AutoDiscoveryResult, DiscoveredAuthenticationMechanism,
        FetchAndParseApiRootFailure, FindApiRootFailure, ParseApiRootFailureReason,
        ParseHomepageResult, XmlrpcDisabledReason, XmlrpcDiscoveryError, extract_rsd_url,
        is_xmlrpc_response, parse_rsd_for_xmlrpc, site_directory_url,
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
use url::Url;
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
                    .fetch_and_parse_advertised_api_root(parsed_site_url, api_root_url, context)
                    .await,
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

    /// Fetches and parses the API root a site advertised, falling back to `<site>/wp-json`
    /// when the advertised address can't be fetched or isn't an API root.
    ///
    /// The advertised address can be wrong even on a working site: a relative link is
    /// resolved against the address that was requested, because a response doesn't say where
    /// a redirect ended up, and a site can advertise a stale address. When the fallback fails
    /// to fetch or parse too, the advertised address's failure is reported. When the fallback
    /// gets past fetching and parsing, its failure is reported instead.
    async fn fetch_and_parse_advertised_api_root(
        &self,
        parsed_site_url: Arc<ParsedUrl>,
        api_root_url: ApiRootUrl,
        context: Option<Arc<RequestContext>>,
    ) -> Result<AutoDiscoveryAttemptSuccess, AutoDiscoveryAttemptFailure> {
        let failure = match self
            .fetch_and_parse_api_root(Arc::clone(&parsed_site_url), &api_root_url, context.clone())
            .await
        {
            Ok(success) => return Ok(success),
            Err(failure) => failure,
        };
        let advertised_failure = |failure| {
            AutoDiscoveryAttemptFailure::from_fetch_and_parse_api_root_failure(
                Arc::clone(&parsed_site_url),
                Arc::clone(&api_root_url.0),
                failure,
            )
        };
        if !matches!(
            failure,
            FetchAndParseApiRootFailure::FetchApiRoot { .. }
                | FetchAndParseApiRootFailure::ParseApiRoot { .. }
        ) {
            // The advertised address answered as WordPress; the failure is about the site.
            return Err(advertised_failure(failure));
        }
        let Some(root_wp_json_url) = Self::root_wp_json_url((*parsed_site_url).clone())
            .filter(|url| !same_ignoring_trailing_slash(&url.inner, &api_root_url.0.inner))
            .map(Arc::new)
        else {
            return Err(advertised_failure(failure));
        };
        match self
            .fetch_and_parse_api_root(
                Arc::clone(&parsed_site_url),
                &ApiRootUrl(Arc::clone(&root_wp_json_url)),
                context,
            )
            .await
        {
            Ok(success) => Ok(success),
            Err(
                FetchAndParseApiRootFailure::FetchApiRoot { .. }
                | FetchAndParseApiRootFailure::ParseApiRoot { .. },
            ) => Err(advertised_failure(failure)),
            Err(fallback_failure) => Err(
                AutoDiscoveryAttemptFailure::from_fetch_and_parse_api_root_failure(
                    parsed_site_url,
                    root_wp_json_url,
                    fallback_failure,
                ),
            ),
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
        // The `Link` header and the `<link>` tag resolve against the same base, so a relative
        // `wp-json/` means the same address in both.
        let homepage_url = site_directory_url(&parsed_site_url.inner);
        // First check if we can find and parse the api root from the link header
        if let Some(api_root_url) =
            self.parse_response_link_header_to_find_api_root(&response, &homepage_url)
        {
            return Ok(ApiRootUrl(api_root_url.into()));
        }
        // If we can't find the api root in the link header, we parse the HTML page to look for it
        // in the link tags
        let parse_html_result =
            ParseHomepageResult::parse_response(&response.body_as_string(), &homepage_url);
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
        homepage_url: &Url,
    ) -> Option<ParsedUrl> {
        response
            .get_link_header_relative_to(API_ROOT_LINK_HEADER, homepage_url)
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
            // `example.com/blog/` ends in an empty segment; drop it, or the result is
            // `example.com/blog//wp-json`.
            .pop_if_empty()
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
    ///
    /// A blocked header is only reported when `api_root_url` has the same origin as
    /// `credentials.site_url`, the address WordPress returned with the password. When the two
    /// differ, a redirect between them may have made the HTTP client drop the header, and
    /// WordPress answers that request the same way as one whose header the server blocked.
    pub async fn verify_issued_application_password(
        &self,
        api_root_url: Arc<ParsedUrl>,
        credentials: WpApiApplicationPasswordDetails,
        context: Option<Arc<RequestContext>>,
    ) -> Result<(), VerifyIssuedApplicationPasswordError> {
        let is_same_origin = ParsedUrl::parse(&credentials.site_url)
            .is_ok_and(|site_url| site_url.inner.origin() == api_root_url.inner.origin());
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
        // Across origins, a redirect may have dropped the header on the client, so the
        // response says nothing about the site's server.
        parsed.map_err(|error| {
            if is_same_origin {
                error.into()
            } else {
                VerifyIssuedApplicationPasswordError::Other { error }
            }
        })?;
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

        // A link that is empty, or isn't an `http(s)` address once resolved, is no better
        // than a missing one.
        let rsd_url = extract_rsd_url(
            &response.body_as_string(),
            &site_directory_url(&parsed_site_url.inner),
        )
        .ok_or(XmlrpcDiscoveryError::EndpointNotFound)?;

        let rsd_response = self
            .perform(
                WpNetworkRequest {
                    uuid: Uuid::new_v4().into(),
                    retry_count: 0,
                    method: RequestMethod::GET,
                    url: WpEndpointUrl(rsd_url.to_string()),
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

        parse_rsd_for_xmlrpc(&rsd_response.body_as_string(), &rsd_url)
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

/// Whether `a` and `b` differ at most by a trailing `/` on the path. WordPress serves the same
/// REST API root at `<site>/wp-json` and `<site>/wp-json/`.
fn same_ignoring_trailing_slash(a: &Url, b: &Url) -> bool {
    let trimmed = |url: &Url| {
        let mut url = url.clone();
        let path = url.path().trim_end_matches('/').to_string();
        url.set_path(&path);
        url
    };
    trimmed(a) == trimmed(b)
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
    const REST_NOT_LOGGED_IN_BODY: &str = r#"{"code":"rest_not_logged_in","message":"You are not currently logged in.","data":{"status":401}}"#;

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
        verify_with_site_url("https://example.com", response).await
    }

    /// Verifies against the API root `https://example.com/wp-json/`, with `site_url` as the
    /// address the callback returned.
    async fn verify_with_site_url(
        site_url: &str,
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
                    site_url: site_url.to_string(),
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
    #[case::subdirectory_install("https://example.com/wordpress")]
    #[case::explicit_default_port("https://example.com:443")]
    #[case::uppercase_host("https://EXAMPLE.com")]
    #[tokio::test]
    async fn test_verify_issued_application_password_same_origin_is_blocked(
        #[case] site_url: &str,
    ) {
        let response = wp_network_response_from_json(REST_NOT_LOGGED_IN_BODY, 401);
        let (result, _) = verify_with_site_url(site_url, response).await;
        assert!(
            matches!(
                result,
                Err(VerifyIssuedApplicationPasswordError::AuthorizationHeaderBlocked { .. })
            ),
            "{result:#?}"
        );
    }

    // A redirect from another origin makes the HTTP client drop the `Authorization` header, so
    // this response does not show that the site's server blocked it.
    #[rstest]
    #[case::different_host("https://www.example.com")]
    #[case::different_scheme("http://example.com")]
    #[case::different_port("https://example.com:8443")]
    #[case::not_a_url("")]
    #[tokio::test]
    async fn test_verify_issued_application_password_other_origin_is_not_blocked(
        #[case] site_url: &str,
    ) {
        let response = wp_network_response_from_json(REST_NOT_LOGGED_IN_BODY, 401);
        let (result, _) = verify_with_site_url(site_url, response).await;
        assert!(
            matches!(
                &result,
                Err(VerifyIssuedApplicationPasswordError::Other {
                    error: WpApiError::WpError {
                        error_code: WpErrorCode::Unauthorized,
                        status_code: 401,
                        ..
                    },
                })
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

    /// Answers each request with the body routed to its URL, or a 404 for any other URL, and
    /// records the URLs it receives.
    #[derive(Default)]
    struct RoutingExecutor {
        routes: Vec<(&'static str, String)>,
        /// The `Link` header each URL answers with.
        link_headers: Vec<(&'static str, &'static str)>,
        requested_urls: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl RequestExecutor for RoutingExecutor {
        async fn execute(
            &self,
            request: Arc<WpNetworkRequest>,
        ) -> Result<WpNetworkResponse, RequestExecutionError> {
            self.requested_urls
                .lock()
                .unwrap()
                .push(request.url.0.clone());
            let (body, status_code) = self
                .routes
                .iter()
                .find(|(url, _)| *url == request.url.0)
                .map_or(("", 404), |(_, body)| (body.as_str(), 200));
            let mut header_map = http::HeaderMap::new();
            if let Some((_, link)) = self
                .link_headers
                .iter()
                .find(|(url, _)| *url == request.url.0)
            {
                header_map.insert(http::header::LINK, http::HeaderValue::from_static(link));
            }
            Ok(WpNetworkResponse {
                request_url: request.url.clone(),
                response_header_map: Arc::new(WpNetworkHeaderMap::new(header_map)),
                ..wp_network_response_from_json(body, status_code)
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

    const RSD_URL: &str = "https://example.com/wp/xmlrpc.php?rsd";
    const XMLRPC_URL: &str = "https://example.com/wp/xmlrpc.php";

    fn homepage_with_rsd_link(href: &str) -> String {
        format!(
            r#"<html><head><link rel="EditURI" type="application/rsd+xml" title="RSD" href="{href}" /></head><body></body></html>"#
        )
    }

    fn rsd_body(api_link: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?><rsd version="1.0" xmlns="http://archipelago.phrasewise.com/rsd"><service><apis><api name="WordPress" blogID="1" preferred="true" apiLink="{api_link}" /></apis></service></rsd>"#
        )
    }

    async fn xmlrpc_from_rsd(
        site_url: &'static str,
        href: &str,
    ) -> (Result<ParsedUrl, XmlrpcDiscoveryError>, Vec<String>) {
        xmlrpc_from_rsd_on_homepage(site_url, homepage_with_rsd_link(href)).await
    }

    async fn xmlrpc_from_rsd_on_homepage(
        site_url: &'static str,
        homepage: String,
    ) -> (Result<ParsedUrl, XmlrpcDiscoveryError>, Vec<String>) {
        xmlrpc_from_rsd_with(site_url, homepage, XMLRPC_URL).await
    }

    async fn xmlrpc_from_rsd_with(
        site_url: &'static str,
        homepage: String,
        api_link: &str,
    ) -> (Result<ParsedUrl, XmlrpcDiscoveryError>, Vec<String>) {
        let executor = Arc::new(RoutingExecutor {
            routes: vec![(site_url, homepage), (RSD_URL, rsd_body(api_link))],
            ..Default::default()
        });
        let client = WpLoginClient::new_with_default_middleware_pipeline(executor.clone());
        let result = client
            .xmlrpc_from_rsd(&ParsedUrl::parse(site_url).unwrap(), None)
            .await;
        let requested_urls = executor.requested_urls.lock().unwrap().clone();
        (result, requested_urls)
    }

    #[rstest]
    #[case::absolute("https://example.com/", RSD_URL)]
    #[case::root_relative("https://example.com/", "/wp/xmlrpc.php?rsd")]
    #[case::protocol_relative("https://example.com/", "//example.com/wp/xmlrpc.php?rsd")]
    #[case::path_relative("https://example.com/", "wp/xmlrpc.php?rsd")]
    #[case::path_relative_from_subdirectory("https://example.com/wp/", "xmlrpc.php?rsd")]
    // A site address is a directory, with or without its trailing slash.
    #[case::path_relative_from_subdirectory_without_slash(
        "https://example.com/wp",
        "xmlrpc.php?rsd"
    )]
    #[case::dot_relative_from_subdirectory_without_slash(
        "https://example.com/wp",
        "./xmlrpc.php?rsd"
    )]
    #[case::surrounding_whitespace("https://example.com/", " /wp/xmlrpc.php?rsd ")]
    #[tokio::test]
    async fn test_xmlrpc_from_rsd_resolves_link_against_homepage(
        #[case] site_url: &'static str,
        #[case] href: &str,
    ) {
        let (result, requested_urls) = xmlrpc_from_rsd(site_url, href).await;
        assert_eq!(requested_urls, [site_url, RSD_URL]);
        assert_eq!(result.unwrap().url(), XMLRPC_URL);
    }

    #[rstest]
    #[case::empty("")]
    #[case::whitespace("  ")]
    #[case::unresolvable("https://[invalid/xmlrpc.php?rsd")]
    #[case::javascript("javascript:void(0)")]
    #[case::data("data:application/rsd+xml,x")]
    #[tokio::test]
    async fn test_xmlrpc_from_rsd_unusable_link_is_not_found(#[case] href: &str) {
        let (result, requested_urls) = xmlrpc_from_rsd("https://example.com/", href).await;
        // Only the homepage is fetched.
        assert_eq!(requested_urls, ["https://example.com/"]);
        assert!(
            matches!(result, Err(XmlrpcDiscoveryError::EndpointNotFound)),
            "{result:#?}"
        );
    }

    #[tokio::test]
    async fn test_xmlrpc_from_rsd_resolves_link_against_base_element() {
        let homepage = r#"<html><head><base href="/wp/"><link rel="EditURI" type="application/rsd+xml" title="RSD" href="xmlrpc.php?rsd" /></head></html>"#;
        let (result, requested_urls) =
            xmlrpc_from_rsd_on_homepage("https://example.com/", homepage.to_string()).await;
        assert_eq!(requested_urls, ["https://example.com/", RSD_URL]);
        assert_eq!(result.unwrap().url(), XMLRPC_URL);
    }

    // WordPress builds the RSD link and its `apiLink` the same way, so a site that makes one
    // relative makes both relative.
    #[rstest]
    #[case::root_relative("/wp/xmlrpc.php")]
    #[case::path_relative("xmlrpc.php")]
    #[case::protocol_relative("//example.com/wp/xmlrpc.php")]
    #[tokio::test]
    async fn test_xmlrpc_from_rsd_resolves_api_link_against_rsd_url(#[case] api_link: &str) {
        let (result, requested_urls) = xmlrpc_from_rsd_with(
            "https://example.com/wp",
            homepage_with_rsd_link("/wp/xmlrpc.php?rsd"),
            api_link,
        )
        .await;
        assert_eq!(requested_urls, ["https://example.com/wp", RSD_URL]);
        assert_eq!(result.unwrap().url(), XMLRPC_URL);
    }

    #[rstest]
    #[case::empty("")]
    #[case::javascript("javascript:void(0)")]
    #[tokio::test]
    async fn test_xmlrpc_from_rsd_unusable_api_link_is_not_found(#[case] api_link: &str) {
        let (result, _) = xmlrpc_from_rsd_with(
            "https://example.com/",
            homepage_with_rsd_link(RSD_URL),
            api_link,
        )
        .await;
        assert!(
            matches!(result, Err(XmlrpcDiscoveryError::EndpointNotFound)),
            "{result:#?}"
        );
    }

    #[tokio::test]
    async fn test_xmlrpc_from_rsd_skips_a_link_without_a_usable_href() {
        let homepage = format!(
            r#"<html><head>
            <link rel="EditURI" type="application/rsd+xml" title="RSD" />
            <link rel="EditURI" type="application/rsd+xml" title="RSD" href="" />
            <link rel="EditURI" type="application/rsd+xml" title="RSD" href="{RSD_URL}" />
            </head></html>"#
        );
        let (result, requested_urls) =
            xmlrpc_from_rsd_on_homepage("https://example.com/", homepage).await;
        assert_eq!(requested_urls, ["https://example.com/", RSD_URL]);
        assert_eq!(result.unwrap().url(), XMLRPC_URL);
    }

    /// An API root for the site at `home`, with application passwords enabled.
    fn api_root_body(home: &str) -> String {
        format!(
            r#"{{"name":"Site","description":"","url":"{home}","home":"{home}","gmt_offset":0,"timezone_string":"","namespaces":["wp/v2"],"authentication":{{"application-passwords":{{"endpoints":{{"authorization":"{home}/wp-admin/authorize-application.php"}}}}}},"routes":{{}}}}"#
        )
    }

    fn homepage_with_api_root_link(href: &str) -> String {
        format!(
            r#"<html><head><link rel="https://api.w.org/" href="{href}" /></head><body></body></html>"#
        )
    }

    /// Runs one discovery attempt for `site_url` against `routes`, and returns the API root it
    /// settled on (or the failure) and every URL it requested.
    async fn discover(
        site_url: &str,
        routes: Vec<(&'static str, String)>,
    ) -> (
        Result<AutoDiscoveryAttemptSuccess, AutoDiscoveryAttemptFailure>,
        Vec<String>,
    ) {
        discover_using(
            site_url,
            RoutingExecutor {
                routes,
                ..Default::default()
            },
        )
        .await
    }

    async fn discover_using(
        site_url: &str,
        executor: RoutingExecutor,
    ) -> (
        Result<AutoDiscoveryAttemptSuccess, AutoDiscoveryAttemptFailure>,
        Vec<String>,
    ) {
        let executor = Arc::new(executor);
        let client = WpLoginClient::new_with_default_middleware_pipeline(executor.clone());
        let result = client
            .attempt_api_discovery(
                AutoDiscoveryAttempt {
                    attempt_site_url: site_url.to_string(),
                    attempt_type: url_discovery::AutoDiscoveryAttemptType::UserInput,
                },
                None,
            )
            .await;
        let requested_urls = executor.requested_urls.lock().unwrap().clone();
        (result.api_discovery_result, requested_urls)
    }

    // A site address is a directory, so a path-relative link on `example.com/blog` stays under
    // `/blog/`, with or without the trailing slash.
    #[rstest]
    #[case::without_slash("https://example.com/blog", "wp-json/")]
    #[case::with_slash("https://example.com/blog/", "wp-json/")]
    #[case::dot_relative("https://example.com/blog", "./wp-json/")]
    #[case::root_relative("https://example.com/blog", "/blog/wp-json/")]
    #[tokio::test]
    async fn test_api_discovery_resolves_link_tag_against_site_directory(
        #[case] site_url: &'static str,
        #[case] href: &str,
    ) {
        let (result, requested_urls) = discover(
            site_url,
            vec![
                (site_url, homepage_with_api_root_link(href)),
                (
                    "https://example.com/blog/wp-json/",
                    api_root_body("https://example.com/blog"),
                ),
            ],
        )
        .await;
        assert_eq!(
            requested_urls,
            [site_url, "https://example.com/blog/wp-json/"]
        );
        assert_eq!(
            result.unwrap().api_root_url.url(),
            "https://example.com/blog/wp-json/"
        );
    }

    // A `Link` header resolves against the same base as the `<link>` tag. `example.com/wp-json/`
    // is a REST API root too, of a different site, so resolving against `example.com/blog`
    // would succeed against the wrong site.
    #[rstest]
    #[case::path_relative("<wp-json/>; rel=\"https://api.w.org/\"")]
    #[case::dot_relative("<./wp-json/>; rel=\"https://api.w.org/\"")]
    #[tokio::test]
    async fn test_api_discovery_resolves_link_header_against_site_directory(
        #[case] link_header: &'static str,
    ) {
        let (result, requested_urls) = discover_using(
            "https://example.com/blog",
            RoutingExecutor {
                routes: vec![
                    ("https://example.com/blog", String::new()),
                    (
                        "https://example.com/blog/wp-json/",
                        api_root_body("https://example.com/blog"),
                    ),
                    (
                        "https://example.com/wp-json/",
                        api_root_body("https://example.com"),
                    ),
                ],
                link_headers: vec![("https://example.com/blog", link_header)],
                ..Default::default()
            },
        )
        .await;
        assert_eq!(
            requested_urls,
            [
                "https://example.com/blog",
                "https://example.com/blog/wp-json/"
            ]
        );
        assert_eq!(
            result.unwrap().api_root_url.url(),
            "https://example.com/blog/wp-json/"
        );
    }

    #[rstest]
    // An advertised root that answers with something other than an API root.
    #[case::not_found("/wrong/")]
    // A relative link resolved against the address that was requested, when a redirect took
    // the homepage somewhere else.
    #[case::other_directory("../elsewhere/wp-json/")]
    #[tokio::test]
    async fn test_api_discovery_falls_back_to_wp_json_when_advertised_root_fails(
        #[case] href: &str,
    ) {
        let (result, requested_urls) = discover(
            "https://example.com/blog",
            vec![
                (
                    "https://example.com/blog",
                    homepage_with_api_root_link(href),
                ),
                (
                    "https://example.com/blog/wp-json",
                    api_root_body("https://example.com/blog"),
                ),
            ],
        )
        .await;
        assert_eq!(requested_urls.len(), 3, "{requested_urls:?}");
        assert_eq!(requested_urls[2], "https://example.com/blog/wp-json");
        assert_eq!(
            result.unwrap().api_root_url.url(),
            "https://example.com/blog/wp-json"
        );
    }

    #[tokio::test]
    async fn test_api_discovery_reports_advertised_root_when_fallback_fails_too() {
        let (result, requested_urls) = discover(
            "https://example.com/blog",
            vec![(
                "https://example.com/blog",
                homepage_with_api_root_link("/wrong/"),
            )],
        )
        .await;
        assert_eq!(
            requested_urls,
            [
                "https://example.com/blog",
                "https://example.com/wrong/",
                "https://example.com/blog/wp-json",
            ]
        );
        match result {
            Err(AutoDiscoveryAttemptFailure::FetchAndParseApiRoot {
                api_root_url,
                fetch_and_parse_api_root_failure: FetchAndParseApiRootFailure::ParseApiRoot { .. },
                ..
            }) => assert_eq!(api_root_url.url(), "https://example.com/wrong/"),
            other => panic!("unexpected result: {other:#?}"),
        }
    }

    // WordPress core advertises `<site>/wp-json/`, with the slash, and serves the same root
    // without it, so the fallback would request the same root again.
    #[rstest]
    #[case::without_slash("https://example.com/blog/wp-json")]
    #[case::with_slash("https://example.com/blog/wp-json/")]
    #[tokio::test]
    async fn test_api_discovery_doesnt_retry_when_advertised_root_is_wp_json(
        #[case] advertised: &str,
    ) {
        let (result, requested_urls) = discover(
            "https://example.com/blog",
            vec![(
                "https://example.com/blog",
                homepage_with_api_root_link(advertised),
            )],
        )
        .await;
        assert_eq!(requested_urls, ["https://example.com/blog", advertised]);
        assert!(result.is_err());
    }

    /// An API root for the site at `home` that offers no way to sign in.
    fn api_root_body_without_authentication(home: &str) -> String {
        format!(
            r#"{{"name":"Site","description":"","url":"{home}","home":"{home}","gmt_offset":0,"timezone_string":"","namespaces":["wp/v2"],"authentication":{{}},"routes":{{}}}}"#
        )
    }

    // An advertised root that answers as WordPress is the site's API root, so its failure is
    // reported without trying `<site>/wp-json`.
    #[tokio::test]
    async fn test_api_discovery_doesnt_fall_back_when_advertised_root_is_wordpress() {
        let (result, requested_urls) = discover(
            "https://example.com/blog",
            vec![
                (
                    "https://example.com/blog",
                    homepage_with_api_root_link("/api/"),
                ),
                (
                    "https://example.com/api/",
                    api_root_body_without_authentication("https://example.com/blog"),
                ),
                (
                    "https://example.com/blog/wp-json",
                    api_root_body("https://example.com/blog"),
                ),
            ],
        )
        .await;
        assert_eq!(
            requested_urls,
            ["https://example.com/blog", "https://example.com/api/"]
        );
        match result {
            Err(AutoDiscoveryAttemptFailure::FetchAndParseApiRoot {
                api_root_url,
                fetch_and_parse_api_root_failure:
                    FetchAndParseApiRootFailure::ApplicationPasswordsNotSupported { .. },
                ..
            }) => assert_eq!(api_root_url.url(), "https://example.com/api/"),
            other => panic!("unexpected result: {other:#?}"),
        }
    }

    // When `<site>/wp-json` answers as WordPress, its failure says more than the advertised
    // root's, so it's the one reported.
    #[tokio::test]
    async fn test_api_discovery_reports_fallback_failure_when_it_got_further() {
        let (result, requested_urls) = discover(
            "https://example.com/blog",
            vec![
                (
                    "https://example.com/blog",
                    homepage_with_api_root_link("/wrong/"),
                ),
                (
                    "https://example.com/blog/wp-json",
                    api_root_body_without_authentication("https://example.com/blog"),
                ),
            ],
        )
        .await;
        assert_eq!(
            requested_urls,
            [
                "https://example.com/blog",
                "https://example.com/wrong/",
                "https://example.com/blog/wp-json",
            ]
        );
        match result {
            Err(AutoDiscoveryAttemptFailure::FetchAndParseApiRoot {
                api_root_url,
                fetch_and_parse_api_root_failure:
                    FetchAndParseApiRootFailure::ApplicationPasswordsNotSupported { .. },
                ..
            }) => assert_eq!(api_root_url.url(), "https://example.com/blog/wp-json"),
            other => panic!("unexpected result: {other:#?}"),
        }
    }

    #[rstest]
    #[case::without_slash("https://example.com/blog", "https://example.com/blog/wp-json")]
    #[case::with_slash("https://example.com/blog/", "https://example.com/blog/wp-json")]
    #[case::root("https://example.com", "https://example.com/wp-json")]
    fn test_root_wp_json_url(#[case] site_url: &str, #[case] expected: &str) {
        let url = WpLoginClient::root_wp_json_url(ParsedUrl::parse(site_url).unwrap()).unwrap();
        assert_eq!(url.url(), expected);
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

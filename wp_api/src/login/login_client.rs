use std::str;
use std::sync::Arc;

use url::Url;

use crate::request::endpoint::{ApiBaseUrl, WpEndpointUrl};
use crate::request::{
    RequestExecutor, RequestMethod, WpNetworkHeaderMap, WpNetworkRequest, WpNetworkRequestBody,
    WpNetworkResponse,
};
use crate::{ParsedUrl, WpLoginCredentials};

use super::url_discovery::{
    self, FetchApiDetailsError, FetchApiRootUrlError, StateInitial, UrlDiscoveryAttemptError,
    UrlDiscoveryAttemptSuccess, UrlDiscoveryError, UrlDiscoveryState, UrlDiscoverySuccess,
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
        let parsed_site_url = parsed_url_state.site_url.clone();
        let state_fetched_api_root_url = self
            .fetch_api_root_url(&parsed_url_state.site_url)
            .await
            .and_then(|r| parsed_url_state.parse_api_root_response(r))
            .map_err(|e| UrlDiscoveryAttemptError::FetchApiRootUrlFailed {
                site_url: Arc::new(parsed_site_url),
                error: e,
            })?;
        match self
            .fetch_wp_api_details(&state_fetched_api_root_url.api_root_url)
            .await
        {
            Ok(r) => state_fetched_api_root_url.parse_api_details_response(r),
            Err(e) => Err(UrlDiscoveryAttemptError::FetchApiDetailsFailed {
                site_url: Arc::new(state_fetched_api_root_url.site_url),
                api_root_url: Arc::new(state_fetched_api_root_url.api_root_url),
                error: e,
            }),
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
        api_root_url: &ParsedUrl,
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

    pub(crate) async fn insert_rest_nonce(
        &self,
        request: &WpNetworkRequest,
        api_base_url: &ApiBaseUrl,
        login: &WpLoginCredentials,
    ) -> Option<WpNetworkRequest> {
        // Only attempt login if the request is to the WordPress site.
        if Url::parse(api_base_url.as_str()).ok()?.host_str()
            != Url::parse(request.url.0.as_str()).ok()?.host_str()
        {
            return None;
        }

        let nonce = self.get_rest_nonce(api_base_url, login).await?;

        let mut request = request.clone();
        let mut headers = request.header_map.as_header_map();
        headers.insert(
            http::header::HeaderName::from_bytes("X-WP-Nonce".as_bytes())
                .expect("This conversion should never fail"),
            nonce.try_into().expect("This conversion should never fail"),
        );
        request.header_map = WpNetworkHeaderMap::new(headers).into();

        Some(request)
    }

    async fn get_rest_nonce(
        &self,
        api_base_url: &ApiBaseUrl,
        login: &WpLoginCredentials,
    ) -> Option<String> {
        let rest_nonce_url = api_base_url.derived_rest_nonce_url();
        let rest_nonce_url_clone = rest_nonce_url.clone();
        let nonce_request = WpNetworkRequest {
            method: RequestMethod::GET,
            url: rest_nonce_url.into(),
            header_map: WpNetworkHeaderMap::new(http::HeaderMap::new()).into(),
            body: None,
        };

        let nonce_from_request = |request: WpNetworkRequest| async move {
            self.request_executor
                .execute(request.into())
                .await
                .ok()
                .and_then(|response| {
                    // A 200 OK response from the `rest_nonce_url` (a.k.a `wp-admin/admin-ajax.php`)
                    // should be the nonce value. However, just in case the site is configured to
                    // return a 200 OK response with other content (for example redirection to
                    // other webpage), here we check the body length here for a light validation of
                    // the nonce value.
                    if response.status_code == 200 {
                        let body = response.body_as_string();
                        if body.len() < 50 {
                            return Some(body);
                        }
                    }
                    None
                })
        };

        if let Some(nonce) = nonce_from_request(nonce_request).await {
            return Some(nonce);
        }

        let mut headers = http::HeaderMap::new();
        headers.insert(
            http::header::CONTENT_TYPE,
            "application/x-www-form-urlencoded".parse().unwrap(),
        );
        let body = serde_urlencoded::to_string([
            ["log", login.username.as_str()],
            ["pwd", login.password.as_str()],
            ["rememberme", "true"],
            ["redirect_to", rest_nonce_url_clone.to_string().as_str()],
        ])
        .unwrap();
        let login_request = WpNetworkRequest {
            method: RequestMethod::POST,
            url: api_base_url.derived_wp_login_url().into(),
            header_map: WpNetworkHeaderMap::new(headers).into(),
            body: Some(WpNetworkRequestBody::new(body.into_bytes()).into()),
        };

        nonce_from_request(login_request).await
    }
}

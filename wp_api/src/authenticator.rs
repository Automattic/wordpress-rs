use futures::lock::Mutex;
use http::header::HeaderMap;
use http::header::HeaderValue;
use std::sync::Arc;
use url::Url;

use crate::{
    request::{
        endpoint::ApiBaseUrl, RequestExecutor, RequestMethod, WpNetworkHeaderMap, WpNetworkRequest,
        WpNetworkRequestBody, WpNetworkResponse,
    },
    RequestExecutionError, WpLoginCredentials,
};

const CONTENT_TYPE_FORM: &str = "application/x-www-form-urlencoded";

#[async_trait::async_trait]
pub trait Authenticator: Send + Sync + std::fmt::Debug {
    // The host of the site to which the authenticator is associated.
    // This can be used to prevent credentials from being sent to other sites.
    fn host(&self) -> &str;

    async fn authentication_headers(
        &self,
        previous_authentication_headers: Option<&HeaderMap>,
        previous_response: Option<&WpNetworkResponse>,
    ) -> Option<HeaderMap>;
}

#[derive(Debug)]
pub(crate) struct NilAuthenticator {}

#[async_trait::async_trait]
impl Authenticator for NilAuthenticator {
    fn host(&self) -> &str {
        ""
    }

    async fn authentication_headers(
        &self,
        previous_authentication_headers: Option<&HeaderMap>,
        previous_response: Option<&WpNetworkResponse>,
    ) -> Option<HeaderMap> {
        None
    }
}

#[derive(Debug)]
pub struct ApplicationPasswordAuthenticator {
    host: String,
    token: String,
}

impl ApplicationPasswordAuthenticator {
    pub fn new(host: String, token: String) -> Self {
        Self { host, token }
    }

    pub fn with_application_password(host: String, username: String, password: String) -> Self {
        use base64::prelude::*;
        let token = BASE64_STANDARD.encode(format!("{}:{}", username, password));
        Self::new(host, token)
    }
}

#[async_trait::async_trait]
impl Authenticator for ApplicationPasswordAuthenticator {
    fn host(&self) -> &str {
        self.host.as_str()
    }

    async fn authentication_headers(
        &self,
        previous_authentication_headers: Option<&HeaderMap>,
        previous_response: Option<&WpNetworkResponse>,
    ) -> Option<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.append(
            http::header::AUTHORIZATION,
            format!("Basic {}", self.token)
                .try_into()
                .expect("token is always a valid header value"),
        );

        Some(headers)
    }
}

#[derive(Debug)]
pub(crate) struct CookieAuthenticator {
    api_base_url: ApiBaseUrl,
    credentials: WpLoginCredentials,
    request_executor: std::sync::Arc<dyn RequestExecutor>,
    nonce: Mutex<Option<String>>,
}

impl CookieAuthenticator {
    pub(crate) fn new(
        api_base_url: ApiBaseUrl,
        credentials: WpLoginCredentials,
        request_executor: std::sync::Arc<dyn RequestExecutor>,
    ) -> Self {
        Self {
            api_base_url,
            credentials,
            request_executor,
            nonce: None.into(),
        }
    }

    /// Fetches the REST nonce (wp-rest-nonce) from the site.
    ///
    /// You can pass an already known invalid nonce to force fetching a new nonce.
    async fn update_rest_nonce(&self, invalid: Option<String>) -> Option<String> {
        let mut nonce_guard = self.nonce.lock().await;

        if invalid.is_some() && invalid == *nonce_guard {
            *nonce_guard = None;
        }

        if let Some(cache) = (*nonce_guard).clone() {
            return Some(cache);
        }

        let mut fetched = self.nonce_from_request(self.nonce_request()).await;

        if fetched.is_none() {
            fetched = self
                .nonce_from_request(self.nonce_request_via_login()?)
                .await;
        }

        if let Some(fetched) = fetched {
            (*nonce_guard).replace(fetched.clone());
            return Some(fetched);
        }

        None
    }

    fn nonce_request(&self) -> WpNetworkRequest {
        WpNetworkRequest {
            method: RequestMethod::GET,
            url: self.api_base_url.derived_rest_nonce_url().into(),
            header_map: WpNetworkHeaderMap::default().into(),
            body: None,
        }
    }

    fn nonce_request_via_login(&self) -> Option<WpNetworkRequest> {
        let mut headers = http::HeaderMap::new();
        headers.insert(
            http::header::CONTENT_TYPE,
            HeaderValue::from_static(CONTENT_TYPE_FORM),
        );

        let body = serde_urlencoded::to_string([
            ["log", self.credentials.username.as_str()],
            ["pwd", self.credentials.password.as_str()],
            ["rememberme", "true"],
            [
                "redirect_to",
                self.api_base_url.derived_rest_nonce_url().as_str(),
            ],
        ])
        .ok()?;

        Some(WpNetworkRequest {
            method: RequestMethod::POST,
            url: self.api_base_url.derived_wp_login_url().into(),
            header_map: WpNetworkHeaderMap::new(headers).into(),
            body: Some(WpNetworkRequestBody::new(body.into_bytes()).into()),
        })
    }

    async fn nonce_from_request(&self, request: WpNetworkRequest) -> Option<String> {
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
    }
}

#[async_trait::async_trait]
impl Authenticator for CookieAuthenticator {
    fn host(&self) -> &str {
        self.api_base_url.host_str()
    }

    async fn authentication_headers(
        &self,
        previous_authentication_headers: Option<&HeaderMap>,
        previous_response: Option<&WpNetworkResponse>,
    ) -> Option<HeaderMap> {
        // No need to proceed if the request has been already sent and its response is not 401.
        if let Some(previous_response) = previous_response {
            if previous_response.status_code != 401 {
                return None;
            }
        }

        let used_nonce = previous_authentication_headers
            .and_then(|f| f.get("X-WP-Nonce"))
            .and_then(|f| f.to_str().ok().map(|f| f.to_string()));
        self.update_rest_nonce(used_nonce).await.map(|nonce| {
            let mut headers = HeaderMap::new();
            headers.insert(
                "X-WP-Nonce",
                nonce
                    .try_into()
                    .expect("This conversion should never fail since nonce is a short string"),
            );
            headers
        })
    }
}

#[derive(Debug)]
pub struct AuthenticatedRequestExecutor {
    authenticator: Arc<dyn Authenticator>,
    request_executor: Arc<dyn RequestExecutor>,
}

impl AuthenticatedRequestExecutor {
    pub fn new(
        authenticator: Arc<dyn Authenticator>,
        request_executor: Arc<dyn RequestExecutor>,
    ) -> Self {
        Self {
            authenticator,
            request_executor,
        }
    }
}

#[async_trait::async_trait]
impl RequestExecutor for AuthenticatedRequestExecutor {
    async fn execute(
        &self,
        request: Arc<WpNetworkRequest>,
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        let should_attempt_authentication = Url::parse(request.url.0.as_str())
            .ok()
            .map(|f| f.host_str() == Some(self.authenticator.host()))
            .unwrap_or(false);

        if !should_attempt_authentication {
            return self.request_executor.execute(request).await;
        }

        let mut original = (*request).clone();
        let auth_headers = self.authenticator.authentication_headers(None, None).await;

        // Authenticate the initial request.
        if let Some(headers) = &auth_headers {
            original.add_headers(headers);
        }

        let initial_response = self.request_executor.execute(original.into()).await;

        // Retry if the request fails due to authentication failure
        if let Ok(response) = &initial_response {
            // Only retry if the response status code is 4xx.
            if (400..500).contains(&response.status_code) {
                let mut original = (*request).clone();
                if let Some(headers) = self
                    .authenticator
                    .authentication_headers(auth_headers.as_ref(), Some(response))
                    .await
                {
                    original.add_headers(&headers);
                    return self.request_executor.execute(original.into()).await;
                }
            }
        }

        initial_response
    }
}

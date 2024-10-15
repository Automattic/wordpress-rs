use http::header::HeaderMap;
use http::header::HeaderValue;
use std::sync::{Arc, RwLock};

use crate::{
    request::{
        endpoint::ApiBaseUrl, RequestExecutor, RequestMethod, WpNetworkHeaderMap, WpNetworkRequest,
        WpNetworkRequestBody, WpNetworkResponse,
    },
    RequestExecutionError, WpLoginCredentials,
};

const CONTENT_TYPE_FORM: &str = "application/x-www-form-urlencoded";

#[derive(Debug)]
pub enum AuthenticationError {
    ReAuthenticationNotApplicable,
    IncorrectCredentials,
    RequestUrlDoesNotMatchSite,
    InvalidWebFormContent,
}

#[async_trait::async_trait]
pub trait Authenticator: Send + Sync + std::fmt::Debug {
    async fn authenticate(
        &self,
        request: &WpNetworkRequest,
    ) -> Result<HeaderMap, AuthenticationError>;

    fn should_reauthenticate(&self, response: &WpNetworkResponse) -> bool {
        response.status_code == 401
    }

    async fn re_authenticate(
        &self,
        request: &WpNetworkRequest,
        previous_response: &WpNetworkResponse,
    ) -> Result<HeaderMap, AuthenticationError>;
}

#[derive(Debug)]
pub(crate) struct NilAuthenticator {}

#[async_trait::async_trait]
impl Authenticator for NilAuthenticator {
    async fn authenticate(
        &self,
        _request: &WpNetworkRequest,
    ) -> Result<HeaderMap, AuthenticationError> {
        Ok(HeaderMap::default())
    }

    async fn re_authenticate(
        &self,
        _request: &WpNetworkRequest,
        _previous_response: &WpNetworkResponse,
    ) -> Result<HeaderMap, AuthenticationError> {
        Ok(HeaderMap::default())
    }
}

#[derive(Debug)]
pub struct ApplicationPasswordAuthenticator {
    token: String,
}

impl ApplicationPasswordAuthenticator {
    pub fn new(token: String) -> Self {
        Self { token }
    }

    pub fn with_application_password(username: String, password: String) -> Self {
        use base64::prelude::*;
        let token = BASE64_STANDARD.encode(format!("{}:{}", username, password));
        Self::new(token)
    }
}

#[async_trait::async_trait]
impl Authenticator for ApplicationPasswordAuthenticator {
    async fn authenticate(
        &self,
        request: &WpNetworkRequest,
    ) -> Result<HeaderMap, AuthenticationError> {
        let mut headers = HeaderMap::new();
        headers.append(
            http::header::AUTHORIZATION,
            format!("Basic {}", self.token)
                .try_into()
                .expect("token is always a valid header value"),
        );

        Ok(headers)
    }

    async fn re_authenticate(
        &self,
        request: &WpNetworkRequest,
        previous_response: &WpNetworkResponse,
    ) -> Result<HeaderMap, AuthenticationError> {
        Err(AuthenticationError::ReAuthenticationNotApplicable)
    }
}

#[derive(Debug)]
pub(crate) struct CookieAuthenticator {
    api_base_url: ApiBaseUrl,
    credentials: WpLoginCredentials,
    request_executor: std::sync::Arc<dyn RequestExecutor>,
    nonce: RwLock<Option<String>>,
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

    async fn get_rest_nonce(&self) -> Result<String, AuthenticationError> {
        if let Some(cache) = self.nonce.read().expect("Failed to unlock nonce").clone() {
            return Ok(cache);
        }

        let mut fetched = self.nonce_from_request(self.nonce_request()).await;

        if fetched.is_none() {
            fetched = self
                .nonce_from_request(self.nonce_request_via_login()?)
                .await;
        }

        if let Some(fetched) = fetched {
            self.nonce
                .write()
                .expect("Failed to unlock nonce")
                .replace(fetched.clone());
            return Ok(fetched);
        }

        Err(AuthenticationError::IncorrectCredentials)
    }

    fn nonce_request(&self) -> WpNetworkRequest {
        WpNetworkRequest {
            method: RequestMethod::GET,
            url: self.api_base_url.derived_rest_nonce_url().into(),
            header_map: WpNetworkHeaderMap::default().into(),
            body: None,
        }
    }

    fn nonce_request_via_login(&self) -> Result<WpNetworkRequest, AuthenticationError> {
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
        .map_err(|err| AuthenticationError::InvalidWebFormContent)?;

        Ok(WpNetworkRequest {
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
    async fn authenticate(
        &self,
        request: &WpNetworkRequest,
    ) -> Result<HeaderMap, AuthenticationError> {
        // Only attempt login if the request is to the WordPress site.
        if let (Ok(api_base_url), Ok(request_url)) = (
            url::Url::parse(self.api_base_url.as_str()),
            url::Url::parse(request.url.0.as_str()),
        ) {
            if api_base_url.host_str() != request_url.host_str() {
                return Err(AuthenticationError::RequestUrlDoesNotMatchSite);
            }
        }

        self.get_rest_nonce().await.map(|nonce| {
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

    async fn re_authenticate(
        &self,
        request: &WpNetworkRequest,
        previous_response: &WpNetworkResponse,
    ) -> Result<HeaderMap, AuthenticationError> {
        *self.nonce.write().expect("Failed to unlock nonce") = None;

        return self.authenticate(request).await;
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
        let mut original = (*request).clone();

        // Authenticate the initial request.
        match self.authenticator.authenticate(&original).await {
            Ok(headers) => {
                original.add_headers(&headers);
            }
            Err(error) => {
                // Do nothing.
                // Any authentication error will be returned later after sending the request.
            }
        }

        let initial_response = self.request_executor.execute(original.into()).await;

        // Retry if the request fails due to authentication failure
        if let Ok(response) = &initial_response {
            if self.authenticator.should_reauthenticate(response) {
                let mut original = (*request).clone();
                if let Ok(headers) = self
                    .authenticator
                    .re_authenticate(&original, response)
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

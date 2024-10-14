use std::sync::{Arc, RwLock};

use crate::{
    request::{
        endpoint::ApiBaseUrl, RequestExecutor, RequestMethod, WpNetworkHeaderMap, WpNetworkRequest,
        WpNetworkRequestBody, WpNetworkResponse,
    },
    RequestExecutionError, WpLoginCredentials,
};

#[async_trait::async_trait]
pub trait Authenticator: Send + Sync + std::fmt::Debug {
    // TODO: Use Result instead
    async fn authenticate(&self, request: &mut WpNetworkRequest) -> bool;

    async fn re_authenticate(
        &self,
        request: &mut WpNetworkRequest,
        previous_response: &WpNetworkResponse,
    ) -> bool;
}

#[derive(Debug)]
pub(crate) struct NilAuthenticator {}

#[async_trait::async_trait]
impl Authenticator for NilAuthenticator {
    async fn authenticate(&self, _request: &mut WpNetworkRequest) -> bool {
        false
    }

    async fn re_authenticate(
        &self,
        _request: &mut WpNetworkRequest,
        _previous_response: &WpNetworkResponse,
    ) -> bool {
        false
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
    async fn authenticate(&self, request: &mut WpNetworkRequest) -> bool {
        request.add_header(
            http::header::AUTHORIZATION,
            format!("Basic {}", self.token)
                .try_into()
                .expect("token is always a valid header value"),
        );

        true
    }

    async fn re_authenticate(
        &self,
        request: &mut WpNetworkRequest,
        previous_response: &WpNetworkResponse,
    ) -> bool {
        false
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

    async fn get_rest_nonce(&self) -> Option<String> {
        if let Some(nonce) = self.nonce.read().expect("Failed to unlock nonce").clone() {
            return Some(nonce);
        }

        let rest_nonce_url = self.api_base_url.derived_rest_nonce_url();
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
            self.nonce
                .write()
                .expect("Failed to unlock nonce")
                .replace(nonce.clone());
            return Some(nonce);
        }

        let mut headers = http::HeaderMap::new();
        headers.insert(
            http::header::CONTENT_TYPE,
            http::header::HeaderValue::from_str("application/x-www-form-urlencoded")
                .expect("This conversion should never fail"),
        );
        let body = serde_urlencoded::to_string([
            ["log", self.credentials.username.as_str()],
            ["pwd", self.credentials.password.as_str()],
            ["rememberme", "true"],
            ["redirect_to", rest_nonce_url_clone.to_string().as_str()],
        ])
        .unwrap();
        let login_request = WpNetworkRequest {
            method: RequestMethod::POST,
            url: self.api_base_url.derived_wp_login_url().into(),
            header_map: WpNetworkHeaderMap::new(headers).into(),
            body: Some(WpNetworkRequestBody::new(body.into_bytes()).into()),
        };

        if let Some(nonce) = nonce_from_request(login_request).await {
            self.nonce
                .write()
                .expect("Failed to unlock nonce")
                .replace(nonce.clone());
            return Some(nonce);
        }

        None
    }
}

#[async_trait::async_trait]
impl Authenticator for CookieAuthenticator {
    async fn authenticate(&self, request: &mut WpNetworkRequest) -> bool {
        // Only attempt login if the request is to the WordPress site.
        if let (Ok(api_base_url), Ok(request_url)) = (
            url::Url::parse(self.api_base_url.as_str()),
            url::Url::parse(request.url.0.as_str()),
        ) {
            if api_base_url.host_str() != request_url.host_str() {
                return false;
            }
        }

        if let Some(nonce) = self.get_rest_nonce().await {
            request.add_header(
                "X-WP-Nonce"
                    .try_into()
                    .expect("This conversion should never fail"),
                nonce.try_into().expect("This conversion should never fail"),
            );
            return true;
        }

        false
    }

    async fn re_authenticate(
        &self,
        request: &mut WpNetworkRequest,
        previous_response: &WpNetworkResponse,
    ) -> bool {
        if previous_response.status_code == 401 {
            *self.nonce.write().expect("Failed to unlock nonce") = None;

            if self.authenticate(request).await {
                return true;
            }
        }

        false
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
        let mut request = (*request).clone();
        self.authenticator.authenticate(&mut request).await;

        let result = self.request_executor.execute(request.clone().into()).await;

        if let Ok(response) = &result {
            if self
                .authenticator
                .re_authenticate(&mut request, response)
                .await
            {
                return self.request_executor.execute(request.clone().into()).await;
            }
        }

        result
    }
}

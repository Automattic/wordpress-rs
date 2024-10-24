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

#[cfg(test)]
mod test {
    use std::collections::HashSet;

    use crate::request::endpoint::WpEndpointUrl;
    use rand::{distributions::Alphanumeric, Rng};
    use rstest::*;

    use super::*;

    #[derive(Debug)]
    enum NonceRequestExecutorStrategy {
        Fail,
        Fixed { nonce: String },
        Random,
    }

    #[derive(Debug)]
    struct NonceRequestExecutor {
        strategy: NonceRequestExecutorStrategy,
    }

    #[async_trait::async_trait]
    impl RequestExecutor for NonceRequestExecutor {
        async fn execute(
            &self,
            request: Arc<WpNetworkRequest>,
        ) -> Result<WpNetworkResponse, RequestExecutionError> {
            match self.strategy {
                NonceRequestExecutorStrategy::Fail => {
                    Err(RequestExecutionError::RequestExecutionFailed {
                        status_code: Some(500),
                        reason: "Unavailable".to_string(),
                    })
                }
                NonceRequestExecutorStrategy::Fixed { ref nonce } => Ok(WpNetworkResponse {
                    status_code: 200,
                    header_map: WpNetworkHeaderMap::default().into(),
                    body: nonce.as_bytes().to_vec(),
                }),
                NonceRequestExecutorStrategy::Random => {
                    let random_nonce: String = rand::thread_rng()
                        .sample_iter(&Alphanumeric)
                        .take(8)
                        .map(char::from)
                        .collect();
                    Ok(WpNetworkResponse {
                        status_code: 200,
                        header_map: WpNetworkHeaderMap::default().into(),
                        body: random_nonce.as_bytes().to_vec(),
                    })
                }
            }
        }
    }

    fn cookie_authenticator(strategy: NonceRequestExecutorStrategy) -> CookieAuthenticator {
        CookieAuthenticator::new(
            "https://example.com".try_into().unwrap(),
            WpLoginCredentials {
                username: "admin".to_string(),
                password: "password".to_string(),
            },
            std::sync::Arc::new(NonceRequestExecutor { strategy }),
        )
    }

    fn request_with_nonce(nonce: Option<String>) -> WpNetworkRequest {
        let mut request = WpNetworkRequest {
            method: RequestMethod::GET,
            url: WpEndpointUrl("https://example.com/wp-json/wp/v2/posts".to_string()),
            header_map: WpNetworkHeaderMap::default().into(),
            body: None,
        };

        if let Some(nonce) = nonce {
            request.add_header("X-WP-Nonce".try_into().unwrap(), nonce.try_into().unwrap());
        }

        request
    }

    fn response_with_status_code(status_code: u16) -> WpNetworkResponse {
        WpNetworkResponse {
            status_code,
            header_map: WpNetworkHeaderMap::default().into(),
            body: vec![],
        }
    }

    #[tokio::test]
    async fn cookie_authenticator_nonce_failure() {
        let authenticator = cookie_authenticator(NonceRequestExecutorStrategy::Fail);
        let headers = authenticator.authentication_headers(None, None).await;
        assert!(headers.is_none());
    }

    #[tokio::test]
    #[rstest]
    #[case(500, None)]
    #[case(400, None)]
    #[case(401, Some("1a2b3c4d"))]
    async fn cookie_authenticator_401_only(
        #[case] status_code: u16,
        #[case] expected: Option<&str>,
    ) {
        let cookie_authenticator = cookie_authenticator(NonceRequestExecutorStrategy::Fixed {
            nonce: "1a2b3c4d".to_string(),
        });

        let response = response_with_status_code(status_code);

        let headers = cookie_authenticator
            .authentication_headers(None, Some(&response))
            .await;
        let nonce = headers
            .as_ref()
            .and_then(|f| f.get("X-WP-Nonce"))
            .map(|f| f.to_str().unwrap());
        assert_eq!(nonce, expected);
    }

    #[tokio::test]
    async fn cookie_authenticator_nonce_is_cached() {
        let authenticator = cookie_authenticator(NonceRequestExecutorStrategy::Random);

        let headers = authenticator.authentication_headers(None, None).await;
        let first_nonce = headers
            .as_ref()
            .and_then(|f| f.get("X-WP-Nonce"))
            .map(|f| f.to_str().unwrap());

        let headers = authenticator.authentication_headers(None, None).await;
        let second_nonce = headers
            .as_ref()
            .and_then(|f| f.get("X-WP-Nonce"))
            .map(|f| f.to_str().unwrap());

        assert!(first_nonce.is_some());
        assert!(second_nonce.is_some());
        assert_eq!(first_nonce, second_nonce);
    }

    #[tokio::test]
    async fn cookie_authenticator_nonce_is_re_fetched() {
        // Scenario:
        // 1. CookieAuthenticator is used to fetch the rest nonce.
        // 2. Later, a request that uses the rest nonce value results in a 401 response.
        // 3. CookieAuthenticator makes another fetch based on the failed request and response info.
        // Expected behavior: CookieAuthenticator re-fetches nonce upon 401 response.

        let authenticator = cookie_authenticator(NonceRequestExecutorStrategy::Random);

        let first_auth_headers = authenticator.authentication_headers(None, None).await;
        let first_nonce = first_auth_headers
            .as_ref()
            .and_then(|f| f.get("X-WP-Nonce"))
            .map(|f| f.to_str().unwrap());

        let unauthenticated_response = response_with_status_code(401);
        let second_auth_headers = authenticator
            .authentication_headers(first_auth_headers.as_ref(), Some(&unauthenticated_response))
            .await;
        let second_nonce = second_auth_headers
            .as_ref()
            .and_then(|f| f.get("X-WP-Nonce"))
            .map(|f| f.to_str().unwrap());

        assert!(first_nonce.is_some());
        assert!(second_nonce.is_some());
        assert_ne!(first_nonce, second_nonce);
    }

    #[tokio::test]
    async fn cookie_authenticator_nonce_no_duplicated_re_fetch() {
        // Scenario:
        // 1. 5 concurrent HTTP requests are made. They use an invalid nonce.
        // 2. 401 responses are returned for all of them.
        // 3. CookieAuthenticator will be used to re-authenticate and fetch a new nonce.
        // Expected behavior: there should only be one nonce-fetching request for all of them.

        let authenticator = cookie_authenticator(NonceRequestExecutorStrategy::Random);

        let request = request_with_nonce(Some("invalid".to_string()));
        let unauthenticated_response = response_with_status_code(401);

        let all_auth_headers = futures::future::join_all((0..5).map(|_| {
            authenticator.authentication_headers(
                Some(request.header_map.as_header_map()),
                Some(&unauthenticated_response),
            )
        }))
        .await;

        let nonce_values: HashSet<Option<String>> = all_auth_headers
            .iter()
            .map(|f| {
                f.as_ref()
                    .and_then(|f| f.get("X-WP-Nonce"))
                    .map(|f| f.to_str().unwrap().to_string())
            })
            .collect();
        assert_eq!(nonce_values.len(), 1);
    }
}

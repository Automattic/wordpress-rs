use std::sync::Arc;

use http::HeaderValue;
use url::Url;
use uuid::Uuid;
use wp_localization::{WpMessages, WpSupportsLocalization};
use wp_localization_macro::WpDeriveLocalizable;

use crate::{
    EmptyAppNotifier,
    login::url_discovery::AutoDiscoveryAttemptSuccess,
    prelude::*,
    request::{
        RequestMethod, WpNetworkRequestBody, endpoint::users_endpoint::UsersRequestExecutor,
    },
};

#[derive(uniffi::Object)]
pub struct WpRestNonceRetrieval {
    details: AutoDiscoveryAttemptSuccess,
    request_executor: Arc<dyn RequestExecutor>,
}

impl std::fmt::Debug for WpRestNonceRetrieval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WpRestNonceRetrieval")
            .field("details", &self.details)
            .field("request_executor", &"<dyn RequestExecutor>")
            .finish()
    }
}

#[uniffi::export]
impl WpRestNonceRetrieval {
    #[uniffi::constructor]
    pub fn new(
        details: AutoDiscoveryAttemptSuccess,
        request_executor: Arc<dyn RequestExecutor>,
    ) -> Self {
        Self {
            details,
            request_executor,
        }
    }

    pub async fn get_nonce(
        &self,
        username: String,
        password: String,
    ) -> Result<String, NonceRetrievalError> {
        // First, try to get the nonce directly. This HTTP request returns
        // a valid nonce if the underlying `request_executor` has valid cookies.
        let mut nonce = self.nonce_from_request(self.nonce_request()).await;

        // If that fails, try to log in with the provided username and password
        if nonce.is_err() {
            nonce = self
                .nonce_from_request(self.nonce_request_via_login(&username, &password))
                .await;
        }

        // Since the "cookies" part is out of our control, we need to verify that the nonce we got
        // is actually for the user we expect.
        if let Ok(nonce) = nonce.as_ref() {
            let api_url = WpOrgSiteApiUrlResolver::new(self.details.api_root_url.clone());
            let auth = WpAuthentication::Nonce {
                nonce: nonce.clone(),
            };
            let users = UsersRequestExecutor::new(
                Arc::new(api_url),
                WpApiClientDelegate {
                    auth_provider: WpAuthenticationProvider::static_with_auth(auth).into(),
                    request_executor: self.request_executor.clone(),
                    middleware_pipeline: WpApiMiddlewarePipeline::default().into(),
                    app_notifier: Arc::new(EmptyAppNotifier),
                },
            );
            let logged_in = users.retrieve_me_with_edit_context().await?.data.username;
            if logged_in != username {
                return Err(NonceRetrievalError::AlreadyLoggedIn {
                    username: logged_in,
                });
            }
        }

        nonce
    }
}

impl WpRestNonceRetrieval {
    fn derived_login_url(&self) -> Url {
        let mut url = self.details.parsed_site_url.inner.clone();
        url.path_segments_mut()
            .expect("The site url is a full URL")
            .push("wp-login.php");
        url
    }

    fn derived_rest_nonce_url(&self) -> Url {
        let mut url = self.derived_login_url();
        url.path_segments_mut()
            .expect("The site url is a full URL")
            .pop() // Remove the "wp-login.php" part
            .push("wp-admin")
            .push("admin-ajax.php");
        url.query_pairs_mut().append_pair("action", "rest-nonce");
        url
    }

    fn nonce_request(&self) -> WpNetworkRequest {
        WpNetworkRequest {
            uuid: Uuid::new_v4().into(),
            retry_count: 0,
            method: RequestMethod::GET,
            url: self.derived_rest_nonce_url().into(),
            header_map: WpNetworkHeaderMap::default().into(),
            body: None,
        }
    }

    fn nonce_request_via_login(&self, username: &str, password: &str) -> WpNetworkRequest {
        let mut headers = http::HeaderMap::new();
        headers.insert(
            http::header::CONTENT_TYPE,
            HeaderValue::from_static("application/x-www-form-urlencoded"),
        );

        let body = serde_urlencoded::to_string([
            ["log", username],
            ["pwd", password],
            ["rememberme", "false"],
            ["redirect_to", self.derived_rest_nonce_url().as_str()],
        ])
        .map(|s| WpNetworkRequestBody::new(s.into_bytes()))
        .ok();

        WpNetworkRequest {
            uuid: Uuid::new_v4().into(),
            retry_count: 0,
            method: RequestMethod::POST,
            url: self.derived_login_url().into(),
            header_map: WpNetworkHeaderMap::new(headers).into(),
            body: body.map(Into::into),
        }
    }

    async fn nonce_from_request(
        &self,
        request: WpNetworkRequest,
    ) -> Result<String, NonceRetrievalError> {
        let response = self
            .request_executor
            .execute(request.into())
            .await
            .map_err(Into::<WpApiError>::into)?;

        if response.status_code == 200 {
            let body = response.body_as_string();
            if (2..=50).contains(&body.len()) {
                return Ok(body);
            }
        }
        Err(NonceRetrievalError::UnexpectedResponse {
            status_code: response.status_code,
            body: response.body_as_string(),
        })
    }
}

#[derive(Debug, thiserror::Error, uniffi::Error, WpDeriveLocalizable)]
pub enum NonceRetrievalError {
    AlreadyLoggedIn { username: String },
    UnexpectedResponse { status_code: u32, body: String },
    ApiError { error: WpApiError },
}

impl WpSupportsLocalization for NonceRetrievalError {
    fn message_bundle(&self) -> crate::MessageBundle<'_> {
        match self {
            NonceRetrievalError::AlreadyLoggedIn { username } => {
                WpMessages::already_logged_in(username)
            }
            NonceRetrievalError::UnexpectedResponse { status_code, .. } => {
                WpMessages::invalid_http_status_code(status_code)
            }
            NonceRetrievalError::ApiError { error } => error.message_bundle(),
        }
    }
}

impl From<WpApiError> for NonceRetrievalError {
    fn from(error: WpApiError) -> Self {
        NonceRetrievalError::ApiError { error }
    }
}

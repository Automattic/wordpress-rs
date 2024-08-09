use async_trait::async_trait;
use futures::Future;
use std::sync::Arc;
use wp_api::{
    request::{
        RequestExecutor, RequestMethod, WpNetworkHeaderMap, WpNetworkRequest, WpNetworkResponse,
    },
    users::UserId,
    ParsedUrl, RequestExecutionError, WpApiClient, WpApiError, WpAuthentication, WpErrorCode,
};
use wp_cli::wp_cli_settings::WpCliSiteSettings;

pub mod fs_utils;
pub mod wp_db;

include!(concat!(env!("OUT_DIR"), "/generated_test_credentials.rs"));

pub(crate) const TEST_SITE_WP_CONTENT_PATH: &str = "/var/www/html/wp-content";

const BACKEND_ADDRESS: &str = "http://127.0.0.1:4000";
const BACKEND_PATH_RESTORE_WP_DB: &str = "/restore-wp-db";
const BACKEND_PATH_RESTORE_WP_CONTENT_PLUGINS: &str = "/restore-wp-content-plugins";
const BACKEND_PATH_SITE_SETTINGS: &str = "/wp-cli/site-settings";

// The first user is also the current user
pub const FIRST_USER_ID: UserId = UserId(1);
pub const FIRST_USER_EMAIL: &str = "test@example.com";
pub const SECOND_USER_ID: UserId = UserId(2);
pub const SECOND_USER_EMAIL: &str = "themeshaperwp+demos@gmail.com";
pub const SECOND_USER_SLUG: &str = "themedemos";
pub const HELLO_DOLLY_PLUGIN_SLUG: &str = "hello-dolly/hello";
pub const CLASSIC_EDITOR_PLUGIN_SLUG: &str = "classic-editor/classic-editor";
pub const WP_ORG_PLUGIN_SLUG_CLASSIC_WIDGETS: &str = "classic-widgets";

pub fn api_client() -> WpApiClient {
    let authentication = WpAuthentication::from_username_and_password(
        TEST_CREDENTIALS_ADMIN_USERNAME.to_string(),
        TEST_CREDENTIALS_ADMIN_PASSWORD.to_string(),
    );
    WpApiClient::new(
        test_site_url(),
        authentication,
        Arc::new(AsyncWpNetworking::default()),
    )
}

pub fn api_client_as_subscriber() -> WpApiClient {
    let authentication = WpAuthentication::from_username_and_password(
        TEST_CREDENTIALS_SUBSCRIBER_USERNAME.to_string(),
        TEST_CREDENTIALS_SUBSCRIBER_PASSWORD.to_string(),
    );
    WpApiClient::new(
        test_site_url(),
        authentication,
        Arc::new(AsyncWpNetworking::default()),
    )
}

pub fn api_client_as_unauthenticated() -> WpApiClient {
    WpApiClient::new(
        test_site_url(),
        WpAuthentication::None,
        Arc::new(AsyncWpNetworking::default()),
    )
}

pub fn test_site_url() -> Arc<ParsedUrl> {
    ParsedUrl::parse(TEST_CREDENTIALS_SITE_URL)
        .expect("Site url is generated by our tooling")
        .into()
}

pub trait AssertWpError<T: std::fmt::Debug> {
    fn assert_wp_error(self, expected_error_code: WpErrorCode);
}

impl<T: std::fmt::Debug> AssertWpError<T> for Result<T, WpApiError> {
    fn assert_wp_error(self, expected_error_code: WpErrorCode) {
        let err = self.unwrap_err();
        if let WpApiError::WpError {
            error_code,
            response,
            ..
        } = err
        {
            assert_eq!(
                expected_error_code, error_code,
                "Incorrect error code. Expected '{:?}', found '{:?}'. Response was: '{:?}'",
                expected_error_code, error_code, response
            );
        } else {
            panic!("Unexpected wp_error '{:?}'", err);
        }
    }
}

pub async fn run_and_restore_wp_db<F, Fut>(f: F)
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = ()>,
{
    f().await;
    let _ = BackendSupport::default().restore_wp_db().await;
}

pub async fn run_and_restore_wp_content_plugins<F, Fut>(f: F)
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = ()>,
{
    f().await;
    let _ = BackendSupport::default().restore_wp_content_plugins().await;
}

#[derive(Debug)]
pub struct AsyncWpNetworking {
    client: reqwest::Client,
}

impl Default for AsyncWpNetworking {
    fn default() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl AsyncWpNetworking {
    pub async fn async_request(
        &self,
        wp_request: Arc<WpNetworkRequest>,
    ) -> Result<WpNetworkResponse, reqwest::Error> {
        let mut request = self
            .client
            .request(
                Self::request_method(wp_request.method()),
                wp_request.url().0.as_str(),
            )
            .headers(wp_request.header_map().as_header_map());
        if let Some(body) = wp_request.body() {
            request = request.body(body.contents());
        }
        let mut response = request.send().await?;

        let header_map = std::mem::take(response.headers_mut());
        Ok(WpNetworkResponse {
            status_code: response.status().as_u16(),
            body: response.bytes().await.unwrap().to_vec(),
            header_map: Arc::new(WpNetworkHeaderMap::new(header_map)),
        })
    }

    fn request_method(method: RequestMethod) -> http::Method {
        match method {
            RequestMethod::GET => reqwest::Method::GET,
            RequestMethod::POST => reqwest::Method::POST,
            RequestMethod::PUT => reqwest::Method::PUT,
            RequestMethod::DELETE => reqwest::Method::DELETE,
            RequestMethod::HEAD => reqwest::Method::HEAD,
        }
    }
}
#[async_trait]
impl RequestExecutor for AsyncWpNetworking {
    async fn execute(
        &self,
        request: Arc<WpNetworkRequest>,
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        self.async_request(request).await.map_err(|err| {
            RequestExecutionError::RequestExecutionFailed {
                status_code: err.status().map(|s| s.as_u16()),
                reason: err.to_string(),
            }
        })
    }
}

pub trait AssertResponse {
    type Item;

    fn assert_response(self) -> Self::Item;
}

impl<T: std::fmt::Debug, E: std::error::Error> AssertResponse for Result<T, E> {
    type Item = T;

    fn assert_response(self) -> T {
        assert!(
            self.is_ok(),
            "Request failed with: {:#?}",
            self.unwrap_err()
        );
        self.unwrap()
    }
}

#[derive(Debug)]
pub struct BackendSupport {
    client: reqwest::Client,
}

impl Default for BackendSupport {
    fn default() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl BackendSupport {
    pub async fn restore_wp_db(&self) -> Result<reqwest::Response, reqwest::Error> {
        self.client
            .get(format!("{}{}", BACKEND_ADDRESS, BACKEND_PATH_RESTORE_WP_DB))
            .send()
            .await
    }
    pub async fn restore_wp_content_plugins(&self) -> Result<reqwest::Response, reqwest::Error> {
        self.client
            .get(format!(
                "{}{}",
                BACKEND_ADDRESS, BACKEND_PATH_RESTORE_WP_CONTENT_PLUGINS
            ))
            .send()
            .await
    }
    pub async fn site_settings(&self) -> Result<WpCliSiteSettings, reqwest::Error> {
        self.client
            .get(format!("{}{}", BACKEND_ADDRESS, BACKEND_PATH_SITE_SETTINGS))
            .send()
            .await?
            .json()
            .await
    }
}

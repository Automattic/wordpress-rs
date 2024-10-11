use async_trait::async_trait;
use jetpack_api::{request::JetpackNetworkResponse, JetpackRequestExecutionError};
use std::sync::Arc;
use wp_api::{
    posts::{CategoryId, MediaId, PostId, TagId},
    request::{
        RequestExecutor, RequestMethod, WpNetworkHeaderMap, WpNetworkRequest, WpNetworkResponse,
    },
    users::UserId,
    ParsedUrl, RequestExecutionError, WpApiClient, WpApiError, WpAuthentication, WpErrorCode,
};

// A `TestCredentials::instance()` function will be generated by this
include!(concat!(env!("OUT_DIR"), "/generated_test_credentials.rs"));

#[derive(Debug, Default)]
pub struct TestCredentials {
    pub site_url: &'static str,
    pub admin_username: &'static str,
    pub admin_password: &'static str,
    pub admin_password_uuid: &'static str,
    pub subscriber_username: &'static str,
    pub subscriber_password: &'static str,
    pub subscriber_password_uuid: &'static str,
    pub author_username: &'static str,
    pub author_password: &'static str,
    pub password_protected_post_id: i32,
    pub password_protected_post_password: &'static str,
    pub password_protected_post_title: &'static str,
    pub trashed_post_id: i32,
}

pub mod backend;

// The first user is also the current user
pub const FIRST_USER_ID: UserId = UserId(1);
pub const FIRST_USER_EMAIL: &str = "test@example.com";
pub const SECOND_USER_ID: UserId = UserId(2);
pub const SECOND_USER_EMAIL: &str = "themeshaperwp+demos@gmail.com";
pub const SECOND_USER_SLUG: &str = "themedemos";
pub const HELLO_DOLLY_PLUGIN_SLUG: &str = "hello-dolly/hello";
pub const CLASSIC_EDITOR_PLUGIN_SLUG: &str = "classic-editor/classic-editor";
pub const WP_ORG_PLUGIN_SLUG_CLASSIC_WIDGETS: &str = "classic-widgets";
pub const FIRST_POST_ID: PostId = PostId(1);
pub const MEDIA_ID_611: MediaId = MediaId(611);
pub const CATEGORY_ID_1: CategoryId = CategoryId(1);
pub const TAG_ID_100: TagId = TagId(100);
pub const POST_TEMPLATE_SINGLE_WITH_SIDEBAR: &str = "single-with-sidebar";

pub fn api_client() -> WpApiClient {
    let authentication = WpAuthentication::from_username_and_password(
        TestCredentials::instance().admin_username.to_string(),
        TestCredentials::instance().admin_password.to_string(),
    );
    WpApiClient::new(
        test_site_url(),
        authentication,
        Arc::new(AsyncWpNetworking::default()),
    )
}

pub fn api_client_as_author() -> WpApiClient {
    let authentication = WpAuthentication::from_username_and_password(
        TestCredentials::instance().author_username.to_string(),
        TestCredentials::instance().author_password.to_string(),
    );
    WpApiClient::new(
        test_site_url(),
        authentication,
        Arc::new(AsyncWpNetworking::default()),
    )
}

pub fn api_client_as_subscriber() -> WpApiClient {
    let authentication = WpAuthentication::from_username_and_password(
        TestCredentials::instance().subscriber_username.to_string(),
        TestCredentials::instance().subscriber_password.to_string(),
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
    ParsedUrl::parse(TestCredentials::instance().site_url)
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

#[async_trait]
impl jetpack_api::request::JetpackRequestExecutor for AsyncWpNetworking {
    async fn execute(
        &self,
        request: Arc<WpNetworkRequest>,
    ) -> Result<JetpackNetworkResponse, JetpackRequestExecutionError> {
        self.async_request(request)
            .await
            .map_err(|err| JetpackRequestExecutionError::RequestExecutionFailed {
                status_code: err.status().map(|s| s.as_u16()),
                reason: err.to_string(),
            })
            .map(JetpackNetworkResponse::from)
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

use async_trait::async_trait;
use futures::Future;
use http::HeaderMap;
use std::{fs::read_to_string, process::Command, sync::Arc};
use wp_api::{
    request::{
        endpoint::ApiBaseUrl, RequestExecutor, RequestMethod, WpNetworkRequest, WpNetworkResponse,
    },
    users::UserId,
    RequestExecutionError, WpApiError, WpAuthentication, WpRequestBuilder, WpRestError,
    WpRestErrorCode, WpRestErrorWrapper,
};

// The first user is also the current user
pub const FIRST_USER_ID: UserId = UserId(1);
pub const SECOND_USER_ID: UserId = UserId(2);
pub const SECOND_USER_EMAIL: &str = "themeshaperwp+demos@gmail.com";
pub const SECOND_USER_SLUG: &str = "themedemos";
pub const HELLO_DOLLY_PLUGIN_SLUG: &str = "hello-dolly/hello";
pub const CLASSIC_EDITOR_PLUGIN_SLUG: &str = "classic-editor/classic-editor";
pub const WP_ORG_PLUGIN_SLUG_CLASSIC_WIDGETS: &str = "classic-widgets";

pub fn request_builder() -> WpRequestBuilder {
    let credentials = read_test_credentials_from_file();
    let authentication = WpAuthentication::from_username_and_password(
        credentials.admin_username,
        credentials.admin_password,
    );
    WpRequestBuilder::new(
        credentials.site_url,
        authentication,
        Arc::new(AsyncWpNetworking::default()),
    )
}

pub fn request_builder_as_subscriber() -> WpRequestBuilder {
    let credentials = read_test_credentials_from_file();
    let authentication = WpAuthentication::from_username_and_password(
        credentials.subscriber_username,
        credentials.subscriber_password,
    );
    WpRequestBuilder::new(
        credentials.site_url,
        authentication,
        Arc::new(AsyncWpNetworking::default()),
    )
}

pub trait AssertWpError<T: std::fmt::Debug> {
    fn assert_wp_error(self, expected_error_code: WpRestErrorCode);
}

impl<T: std::fmt::Debug> AssertWpError<T> for Result<T, WpApiError> {
    fn assert_wp_error(self, expected_error_code: WpRestErrorCode) {
        let expected_status_code =
            expected_status_code_for_wp_rest_error_code(&expected_error_code);
        let err = self.unwrap_err();
        if let WpApiError::RestError {
            rest_error:
                WpRestErrorWrapper::Recognized(WpRestError {
                    code: error_code,
                    message: _,
                }),
            status_code,
            response,
        } = err
        {
            assert_eq!(
                expected_error_code, error_code,
                "Incorrect error code. Expected '{:?}', found '{:?}'. Response was: '{:?}'",
                expected_error_code, error_code, response
            );
            assert_eq!(
                expected_status_code, status_code,
                "Incorrect status code. Expected '{:?}', found '{:?}'. Response was: '{:?}'",
                expected_status_code, status_code, response
            );
        } else if let WpApiError::RestError {
            rest_error: WpRestErrorWrapper::Unrecognized(unrecognized_error),
            status_code,
            response,
        } = err
        {
            panic!(
                "Received unhandled WpRestError variant: '{:?}' with status_code: '{}'. Response was: '{:?}'",
                unrecognized_error, status_code, response
            );
        } else {
            panic!("Unexpected wp_error '{:?}'", err);
        }
    }
}

#[derive(Debug)]
pub struct TestCredentials {
    pub site_url: ApiBaseUrl,
    pub admin_username: String,
    pub admin_password: String,
    pub subscriber_username: String,
    pub subscriber_password: String,
}

pub fn read_test_credentials_from_file() -> TestCredentials {
    let file_contents = read_to_string("../test_credentials").unwrap();
    let lines: Vec<&str> = file_contents.lines().collect();
    TestCredentials {
        site_url: lines[0].try_into().unwrap(),
        admin_username: lines[1].to_string(),
        admin_password: lines[2].to_string(),
        subscriber_username: lines[3].to_string(),
        subscriber_password: lines[4].to_string(),
    }
}

fn expected_status_code_for_wp_rest_error_code(error_code: &WpRestErrorCode) -> u16 {
    match error_code {
        WpRestErrorCode::CannotActivatePlugin => 403,
        WpRestErrorCode::CannotCreateUser => 403,
        WpRestErrorCode::CannotDeactivatePlugin => 403,
        WpRestErrorCode::CannotDeleteActivePlugin => 400,
        WpRestErrorCode::CannotEdit => 403,
        WpRestErrorCode::CannotEditRoles => 403,
        WpRestErrorCode::CannotInstallPlugin => 403,
        WpRestErrorCode::CannotManageNetworkPlugins => 403,
        WpRestErrorCode::CannotManagePlugins => 403,
        WpRestErrorCode::CannotViewPlugin => 403,
        WpRestErrorCode::CannotViewPlugins => 403,
        WpRestErrorCode::ForbiddenContext => 403,
        WpRestErrorCode::ForbiddenOrderBy => 403,
        WpRestErrorCode::ForbiddenWho => 403,
        WpRestErrorCode::NetworkOnlyPlugin => 400,
        WpRestErrorCode::PluginNotFound => 404,
        WpRestErrorCode::InvalidParam => 400,
        WpRestErrorCode::TrashNotSupported => 501,
        WpRestErrorCode::Unauthorized => 401,
        WpRestErrorCode::UserCannotDelete => 403,
        WpRestErrorCode::UserCannotView => 403,
        WpRestErrorCode::UserCreate => 500,
        WpRestErrorCode::UserExists => 400,
        WpRestErrorCode::UserInvalidArgument => 400,
        WpRestErrorCode::UserInvalidEmail => 400,
        WpRestErrorCode::UserInvalidId => 404,
        WpRestErrorCode::UserInvalidPassword => 400,
        WpRestErrorCode::UserInvalidReassign => 400,
        WpRestErrorCode::UserInvalidRole => 400,
        WpRestErrorCode::UserInvalidSlug => 400,
        WpRestErrorCode::UserInvalidUsername => 400,
    }
}

pub async fn run_and_restore_wp_content_plugins<F, Fut>(f: F)
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = ()>,
{
    f().await;
    println!("Restoring wp-content/plugins..");
    Command::new("make")
        .arg("-C")
        .arg("../")
        .arg("restore-wp-content-plugins")
        .status()
        .expect("Failed to restore wp-content/plugins");
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
        wp_request: WpNetworkRequest,
    ) -> Result<WpNetworkResponse, reqwest::Error> {
        let request_headers: HeaderMap = (&wp_request.header_map).try_into().unwrap();

        let mut request = self
            .client
            .request(Self::request_method(wp_request.method), wp_request.url.0)
            .headers(request_headers);
        if let Some(body) = wp_request.body {
            request = request.body(body);
        }
        let response = request.send().await?;

        Ok(WpNetworkResponse {
            status_code: response.status().as_u16(),
            body: response.bytes().await.unwrap().to_vec(),
            header_map: None, // TODO: Properly read the headers
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
        request: WpNetworkRequest,
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

impl<T: std::fmt::Debug> AssertResponse for Result<T, WpApiError> {
    type Item = T;

    fn assert_response(self) -> T {
        assert!(self.is_ok(), "Response was: '{:?}'", self);
        self.unwrap()
    }
}

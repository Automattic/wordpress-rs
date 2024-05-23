use futures::Future;
use http::HeaderMap;
use std::{fs::read_to_string, process::Command};
use wp_api::{
    request::{WPNetworkRequest, WPNetworkResponse},
    users::UserId,
    WPApiError, WPApiHelper, WPAuthentication, WPRestError, WPRestErrorCode, WPRestErrorWrapper,
};

// The first user is also the current user
pub const FIRST_USER_ID: UserId = UserId(1);
pub const SECOND_USER_ID: UserId = UserId(2);
pub const SECOND_USER_EMAIL: &str = "themeshaperwp+demos@gmail.com";
pub const SECOND_USER_SLUG: &str = "themedemos";
pub const HELLO_DOLLY_PLUGIN_SLUG: &str = "hello-dolly/hello";
pub const CLASSIC_EDITOR_PLUGIN_SLUG: &str = "classic-editor/classic-editor";
pub const WP_ORG_PLUGIN_SLUG_CLASSIC_WIDGETS: &str = "classic-widgets";

pub fn api() -> WPApiHelper {
    let credentials = read_test_credentials_from_file();
    let authentication = WPAuthentication::from_username_and_password(
        credentials.admin_username,
        credentials.admin_password,
    );
    WPApiHelper::new(credentials.site_url, authentication)
}

pub fn api_as_subscriber() -> WPApiHelper {
    let credentials = read_test_credentials_from_file();
    let authentication = WPAuthentication::from_username_and_password(
        credentials.subscriber_username,
        credentials.subscriber_password,
    );
    WPApiHelper::new(credentials.site_url, authentication)
}

pub trait WPNetworkRequestExecutor {
    fn execute(
        self,
    ) -> impl std::future::Future<Output = Result<WPNetworkResponse, reqwest::Error>> + Send;
}

impl WPNetworkRequestExecutor for WPNetworkRequest {
    async fn execute(self) -> Result<WPNetworkResponse, reqwest::Error> {
        AsyncWPNetworking::default().async_request(self).await
    }
}

pub trait WPNetworkResponseParser {
    fn parse<F, T>(&self, parser: F) -> Result<T, WPApiError>
    where
        F: Fn(&WPNetworkResponse) -> Result<T, WPApiError>;
}

impl WPNetworkResponseParser for WPNetworkResponse {
    fn parse<F, T>(&self, parser: F) -> Result<T, WPApiError>
    where
        F: Fn(&WPNetworkResponse) -> Result<T, WPApiError>,
    {
        parser(self)
    }
}

pub trait AssertWpError<T: std::fmt::Debug> {
    fn assert_wp_error(self, expected_error_code: WPRestErrorCode);
}

impl<T: std::fmt::Debug> AssertWpError<T> for Result<T, WPApiError> {
    fn assert_wp_error(self, expected_error_code: WPRestErrorCode) {
        let expected_status_code =
            expected_status_code_for_wp_rest_error_code(&expected_error_code);
        let err = self.unwrap_err();
        if let WPApiError::RestError {
            rest_error:
                WPRestErrorWrapper::Recognized(WPRestError {
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
        } else if let WPApiError::RestError {
            rest_error: WPRestErrorWrapper::Unrecognized(unrecognized_error),
            status_code,
            response,
        } = err
        {
            panic!(
                "Received unhandled WPRestError variant: '{:?}' with status_code: '{}'. Response was: '{:?}'",
                unrecognized_error, status_code, response
            );
        } else {
            panic!("Unexpected wp_error '{:?}'", err);
        }
    }
}

pub struct TestCredentials {
    pub site_url: String,
    pub admin_username: String,
    pub admin_password: String,
    pub subscriber_username: String,
    pub subscriber_password: String,
}

pub fn read_test_credentials_from_file() -> TestCredentials {
    let file_contents = read_to_string("../test_credentials").unwrap();
    let lines: Vec<&str> = file_contents.lines().collect();
    TestCredentials {
        site_url: lines[0].to_string(),
        admin_username: lines[1].to_string(),
        admin_password: lines[2].to_string(),
        subscriber_username: lines[3].to_string(),
        subscriber_password: lines[4].to_string(),
    }
}

fn expected_status_code_for_wp_rest_error_code(error_code: &WPRestErrorCode) -> u16 {
    match error_code {
        WPRestErrorCode::CannotActivatePlugin => 403,
        WPRestErrorCode::CannotCreateUser => 403,
        WPRestErrorCode::CannotDeactivatePlugin => 403,
        WPRestErrorCode::CannotDeleteActivePlugin => 400,
        WPRestErrorCode::CannotEdit => 403,
        WPRestErrorCode::CannotEditRoles => 403,
        WPRestErrorCode::CannotInstallPlugin => 403,
        WPRestErrorCode::CannotManageNetworkPlugins => 403,
        WPRestErrorCode::CannotManagePlugins => 403,
        WPRestErrorCode::CannotViewPlugin => 403,
        WPRestErrorCode::CannotViewPlugins => 403,
        WPRestErrorCode::ForbiddenContext => 403,
        WPRestErrorCode::ForbiddenOrderBy => 403,
        WPRestErrorCode::ForbiddenWho => 403,
        WPRestErrorCode::NetworkOnlyPlugin => 400,
        WPRestErrorCode::PluginNotFound => 404,
        WPRestErrorCode::InvalidParam => 400,
        WPRestErrorCode::TrashNotSupported => 501,
        WPRestErrorCode::Unauthorized => 401,
        WPRestErrorCode::UserCannotDelete => 403,
        WPRestErrorCode::UserCannotView => 403,
        WPRestErrorCode::UserCreate => 500,
        WPRestErrorCode::UserExists => 400,
        WPRestErrorCode::UserInvalidArgument => 400,
        WPRestErrorCode::UserInvalidEmail => 400,
        WPRestErrorCode::UserInvalidId => 404,
        WPRestErrorCode::UserInvalidPassword => 400,
        WPRestErrorCode::UserInvalidReassign => 400,
        WPRestErrorCode::UserInvalidRole => 400,
        WPRestErrorCode::UserInvalidSlug => 400,
        WPRestErrorCode::UserInvalidUsername => 400,
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

pub struct AsyncWPNetworking {
    client: reqwest::Client,
}

impl Default for AsyncWPNetworking {
    fn default() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl AsyncWPNetworking {
    pub async fn async_request(
        &self,
        wp_request: WPNetworkRequest,
    ) -> Result<WPNetworkResponse, reqwest::Error> {
        let request_headers: HeaderMap = (&wp_request.header_map).try_into().unwrap();

        let mut request = self
            .client
            .request(Self::request_method(wp_request.method), wp_request.url)
            .headers(request_headers);
        if let Some(body) = wp_request.body {
            request = request.body(body);
        }
        let response = request.send().await?;

        Ok(WPNetworkResponse {
            status_code: response.status().as_u16(),
            body: response.bytes().await.unwrap().to_vec(),
            header_map: None, // TODO: Properly read the headers
        })
    }

    fn request_method(method: wp_api::RequestMethod) -> http::Method {
        match method {
            wp_api::RequestMethod::GET => reqwest::Method::GET,
            wp_api::RequestMethod::POST => reqwest::Method::POST,
            wp_api::RequestMethod::PUT => reqwest::Method::PUT,
            wp_api::RequestMethod::DELETE => reqwest::Method::DELETE,
            wp_api::RequestMethod::HEAD => reqwest::Method::HEAD,
        }
    }
}

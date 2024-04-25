use base64::prelude::*;
use std::fs::read_to_string;
use wp_api::{
    UserId, WPApiError, WPApiHelper, WPAuthentication, WPNetworkRequest, WPNetworkResponse,
    WPRestError, WPRestErrorCode,
};

use wp_networking::AsyncWPNetworking;

// The first user is also the current user
pub const FIRST_USER_ID: UserId = UserId(1);
pub const SECOND_USER_ID: UserId = UserId(2);

pub fn api() -> WPApiHelper {
    let credentials = test_credentials();
    let auth_base64_token = BASE64_STANDARD.encode(format!(
        "{}:{}",
        credentials.admin_username, credentials.admin_password
    ));
    let authentication = WPAuthentication::AuthorizationHeader {
        token: auth_base64_token,
    };
    WPApiHelper::new(credentials.site_url, authentication)
}

pub fn api_as_subscriber() -> WPApiHelper {
    let credentials = test_credentials();
    let auth_base64_token = BASE64_STANDARD.encode(format!(
        "{}:{}",
        credentials.subscriber_username, credentials.subscriber_password
    ));
    let authentication = WPAuthentication::AuthorizationHeader {
        token: auth_base64_token,
    };
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
        let err = self.unwrap_err();
        if let WPApiError::ClientError {
            rest_error: Some(WPRestError { code: error_code }),
            error_type: _,
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
                expected_error_code.status_code(),
                status_code,
                "Incorrect status code. Expected '{:?}', found '{:?}'. Response was: '{:?}'",
                expected_error_code.status_code(),
                status_code,
                response
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

pub fn test_credentials() -> TestCredentials {
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

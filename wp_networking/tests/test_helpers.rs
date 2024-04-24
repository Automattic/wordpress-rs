use base64::prelude::*;
use std::fs::read_to_string;
use wp_api::{
    UserId, WPApiError, WPApiHelper, WPAuthentication, WPCodedError, WPErrorCode, WPNetworkRequest,
    WPNetworkResponse,
};

use wp_networking::AsyncWPNetworking;

// The first user is also the current user
pub const FIRST_USER_ID: UserId = UserId(1);
pub const SECOND_USER_ID: UserId = UserId(2);

pub fn api() -> WPApiHelper {
    let (site_url, username, password) = test_credentials();
    let auth_base64_token = BASE64_STANDARD.encode(format!("{}:{}", username, password));
    let authentication = WPAuthentication::AuthorizationHeader {
        token: auth_base64_token,
    };

    WPApiHelper::new(site_url, authentication)
}

pub fn test_credentials() -> (String, String, String) {
    let file_contents = read_to_string("../test_credentials").unwrap();
    let lines: Vec<&str> = file_contents.lines().collect();
    (
        lines[0].to_string(),
        lines[1].to_string(),
        lines[2].to_string(),
    )
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
    fn assert_wp_error(self, expected_error_code: WPErrorCode, expected_status_code: u16);
}

impl<T: std::fmt::Debug> AssertWpError<T> for Result<T, WPApiError> {
    fn assert_wp_error(self, expected_error_code: WPErrorCode, expected_status_code: u16) {
        let err = self.unwrap_err();
        if let WPApiError::ClientError {
            coded_error: Some(WPCodedError { code: error_code }),
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
                expected_status_code, status_code,
                "Incorrect status code. Expected '{:?}', found '{:?}'. Response was: '{:?}'",
                expected_status_code, status_code, response
            );
        } else {
            panic!("Unexpected wp_error");
        }
    }
}

use http::StatusCode;
use serde::Deserialize;

#[derive(Debug, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum WPApiError {
    #[error(
        "Client error with type '{:?}' and status_code '{}'",
        error_type,
        status_code
    )]
    ClientError {
        coded_error: Option<WPCodedError>,
        error_type: ClientErrorType,
        status_code: u16,
        response: String,
    },
    #[error(
        "Server error with status_code \nStatus Code: {}\nResponse: {}",
        status_code,
        response
    )]
    ServerError { status_code: u16, response: String },
    #[error("Error while parsing. \nReason: {}\nResponse: {}", reason, response)]
    ParsingError { reason: String, response: String },
    #[error("Error that's not yet handled by the library")]
    UnknownError,
}

#[derive(Debug, PartialEq, Eq, uniffi::Enum)]
pub enum ClientErrorType {
    BadRequest,
    TooManyRequests,
    Other,
}

impl ClientErrorType {
    pub fn from_status_code(status_code: u16) -> Option<Self> {
        if let Ok(status_code) = StatusCode::from_u16(status_code) {
            if status_code.is_client_error() {
                if status_code == StatusCode::BAD_REQUEST {
                    Some(Self::BadRequest)
                } else if status_code == StatusCode::TOO_MANY_REQUESTS {
                    Some(Self::TooManyRequests)
                } else {
                    Some(Self::Other)
                }
            } else {
                None
            }
        } else {
            None
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, uniffi::Record)]
pub struct WPCodedError {
    pub code: WPErrorCode,
}

#[derive(Debug, Deserialize, PartialEq, Eq, uniffi::Error)]
pub enum WPErrorCode {
    #[serde(rename = "rest_user_invalid_id")]
    UserInvalidId,
    #[serde(rename = "rest_user_invalid_reassign")]
    UserInvalidReassign,
    #[serde(rename = "rest_not_logged_in")]
    Unauthorized,
}

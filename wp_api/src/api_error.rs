use http::StatusCode;
use serde::*;

use crate::WPNetworkResponse;

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum WPApiError {
    #[error("Endpoint error with code '{}'", error.code)]
    EndpointError {
        status_code: u16,
        error: WPRestError,
    },
    #[error("Unacceptable status code: {}\n", response.status_code)]
    UnacceptableStatusCodeError { response: WPNetworkResponse },
    #[error("Error while parsing. \nReason: {}\n", reason)]
    ParsingError {
        reason: String,
        response: WPNetworkResponse,
    },
    #[error("Error that's not yet handled by the library")]
    UnknownError { response: WPNetworkResponse },
}

#[derive(Debug, uniffi::Enum)]
pub enum ClientErrorType {
    BadRequest,
    Unauthorized,
    TooManyRequests,
    Other,
}

impl ClientErrorType {
    pub fn from_status_code(status_code: u16) -> Option<Self> {
        if let Ok(status_code) = StatusCode::from_u16(status_code) {
            if status_code.is_client_error() {
                if status_code == StatusCode::BAD_REQUEST {
                    Some(Self::BadRequest)
                } else if status_code == StatusCode::UNAUTHORIZED {
                    Some(Self::Unauthorized)
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

#[derive(Debug, uniffi::Record)]
pub struct WPRestError {
    pub code: String,
    pub message: String,
    pub data_json: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WPRestErrorInternal {
    code: String,
    message: String,
    data: Option<serde_json::Value>,
}

impl WPRestError {
    pub fn from_slice(body: &[u8]) -> Option<Self> {
        serde_json::from_slice(body)
            .map(|f: WPRestErrorInternal| f.into())
            .ok()
    }

    pub fn from_json_str(body: &str) -> Option<Self> {
        serde_json::from_str(body)
            .map(|f: WPRestErrorInternal| f.into())
            .ok()
    }
}

impl From<WPRestErrorInternal> for WPRestError {
    fn from(internal: WPRestErrorInternal) -> Self {
        Self {
            code: internal.code,
            message: internal.message,
            data_json: serde_json::to_string(&internal.data).ok(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rest_error() {
        let response_body = r#"{"code":"rest_post_invalid_page_number","message":"The page number requested is larger than the number of pages available.","data":{"status":400}}"#;
        let rest_error: Option<WPRestError> = WPRestError::from_json_str(response_body);
        assert!(rest_error.is_some());

        let unwrapped = rest_error.unwrap();
        assert_eq!(unwrapped.code, "rest_post_invalid_page_number");
        assert_eq!(
            unwrapped.message,
            "The page number requested is larger than the number of pages available."
        );
        assert_eq!(unwrapped.data_json, Some(r#"{"status":400}"#.to_string()));
    }
}

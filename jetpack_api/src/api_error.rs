use wp_api::{
    request::request_or_response_body_as_string, ParsedRequestError, WpError, WpErrorCode,
};

#[derive(Debug, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum JetpackApiError {
    #[error("Status code ({}) is not valid", status_code)]
    InvalidHttpStatusCode { status_code: u16 },
    #[error(
        "Request execution failed!\nStatus Code: '{:?}'.\nResponse: '{}'",
        status_code,
        reason
    )]
    RequestExecutionFailed {
        status_code: Option<u16>,
        reason: String,
    },
    #[error("Error while parsing. \nReason: {}\nResponse: {}", reason, response)]
    ResponseParsingError { reason: String, response: String },
    #[error("Error while parsing site url: {}", reason)]
    SiteUrlParsingError { reason: String },
    #[error(
        "Error that's not yet handled by the library:\nStatus Code: '{}'.\nResponse: '{}'",
        status_code,
        response
    )]
    UnknownError { status_code: u16, response: String },
    #[error(
        "WpError {{\n\tstatus_code: {}\n\terror_code: {:?}\n\terror_message: \"{}\"\n\tresponse: \"{}\"\n}}",
        status_code,
        error_code,
        error_message,
        response
    )]
    WpError {
        error_code: WpErrorCode,
        error_message: String,
        status_code: u16,
        response: String,
    },
}

impl ParsedRequestError for JetpackApiError {
    fn try_parse(response_body: &[u8], response_status_code: u16) -> Option<Self> {
        if let Some(wp_error) = WpError::try_parse(response_body, response_status_code) {
            Some(Self::WpError {
                error_code: wp_error.code,
                error_message: wp_error.message,
                status_code: response_status_code,
                response: request_or_response_body_as_string(response_body),
            })
        } else {
            match http::StatusCode::from_u16(response_status_code) {
                Ok(status) => {
                    if status.is_client_error() || status.is_server_error() {
                        Some(Self::UnknownError {
                            status_code: response_status_code,
                            response: request_or_response_body_as_string(response_body),
                        })
                    } else {
                        None
                    }
                }
                Err(_) => Some(Self::InvalidHttpStatusCode {
                    status_code: response_status_code,
                }),
            }
        }
    }

    fn as_parse_error(reason: String, response: String) -> Self {
        Self::ResponseParsingError { reason, response }
    }
}

#[derive(Debug, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum JetpackRequestExecutionError {
    #[error(
        "Request execution failed!\nStatus Code: '{:?}'.\nResponse: '{}'",
        status_code,
        reason
    )]
    RequestExecutionFailed {
        status_code: Option<u16>,
        reason: String,
    },
}

impl From<JetpackRequestExecutionError> for JetpackApiError {
    fn from(value: JetpackRequestExecutionError) -> Self {
        match value {
            JetpackRequestExecutionError::RequestExecutionFailed {
                status_code,
                reason,
            } => Self::RequestExecutionFailed {
                status_code,
                reason,
            },
        }
    }
}

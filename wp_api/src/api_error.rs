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
        rest_error: Option<WPRestError>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
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
pub struct WPRestError {
    pub code: WPRestErrorCode,
}

#[derive(Debug, Deserialize, PartialEq, Eq, uniffi::Error)]
pub enum WPRestErrorCode {
    #[serde(rename = "rest_cannot_create_user")]
    CannotCreateUser,
    #[serde(rename = "rest_cannot_edit")]
    CannotEdit,
    #[serde(rename = "rest_cannot_edit_roles")]
    CannotEditRoles,
    #[serde(rename = "rest_forbidden_context")]
    ForbiddenContext,
    #[serde(rename = "rest_forbidden_orderby")]
    ForbiddenOrderBy,
    #[serde(rename = "rest_forbidden_who")]
    ForbiddenWho,
    #[serde(rename = "rest_user_cannot_view")]
    UserCannotView,
    // TODO: Not tested because it requires multi-site?
    // https://github.com/WordPress/WordPress/blob/master/wp-includes/rest-api/endpoints/class-wp-rest-users-controller.php#L584-L588
    #[serde(rename = "rest_user_create")]
    UserCreate,
    #[serde(rename = "rest_user_exists")]
    UserExists,
    #[serde(rename = "rest_user_invalid_id")]
    UserInvalidId,
    #[serde(rename = "rest_user_invalid_reassign")]
    UserInvalidReassign,
    #[serde(rename = "rest_not_logged_in")]
    Unauthorized,
}

impl WPRestErrorCode {
    pub fn status_code(&self) -> u16 {
        match self {
            Self::CannotCreateUser => 403,
            Self::CannotEdit => 403,
            Self::CannotEditRoles => 403,
            Self::ForbiddenContext => 403,
            Self::ForbiddenOrderBy => 403,
            Self::ForbiddenWho => 403,
            Self::UserCannotView => 403,
            Self::UserCreate => 500,
            Self::UserExists => 400,
            Self::UserInvalidId => 404,
            Self::UserInvalidReassign => 400,
            Self::Unauthorized => 401,
        }
    }
}

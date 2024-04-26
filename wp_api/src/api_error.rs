use serde::Deserialize;

#[derive(Debug, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum WPApiError {
    #[error("Rest error '{:?}' with Status Code '{}'", rest_error, status_code)]
    RestError {
        rest_error: WPRestErrorWrapper,
        status_code: u16,
        response: String,
    },
    #[error("Error while parsing. \nReason: {}\nResponse: {}", reason, response)]
    ParsingError { reason: String, response: String },
    #[error(
        "Error that's not yet handled by the library:\nStatus Code: '{}'.\nResponse: '{}'",
        status_code,
        response
    )]
    UnknownError { status_code: u16, response: String },
}

#[derive(serde::Deserialize, PartialEq, Eq, Debug, uniffi::Enum)]
#[serde(untagged)]
pub enum WPRestErrorWrapper {
    Recognized(WPRestError),
    Unrecognized(UnrecognizedWPRestError),
}

#[derive(Debug, Deserialize, PartialEq, Eq, uniffi::Record)]
pub struct WPRestError {
    pub code: WPRestErrorCode,
    pub message: String,
}

#[derive(Debug, Deserialize, PartialEq, Eq, uniffi::Record)]
pub struct UnrecognizedWPRestError {
    pub code: String,
    pub message: String,
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
    #[serde(rename = "rest_invalid_param")]
    InvalidParam,
    #[serde(rename = "rest_not_logged_in")]
    Unauthorized,
    #[serde(rename = "rest_user_cannot_delete")]
    UserCannotDelete,
    #[serde(rename = "rest_user_cannot_view")]
    UserCannotView,
    #[serde(rename = "rest_user_invalid_email")]
    UserInvalidEmail,
    #[serde(rename = "rest_user_invalid_id")]
    UserInvalidId,
    #[serde(rename = "rest_user_invalid_reassign")]
    UserInvalidReassign,
    #[serde(rename = "rest_user_invalid_role")]
    UserInvalidRole,
    #[serde(rename = "rest_user_invalid_slug")]
    UserInvalidSlug,
    // Tested, but we believe these errors are imppossible to get unless the requests are manually modified
    #[serde(rename = "rest_user_exists")]
    UserExists,
    #[serde(rename = "rest_user_invalid_argument")]
    UserInvalidArgument,
    #[serde(rename = "rest_trash_not_supported")]
    TrashNotSupported,
    // Untested, because we believe these errors require multisite
    #[serde(rename = "rest_user_create")]
    UserCreate,
    // Untested, because we believe these errors are impossible to get
    #[serde(rename = "rest_user_invalid_username")]
    UserInvalidUsername,
    #[serde(rename = "rest_user_invalid_password")]
    UserInvalidPassword,
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
            Self::InvalidParam => 400,
            Self::TrashNotSupported => 501,
            Self::Unauthorized => 401,
            Self::UserCannotDelete => 403,
            Self::UserCannotView => 403,
            Self::UserCreate => 500,
            Self::UserExists => 400,
            Self::UserInvalidArgument => 400,
            Self::UserInvalidEmail => 400,
            Self::UserInvalidId => 404,
            Self::UserInvalidPassword => 400,
            Self::UserInvalidReassign => 400,
            Self::UserInvalidRole => 400,
            Self::UserInvalidSlug => 400,
            Self::UserInvalidUsername => 400,
        }
    }
}

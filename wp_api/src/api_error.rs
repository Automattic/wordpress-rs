use serde::Deserialize;

use crate::request::request_or_response_body_as_string;

pub trait ParsedRequestError
where
    Self: Sized,
{
    fn try_parse(response_body: &[u8], response_status_code: u16) -> Option<Self>;
    fn as_parse_error(reason: String, response: String) -> Self;
}

#[derive(Debug, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum WpApiError {
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

impl ParsedRequestError for WpApiError {
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
                Err(_) => Some(WpApiError::InvalidHttpStatusCode {
                    status_code: response_status_code,
                }),
            }
        }
    }

    fn as_parse_error(reason: String, response: String) -> Self {
        Self::ResponseParsingError { reason, response }
    }
}

// This type is used to parse the API errors. It then gets converted to `WpApiError::WpError`.
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct WpError {
    pub code: WpErrorCode,
    pub message: String,
}

impl WpError {
    pub fn try_parse(response_body: &[u8], response_status_code: u16) -> Option<Self> {
        serde_json::from_slice::<WpError>(response_body).ok()
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, uniffi::Error)]
pub enum WpErrorCode {
    #[serde(rename = "rest_already_trashed")]
    AlreadyTrashed,
    #[serde(rename = "rest_application_password_not_found")]
    ApplicationPasswordNotFound,
    #[serde(rename = "rest_cannot_create")]
    CannotCreate,
    #[serde(rename = "rest_cannot_create_application_passwords")]
    CannotCreateApplicationPasswords,
    #[serde(rename = "rest_cannot_create_user")]
    CannotCreateUser,
    #[serde(rename = "rest_cannot_delete")]
    CannotDelete,
    #[serde(rename = "rest_cannot_delete_active_plugin")]
    CannotDeleteActivePlugin,
    #[serde(rename = "rest_cannot_delete_application_password")]
    CannotDeleteApplicationPassword,
    #[serde(rename = "rest_cannot_delete_application_passwords")]
    CannotDeleteApplicationPasswords,
    #[serde(rename = "rest_cannot_edit")]
    CannotEdit,
    #[serde(rename = "rest_cannot_edit_application_password")]
    CannotEditApplicationPassword,
    #[serde(rename = "rest_cannot_edit_roles")]
    CannotEditRoles,
    #[serde(rename = "rest_cannot_install_plugin")]
    CannotInstallPlugin,
    #[serde(rename = "rest_cannot_introspect_app_password_for_non_authenticated_user")]
    CannotIntrospectAppPasswordForNonAuthenticatedUser,
    #[serde(rename = "rest_cannot_list_application_passwords")]
    CannotListApplicationPasswords,
    #[serde(rename = "rest_cannot_manage_plugins")]
    CannotManagePlugins,
    #[serde(rename = "rest_cannot_read_application_password")]
    CannotReadApplicationPassword,
    #[serde(rename = "rest_cannot_view")]
    CannotView,
    #[serde(rename = "rest_cannot_view_plugin")]
    CannotViewPlugin,
    #[serde(rename = "rest_cannot_view_plugins")]
    CannotViewPlugins,
    #[serde(rename = "empty_content")]
    EmptyContent,
    #[serde(rename = "rest_forbidden_context")]
    ForbiddenContext,
    #[serde(rename = "rest_forbidden_orderby")]
    ForbiddenOrderBy,
    #[serde(rename = "rest_forbidden_who")]
    ForbiddenWho,
    #[serde(rename = "rest_invalid_author")]
    InvalidAuthor,
    #[serde(rename = "rest_invalid_field")]
    InvalidField,
    #[serde(rename = "rest_invalid_param")]
    InvalidParam,
    #[serde(rename = "rest_no_search_term_defined")]
    NoSearchTermDefined,
    #[serde(rename = "rest_orderby_include_missing_include")]
    OrderbyIncludeMissingInclude,
    #[serde(rename = "rest_plugin_not_found")]
    PluginNotFound,
    #[serde(rename = "rest_post_incorrect_password")]
    PostIncorrectPassword,
    #[serde(rename = "rest_post_invalid_id")]
    PostInvalidId,
    #[serde(rename = "rest_post_invalid_page_number")]
    PostInvalidPageNumber,
    #[serde(rename = "rest_type_invalid")]
    TypeInvalid,
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
    // ------------------------------------------------------------------------------------
    // Untested, because we are unable to create the necessary conditions for them
    // ------------------------------------------------------------------------------------
    #[serde(rename = "application_passwords_disabled")]
    ApplicationPasswordsDisabled,
    #[serde(rename = "application_passwords_disabled_for_user")]
    ApplicationPasswordsDisabledForUser,
    #[serde(rename = "rest_cannot_assign_sticky")]
    CannotAssignSticky,
    #[serde(rename = "rest_cannot_assign_term")]
    CannotAssignTerm,
    #[serde(rename = "rest_cannot_edit_others")]
    CannotEditOthers,
    #[serde(rename = "rest_cannot_manage_application_passwords")]
    CannotManageApplicationPasswords,
    #[serde(rename = "rest_cannot_publish")]
    CannotPublish,
    #[serde(rename = "rest_cannot_read_type")]
    CannotReadType,
    #[serde(rename = "rest_forbidden_status")]
    ForbiddenStatus,
    #[serde(rename = "rest_invalid_featured_media")]
    InvalidFeaturedMedia,
    #[serde(rename = "rest_no_authenticated_app_password")]
    NoAuthenticatedAppPassword,
    #[serde(rename = "rest_user_cannot_delete_post")]
    UserCannotDeletePost, // See `rest_cannot_delete` instead
    // ------------------------------------------------------------------------------------
    // Untested, because we believe these errors require multisite
    // ------------------------------------------------------------------------------------
    #[serde(rename = "rest_cannot_manage_network_plugins")]
    CannotManageNetworkPlugins,
    #[serde(rename = "rest_network_only_plugin")]
    NetworkOnlyPlugin,
    #[serde(rename = "rest_user_create")]
    UserCreate,
    // ------------------------------------------------------------------------------------
    // Untested, because we don't think these errors are possible to get while using this library
    // ------------------------------------------------------------------------------------
    /// If a plugin is tried to be activated without the `activate_plugin` permission.
    /// However, in a default setup a prior check of `activate_plugins` will fail
    /// resulting in `CannotManagePlugins` error instead.
    #[serde(rename = "rest_cannot_activate_plugin")]
    CannotActivatePlugin,
    /// If a plugin is tried to be deactivated without the `deactivate_plugin` permission.
    /// However, in a default setup a prior check of `deactivate_plugin` will fail
    /// resulting in `CannotManagePlugins` error instead.
    #[serde(rename = "rest_cannot_deactivate_plugin")]
    CannotDeactivatePlugin,
    // If the create post request includes an id.
    #[serde(rename = "rest_post_exists")]
    PostExists,
    // If `force=true` is missing from delete user request.
    // If trash is not supported for the post type: https://github.com/WordPress/WordPress/blob/6.6.2/wp-includes/rest-api/endpoints/class-wp-rest-posts-controller.php#L1011-L1029
    #[serde(rename = "rest_trash_not_supported")]
    TrashNotSupported,
    // If the create user request includes an id.
    #[serde(rename = "rest_user_exists")]
    UserExists,
    // If username is included in the update user request.
    #[serde(rename = "rest_user_invalid_argument")]
    UserInvalidArgument,
    #[serde(rename = "rest_user_invalid_username")]
    UserInvalidUsername,
    #[serde(rename = "rest_user_invalid_password")]
    UserInvalidPassword,
    // ------------------------------------------------------------------------------------
    // All WpCore internal errors _should_ be wrapped as a `WpRestErrorCode` by the server.
    // However, in some cases they are sent back directly.
    // ------------------------------------------------------------------------------------
    #[serde(rename = "could_not_remove_plugin")]
    WpCoreCouldNotRemovePlugin,
    #[serde(rename = "could_not_resume_plugin")]
    WpCoreCouldNotResumePlugin,
    #[serde(rename = "folder_exists")]
    WpCoreFolderExists,
    #[serde(rename = "fs_error")]
    WpCoreFsError,
    #[serde(rename = "fs_no_plugins_dir")]
    WpCoreFsNoPluginsDir,
    #[serde(rename = "fs_unavailable")]
    WpCoreFsUnavailable,
    #[serde(rename = "no_plugin_header")]
    WpCoreNoPluginHeader,
    #[serde(rename = "plugin_invalid")]
    WpCorePluginInvalid,
    #[serde(rename = "plugin_missing_dependencies")]
    WpCorePluginMissingDependencies,
    #[serde(rename = "plugin_not_found")]
    WpCorePluginNotFound,
    #[serde(rename = "plugin_php_incompatible")]
    WpCorePluginPhpIncompatible,
    #[serde(rename = "plugin_wp_incompatible")]
    WpCorePluginWpIncompatible,
    #[serde(rename = "plugin_wp_php_incompatible")]
    WpCorePluginWpPhpIncompatible,
    #[serde(rename = "plugins_invalid")]
    WpCorePluginsInvalid,
    #[serde(rename = "plugins_api_failed")]
    WpCorePluginsApiFailed,
    #[serde(rename = "unable_to_connect_to_filesystem")]
    WpCoreUnableToConnectToFilesystem,
    #[serde(rename = "unable_to_determine_installed_plugin")]
    WpCoreUnableToDetermineInstalledPlugin,
    #[serde(rename = "unexpected_output")]
    WpCoreUnexpectedOutput,
    // ------------------------------------------------------------------------------------
    // Fallback to a `String` error code
    // ------------------------------------------------------------------------------------
    #[serde(untagged)]
    CustomError(String),
}

#[derive(Debug, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum RequestExecutionError {
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

impl From<RequestExecutionError> for WpApiError {
    fn from(value: RequestExecutionError) -> Self {
        match value {
            RequestExecutionError::RequestExecutionFailed {
                status_code,
                reason,
            } => Self::RequestExecutionFailed {
                status_code,
                reason,
            },
        }
    }
}

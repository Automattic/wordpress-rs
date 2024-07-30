use serde::Deserialize;

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

#[derive(Debug, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum WpApiError {
    #[error(
        "Request execution failed!\nStatus Code: '{:?}'.\nResponse: '{}'",
        status_code,
        reason
    )]
    RequestExecutionFailed {
        status_code: Option<u16>,
        reason: String,
    },
    #[error("Rest error '{:?}' with Status Code '{}'", error_code, status_code)]
    WpError {
        error_code: WpRestErrorCode,
        error_message: String,
        status_code: u16,
        response: String,
    },
    #[error("Error while parsing site url: {}", reason)]
    SiteUrlParsingError { reason: String },
    #[error("Error while parsing. \nReason: {}\nResponse: {}", reason, response)]
    ParsingError { reason: String, response: String },
    #[error(
        "Error that's not yet handled by the library:\nStatus Code: '{}'.\nResponse: '{}'",
        status_code,
        response
    )]
    UnknownError { status_code: u16, response: String },
}

// Used to parse the errors from API then converted to `WpApiError::WpError`
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub(crate) struct WpError {
    pub code: WpRestErrorCode,
    pub message: String,
}

#[derive(Debug, Deserialize, PartialEq, Eq, uniffi::Error)]
pub enum WpRestErrorCode {
    #[serde(rename = "rest_application_password_not_found")]
    ApplicationPasswordNotFound,
    #[serde(rename = "rest_cannot_create_application_passwords")]
    CannotCreateApplicationPasswords,
    #[serde(rename = "rest_cannot_create_user")]
    CannotCreateUser,
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
    #[serde(rename = "rest_cannot_introspect_app_password_for_non_authenticated_user")]
    CannotIntrospectAppPasswordForNonAuthenticatedUser,
    #[serde(rename = "rest_cannot_install_plugin")]
    CannotInstallPlugin,
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
    #[serde(rename = "rest_forbidden_context")]
    ForbiddenContext,
    #[serde(rename = "rest_forbidden_orderby")]
    ForbiddenOrderBy,
    #[serde(rename = "rest_forbidden_who")]
    ForbiddenWho,
    #[serde(rename = "rest_invalid_param")]
    InvalidParam,
    #[serde(rename = "rest_plugin_not_found")]
    PluginNotFound,
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
    // ---
    // Untested, because we are unable to create the necessary conditions for them
    // ---
    #[serde(rename = "application_passwords_disabled")]
    ApplicationPasswordsDisabled,
    #[serde(rename = "application_passwords_disabled_for_user")]
    ApplicationPasswordsDisabledForUser,
    #[serde(rename = "rest_cannot_manage_application_passwords")]
    CannotManageApplicationPasswords,
    #[serde(rename = "rest_cannot_read_type")]
    CannotReadType,
    #[serde(rename = "rest_no_authenticated_app_password")]
    NoAuthenticatedAppPassword,
    // ---
    // Untested, because we believe these errors require multisite
    // ---
    #[serde(rename = "rest_cannot_manage_network_plugins")]
    CannotManageNetworkPlugins,
    #[serde(rename = "rest_network_only_plugin")]
    NetworkOnlyPlugin,
    #[serde(rename = "rest_user_create")]
    UserCreate,
    // ---
    // Untested, because we don't think these errors are possible to get while using this library
    // ---
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
    // If `force=true` is missing from delete user request.
    #[serde(rename = "rest_trash_not_supported")]
    TrashNotSupported,
    // If the create user url includes an existing user id.
    #[serde(rename = "rest_user_exists")]
    UserExists,
    // If username is included in the update user request.
    #[serde(rename = "rest_user_invalid_argument")]
    UserInvalidArgument,
    #[serde(rename = "rest_user_invalid_username")]
    UserInvalidUsername,
    #[serde(rename = "rest_user_invalid_password")]
    UserInvalidPassword,
    // All WpCore internal errors _should_ be wrapped as a `WpRestErrorCode` by the server. However,
    // there is a good chance that some internal errors do make it into the response, so these error
    // types are provided.
    #[serde(rename = "fs_error")]
    WpCoreFsError,
    #[serde(rename = "fs_no_plugins_dir")]
    WpCoreFsNoPluginsDir,
    #[serde(rename = "fs_unavailable")]
    WpCoreFsUnavailable,
    #[serde(rename = "could_not_remove_plugin")]
    WpCoreCouldNotRemovePlugin,
    #[serde(rename = "could_not_resume_plugin")]
    WpCoreCouldNotResumePlugin,
    #[serde(rename = "no_plugin_header")]
    WpCoreNoPluginHeader,
    #[serde(rename = "plugin_missing_dependencies")]
    WpCorePluginMissingDependencies,
    #[serde(rename = "plugin_not_found")]
    WpCorePluginNotFound,
    #[serde(rename = "plugin_invalid")]
    WpCorePluginInvalid,
    #[serde(rename = "plugin_php_incompatible")]
    WpCorePluginPhpIncompatible,
    #[serde(rename = "plugin_wp_incompatible")]
    WpCorePluginWpIncompatible,
    #[serde(rename = "plugin_wp_php_incompatible")]
    WpCorePluginWpPhpIncompatible,
    #[serde(rename = "plugins_invalid")]
    WpCorePluginsInvalid,
    #[serde(rename = "unable_to_connect_to_filesystem")]
    WpCoreUnableToConnectToFilesystem,
    #[serde(rename = "unable_to_determine_installed_plugin")]
    WpCoreUnableToDetermineInstalledPlugin,
    #[serde(rename = "unexpected_output")]
    WpCoreUnexpectedOutput,
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

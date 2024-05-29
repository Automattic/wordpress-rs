use serde::Deserialize;

#[derive(Debug, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum WpApiError {
    #[error("Rest error '{:?}' with Status Code '{}'", rest_error, status_code)]
    RestError {
        rest_error: WPRestErrorWrapper,
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
    #[serde(rename = "rest_cannot_delete_active_plugin")]
    CannotDeleteActivePlugin,
    #[serde(rename = "rest_cannot_edit")]
    CannotEdit,
    #[serde(rename = "rest_cannot_edit_roles")]
    CannotEditRoles,
    #[serde(rename = "rest_cannot_install_plugin")]
    CannotInstallPlugin,
    #[serde(rename = "rest_cannot_manage_plugins")]
    CannotManagePlugins,
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
    // Tested, but we believe these errors are imppossible to get unless the requests are manually modified
    // ---
    #[serde(rename = "rest_user_exists")]
    UserExists,
    #[serde(rename = "rest_user_invalid_argument")]
    UserInvalidArgument,
    #[serde(rename = "rest_trash_not_supported")]
    TrashNotSupported,
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
    // Untested, because we believe these errors are impossible to get
    // ---
    /// Happens while activating a plugin without the `activate_plugin` permission.
    /// However, in a default setup a prior check of `activate_plugins` will fail
    /// resulting in `CannotManagePlugins` error instead.
    #[serde(rename = "rest_cannot_activate_plugin")]
    CannotActivatePlugin,
    /// Happens while deactivating a plugin without the `deactivate_plugin` permission.
    /// However, in a default setup a prior check of `activate_plugins` will fail
    /// resulting in `CannotManagePlugins` error instead.
    #[serde(rename = "rest_cannot_deactivate_plugin")]
    CannotDeactivatePlugin,
    #[serde(rename = "rest_user_invalid_username")]
    UserInvalidUsername,
    #[serde(rename = "rest_user_invalid_password")]
    UserInvalidPassword,
}

// All internal errors _should_ be wrapped as a `WPRestErrorCode` by the server. However, there
// is a good chance that some internal errors do make it into the response, so these error types
// are provided.
//
// Currently, we don't parse the response for these error types, but we could consider adding it
// as a fallback. For the moment, clients can manually try parsing an `Unrecognized` error
// into this type.
#[derive(Debug, Deserialize, PartialEq, Eq, uniffi::Error)]
pub enum WPInternalErrorCode {
    #[serde(rename = "fs_error")]
    FsError,
    #[serde(rename = "fs_no_plugins_dir")]
    FsNoPluginsDir,
    #[serde(rename = "fs_unavailable")]
    FsUnavailable,
    #[serde(rename = "could_not_remove_plugin")]
    CouldNotRemovePlugin,
    #[serde(rename = "could_not_resume_plugin")]
    CouldNotResumePlugin,
    #[serde(rename = "no_plugin_header")]
    NoPluginHeader,
    #[serde(rename = "plugin_missing_dependencies")]
    PluginMissingDependencies,
    #[serde(rename = "plugin_not_found")]
    PluginNotFound,
    #[serde(rename = "plugin_invalid")]
    PluginInvalid,
    #[serde(rename = "plugin_php_incompatible")]
    PluginPhpIncompatible,
    #[serde(rename = "plugin_wp_incompatible")]
    PluginWpIncompatible,
    #[serde(rename = "plugin_wp_php_incompatible")]
    PluginWpPhpIncompatible,
    #[serde(rename = "plugins_invalid")]
    PluginsInvalid,
    #[serde(rename = "unable_to_connect_to_filesystem")]
    UnableToConnectToFilesystem,
    #[serde(rename = "unable_to_determine_installed_plugin")]
    UnableToDetermineInstalledPlugin,
    #[serde(rename = "unexpected_output")]
    UnexpectedOutput,
}

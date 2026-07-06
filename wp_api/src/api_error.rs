use crate::request::WpRedirect;
use crate::request::{
    HttpAuthMethod, HttpAuthMethodParsingError, RequestMethod, WpNetworkResponse,
};
use serde::Deserialize;
use wp_localization::{MessageBundle, WpMessages, WpSupportsLocalization};
use wp_localization_macro::WpDeriveLocalizable;

pub trait ParsedRequestError
where
    Self: Sized,
{
    fn try_parse(response: &WpNetworkResponse) -> Option<Self>;
    fn as_parse_error(
        reason: String,
        response: String,
        request_url: String,
        request_method: RequestMethod,
    ) -> Self;
}

pub trait MaybeWpError {
    fn wp_error_code(&self) -> Option<&WpErrorCode>;

    fn is_unauthorized_error(&self) -> Option<bool>;
}

#[derive(Debug, PartialEq, Eq, thiserror::Error, uniffi::Error, WpDeriveLocalizable)]
pub enum WpApiError {
    InvalidHttpStatusCode {
        status_code: u32,
        request_url: String,
        request_method: RequestMethod,
    },
    RequestExecutionFailed {
        status_code: Option<u32>,
        redirects: Option<Vec<WpRedirect>>,
        reason: RequestExecutionErrorReason,
        request_url: String,
        request_method: RequestMethod,
    },
    MediaFileNotFound {
        file_path: String,
    },
    ResponseParsingError {
        reason: String,
        response: String,
        request_url: String,
        request_method: RequestMethod,
    },
    SiteUrlParsingError {
        reason: String,
    },
    UnknownError {
        status_code: u32,
        response: String,
        request_url: String,
        request_method: RequestMethod,
    },
    WpError {
        error_code: WpErrorCode,
        error_message: String,
        status_code: u32,
        response: String,
        request_url: String,
        request_method: RequestMethod,
    },
}

impl WpApiError {
    pub fn status_code(&self) -> Option<u32> {
        match self {
            WpApiError::InvalidHttpStatusCode { status_code, .. }
            | WpApiError::UnknownError { status_code, .. }
            | WpApiError::WpError { status_code, .. } => Some(*status_code),
            WpApiError::RequestExecutionFailed { status_code, .. } => *status_code,
            WpApiError::MediaFileNotFound { .. }
            | WpApiError::ResponseParsingError { .. }
            | WpApiError::SiteUrlParsingError { .. } => None,
        }
    }
}

impl MaybeWpError for WpApiError {
    fn wp_error_code(&self) -> Option<&WpErrorCode> {
        match self {
            WpApiError::WpError { error_code, .. } => Some(error_code),
            _ => None,
        }
    }

    fn is_unauthorized_error(&self) -> Option<bool> {
        self.wp_error_code().map(|e| e.is_unauthorized())
    }
}

impl<T, E> MaybeWpError for Result<T, E>
where
    E: MaybeWpError,
{
    fn wp_error_code(&self) -> Option<&WpErrorCode> {
        if let Err(e) = self {
            e.wp_error_code()
        } else {
            None
        }
    }

    fn is_unauthorized_error(&self) -> Option<bool> {
        self.wp_error_code().map(|e| e.is_unauthorized())
    }
}

impl WpSupportsLocalization for WpApiError {
    fn message_bundle(&self) -> MessageBundle<'_> {
        match self {
            WpApiError::InvalidHttpStatusCode { status_code, .. } => {
                WpMessages::invalid_http_status_code(status_code)
            }
            WpApiError::RequestExecutionFailed { reason, .. } => reason.message_bundle(),
            WpApiError::MediaFileNotFound { file_path } => {
                WpMessages::media_file_not_found(file_path)
            }
            WpApiError::ResponseParsingError { reason, .. } => {
                WpMessages::response_parsing_error(reason)
            }
            WpApiError::SiteUrlParsingError { .. } => WpMessages::url_parsing_error(),
            WpApiError::UnknownError { .. } => WpMessages::wp_api_error_generic_error(),
            WpApiError::WpError { error_message, .. } => {
                WpMessages::site_error_message(error_message)
            }
        }
    }
}

impl ParsedRequestError for WpApiError {
    fn try_parse(response: &WpNetworkResponse) -> Option<Self> {
        let request_url = response.request_url.0.clone();
        let request_method = response.request_method.clone();
        if let Some(wp_error) = WpError::try_parse(&response.body) {
            Some(Self::WpError {
                error_code: wp_error.code,
                error_message: wp_error.message,
                status_code: response.status_code,
                response: response.body_as_string(),
                request_url,
                request_method,
            })
        } else {
            if let Some(reason) = RequestExecutionErrorReason::try_from_response(response) {
                return Some(WpApiError::RequestExecutionFailed {
                    status_code: Some(response.status_code),
                    redirects: None,
                    reason,
                    request_url,
                    request_method,
                });
            }

            match http::StatusCode::from_u16(response.status_code as u16) {
                Ok(status) => {
                    if status.is_client_error() || status.is_server_error() {
                        Some(Self::UnknownError {
                            status_code: response.status_code,
                            response: response.body_as_string(),
                            request_url,
                            request_method,
                        })
                    } else {
                        None
                    }
                }
                Err(_) => Some(WpApiError::InvalidHttpStatusCode {
                    status_code: response.status_code,
                    request_url,
                    request_method,
                }),
            }
        }
    }

    fn as_parse_error(
        reason: String,
        response: String,
        request_url: String,
        request_method: RequestMethod,
    ) -> Self {
        Self::ResponseParsingError {
            reason,
            response,
            request_url,
            request_method,
        }
    }
}

// This type is used to parse the API errors. It then gets converted to `WpApiError::WpError`.
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct WpError {
    pub code: WpErrorCode,
    pub message: String,
}

impl WpError {
    pub fn try_parse(response_body: &[u8]) -> Option<Self> {
        serde_json::from_slice::<WpError>(response_body).ok()
    }

    pub fn try_parse_from_file(file: std::fs::File) -> Option<Self> {
        serde_json::from_reader(file).ok()
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, uniffi::Error)]
pub enum WpErrorCode {
    #[serde(rename = "rest_already_trashed")]
    AlreadyTrashed,
    #[serde(rename = "rest_application_password_not_found")]
    ApplicationPasswordNotFound,
    #[serde(rename = "block_cannot_read")]
    BlockCannotRead,
    #[serde(rename = "rest_block_directory_cannot_view")]
    BlockDirectoryCannotView,
    #[serde(rename = "block_invalid")]
    BlockInvalid,
    #[serde(rename = "rest_block_type_cannot_view")]
    BlockTypeCannotView,
    #[serde(rename = "rest_block_type_invalid")]
    BlockTypeInvalid,
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
    #[serde(rename = "rest_cannot_manage_templates")]
    CannotManageTemplates,
    #[serde(rename = "rest_cannot_manage_widgets")]
    CannotManageWidgets,
    #[serde(rename = "rest_cannot_read")]
    CannotRead,
    #[serde(rename = "rest_cannot_read_application_password")]
    CannotReadApplicationPassword,
    #[serde(rename = "rest_cannot_read_post")]
    CannotReadPost,
    #[serde(rename = "rest_cannot_read_status")]
    CannotReadStatus,
    #[serde(rename = "rest_cannot_update")]
    CannotUpdate,
    #[serde(rename = "rest_cannot_view")]
    CannotView,
    #[serde(rename = "rest_cannot_view_active_theme")]
    CannotViewActiveTheme,
    #[serde(rename = "rest_cannot_view_plugin")]
    CannotViewPlugin,
    #[serde(rename = "rest_cannot_view_plugins")]
    CannotViewPlugins,
    #[serde(rename = "rest_cannot_view_themes")]
    CannotViewThemes,
    #[serde(rename = "comment_author_column_length")]
    CommentAuthorColumnLength,
    #[serde(rename = "rest_comment_author_data_required")]
    CommentAuthorDataRequired,
    #[serde(rename = "comment_author_email_column_length")]
    CommentAuthorEmailColumnLength,
    #[serde(rename = "rest_comment_author_invalid")]
    CommentAuthorInvalid,
    #[serde(rename = "comment_author_url_column_length")]
    CommentAuthorUrlColumnLength,
    #[serde(rename = "rest_comment_closed")]
    CommentClosed,
    #[serde(rename = "comment_content_column_length")]
    ContentColumnLength,
    #[serde(rename = "rest_comment_content_invalid")]
    CommentContentInvalid,
    #[serde(rename = "rest_comment_draft_post")]
    CommentDraftPost,
    #[serde(rename = "rest_comment_invalid_author")]
    CommentInvalidAuthor,
    #[serde(rename = "rest_comment_invalid_author_ip")]
    CommentInvalidAuthorIp,
    #[serde(rename = "rest_comment_invalid_id")]
    CommentInvalidId,
    #[serde(rename = "rest_comment_invalid_post_id")]
    CommentInvalidPostId,
    #[serde(rename = "rest_comment_invalid_status")]
    CommentInvalidStatus,
    #[serde(rename = "rest_comment_trash_post")]
    CommentTrashPost,
    #[serde(rename = "empty_content")]
    EmptyContent,
    #[serde(rename = "rest_forbidden")]
    Forbidden,
    #[serde(rename = "rest_forbidden_context")]
    ForbiddenContext,
    #[serde(rename = "rest_forbidden_param")]
    ForbiddenParam,
    #[serde(rename = "rest_forbidden_orderby")]
    ForbiddenOrderBy,
    #[serde(rename = "rest_forbidden_who")]
    ForbiddenWho,
    #[serde(rename = "rest_invalid_author")]
    InvalidAuthor,
    #[serde(rename = "rest_invalid_field")]
    InvalidField,
    #[serde(rename = "rest_invalid_menu_location")]
    InvalidMenuLocation,
    #[serde(rename = "rest_invalid_param")]
    InvalidParam,
    #[serde(rename = "rest_invalid_template")]
    InvalidTemplate,
    #[serde(rename = "rest_invalid_widget")]
    InvalidWidget,
    #[serde(rename = "menu_exists")]
    MenuExists,
    #[serde(rename = "rest_menu_location_invalid")]
    MenuLocationInvalid,
    #[serde(rename = "rest_no_search_term_defined")]
    NoSearchTermDefined,
    #[serde(rename = "rest_orderby_include_missing_include")]
    OrderbyIncludeMissingInclude,
    #[serde(rename = "rest_pattern_directory_cannot_view")]
    PatternDirectoryCannotView,
    #[serde(rename = "rest_plugin_not_found")]
    PluginNotFound,
    #[serde(rename = "rest_post_incorrect_password")]
    PostIncorrectPassword,
    #[serde(rename = "rest_post_invalid_id")]
    PostInvalidId,
    #[serde(rename = "rest_post_invalid_page_number")]
    PostInvalidPageNumber,
    #[serde(rename = "rest_post_invalid_parent")]
    PostInvalidParent,
    #[serde(rename = "rest_post_invalid_type")]
    PostInvalidType,
    #[serde(rename = "rest_post_no_autosave")]
    PostNoAutosave,
    #[serde(rename = "rest_revision_invalid_offset_number")]
    RevisionInvalidOffsetNumber,
    #[serde(rename = "rest_revision_invalid_page_number")]
    RevisionInvalidPageNumber,
    #[serde(rename = "rest_sidebar_not_found")]
    SidebarNotFound,
    #[serde(rename = "rest_status_invalid")]
    StatusInvalid,
    #[serde(rename = "rest_taxonomy_invalid")]
    TaxonomyInvalid,
    #[serde(rename = "rest_template_not_found")]
    TemplateNotFound,
    #[serde(rename = "rest_term_invalid")]
    TermInvalid,
    #[serde(rename = "rest_title_required")]
    TitleRequired,
    #[serde(rename = "rest_term_invalid_id")]
    TermInvalidId,
    #[serde(rename = "rest_theme_not_found")]
    ThemeNotFound,
    #[serde(rename = "rest_type_invalid")]
    TypeInvalid,
    #[serde(rename = "rest_not_logged_in")]
    Unauthorized,
    #[serde(rename = "rest_upload_no_data")]
    UploadNoData,
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
    #[serde(rename = "rest_widget_not_found")]
    WidgetNotFound,
    #[serde(rename = "rest_widget_type_invalid")]
    WidgetTypeInvalid,
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
    #[serde(rename = "rest_cannot_edit_file_type")]
    CannotEditFileType,
    #[serde(rename = "rest_cannot_edit_image")]
    CannotEditImage,
    #[serde(rename = "rest_cannot_edit_others")]
    CannotEditOthers,
    #[serde(rename = "rest_cannot_manage_application_passwords")]
    CannotManageApplicationPasswords,
    #[serde(rename = "rest_cannot_publish")]
    CannotPublish,
    #[serde(rename = "rest_cannot_read_type")]
    CannotReadType,
    #[serde(rename = "comment_duplicate")]
    CommentDuplicate,
    #[serde(rename = "rest_comment_failed_create")]
    CommentFailedCreate,
    #[serde(rename = "rest_comment_failed_edit")]
    CommentFailedEdit,
    #[serde(rename = "comment_flood")]
    CommentFlood,
    #[serde(rename = "rest_comment_login_required")]
    CommentLoginRequired,
    #[serde(rename = "rest_forbidden_status")]
    ForbiddenStatus,
    #[serde(rename = "rest_image_not_edited")]
    ImageNotEdited,
    #[serde(rename = "rest_image_crop_failed")]
    ImageCropFailed,
    #[serde(rename = "rest_image_rotation_failed")]
    ImageRotationFailed,
    #[serde(rename = "rest_invalid_featured_media")]
    InvalidFeaturedMedia,
    #[serde(rename = "rest_no_authenticated_app_password")]
    NoAuthenticatedAppPassword,
    #[serde(rename = "rest_no_featured_media")]
    NoFeaturedMedia,
    #[serde(rename = "rest_search_handler_error")]
    SearchHandlerError,
    #[serde(rename = "rest_search_invalid_page_number")]
    SearchInvalidPageNumber,
    #[serde(rename = "rest_search_invalid_type")]
    SearchInvalidType,
    #[serde(rename = "rest_template_insert_error")]
    TemplateInsertError,
    #[serde(rename = "rest_upload_file_error")]
    UploadFileError,
    #[serde(rename = "rest_upload_file_too_big")]
    UploadFileTooBig,
    #[serde(rename = "rest_upload_hash_mismatch")]
    UploadHashMismatch,
    #[serde(rename = "rest_upload_invalid_disposition")]
    UploadInvalidDisposition,
    #[serde(rename = "rest_upload_limited_space")]
    UploadLimitedSpace,
    #[serde(rename = "rest_upload_no_content_disposition")]
    UploadNoContentDisposition,
    #[serde(rename = "rest_upload_no_content_type")]
    UploadNoContentType,
    #[serde(rename = "rest_upload_sideload_error")]
    UploadSideloadError,
    #[serde(rename = "rest_upload_user_quota_exceeded")]
    UploadUserQuotaExceeded,
    #[serde(rename = "rest_url_required")]
    UrlRequired,
    #[serde(rename = "rest_user_cannot_delete_post")]
    UserCannotDeletePost, // See `rest_cannot_delete` instead
    #[serde(rename = "rest_unknown_attachment")]
    UnknownAttachment,
    #[serde(rename = "rest_unknown_image_file_type")]
    UnknownImageFileType,
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
    // If the create comment request includes an id.
    #[serde(rename = "rest_comment_exists")]
    CommentExists,
    /// If a plugin is tried to be deactivated without the `deactivate_plugin` permission.
    /// However, in a default setup a prior check of `deactivate_plugin` will fail
    /// resulting in `CannotManagePlugins` error instead.
    #[serde(rename = "rest_cannot_deactivate_plugin")]
    CannotDeactivatePlugin,
    // If a `comment_type` parameter is passed while creating / editing a comment.
    #[serde(rename = "rest_invalid_comment_type")]
    InvalidCommentType,
    // If a menu item URL fails sanitize_url() validation (e.g., javascript: protocol).
    // Defined in schema validation callback (https://github.com/WordPress/WordPress/blob/6.8/wp-includes/rest-api/endpoints/class-wp-rest-menu-items-controller.php#L881-L884),
    // but WordPress wraps validation errors in `rest_invalid_param` before returning them.
    // See: https://github.com/WordPress/WordPress/blob/6.8/wp-includes/rest-api/class-wp-rest-request.php#L936-L953
    #[serde(rename = "rest_invalid_url")]
    InvalidUrl,
    // If the create post request includes an id.
    #[serde(rename = "rest_post_exists")]
    PostExists,
    /// If a revision doesn't belong to the specified parent post.
    /// However, WordPress validates revision existence first via `get_revision()` which will
    /// return `rest_post_invalid_id` before checking parent-child relationships.
    #[serde(rename = "rest_revision_parent_id_mismatch")]
    RevisionParentIdMismatch,
    // If a create/update request to a non-hierarchical endpoint, such as `/tags`, include
    // `parent` argument
    #[serde(rename = "rest_taxonomy_not_hierarchical")]
    TaxonomyNotHierarchical,
    // If the template is already trashed, the server returns `rest_template_not_found`
    #[serde(rename = "rest_template_already_trashed")]
    TemplateAlreadyTrashed,
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

impl WpErrorCode {
    fn is_unauthorized(&self) -> bool {
        self == &Self::Unauthorized
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, uniffi::Error, WpDeriveLocalizable)]
pub enum RequestExecutionError {
    RequestExecutionFailed {
        status_code: Option<u32>,
        redirects: Option<Vec<WpRedirect>>,
        reason: RequestExecutionErrorReason,
        request_url: String,
        request_method: RequestMethod,
    },
    MediaFileNotFound {
        file_path: String,
    },
}

impl WpSupportsLocalization for RequestExecutionError {
    fn message_bundle(&self) -> MessageBundle<'_> {
        match self {
            RequestExecutionError::RequestExecutionFailed { reason, .. } => reason.message_bundle(),
            RequestExecutionError::MediaFileNotFound { file_path } => {
                WpMessages::media_file_not_found(file_path)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum InvalidSslErrorReason {
    CertificateNotValidForName {
        hostname: String,
        presented_hostnames: Vec<String>,
    },
    GenericSslError,
}

impl InvalidSslErrorReason {
    fn message_bundle(&self) -> MessageBundle<'_> {
        match self {
            Self::CertificateNotValidForName { .. } => {
                WpMessages::invalid_ssl_error_certificate_not_valid_for_name()
            }
            Self::GenericSslError => WpMessages::invalid_ssl_error_generic_ssl_error(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum, WpDeriveLocalizable)]
pub enum RequestExecutionErrorReason {
    // A case where there's an SSL certificate present, but it's untrusted (maybe it's self-signed, expired, or for the wrong domain)
    InvalidSslError {
        reason: InvalidSslErrorReason,
    },
    NonExistentSiteError {
        error_message: Option<String>,
        suggested_action: Option<String>,
    },
    HttpAuthenticationRequiredError {
        hostname: String,
        method: Option<HttpAuthMethod>,
    },
    HttpAuthenticationRejectedError {
        hostname: String,
        method: Option<HttpAuthMethod>,
    },
    HttpForbiddenError {
        hostname: String,
    },
    HttpTimeoutError,
    MisconfiguredHttpAuthenticationError {
        issue: HttpAuthMethodParsingError,
    },
    MisconfiguredRateLimitError,
    DeviceIsOfflineError {
        error_message: String,
    },
    CancellationError,
    HttpError {
        reason: String,
    },
    GenericError {
        error_message: String,
    },
}

impl RequestExecutionErrorReason {
    pub fn try_from_response(response: &WpNetworkResponse) -> Option<Self> {
        if response.status_code != 401 && response.status_code != 403 {
            return None;
        }

        // TODO: We are currently parsing the response for `WpError` twice. There is currently no
        // good way to avoid it, but we are planning to rework some of the error handling once we
        // finish the login work. At that time, we'll try to remove the double parsing.
        if WpError::try_parse(&response.body).is_some() {
            // If the response is a `WpError`, don't map it to an auth error
            return None;
        }

        let reason = match response.get_http_auth_method() {
            Ok(maybe_method) => match maybe_method {
                Some(method) => {
                    if response.request_header_map.has_http_authentication() {
                        RequestExecutionErrorReason::HttpAuthenticationRejectedError {
                            hostname: response.request_url.0.clone(),
                            method: Some(method),
                        }
                    } else {
                        RequestExecutionErrorReason::HttpAuthenticationRequiredError {
                            hostname: response.request_url.0.clone(),
                            method: Some(method),
                        }
                    }
                }
                None => {
                    if response.request_header_map.has_http_authentication() {
                        RequestExecutionErrorReason::HttpAuthenticationRejectedError {
                            hostname: response.request_url.0.clone(),
                            method: None,
                        }
                    } else {
                        RequestExecutionErrorReason::HttpForbiddenError {
                            hostname: response.request_url.0.clone(),
                        }
                    }
                }
            },
            Err(e) => {
                RequestExecutionErrorReason::MisconfiguredHttpAuthenticationError { issue: e }
            }
        };
        Some(reason)
    }
}

impl WpSupportsLocalization for RequestExecutionErrorReason {
    fn message_bundle(&self) -> MessageBundle<'_> {
        match self {
            RequestExecutionErrorReason::InvalidSslError { reason } => reason.message_bundle(),
            RequestExecutionErrorReason::NonExistentSiteError { .. } => {
                WpMessages::non_existent_site_error()
            }
            RequestExecutionErrorReason::HttpAuthenticationRequiredError { hostname, .. } => {
                WpMessages::http_authentication_required_error(hostname)
            }
            RequestExecutionErrorReason::HttpAuthenticationRejectedError { hostname, .. } => {
                WpMessages::http_authentication_rejected_error(hostname)
            }
            RequestExecutionErrorReason::MisconfiguredHttpAuthenticationError { .. } => {
                WpMessages::misconfigured_http_authentication_error()
            }
            RequestExecutionErrorReason::MisconfiguredRateLimitError => {
                WpMessages::misconfigured_rate_limit_error()
            }
            RequestExecutionErrorReason::HttpForbiddenError { hostname } => {
                WpMessages::http_forbidden_error(hostname)
            }
            RequestExecutionErrorReason::DeviceIsOfflineError { error_message } => {
                WpMessages::just(error_message)
            }
            RequestExecutionErrorReason::HttpError { reason } => {
                WpMessages::http_server_error(reason)
            }
            RequestExecutionErrorReason::GenericError { error_message } => {
                WpMessages::just(error_message)
            }
            RequestExecutionErrorReason::HttpTimeoutError => WpMessages::http_timeout_error(),
            RequestExecutionErrorReason::CancellationError => WpMessages::http_cancellation_error(),
        }
    }
}

impl From<RequestExecutionError> for WpApiError {
    fn from(value: RequestExecutionError) -> Self {
        match value {
            RequestExecutionError::RequestExecutionFailed {
                status_code,
                redirects,
                reason,
                request_url,
                request_method,
            } => Self::RequestExecutionFailed {
                status_code,
                redirects,
                reason,
                request_url,
                request_method,
            },
            RequestExecutionError::MediaFileNotFound { file_path } => {
                Self::MediaFileNotFound { file_path }
            }
        }
    }
}

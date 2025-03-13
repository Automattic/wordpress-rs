use super::WpApiDetails;
use crate::{
    login::KnownApplicationPasswordBlockingPlugin,
    request::{WpNetworkHeaderMap, WpRedirect},
    ParseUrlError, ParsedUrl, RequestExecutionError, RequestExecutionErrorReason,
};
use scraper::{Html, Selector};
use serde::Deserialize;
use std::{collections::HashMap, fmt::Display, sync::Arc};
use wp_localization::{WpMessages, WpSupportsLocalization};
use wp_localization_macro::WpDeriveLocalizable;

pub(crate) const API_ROOT_LINK_HEADER: &str = "https://api.w.org/";

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct AutoDiscoveryAttempt {
    pub(crate) attempt_site_url: String,
    pub(crate) attempt_type: AutoDiscoveryAttemptType,
}

impl AutoDiscoveryAttempt {
    fn new(attempt_site_url: impl Into<String>, attempt_type: AutoDiscoveryAttemptType) -> Self {
        Self {
            attempt_site_url: attempt_site_url.into(),
            attempt_type,
        }
    }

    fn with_user_input_attempt_type(attempt_site_url: impl Into<String>) -> Self {
        Self::new(attempt_site_url, AutoDiscoveryAttemptType::UserInput)
    }

    fn with_auto_https_attempt_type(attempt_site_url: impl Into<String>) -> Self {
        Self::new(attempt_site_url, AutoDiscoveryAttemptType::AutoHttps)
    }

    fn with_auto_remove_wp_admin_suffix_attempt_type(attempt_site_url: impl Into<String>) -> Self {
        Self::new(
            attempt_site_url,
            AutoDiscoveryAttemptType::AutoRemoveWpAdminSuffix,
        )
    }

    fn with_auto_remove_wp_login_suffix_attempt_type(attempt_site_url: impl Into<String>) -> Self {
        Self::new(
            attempt_site_url,
            AutoDiscoveryAttemptType::AutoRemoveWpLoginSuffix,
        )
    }
}

#[derive(Debug, uniffi::Record)]
pub struct AutoDiscoveryUniffiResult {
    pub user_input_attempt: Arc<AutoDiscoveryAttemptResult>,
    pub successful_attempt: Option<Arc<AutoDiscoveryAttemptResult>>,
    pub auto_https_attempt: Option<Arc<AutoDiscoveryAttemptResult>>,
    pub auto_dot_php_extension_for_wp_admin_attempt: Option<Arc<AutoDiscoveryAttemptResult>>,
    pub is_successful: bool,
}

impl From<AutoDiscoveryResult> for AutoDiscoveryUniffiResult {
    fn from(value: AutoDiscoveryResult) -> Self {
        let get_attempt_result = |attempt_type| {
            value
                .get_attempt(&attempt_type)
                .map(|a| Arc::new(a.clone()))
        };
        Self {
            user_input_attempt: Arc::new(value.user_input_attempt().clone()),
            successful_attempt: value.find_successful().map(|a| Arc::new(a.clone())),
            auto_https_attempt: get_attempt_result(AutoDiscoveryAttemptType::AutoHttps),
            auto_dot_php_extension_for_wp_admin_attempt: get_attempt_result(
                AutoDiscoveryAttemptType::AutoRemoveWpAdminSuffix,
            ),
            is_successful: value.is_successful(),
        }
    }
}

#[derive(Debug)]
pub struct AutoDiscoveryResult {
    pub attempts: HashMap<AutoDiscoveryAttemptType, AutoDiscoveryAttemptResult>,
}

impl AutoDiscoveryResult {
    pub fn is_successful(&self) -> bool {
        self.attempts
            .iter()
            .any(|(_, result)| result.is_successful())
    }

    pub fn find_successful(&self) -> Option<&AutoDiscoveryAttemptResult> {
        // If the user attempt is successful, prefer it over other attempts
        let user_input_attempt = self.user_input_attempt();
        if user_input_attempt.is_successful() {
            return Some(user_input_attempt);
        }
        self.attempts.iter().find_map(|(_, result)| {
            if result.is_successful() {
                Some(result)
            } else {
                None
            }
        })
    }

    pub fn user_input_attempt(&self) -> &AutoDiscoveryAttemptResult {
        self.get_attempt(&AutoDiscoveryAttemptType::UserInput)
            .expect("User input url is always attempted")
    }

    pub fn auto_discovery_attempt(&self) -> &AutoDiscoveryAttemptResult {
        self.get_attempt(&AutoDiscoveryAttemptType::AutoHttps)
            .expect("Auto discovery url is always attempted")
    }

    pub fn get_attempt(
        &self,
        attempt_type: &AutoDiscoveryAttemptType,
    ) -> Option<&AutoDiscoveryAttemptResult> {
        self.attempts.get(attempt_type)
    }

    pub fn is_wordpress_site(&self) -> bool {
        self.attempts
            .iter()
            .any(|(_, result)| result.is_wordpress_site())
    }
}

#[derive(Debug, Clone, uniffi::Object)]
pub struct AutoDiscoveryAttemptResult {
    pub attempt_type: AutoDiscoveryAttemptType,
    pub attempt_site_url: String,
    pub api_discovery_result: Result<AutoDiscoveryAttemptSuccess, AutoDiscoveryAttemptFailure>,
    pub is_wordpress_site: IsWordPressSiteAttemptResult,
}

#[uniffi::export]
impl AutoDiscoveryAttemptResult {
    fn attempt_site_url(&self) -> String {
        self.attempt_site_url.clone()
    }

    pub fn error_message(&self) -> Option<String> {
        match &self.api_discovery_result {
            Ok(_) => None,
            Err(error) => Some(error.to_string()),
        }
    }

    fn is_successful(&self) -> bool {
        self.api_discovery_result.is_ok()
    }

    fn is_network_error(&self) -> bool {
        match &self.api_discovery_result {
            Ok(_) => false,
            Err(error) => error.is_network_error(),
        }
    }

    fn is_user_input_attempt(&self) -> bool {
        matches!(self.attempt_type, AutoDiscoveryAttemptType::UserInput)
    }

    fn has_failed_to_parse_site_url(&self) -> bool {
        match &self.api_discovery_result {
            Ok(_success) => false,
            Err(error) => error.parsed_site_url().is_none(),
        }
    }

    fn has_failed_to_parse_api_root_url(&self) -> Option<bool> {
        match &self.api_discovery_result {
            Ok(_success) => Some(false),
            Err(error) => error.has_failed_to_parse_api_root_url(),
        }
    }

    fn has_failed_to_parse_api_details(&self) -> Option<bool> {
        match &self.api_discovery_result {
            Ok(_success) => Some(false),
            Err(error) => error.has_failed_to_parse_api_details(),
        }
    }

    fn parsed_site_url(&self) -> Option<Arc<ParsedUrl>> {
        match &self.api_discovery_result {
            Ok(success) => Some(Arc::new(success.parsed_site_url.clone())),
            Err(error) => error.parsed_site_url().map(|p| Arc::new(p.clone())),
        }
    }

    fn api_root_url(&self) -> Option<Arc<ParsedUrl>> {
        match &self.api_discovery_result {
            Ok(success) => Some(Arc::new(success.api_root_url.clone())),
            Err(error) => error.api_root_url().map(|p| Arc::new(p.clone())),
        }
    }

    fn api_details(&self) -> Option<Arc<WpApiDetails>> {
        match &self.api_discovery_result {
            Ok(success) => Some(Arc::new(success.api_details.clone())),
            Err(_) => None,
        }
    }

    fn is_local_dev_environment(&self) -> bool {
        match &self.api_discovery_result {
            Ok(success) => is_local_dev_environment_url(&success.parsed_site_url),
            Err(error) => {
                if let Some(parsed_url) = error.parsed_site_url() {
                    is_local_dev_environment_url(parsed_url)
                } else {
                    false
                }
            }
        }
    }

    /// Does the site look like a WordPress site?
    fn is_wordpress_site(&self) -> bool {
        self.is_wordpress_site.is_successful()
    }
}

impl AutoDiscoveryAttemptResult {
    pub fn error(&self) -> Option<AutoDiscoveryAttemptFailure> {
        if let Err(error) = &self.api_discovery_result {
            Some(error.clone())
        } else {
            None
        }
    }
}

// Does the given URL look like it's on a local development environment for the purposes of the Login Spec?
pub fn is_local_dev_environment_url(parsed_site_url: &ParsedUrl) -> bool {
    if let Some(hostname) = parsed_site_url.inner.host_str() {
        return hostname == "localhost"
            || hostname == "127.0.0.1"
            || hostname.ends_with(".local")
            || hostname.ends_with(".test");
    }

    false
}

#[derive(Debug, Clone, uniffi::Object)]
pub struct IsWordPressSiteAttemptResult {
    pub api_link_header_result: Result<FindApiRootLinkHeaderSuccess, FindApiRootLinkHeaderFailure>,
    pub fetch_wp_json_result: Result<FetchWpJsonSuccess, FetchWpJsonFailure>,
    pub parse_html_result: Result<IsWordPressSiteParseHtmlResult, ParseHtmlFailure>,
}

impl IsWordPressSiteAttemptResult {
    pub fn is_successful(&self) -> bool {
        self.api_link_header_result.is_ok()
            || self.fetch_wp_json_result.is_ok()
            || self
                .parse_html_result
                .as_ref()
                .map(|r| {
                    r.has_wordpress_generator_meta_tag
                        || r.mentions_wp_content
                        || r.mentions_wp_includes
                })
                .unwrap_or(false)
    }
}

#[derive(Debug, Clone)]
pub struct FindApiRootLinkHeaderSuccess {
    pub parsed_site_url: ParsedUrl,
    pub api_root_url: ParsedUrl,
}

#[derive(Debug, Clone)]
pub struct FetchWpJsonSuccess {
    pub wp_json_url: ParsedUrl,
    pub root_wp_json: RootWpJson,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RootWpJson {
    pub namespaces: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct IsWordPressSiteParseHtmlResult {
    /// `href` attribute of a link tag if it has `rel` attribute of "https://api.w.org/".
    /// For example:
    /// <link href="http://localhost/wp-json/" rel="https://api.w.org/">
    pub api_root_url_from_link_tag: Option<ParsedUrl>,
    /// Whether the HTML has 'generator' meta tag that mentions `WordPress`
    pub has_wordpress_generator_meta_tag: bool,
    /// Whether the HTML `link`, `script`, `style` tags mention `wp-content`
    pub mentions_wp_content: bool,
    /// Whether the HTML `link`, `script`, `style` tags mention `wp-includes`
    pub mentions_wp_includes: bool,
}

impl IsWordPressSiteParseHtmlResult {
    const HTML_ATTR_NAME: &str = "name";
    const HTML_ATTR_CONTENT: &str = "content";
    const HTML_ATTR_HREF: &str = "href";
    const HTML_ATTR_REL: &str = "rel";
    const HTML_ATTR_SRC: &str = "src";
    const META_TAG_GENERATOR: &str = "generator";
    const META_TAG_GENERATOR_CONTENT_INCLUDES: &str = "WordPress";
    const SELECTOR_META: &str = "meta";
    const SELECTOR_LINK: &str = "link";
    const SELECTOR_SCRIPT: &str = "script";
    const WP_CONTENT: &str = "wp-content";
    const WP_INCLUDES: &str = "wp-includes";

    pub fn parse_response(response_body: &str) -> Self {
        let html = Html::parse_document(response_body);

        // Search for the mention of `wp-content` and `wp-includes` in `link`, `script` & `style`
        // tags
        let link_selector =
            Selector::parse(Self::SELECTOR_LINK).expect("'link' is a valid selector");
        let script_selector =
            Selector::parse(Self::SELECTOR_SCRIPT).expect("'script' is a valid selector");
        let (mentions_wp_content, mentions_wp_includes) = html
            .select(&link_selector)
            .flat_map(|e| e.attr(Self::HTML_ATTR_HREF))
            .chain(
                html.select(&script_selector)
                    .flat_map(|e| e.attr(Self::HTML_ATTR_SRC)),
            )
            .fold(
                (false, false),
                |(check_wp_content, check_wp_includes), e| {
                    (
                        check_wp_content || e.contains(Self::WP_CONTENT),
                        check_wp_includes || e.contains(Self::WP_INCLUDES),
                    )
                },
            );
        let api_root_url_from_link_tag = html.select(&link_selector).find_map(|e| {
            if let Some(API_ROOT_LINK_HEADER) = e.attr(Self::HTML_ATTR_REL) {
                e.attr(Self::HTML_ATTR_HREF)
                    .and_then(|u| ParsedUrl::parse(u).ok())
            } else {
                None
            }
        });

        Self {
            api_root_url_from_link_tag,
            has_wordpress_generator_meta_tag: Self::html_has_generator_tag(&html),
            mentions_wp_content,
            mentions_wp_includes,
        }
    }

    fn html_has_generator_tag(html: &Html) -> bool {
        let meta_selector =
            Selector::parse(Self::SELECTOR_META).expect("'meta' is a valid selector");
        html.select(&meta_selector).any(|e| {
            e.value().attr(Self::HTML_ATTR_NAME) == Some(Self::META_TAG_GENERATOR)
                && e.value()
                    .attr(Self::HTML_ATTR_CONTENT)
                    .unwrap_or_default()
                    .contains(Self::META_TAG_GENERATOR_CONTENT_INCLUDES)
        })
    }
}

#[derive(Debug, Clone)]
pub enum FindApiRootLinkHeaderFailure {
    ParseSiteUrl {
        error: ParseUrlError,
    },
    FetchApiRootUrl {
        parsed_site_url: ParsedUrl,
        error: RequestExecutionError,
    },
    ParseApiRootUrl {
        parsed_site_url: ParsedUrl,
        error: ParseApiRootUrlError,
    },
}

#[derive(Debug, Clone)]
pub enum FetchWpJsonFailure {
    ParseSiteUrl {
        error: ParseUrlError,
    },
    FetchWpJson {
        wp_json_url: ParsedUrl,
        error: RequestExecutionError,
    },
    ParseWpJson {
        wp_json_url: ParsedUrl,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseHtmlFailure {
    ParseSiteUrl { error: ParseUrlError },
    FetchSite { error: RequestExecutionError },
}

#[derive(Debug, Clone)]
pub struct AutoDiscoveryAttemptSuccess {
    pub parsed_site_url: ParsedUrl,
    pub api_root_url: ParsedUrl,
    pub api_details: WpApiDetails,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum AutoDiscoveryAttemptFailure {
    #[error("{error}")]
    ParseSiteUrl { error: ParseUrlError },
    #[error("{error}")]
    FetchApiRootUrl {
        parsed_site_url: ParsedUrl,
        error: RequestExecutionError,
    },
    #[error("{error}")]
    ParseApiRootUrl {
        parsed_site_url: ParsedUrl,
        error: ParseApiRootUrlError,
    },
    #[error("{error}")]
    FetchApiDetails {
        parsed_site_url: ParsedUrl,
        api_root_url: ParsedUrl,
        error: RequestExecutionError,
    },
    #[error("Failed to parse api details: {:#?}", parsing_error_message)]
    ParseApiDetails {
        parsed_site_url: ParsedUrl,
        api_root_url: ParsedUrl,
        parsing_error_message: String,
    },
    #[error("{}", reason.as_ref().map(|r| r.error_message(parsed_site_url)).unwrap_or("Application Passwords are not supported".to_string()))]
    ApplicationPasswordsNotSupported {
        parsed_site_url: ParsedUrl,
        api_root_url: ParsedUrl,
        api_details: WpApiDetails,
        reason: Option<ApplicationPasswordsNotSupportedReason>,
    },
}

#[derive(Debug, Clone)]
pub enum ApplicationPasswordsNotSupportedReason {
    ApplicationPasswordBlockedByPlugin {
        plugin: KnownApplicationPasswordBlockingPlugin,
    },
    ApplicationPasswordBlockedByMultiplePlugins,
    SiteIsLocalDevelopmentEnvironment,
    ApplicationPasswordsDisabledForHttpSite,
}

impl ApplicationPasswordsNotSupportedReason {
    fn error_message(&self, parsed_site_url: impl Display) -> String {
        match self {
            Self::ApplicationPasswordBlockedByPlugin { plugin } => format!("Unable to login to {} – the {} plugin might have disabled Application Passwords. Please visit {} to learn more", parsed_site_url, plugin.name, plugin.support_url),
            Self::ApplicationPasswordBlockedByMultiplePlugins => format!("Unable to login to {} – there are multiple installed plugins that might have disabled Application Passwords. Please disable them and try again.", parsed_site_url),
            Self::SiteIsLocalDevelopmentEnvironment => "This site is a local development environment. You'll need to enable application passwords to connect to it with the app.".to_string(),
            Self::ApplicationPasswordsDisabledForHttpSite => "Application Passwords is not enabled for this site – this is likely because we can't establish a secure connection to it. Please add an SSL certificate to this site and try again.".to_string(),
        }
    }
}

impl AutoDiscoveryAttemptFailure {
    pub fn is_network_error(&self) -> bool {
        match self {
            AutoDiscoveryAttemptFailure::FetchApiRootUrl { .. } => true,
            AutoDiscoveryAttemptFailure::FetchApiDetails { .. } => true,
            AutoDiscoveryAttemptFailure::ParseSiteUrl { .. } => false,
            AutoDiscoveryAttemptFailure::ParseApiRootUrl { .. } => false,
            AutoDiscoveryAttemptFailure::ParseApiDetails { .. } => false,
            AutoDiscoveryAttemptFailure::ApplicationPasswordsNotSupported { .. } => false,
        }
    }

    pub fn parsed_site_url(&self) -> Option<&ParsedUrl> {
        match self {
            AutoDiscoveryAttemptFailure::ParseSiteUrl { .. } => None,
            AutoDiscoveryAttemptFailure::FetchApiRootUrl {
                parsed_site_url, ..
            } => Some(parsed_site_url),
            AutoDiscoveryAttemptFailure::ParseApiRootUrl {
                parsed_site_url, ..
            } => Some(parsed_site_url),
            AutoDiscoveryAttemptFailure::FetchApiDetails {
                parsed_site_url, ..
            } => Some(parsed_site_url),
            AutoDiscoveryAttemptFailure::ParseApiDetails {
                parsed_site_url, ..
            } => Some(parsed_site_url),
            AutoDiscoveryAttemptFailure::ApplicationPasswordsNotSupported {
                parsed_site_url,
                ..
            } => Some(parsed_site_url),
        }
    }

    // If it failed while parsing the site url or fetching the api root url, we never tried to
    // parse it, so we return `None`
    //
    // If we fail to parse with `AutoDiscoveryAttemptFailure::ParseApiRootUrl`, we return
    // `Some(true)`, because that's exactly when the failure happened.
    //
    // If an error occurs after parsing the api root url, we return `Some(false)`.
    pub fn has_failed_to_parse_api_root_url(&self) -> Option<bool> {
        match self {
            AutoDiscoveryAttemptFailure::ParseSiteUrl { .. } => None,
            AutoDiscoveryAttemptFailure::FetchApiRootUrl { .. } => None,
            AutoDiscoveryAttemptFailure::ParseApiRootUrl { .. } => Some(true),
            AutoDiscoveryAttemptFailure::FetchApiDetails { .. } => Some(false),
            AutoDiscoveryAttemptFailure::ParseApiDetails { .. } => Some(false),
            AutoDiscoveryAttemptFailure::ApplicationPasswordsNotSupported { .. } => Some(false),
        }
    }

    pub fn api_root_url(&self) -> Option<&ParsedUrl> {
        match self {
            AutoDiscoveryAttemptFailure::ParseSiteUrl { .. } => None,
            AutoDiscoveryAttemptFailure::FetchApiRootUrl { .. } => None,
            AutoDiscoveryAttemptFailure::ParseApiRootUrl { .. } => None,
            AutoDiscoveryAttemptFailure::FetchApiDetails { api_root_url, .. } => Some(api_root_url),
            AutoDiscoveryAttemptFailure::ParseApiDetails { api_root_url, .. } => Some(api_root_url),
            AutoDiscoveryAttemptFailure::ApplicationPasswordsNotSupported {
                api_root_url, ..
            } => Some(api_root_url),
        }
    }

    // If it failed while parsing the site url, fetching the api root url, parsing the api root url
    // or fetching the api details, we never tried to parse it, so we return `None`.
    //
    // If we fail to parse with `AutoDiscoveryAttemptFailure::ParseApiDetails`, we return
    // `Some(true)`, because that's exactly when the failure happened.
    pub fn has_failed_to_parse_api_details(&self) -> Option<bool> {
        match self {
            AutoDiscoveryAttemptFailure::ParseSiteUrl { .. } => None,
            AutoDiscoveryAttemptFailure::FetchApiRootUrl { .. } => None,
            AutoDiscoveryAttemptFailure::ParseApiRootUrl { .. } => None,
            AutoDiscoveryAttemptFailure::FetchApiDetails { .. } => None,
            AutoDiscoveryAttemptFailure::ParseApiDetails { .. } => Some(true),
            AutoDiscoveryAttemptFailure::ApplicationPasswordsNotSupported { .. } => Some(false),
        }
    }

    pub fn is_application_passwords_disabled(&self) -> Option<bool> {
        match self {
            AutoDiscoveryAttemptFailure::ParseSiteUrl { .. } => None,
            AutoDiscoveryAttemptFailure::FetchApiRootUrl { .. } => None,
            AutoDiscoveryAttemptFailure::ParseApiRootUrl { .. } => None,
            AutoDiscoveryAttemptFailure::FetchApiDetails { .. } => None,
            AutoDiscoveryAttemptFailure::ParseApiDetails { .. } => None,
            AutoDiscoveryAttemptFailure::ApplicationPasswordsNotSupported { .. } => Some(true),
        }
    }
}

impl From<FindApiRootLinkHeaderFailure> for AutoDiscoveryAttemptFailure {
    fn from(value: FindApiRootLinkHeaderFailure) -> Self {
        match value {
            FindApiRootLinkHeaderFailure::ParseSiteUrl { error } => Self::ParseSiteUrl { error },
            FindApiRootLinkHeaderFailure::FetchApiRootUrl {
                parsed_site_url,
                error,
            } => Self::FetchApiRootUrl {
                parsed_site_url,
                error,
            },
            FindApiRootLinkHeaderFailure::ParseApiRootUrl {
                parsed_site_url,
                error,
            } => Self::ParseApiRootUrl {
                parsed_site_url,
                error,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, uniffi::Enum)]
pub enum AutoDiscoveryAttemptType {
    UserInput,
    AutoHttps,
    AutoRemoveWpAdminSuffix,
    AutoRemoveWpLoginSuffix,
}

pub(crate) fn construct_attempts(input_site_url: String) -> Vec<AutoDiscoveryAttempt> {
    let mut attempts = vec![AutoDiscoveryAttempt::with_user_input_attempt_type(
        input_site_url.clone(),
    )];
    if !input_site_url.starts_with("http") {
        attempts.push(AutoDiscoveryAttempt::with_auto_https_attempt_type(format!(
            "https://{}",
            input_site_url
        )));
    } else if !input_site_url.starts_with("https") {
        // Url starts with `http`, but not `https`
        attempts.push(AutoDiscoveryAttempt::with_auto_https_attempt_type(
            input_site_url.replacen("http", "https", 1),
        ));
    }
    if let Some(a) = input_site_url
        .strip_suffix("wp-admin")
        .or_else(|| input_site_url.strip_suffix("wp-admin/"))
        .or_else(|| input_site_url.strip_suffix("wp-admin.php"))
    {
        attempts.push(AutoDiscoveryAttempt::with_auto_remove_wp_admin_suffix_attempt_type(a));
    }
    if let Some(a) = input_site_url
        .strip_suffix("wp-login")
        .or_else(|| input_site_url.strip_suffix("wp-login/"))
        .or_else(|| input_site_url.strip_suffix("wp-login.php"))
    {
        attempts.push(AutoDiscoveryAttempt::with_auto_remove_wp_login_suffix_attempt_type(a));
    }
    attempts
}

#[derive(Debug, Clone, thiserror::Error, uniffi::Error, WpDeriveLocalizable)]
pub enum ParseApiRootUrlError {
    ApiRootLinkHeaderNotFound {
        status_code: u16,
        header_map: Arc<WpNetworkHeaderMap>,
    },
}

impl WpSupportsLocalization for ParseApiRootUrlError {
    fn message_bundle(&self) -> wp_localization::MessageBundle {
        match self {
            ParseApiRootUrlError::ApiRootLinkHeaderNotFound {
                status_code,
                header_map,
            } => WpMessages::api_root_link_header_not_found(
                status_code.to_string(),
                format!("{:#?}", header_map),
            ),
        }
    }
}

#[derive(Debug, thiserror::Error, uniffi::Error, WpDeriveLocalizable)]
pub enum FetchApiDetailsError {
    RequestExecutionFailed {
        status_code: Option<u16>,
        redirects: Option<Vec<WpRedirect>>,
        reason: RequestExecutionErrorReason,
    },
    ApiDetailsCouldntBeParsed {
        reason: String,
        response: String,
    },
}

impl WpSupportsLocalization for FetchApiDetailsError {
    fn message_bundle(&self) -> wp_localization::MessageBundle {
        match self {
            FetchApiDetailsError::RequestExecutionFailed {
                status_code,
                reason,
                ..
            } => WpMessages::fetch_api_details_request_execution_failed(
                format!("{:#?}", status_code),
                reason.to_string(),
            ),
            FetchApiDetailsError::ApiDetailsCouldntBeParsed { response, .. } => {
                WpMessages::fetch_api_details_api_details_couldnt_be_parsed(response.to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AutoDiscoveryAttempt as A;
    use super::*;
    use rstest::*;

    #[rstest]
    #[case::localhost("localhost", vec![A::with_user_input_attempt_type("localhost"), A::with_auto_https_attempt_type("https://localhost")])]
    #[case::http_localhost("http://localhost", vec![A::with_user_input_attempt_type("http://localhost"), A::with_auto_https_attempt_type("https://localhost")])]
    #[case::http_localhost_wp_json("http://localhost/wp-json", vec![A::with_user_input_attempt_type("http://localhost/wp-json"), A::with_auto_https_attempt_type("https://localhost/wp-json")])]
    #[case::http_localhost_wp_admin_php("http://localhost/wp-admin.php", vec![A::with_user_input_attempt_type("http://localhost/wp-admin.php"), A::with_auto_https_attempt_type("https://localhost/wp-admin.php"), A::with_auto_remove_wp_admin_suffix_attempt_type("http://localhost/")])]
    #[case::http_localhost_wp_admin("http://localhost/wp-admin", vec![A::with_user_input_attempt_type("http://localhost/wp-admin"), A::with_auto_https_attempt_type("https://localhost/wp-admin") ,A::with_auto_remove_wp_admin_suffix_attempt_type("http://localhost/")])]
    #[case::http_localhost_wp_admin_slash("http://localhost/wp-admin/", vec![A::with_user_input_attempt_type("http://localhost/wp-admin/"), A::with_auto_https_attempt_type("https://localhost/wp-admin/"), A::with_auto_remove_wp_admin_suffix_attempt_type("http://localhost/")])]
    #[case::http_localhost_wp_login_php("http://localhost/wp-login.php", vec![A::with_user_input_attempt_type("http://localhost/wp-login.php"), A::with_auto_https_attempt_type("https://localhost/wp-login.php"), A::with_auto_remove_wp_login_suffix_attempt_type("http://localhost/")])]
    #[case::http_localhost_wp_login("http://localhost/wp-login", vec![A::with_user_input_attempt_type("http://localhost/wp-login"), A::with_auto_https_attempt_type("https://localhost/wp-login"), A::with_auto_remove_wp_login_suffix_attempt_type("http://localhost/")])]
    #[case::http_localhost_wp_login_slash("http://localhost/wp-login/", vec![A::with_user_input_attempt_type("http://localhost/wp-login/"), A::with_auto_https_attempt_type("https://localhost/wp-login/"), A::with_auto_remove_wp_login_suffix_attempt_type("http://localhost/")])]
    #[case::automatticwidgets_wp_json("automatticwidgets.wpcomstaging.com/wp-json", vec![A::with_user_input_attempt_type("automatticwidgets.wpcomstaging.com/wp-json"), A::with_auto_https_attempt_type("https://automatticwidgets.wpcomstaging.com/wp-json")])]
    #[case::automatticwidgets_https("https://automatticwidgets.wpcomstaging.com", vec![A::with_user_input_attempt_type("https://automatticwidgets.wpcomstaging.com")])]
    #[case::automatticwidgets_https_wp_json(
        "https://automatticwidgets.wpcomstaging.com/wp-json",
        vec![A::with_user_input_attempt_type("https://automatticwidgets.wpcomstaging.com/wp-json")]
    )]
    fn test_construct_attempts(
        #[case] input_site_url: &str,
        #[case] expected_attempts: Vec<AutoDiscoveryAttempt>,
    ) {
        assert_eq!(
            construct_attempts(input_site_url.to_string()),
            expected_attempts
        )
    }

    #[rstest]
    #[case("http://localhost", true)]
    #[case("http://localhost.local", true)]
    #[case("http://localhost.test", true)]
    #[case("http://example.com", false)]
    #[case("http://127.0.0.1", true)]
    #[case("http://example.com", false)]
    fn test_is_local_dev_environment_url(#[case] url: &str, #[case] expected: bool) {
        let parsed_url = ParsedUrl::parse(url).unwrap();
        assert_eq!(is_local_dev_environment_url(&parsed_url), expected);
    }

    #[test]
    fn test_parse_api_root_url_error_message_bundle() {
        let e = example_parse_api_root_url_error();

        let message_bundle = e.message_bundle();
        assert_eq!(message_bundle.key(), "api_root_link_header_not_found");
        let message_args = message_bundle.args().unwrap();
        assert_eq!(message_args["status_code"], "404");
        assert_eq!(message_args["header_map"], "WpNetworkHeaderMap {\n    inner: {\n        \"accept\": \"application/json\",\n    },\n}");
    }

    #[test]
    fn test_parse_api_root_url_error_derive_localizable() {
        let expected="Api root link header not found!\nStatus Code: '\u{2068}404\u{2069}'\nHeader Map: '\u{2068}WpNetworkHeaderMap {\n    inner: {\n        \"accept\": \"application/json\",\n    },\n}\u{2069}'";

        assert_eq!(example_parse_api_root_url_error().to_string(), expected);
    }

    fn example_parse_api_root_url_error() -> ParseApiRootUrlError {
        let header_map: WpNetworkHeaderMap = {
            let mut map = http::HeaderMap::new();
            map.insert(
                http::header::ACCEPT,
                http::HeaderValue::from_static("application/json"),
            );
            map.into()
        };
        ParseApiRootUrlError::ApiRootLinkHeaderNotFound {
            status_code: 404,
            header_map: header_map.into(),
        }
    }
}

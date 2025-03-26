use super::WpApiDetails;
use crate::{
    ParseUrlError, ParsedUrl, RequestExecutionError, RequestExecutionErrorReason, WpErrorCode,
    login::KnownApplicationPasswordBlockingPlugin, request::WpRedirect,
};
use scraper::{Html, Selector};
use serde::Deserialize;
use std::{collections::HashMap, sync::Arc};
use wp_localization::{MessageBundle, WpMessages, WpSupportsLocalization};
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

    fn with_auto_stripped_https_attempt_type(attempt_site_url: impl Into<String>) -> Self {
        Self::new(
            attempt_site_url,
            AutoDiscoveryAttemptType::AutoStrippedHttps,
        )
    }

    fn maybe_auto_stripped_https_attempt_type_from_input(
        input_site_url: impl Into<String>,
    ) -> Option<Self> {
        let input_url_as_string: String = input_site_url.into();
        let processed_site_url = input_url_as_string
            .strip_suffix("wp-admin")
            .or_else(|| input_url_as_string.strip_suffix("wp-admin/"))
            .or_else(|| input_url_as_string.strip_suffix("wp-admin.php"))
            .or_else(|| input_url_as_string.strip_suffix("wp-login"))
            .or_else(|| input_url_as_string.strip_suffix("wp-login/"))
            .or_else(|| input_url_as_string.strip_suffix("wp-login.php"))
            .unwrap_or(input_url_as_string.as_str());
        let url = if !processed_site_url.starts_with("http") {
            format!("https://{}", processed_site_url)
        } else if !processed_site_url.starts_with("https") {
            processed_site_url.replacen("http", "https", 1)
        } else {
            if processed_site_url == input_url_as_string {
                // The `input_site_url` hasn't been modified in any way, so no need to add an
                // additional attempt for it
                return None;
            }
            processed_site_url.to_string()
        };
        Some(Self::with_auto_stripped_https_attempt_type(url))
    }
}

#[derive(Debug, uniffi::Record)]
pub struct AutoDiscoveryUniffiResult {
    pub user_input_attempt: Arc<AutoDiscoveryAttemptResult>,
    pub successful_attempt: Option<Arc<AutoDiscoveryAttemptResult>>,
    pub auto_stripped_https_attempt: Option<Arc<AutoDiscoveryAttemptResult>>,
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
            auto_stripped_https_attempt: get_attempt_result(
                AutoDiscoveryAttemptType::AutoStrippedHttps,
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

    pub fn get_attempt(
        &self,
        attempt_type: &AutoDiscoveryAttemptType,
    ) -> Option<&AutoDiscoveryAttemptResult> {
        self.attempts.get(attempt_type)
    }
}

#[derive(Debug, Clone, uniffi::Object)]
pub struct AutoDiscoveryAttemptResult {
    pub attempt_type: AutoDiscoveryAttemptType,
    pub attempt_site_url: String,
    pub api_discovery_result: Result<AutoDiscoveryAttemptSuccess, AutoDiscoveryAttemptFailure>,
}

#[uniffi::export]
impl AutoDiscoveryAttemptResult {
    fn attempt_site_url(&self) -> String {
        self.attempt_site_url.clone()
    }

    fn api_discovery_error(&self) -> Option<AutoDiscoveryAttemptFailure> {
        self.api_discovery_result.as_ref().err().cloned()
    }

    fn error_message(&self) -> Option<String> {
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

    fn parsed_site_url(&self) -> Option<Arc<ParsedUrl>> {
        match &self.api_discovery_result {
            Ok(success) => Some(Arc::clone(&success.parsed_site_url)),
            Err(error) => error.parsed_site_url().map(|p| Arc::new(p.clone())),
        }
    }

    fn api_root_url(&self) -> Option<Arc<ParsedUrl>> {
        match &self.api_discovery_result {
            Ok(success) => Some(Arc::clone(&success.api_root_url)),
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
}

impl AutoDiscoveryAttemptResult {
    pub(crate) fn from_parse_site_url_error(
        attempt: AutoDiscoveryAttempt,
        error: ParseUrlError,
    ) -> Self {
        Self {
            attempt_type: attempt.attempt_type,
            attempt_site_url: attempt.attempt_site_url,
            api_discovery_result: Err(AutoDiscoveryAttemptFailure::ParseSiteUrl {
                error: error.clone(),
            }),
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

#[derive(Debug, Clone)]
pub struct ApiRootUrl(pub Arc<ParsedUrl>);

#[derive(Debug, Clone)]
pub struct FetchWpJsonSuccess {
    pub wp_json_url: Arc<ParsedUrl>,
    pub root_wp_json: RootWpJson,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RootWpJson {
    pub namespaces: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ParseHomepageResult {
    /// `href` attribute of a link tag if it has `rel` attribute of "https://api.w.org/".
    /// For example:
    /// <link href="http://localhost/wp-json/" rel="https://api.w.org/">
    pub api_root_url_from_link_tag: Option<Arc<ParsedUrl>>,
    /// Whether the HTML has 'generator' meta tag that mentions `WordPress`
    pub has_wordpress_generator_meta_tag: bool,
    /// Whether the HTML `link`, `script`, `style` tags mention `wp-content`
    pub mentions_wp_content: bool,
    /// Whether the HTML `link`, `script`, `style` tags mention `wp-includes`
    pub mentions_wp_includes: bool,
}

impl ParseHomepageResult {
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
                    .and_then(|u| ParsedUrl::parse(u).ok().map(Arc::new))
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

    pub fn does_look_like_a_wp_site(&self) -> bool {
        self.api_root_url_from_link_tag.is_some()
            || self.mentions_wp_content
            || self.mentions_wp_includes
            || self.has_wordpress_generator_meta_tag
    }
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

#[derive(Debug, Clone)]
pub struct AutoDiscoveryAttemptSuccess {
    pub parsed_site_url: Arc<ParsedUrl>,
    pub api_root_url: Arc<ParsedUrl>,
    pub api_details: WpApiDetails,
}

#[derive(Debug, Clone, thiserror::Error, uniffi::Error, WpDeriveLocalizable)]
pub enum AutoDiscoveryAttemptFailure {
    ParseSiteUrl {
        error: ParseUrlError,
    },
    FindApiRoot {
        parsed_site_url: Arc<ParsedUrl>,
        find_api_root_failure: FindApiRootFailure,
    },
    FetchAndParseApiRoot {
        parsed_site_url: Arc<ParsedUrl>,
        api_root_url: Arc<ParsedUrl>,
        fetch_and_parse_api_root_failure: FetchAndParseApiRootFailure,
    },
}

impl WpSupportsLocalization for AutoDiscoveryAttemptFailure {
    fn message_bundle(&self) -> MessageBundle {
        match self {
            Self::ParseSiteUrl { error } => error.message_bundle(),
            Self::FindApiRoot {
                find_api_root_failure,
                ..
            } => find_api_root_failure.message_bundle(),
            Self::FetchAndParseApiRoot {
                parsed_site_url,
                fetch_and_parse_api_root_failure,
                ..
            } => fetch_and_parse_api_root_failure.message_bundle(parsed_site_url),
        }
    }
}

#[derive(Debug, Clone, uniffi::Enum)]
pub enum FindApiRootFailure {
    FetchHomepage { error: RequestExecutionError },
    // if no WP mentions
    ProbablyNotAWordPressSite,
    // WP mentions
    RestApiDisabled,
}

impl FindApiRootFailure {
    fn message_bundle(&self) -> MessageBundle {
        match self {
            Self::FetchHomepage { error } => error.message_bundle(),
            Self::ProbablyNotAWordPressSite => WpMessages::probably_not_wordpress_site(),
            Self::RestApiDisabled => WpMessages::rest_api_disabled(),
        }
    }
}

#[derive(Debug, Clone, uniffi::Enum)]
pub enum FetchAndParseApiRootFailure {
    FetchApiRoot {
        error: RequestExecutionError,
    },
    ParseApiRoot {
        parsing_error_message: String,
    },
    WpError {
        error_code: WpErrorCode,
        error_message: String,
        status_code: u16,
    },
    ApplicationPasswordsNotSupported {
        api_details: Arc<WpApiDetails>,
        reason: Option<ApplicationPasswordsNotSupportedReason>,
    },
}

impl FetchAndParseApiRootFailure {
    fn message_bundle(&self, parsed_site_url: impl std::fmt::Display) -> MessageBundle {
        match self {
            Self::FetchApiRoot { error } => error.message_bundle(),
            Self::ParseApiRoot { .. } => WpMessages::parse_api_root(),
            Self::WpError { error_message, .. } => WpMessages::site_error_message(error_message),
            Self::ApplicationPasswordsNotSupported { reason, .. } => reason
                .as_ref()
                .map(|r| r.message_bundle(parsed_site_url))
                .unwrap_or(WpMessages::application_passwords_not_supported()),
        }
    }
}

#[derive(Debug, Clone, uniffi::Enum)]
pub enum ApplicationPasswordsNotSupportedReason {
    ApplicationPasswordBlockedByPlugin {
        plugin: KnownApplicationPasswordBlockingPlugin,
    },
    ApplicationPasswordBlockedByMultiplePlugins,
    SiteIsLocalDevelopmentEnvironment,
    ApplicationPasswordsDisabledForHttpSite,
}

impl ApplicationPasswordsNotSupportedReason {
    fn message_bundle(&self, parsed_site_url: impl std::fmt::Display) -> MessageBundle {
        match self {
            Self::ApplicationPasswordBlockedByPlugin { plugin } => {
                WpMessages::application_password_blocked_by_plugin(
                    parsed_site_url.to_string(),
                    &plugin.name,
                    &plugin.support_url,
                )
            }
            Self::ApplicationPasswordBlockedByMultiplePlugins => {
                WpMessages::application_password_blocked_by_multiple_plugins(
                    parsed_site_url.to_string(),
                )
            }
            Self::SiteIsLocalDevelopmentEnvironment => {
                WpMessages::site_is_local_development_environment()
            }
            Self::ApplicationPasswordsDisabledForHttpSite => {
                WpMessages::application_passwords_disabled_for_http_site()
            }
        }
    }
}

impl AutoDiscoveryAttemptFailure {
    pub fn from_find_api_root_failure(
        parsed_site_url: Arc<ParsedUrl>,
        find_api_root_failure: FindApiRootFailure,
    ) -> Self {
        Self::FindApiRoot {
            parsed_site_url,
            find_api_root_failure,
        }
    }

    pub fn from_fetch_and_parse_api_root_failure(
        parsed_site_url: Arc<ParsedUrl>,
        api_root_url: Arc<ParsedUrl>,
        fetch_and_parse_api_root_failure: FetchAndParseApiRootFailure,
    ) -> Self {
        Self::FetchAndParseApiRoot {
            parsed_site_url,
            api_root_url,
            fetch_and_parse_api_root_failure,
        }
    }

    pub fn is_network_error(&self) -> bool {
        match self {
            Self::ParseSiteUrl { .. } => false,
            Self::FindApiRoot {
                find_api_root_failure,
                ..
            } => find_api_root_failure.is_network_error(),
            Self::FetchAndParseApiRoot {
                fetch_and_parse_api_root_failure,
                ..
            } => fetch_and_parse_api_root_failure.is_network_error(),
        }
    }

    pub fn parsed_site_url(&self) -> Option<&ParsedUrl> {
        match self {
            Self::ParseSiteUrl { .. } => None,
            Self::FindApiRoot {
                parsed_site_url, ..
            } => Some(parsed_site_url),
            Self::FetchAndParseApiRoot {
                parsed_site_url, ..
            } => Some(parsed_site_url),
        }
    }

    pub fn api_root_url(&self) -> Option<&ParsedUrl> {
        match self {
            Self::ParseSiteUrl { .. } => None,
            Self::FindApiRoot { .. } => None,
            Self::FetchAndParseApiRoot { api_root_url, .. } => Some(api_root_url),
        }
    }

    pub fn is_application_passwords_disabled(&self) -> Option<bool> {
        match self {
            Self::ParseSiteUrl { .. } => None,
            Self::FindApiRoot { .. } => None,
            Self::FetchAndParseApiRoot {
                fetch_and_parse_api_root_failure,
                ..
            } => fetch_and_parse_api_root_failure.is_application_passwords_disabled(),
        }
    }
}

impl FindApiRootFailure {
    pub fn is_network_error(&self) -> bool {
        match self {
            Self::FetchHomepage { .. } => true,
            Self::ProbablyNotAWordPressSite { .. } => false,
            Self::RestApiDisabled { .. } => false,
        }
    }
}

impl FetchAndParseApiRootFailure {
    pub fn is_network_error(&self) -> bool {
        match self {
            Self::FetchApiRoot { .. } => true,
            Self::ParseApiRoot { .. } => false,
            Self::WpError { .. } => false,
            Self::ApplicationPasswordsNotSupported { .. } => false,
        }
    }

    pub fn is_application_passwords_disabled(&self) -> Option<bool> {
        match self {
            Self::FetchApiRoot { .. } => None,
            Self::ParseApiRoot { .. } => None,
            Self::WpError { .. } => None,
            Self::ApplicationPasswordsNotSupported { .. } => Some(true),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, uniffi::Enum)]
pub enum AutoDiscoveryAttemptType {
    UserInput,
    // Removes `/wp-login` & `/wp-admin` suffixes and replaces `http` with `https`
    AutoStrippedHttps,
}

pub(crate) fn construct_attempts(input_site_url: String) -> Vec<AutoDiscoveryAttempt> {
    let mut attempts = vec![AutoDiscoveryAttempt::with_user_input_attempt_type(
        input_site_url.clone(),
    )];
    if let Some(auto_https_attempt) =
        AutoDiscoveryAttempt::maybe_auto_stripped_https_attempt_type_from_input(
            input_site_url.as_str(),
        )
    {
        attempts.push(auto_https_attempt);
    }
    attempts
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
    fn message_bundle(&self) -> MessageBundle {
        match self {
            FetchApiDetailsError::RequestExecutionFailed { reason, .. } => reason.message_bundle(),
            FetchApiDetailsError::ApiDetailsCouldntBeParsed { reason, .. } => {
                WpMessages::response_parsing_error(reason)
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
    #[case::localhost("localhost", vec![A::with_user_input_attempt_type("localhost"), A::with_auto_stripped_https_attempt_type("https://localhost")])]
    #[case::http_localhost("http://localhost", vec![A::with_user_input_attempt_type("http://localhost"), A::with_auto_stripped_https_attempt_type("https://localhost")])]
    #[case::http_localhost_wp_json("http://localhost/wp-json", vec![A::with_user_input_attempt_type("http://localhost/wp-json"), A::with_auto_stripped_https_attempt_type("https://localhost/wp-json")])]
    #[case::http_localhost_wp_admin_php("http://localhost/wp-admin.php", vec![A::with_user_input_attempt_type("http://localhost/wp-admin.php"), A::with_auto_stripped_https_attempt_type("https://localhost/")])]
    #[case::http_localhost_wp_admin("http://localhost/wp-admin", vec![A::with_user_input_attempt_type("http://localhost/wp-admin"), A::with_auto_stripped_https_attempt_type("https://localhost/")])]
    #[case::http_localhost_wp_admin_slash("http://localhost/wp-admin/", vec![A::with_user_input_attempt_type("http://localhost/wp-admin/"), A::with_auto_stripped_https_attempt_type("https://localhost/")])]
    #[case::http_localhost_wp_login_php("http://localhost/wp-login.php", vec![A::with_user_input_attempt_type("http://localhost/wp-login.php"), A::with_auto_stripped_https_attempt_type("https://localhost/")])]
    #[case::http_localhost_wp_login("http://localhost/wp-login", vec![A::with_user_input_attempt_type("http://localhost/wp-login"), A::with_auto_stripped_https_attempt_type("https://localhost/")])]
    #[case::http_localhost_wp_login_slash("http://localhost/wp-login/", vec![A::with_user_input_attempt_type("http://localhost/wp-login/"), A::with_auto_stripped_https_attempt_type("https://localhost/")])]
    #[case::automatticwidgets_wp_json("automatticwidgets.wpcomstaging.com/wp-json", vec![A::with_user_input_attempt_type("automatticwidgets.wpcomstaging.com/wp-json"), A::with_auto_stripped_https_attempt_type("https://automatticwidgets.wpcomstaging.com/wp-json")])]
    #[case::automatticwidgets_https("https://automatticwidgets.wpcomstaging.com", vec![A::with_user_input_attempt_type("https://automatticwidgets.wpcomstaging.com")])]
    #[case::automatticwidgets_https_wp_json(
        "https://automatticwidgets.wpcomstaging.com/wp-json",
        vec![A::with_user_input_attempt_type("https://automatticwidgets.wpcomstaging.com/wp-json")]
    )]
    #[case::automatticwidgets_https_wp_admin("https://automatticwidgets.wpcomstaging.com/wp-admin", vec![A::with_user_input_attempt_type("https://automatticwidgets.wpcomstaging.com/wp-admin"), A::with_auto_stripped_https_attempt_type("https://automatticwidgets.wpcomstaging.com/")])]
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
}

use super::WpApiDetails;
use crate::{request::WpNetworkHeaderMap, ParseUrlError, ParsedUrl, RequestExecutionError};
use std::{collections::HashMap, sync::Arc};

use crate::LOCALES;

use fluent_bundle::FluentValue;
use fluent_langneg::convert_vec_str_to_langids_lossy;
use fluent_langneg::negotiate_languages;
use fluent_langneg::NegotiationStrategy;
use fluent_templates::Loader;

const API_ROOT_LINK_HEADER: &str = "https://api.w.org/";

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
                AutoDiscoveryAttemptType::AutoDotPhpExtensionForWpAdmin,
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
    pub result: Result<AutoDiscoveryAttemptSuccess, AutoDiscoveryAttemptFailure>,
}

#[uniffi::export]
impl AutoDiscoveryAttemptResult {
    fn attempt_site_url(&self) -> String {
        self.attempt_site_url.clone()
    }

    fn error_message(&self, locale_id: String) -> Option<String> {
        match &self.result {
            Ok(_) => None,
            Err(error) => error.localized_error_message(locale_id),
        }
    }

    fn is_successful(&self) -> bool {
        self.result.is_ok()
    }

    fn is_network_error(&self) -> bool {
        match &self.result {
            Ok(_) => false,
            Err(error) => error.is_network_error(),
        }
    }

    fn is_user_input_attempt(&self) -> bool {
        matches!(self.attempt_type, AutoDiscoveryAttemptType::UserInput)
    }

    fn has_failed_to_parse_site_url(&self) -> bool {
        match &self.result {
            Ok(success) => false,
            Err(error) => error.parsed_site_url().is_none(),
        }
    }

    fn has_failed_to_parse_api_root_url(&self) -> Option<bool> {
        match &self.result {
            Ok(success) => Some(false),
            Err(error) => error.has_failed_to_parse_api_root_url(),
        }
    }

    fn has_failed_to_parse_api_details(&self) -> Option<bool> {
        match &self.result {
            Ok(success) => Some(false),
            Err(error) => error.has_failed_to_parse_api_details(),
        }
    }

    fn parsed_site_url(&self) -> Option<Arc<ParsedUrl>> {
        match &self.result {
            Ok(success) => Some(Arc::new(success.parsed_site_url.clone())),
            Err(error) => error.parsed_site_url().map(|p| Arc::new(p.clone())),
        }
    }

    fn api_root_url(&self) -> Option<Arc<ParsedUrl>> {
        match &self.result {
            Ok(success) => Some(Arc::new(success.api_root_url.clone())),
            Err(error) => error.api_root_url().map(|p| Arc::new(p.clone())),
        }
    }

    fn api_details(&self) -> Option<Arc<WpApiDetails>> {
        match &self.result {
            Ok(success) => Some(Arc::new(success.api_details.clone())),
            Err(_) => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AutoDiscoveryAttemptSuccess {
    pub parsed_site_url: ParsedUrl,
    pub api_root_url: ParsedUrl,
    pub api_details: WpApiDetails,
}

#[derive(Debug, Clone)]
pub enum AutoDiscoveryAttemptFailure {
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
    FetchApiDetails {
        parsed_site_url: ParsedUrl,
        api_root_url: ParsedUrl,
        error: RequestExecutionError,
    },
    ParseApiDetails {
        parsed_site_url: ParsedUrl,
        api_root_url: ParsedUrl,
        parsing_error_message: String,
    },
}

impl AutoDiscoveryAttemptFailure {
    pub fn into_attempt_result(
        self,
        attempt_type: AutoDiscoveryAttemptType,
        attempt_site_url: String,
    ) -> AutoDiscoveryAttemptResult {
        AutoDiscoveryAttemptResult {
            attempt_type,
            attempt_site_url,
            result: Err(self),
        }
    }

    pub fn error_message(&self) -> String {
        match self {
            AutoDiscoveryAttemptFailure::ParseSiteUrl { error } => error.to_string(),
            AutoDiscoveryAttemptFailure::FetchApiRootUrl { error, .. } => error.to_string(),
            AutoDiscoveryAttemptFailure::ParseApiRootUrl { error, .. } => error.to_string(),
            AutoDiscoveryAttemptFailure::FetchApiDetails { error, .. } => error.to_string(),
            AutoDiscoveryAttemptFailure::ParseApiDetails {
                parsing_error_message,
                ..
            } => {
                format!("Failed to parse api details: {:#?}", parsing_error_message)
            }
        }
    }

    pub fn is_network_error(&self) -> bool {
        match self {
            AutoDiscoveryAttemptFailure::FetchApiRootUrl { .. } => true,
            AutoDiscoveryAttemptFailure::FetchApiDetails { .. } => true,
            AutoDiscoveryAttemptFailure::ParseSiteUrl { .. } => false,
            AutoDiscoveryAttemptFailure::ParseApiRootUrl { .. } => false,
            AutoDiscoveryAttemptFailure::ParseApiDetails { .. } => false,
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
            AutoDiscoveryAttemptFailure::FetchApiDetails { api_root_url, .. } => Some(false),
            AutoDiscoveryAttemptFailure::ParseApiDetails { api_root_url, .. } => Some(false),
        }
    }

    pub fn api_root_url(&self) -> Option<&ParsedUrl> {
        match self {
            AutoDiscoveryAttemptFailure::ParseSiteUrl { .. } => None,
            AutoDiscoveryAttemptFailure::FetchApiRootUrl { .. } => None,
            AutoDiscoveryAttemptFailure::ParseApiRootUrl { .. } => None,
            AutoDiscoveryAttemptFailure::FetchApiDetails { api_root_url, .. } => Some(api_root_url),
            AutoDiscoveryAttemptFailure::ParseApiDetails { api_root_url, .. } => Some(api_root_url),
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
            AutoDiscoveryAttemptFailure::FetchApiDetails { api_root_url, .. } => None,
            AutoDiscoveryAttemptFailure::ParseApiDetails { api_root_url, .. } => Some(true),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, uniffi::Enum)]
pub enum AutoDiscoveryAttemptType {
    UserInput,
    AutoHttps,
    AutoDotPhpExtensionForWpAdmin,
}

impl AutoDiscoveryAttemptType {
    fn is_the_site_url_same_as_the_user_input(&self) -> bool {
        matches!(self, AutoDiscoveryAttemptType::UserInput)
    }
}

pub(crate) fn construct_attempts(input_site_url: String) -> Vec<AutoDiscoveryAttempt> {
    let mut attempts = vec![AutoDiscoveryAttempt::new(
        input_site_url.clone(),
        AutoDiscoveryAttemptType::UserInput,
    )];
    if !input_site_url.starts_with("http") {
        attempts.push(AutoDiscoveryAttempt::new(
            format!("https://{}", input_site_url),
            AutoDiscoveryAttemptType::AutoHttps,
        ))
    }
    if input_site_url.ends_with("wp-admin") {
        attempts.push(AutoDiscoveryAttempt::new(
            format!("{}.php", input_site_url),
            AutoDiscoveryAttemptType::AutoDotPhpExtensionForWpAdmin,
        ))
    } else if input_site_url.ends_with("wp-admin/") {
        let mut s = input_site_url.clone();
        s.pop()
            .expect("Already verified that there is at least one char");
        attempts.push(AutoDiscoveryAttempt::new(
            format!("{}.php", s),
            AutoDiscoveryAttemptType::AutoDotPhpExtensionForWpAdmin,
        ))
    }
    attempts
}

#[derive(Debug, Clone, thiserror::Error, uniffi::Error)]
pub enum ParseApiRootUrlError {
    #[error(
        "Api root link header not found!\nStatus Code: '{:#?}'\nHeader Map: '{:#?}'",
        status_code,
        header_map
    )]
    ApiRootLinkHeaderNotFound {
        site_url: Arc<ParsedUrl>,
        header_map: Arc<WpNetworkHeaderMap>,
        status_code: u16,
    },
}

impl WpLocalizedError for ParseApiRootUrlError {
    fn localized_error_message(&self, lang_id: String) -> Option<String> {
        let message = match self {
            ParseApiRootUrlError::ApiRootLinkHeaderNotFound {
                site_url,
                header_map,
                status_code,
            } => localized_message_with_args(
                &lang_id,
                "parse_api_root_url_error_api_root_link_header_not_found",
                &HashMap::from([("site_url", site_url.url().into())]),
            ),
        };
        Some(message)
    }
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum FetchApiDetailsError {
    #[error(
        "Request execution failed!\nStatus Code: '{:?}'.\nResponse: '{}'",
        status_code,
        reason
    )]
    RequestExecutionFailed {
        status_code: Option<u16>,
        reason: String,
    },
    #[error("Api details couldn't be parsed from response: {:?}", response)]
    ApiDetailsCouldntBeParsed { reason: String, response: String },
}

#[uniffi::export(with_foreign)]
pub trait WpLocalizedError: Send + Sync {
    fn localized_error_message(&self, locale_id: String) -> Option<String>;
}

impl WpLocalizedError for AutoDiscoveryAttemptFailure {
    fn localized_error_message(&self, lang_id: String) -> Option<String> {
        match self {
            AutoDiscoveryAttemptFailure::ParseSiteUrl { error } => Some(localized_message(
                &lang_id,
                "auto_discovery_attempt_failure_parse_site_url",
            )),
            AutoDiscoveryAttemptFailure::FetchApiRootUrl {
                parsed_site_url,
                error,
            } => Some(localized_message_with_args(
                &lang_id,
                "auto_discovery_attempt_failure_fetch_api_root_url",
                &HashMap::from([("site_url", parsed_site_url.url().into())]),
            )),
            AutoDiscoveryAttemptFailure::ParseApiRootUrl {
                parsed_site_url,
                error,
            } => error.localized_error_message(lang_id),
            AutoDiscoveryAttemptFailure::FetchApiDetails {
                parsed_site_url,
                api_root_url,
                error,
            } => Some(localized_message_with_args(
                &lang_id,
                "auto_discovery_attempt_failure_fetch_api_details",
                &HashMap::from([("api_url", api_root_url.url().into())]),
            )),
            AutoDiscoveryAttemptFailure::ParseApiDetails {
                parsed_site_url,
                api_root_url,
                parsing_error_message,
            } => Some(localized_message_with_args(
                &lang_id,
                "auto_discovery_attempt_failure_parse_api_details",
                &HashMap::from([("api_url", api_root_url.url().into())]),
            )),
        }
    }
}

fn locale_language_id(lang_id: &str) -> unic_langid::LanguageIdentifier {
    // Look up the translated message for `message_key` in `lang_id`.
    let requested = convert_vec_str_to_langids_lossy([lang_id]);
    let default: icu_locid::LanguageIdentifier = icu_locid::langid!("en-US");
    let available: Vec<icu_locid::LanguageIdentifier> = LOCALES
        .locales()
        .filter_map(|f| f.to_string().parse().ok())
        .collect();

    let supported = negotiate_languages(
        &requested,
        &available,
        Some(&default),
        NegotiationStrategy::Filtering,
    );

    supported
        .first()
        .unwrap_or(&&default)
        .to_string()
        .parse()
        .unwrap_or(unic_langid::langid!("en-US"))
}

pub fn localized_message(lang_id: &str, message_key: &str) -> String {
    LOCALES.lookup(&locale_language_id(lang_id), message_key)
}

pub fn localized_message_with_args<T: AsRef<str>>(
    lang_id: &str,
    message_key: &str,
    args: &HashMap<T, FluentValue>,
) -> String {
    LOCALES.lookup_with_args(&locale_language_id(lang_id), message_key, args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[rstest]
    #[case("localhost", vec![AutoDiscoveryAttempt::new("localhost", AutoDiscoveryAttemptType::UserInput), AutoDiscoveryAttempt::new("https://localhost", AutoDiscoveryAttemptType::AutoHttps)])]
    #[case("http://localhost", vec![AutoDiscoveryAttempt::new("http://localhost", AutoDiscoveryAttemptType::UserInput)])]
    #[case("http://localhost/wp-json", vec![AutoDiscoveryAttempt::new("http://localhost/wp-json", AutoDiscoveryAttemptType::UserInput)])]
    #[case("http://localhost/wp-admin.php", vec![AutoDiscoveryAttempt::new("http://localhost/wp-admin.php", AutoDiscoveryAttemptType::UserInput)])]
    #[case("http://localhost/wp-admin", vec![AutoDiscoveryAttempt::new("http://localhost/wp-admin", AutoDiscoveryAttemptType::UserInput), AutoDiscoveryAttempt::new("http://localhost/wp-admin.php", AutoDiscoveryAttemptType::AutoDotPhpExtensionForWpAdmin)])]
    #[case("http://localhost/wp-admin/", vec![AutoDiscoveryAttempt::new("http://localhost/wp-admin/", AutoDiscoveryAttemptType::UserInput), AutoDiscoveryAttempt::new("http://localhost/wp-admin.php", AutoDiscoveryAttemptType::AutoDotPhpExtensionForWpAdmin)])]
    #[case("orchestremetropolitain.com/wp-json", vec![AutoDiscoveryAttempt::new("orchestremetropolitain.com/wp-json", AutoDiscoveryAttemptType::UserInput), AutoDiscoveryAttempt::new("https://orchestremetropolitain.com/wp-json", AutoDiscoveryAttemptType::AutoHttps)])]
    #[case("https://orchestremetropolitain.com", vec![AutoDiscoveryAttempt::new("https://orchestremetropolitain.com", AutoDiscoveryAttemptType::UserInput)])]
    #[case(
        "https://orchestremetropolitain.com/fr/",
        vec![AutoDiscoveryAttempt::new("https://orchestremetropolitain.com/fr/", AutoDiscoveryAttemptType::UserInput)]
    )]
    #[case(
        "https://orchestremetropolitain.com/wp-json",
        vec![AutoDiscoveryAttempt::new("https://orchestremetropolitain.com/wp-json", AutoDiscoveryAttemptType::UserInput)]
    )]
    fn test_construct_attempts(
        #[case] input_site_url: &str,
        #[case] mut expected_attempts: Vec<AutoDiscoveryAttempt>,
    ) {
        let mut found_attempts = construct_attempts(input_site_url.to_string());
        found_attempts.sort();
        expected_attempts.sort();
        assert_eq!(found_attempts, expected_attempts)
    }
}

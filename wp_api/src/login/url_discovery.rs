use super::WpApiDetails;
use crate::{request::WpNetworkHeaderMap, ParseUrlError, ParsedUrl, RequestExecutionError};
use std::{collections::HashMap, sync::Arc};

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
    pub attempts: HashMap<AutoDiscoveryAttemptType, Arc<AutoDiscoveryAttemptResult>>,
}

impl From<AutoDiscoveryResult> for AutoDiscoveryUniffiResult {
    fn from(value: AutoDiscoveryResult) -> Self {
        Self {
            attempts: value
                .attempts
                .into_iter()
                .map(|(k, v)| (k, Arc::new(v)))
                .collect(),
        }
    }
}

#[derive(Debug)]
pub struct AutoDiscoveryResult {
    pub attempts: HashMap<AutoDiscoveryAttemptType, AutoDiscoveryAttemptResult>,
}

impl AutoDiscoveryResult {
    pub fn find_successful(self) -> Option<AutoDiscoveryAttemptResult> {
        self.attempts
            .into_iter()
            .find(|(attempt_type, result)| result.result.is_ok())
            .map(|(attempt_type, result)| result)
    }
}

#[derive(Debug, uniffi::Object)]
pub struct AutoDiscoveryAttemptResult {
    pub attempt_type: AutoDiscoveryAttemptType,
    pub attempt_site_url: String,
    pub result: Result<AutoDiscoveryAttemptSuccess, AutoDiscoveryAttemptFailure>,
}

#[derive(Debug)]
pub struct AutoDiscoveryAttemptSuccess {
    pub parsed_site_url: ParsedUrl,
    pub api_root_url: ParsedUrl,
    pub api_details: WpApiDetails,
}

#[derive(Debug)]
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
        error: serde_json::Error,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, uniffi::Enum)]
pub enum AutoDiscoveryAttemptType {
    Original,
    AutoHttps,
    AutoDotPhpExtensionForWpAdmin,
}

impl AutoDiscoveryAttemptType {
    fn is_the_site_url_same_as_the_user_input(&self) -> bool {
        matches!(self, AutoDiscoveryAttemptType::Original)
    }
}

pub(crate) fn construct_attempts(input_site_url: String) -> Vec<AutoDiscoveryAttempt> {
    let mut attempts = vec![AutoDiscoveryAttempt::new(
        input_site_url.clone(),
        AutoDiscoveryAttemptType::Original,
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

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum ParseApiRootUrlError {
    #[error(
        "Api root link header not found!\nStatus Code: '{:#?}'\nHeader Map: '{:#?}'",
        status_code,
        header_map
    )]
    ApiRootLinkHeaderNotFound {
        header_map: Arc<WpNetworkHeaderMap>,
        status_code: u16,
    },
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

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[rstest]
    #[case("localhost", vec![AutoDiscoveryAttempt::new("localhost", AutoDiscoveryAttemptType::Original), AutoDiscoveryAttempt::new("https://localhost", AutoDiscoveryAttemptType::AutoHttps)])]
    #[case("http://localhost", vec![AutoDiscoveryAttempt::new("http://localhost", AutoDiscoveryAttemptType::Original)])]
    #[case("http://localhost/wp-json", vec![AutoDiscoveryAttempt::new("http://localhost/wp-json", AutoDiscoveryAttemptType::Original)])]
    #[case("http://localhost/wp-admin.php", vec![AutoDiscoveryAttempt::new("http://localhost/wp-admin.php", AutoDiscoveryAttemptType::Original)])]
    #[case("http://localhost/wp-admin", vec![AutoDiscoveryAttempt::new("http://localhost/wp-admin", AutoDiscoveryAttemptType::Original), AutoDiscoveryAttempt::new("http://localhost/wp-admin.php", AutoDiscoveryAttemptType::AutoDotPhpExtensionForWpAdmin)])]
    #[case("http://localhost/wp-admin/", vec![AutoDiscoveryAttempt::new("http://localhost/wp-admin/", AutoDiscoveryAttemptType::Original), AutoDiscoveryAttempt::new("http://localhost/wp-admin.php", AutoDiscoveryAttemptType::AutoDotPhpExtensionForWpAdmin)])]
    #[case("orchestremetropolitain.com/wp-json", vec![AutoDiscoveryAttempt::new("orchestremetropolitain.com/wp-json", AutoDiscoveryAttemptType::Original), AutoDiscoveryAttempt::new("https://orchestremetropolitain.com/wp-json", AutoDiscoveryAttemptType::AutoHttps)])]
    #[case("https://orchestremetropolitain.com", vec![AutoDiscoveryAttempt::new("https://orchestremetropolitain.com", AutoDiscoveryAttemptType::Original)])]
    #[case(
        "https://orchestremetropolitain.com/fr/",
        vec![AutoDiscoveryAttempt::new("https://orchestremetropolitain.com/fr/", AutoDiscoveryAttemptType::Original)]
    )]
    #[case(
        "https://orchestremetropolitain.com/wp-json",
        vec![AutoDiscoveryAttempt::new("https://orchestremetropolitain.com/wp-json", AutoDiscoveryAttemptType::Original)]
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

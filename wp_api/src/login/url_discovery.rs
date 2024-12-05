use std::{collections::HashMap, sync::Arc};

use crate::{
    request::{WpNetworkHeaderMap, WpNetworkResponse},
    ParseUrlError, ParsedUrl, RequestExecutionError,
};

use super::WpApiDetails;

const API_ROOT_LINK_HEADER: &str = "https://api.w.org/";

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct AutoDiscoveryAttempt {
    pub(crate) site_url: String,
    pub(crate) attempt_type: AutoDiscoveryAttemptType,
}

impl AutoDiscoveryAttempt {
    fn new(site_url: impl Into<String>, attempt_type: AutoDiscoveryAttemptType) -> Self {
        Self {
            site_url: site_url.into(),
            attempt_type,
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum AutoDiscoveryAttemptType {
    Original,
    AutoHttps,
    AutoDotPhpExtensionForWpAdmin,
}

pub fn construct_attempts(input_site_url: String) -> Vec<AutoDiscoveryAttempt> {
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

#[derive(Debug, uniffi::Enum)]
pub enum UrlDiscoveryState {
    Success(UrlDiscoveryAttemptSuccess),
    Failure(UrlDiscoveryAttemptError),
}

#[derive(Debug, uniffi::Record)]
pub struct UrlDiscoveryAttemptSuccess {
    pub site_url: Arc<ParsedUrl>,
    pub api_details: Arc<WpApiDetails>,
    pub api_root_url: Arc<ParsedUrl>,
}

#[derive(Debug, uniffi::Enum)]
pub enum UrlDiscoveryAttemptError {
    FailedToParseSiteUrl {
        site_url: String,
        error: ParseUrlError,
    },
    FetchApiRootUrlFailed {
        site_url: Arc<ParsedUrl>,
        error: FetchApiRootUrlError,
    },
    FetchApiDetailsFailed {
        site_url: Arc<ParsedUrl>,
        api_root_url: Arc<ParsedUrl>,
        error: FetchApiDetailsError,
    },
}

impl UrlDiscoveryAttemptError {
    pub fn site_url(&self) -> String {
        match self {
            UrlDiscoveryAttemptError::FailedToParseSiteUrl { site_url, .. } => site_url.clone(),
            UrlDiscoveryAttemptError::FetchApiRootUrlFailed { site_url, .. } => site_url.url(),
            UrlDiscoveryAttemptError::FetchApiDetailsFailed { site_url, .. } => site_url.url(),
        }
    }
}

#[derive(Debug, uniffi::Record)]
pub struct UrlDiscoverySuccess {
    pub site_url: Arc<ParsedUrl>,
    pub api_details: Arc<WpApiDetails>,
    pub api_root_url: Arc<ParsedUrl>,
    pub attempts: HashMap<String, UrlDiscoveryState>,
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum UrlDiscoveryError {
    #[error("Url discovery failed: {:?}", attempts)]
    UrlDiscoveryFailed {
        attempts: HashMap<String, UrlDiscoveryState>,
    },
}

#[derive(Debug)]
pub(super) struct StateInitial {
    pub site_url: String,
}

impl StateInitial {
    pub fn new(site_url: &str) -> Self {
        Self {
            site_url: site_url.to_string(),
        }
    }

    pub fn parse(self) -> Result<StateParsedUrl, ParseUrlError> {
        ParsedUrl::parse(self.site_url.as_str()).map(StateParsedUrl::new)
    }
}

#[derive(Debug)]
pub(super) struct StateParsedUrl {
    pub site_url: ParsedUrl,
}

impl StateParsedUrl {
    fn new(site_url: ParsedUrl) -> Self {
        Self { site_url }
    }

    pub fn parse_api_root_response(
        self,
        response: WpNetworkResponse,
    ) -> Result<StateFetchedApiRootUrl, FetchApiRootUrlError> {
        match response
            .get_link_header(API_ROOT_LINK_HEADER)
            .into_iter()
            .nth(0)
        {
            Some(url) => Ok(StateFetchedApiRootUrl {
                site_url: self.site_url,
                api_root_url: ParsedUrl::new(url),
            }),
            None => Err(FetchApiRootUrlError::ApiRootLinkHeaderNotFound {
                header_map: response.header_map,
                status_code: response.status_code,
            }),
        }
    }
}

#[derive(Debug)]
pub(super) struct StateFetchedApiRootUrl {
    pub site_url: ParsedUrl,
    pub api_root_url: ParsedUrl,
}

impl StateFetchedApiRootUrl {
    pub fn parse_api_details_response(
        self,
        response: WpNetworkResponse,
    ) -> Result<UrlDiscoveryAttemptSuccess, UrlDiscoveryAttemptError> {
        match serde_json::from_slice::<WpApiDetails>(&response.body) {
            Ok(api_details) => Ok(UrlDiscoveryAttemptSuccess {
                site_url: Arc::new(self.site_url),
                api_details: Arc::new(api_details),
                api_root_url: Arc::new(self.api_root_url),
            }),
            Err(err) => {
                let e = FetchApiDetailsError::ApiDetailsCouldntBeParsed {
                    reason: err.to_string(),
                    response: response.body_as_string(),
                };
                Err(UrlDiscoveryAttemptError::FetchApiDetailsFailed {
                    site_url: Arc::new(self.site_url),
                    api_root_url: Arc::new(self.api_root_url),
                    error: e,
                })
            }
        }
    }
}

impl From<StateFetchedApiDetails> for UrlDiscoveryAttemptSuccess {
    fn from(state: StateFetchedApiDetails) -> Self {
        UrlDiscoveryAttemptSuccess {
            site_url: Arc::new(state.site_url),
            api_details: Arc::new(state.api_details),
            api_root_url: Arc::new(state.api_root_url),
        }
    }
}

#[derive(Debug)]
pub(super) struct StateFetchedApiDetails {
    pub site_url: ParsedUrl,
    pub api_details: WpApiDetails,
    pub api_root_url: ParsedUrl,
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum FetchApiRootUrlError {
    #[error(
        "Request execution failed!\nStatus Code: '{:?}'\nResponse: '{}'",
        status_code,
        reason
    )]
    RequestExecutionFailed {
        status_code: Option<u16>,
        reason: String,
    },
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

impl From<RequestExecutionError> for FetchApiRootUrlError {
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

impl From<RequestExecutionError> for FetchApiDetailsError {
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

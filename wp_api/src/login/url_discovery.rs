use std::{collections::HashMap, sync::Arc};

use crate::{
    request::{WpNetworkHeaderMap, WpNetworkResponse},
    ParseUrlError, ParsedUrl, RequestExecutionError,
};

use super::WpApiDetails;

const API_ROOT_LINK_HEADER: &str = "https://api.w.org/";

pub fn construct_attempts(input_site_url: String) -> Vec<String> {
    let mut attempts = vec![input_site_url.clone()];
    if !input_site_url.starts_with("http") {
        attempts.push(format!("https://{}", input_site_url))
    }
    if input_site_url.ends_with("wp-admin") {
        attempts.push(format!("{}.php", input_site_url))
    } else if input_site_url.ends_with("wp-admin/") {
        let mut s = input_site_url.clone();
        s.pop()
            .expect("Already verified that there is at least one char");
        attempts.push(format!("{}.php", s));
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
    #[case("localhost", vec!["localhost", "https://localhost"])]
    #[case("http://localhost", vec!["http://localhost"])]
    #[case("http://localhost/wp-json", vec!["http://localhost/wp-json"])]
    #[case("http://localhost/wp-admin.php", vec!["http://localhost/wp-admin.php"])]
    #[case("http://localhost/wp-admin", vec!["http://localhost/wp-admin", "http://localhost/wp-admin.php"])]
    #[case("http://localhost/wp-admin/", vec!["http://localhost/wp-admin/", "http://localhost/wp-admin.php"])]
    #[case("orchestremetropolitain.com/wp-json", vec!["orchestremetropolitain.com/wp-json", "https://orchestremetropolitain.com/wp-json"])]
    #[case("https://orchestremetropolitain.com", vec!["https://orchestremetropolitain.com"])]
    #[case(
        "https://orchestremetropolitain.com/fr/",
        vec!["https://orchestremetropolitain.com/fr/"]
    )]
    #[case(
        "https://orchestremetropolitain.com/wp-json",
        vec!["https://orchestremetropolitain.com/wp-json"]
    )]
    fn test_construct_attempts(
        #[case] input_site_url: &str,
        #[case] mut expected_attempts: Vec<&str>,
    ) {
        let mut found_attempts = construct_attempts(input_site_url.to_string());
        found_attempts.sort();
        expected_attempts.sort();
        assert_eq!(found_attempts, expected_attempts)
    }
}

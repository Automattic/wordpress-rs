use std::sync::Arc;
use url::Url;

use crate::{
    request::{WpNetworkHeaderMap, WpNetworkResponse},
    RequestExecutionError,
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

#[derive(Debug, uniffi::Record)]
pub struct UrlDiscoverySuccess {
    pub site_url: Arc<ParsedUrl>,
    pub api_details: Arc<WpApiDetails>,
    pub api_root_url: Arc<ParsedUrl>,
    pub attempts: Vec<UrlDiscoveryState>,
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum UrlDiscoveryError {
    #[error("Url discovery failed: {:?}", attempts)]
    UrlDiscoveryFailed { attempts: Vec<UrlDiscoveryState> },
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
    pub site_url: Arc<ParsedUrl>,
}

impl StateParsedUrl {
    fn new(parsed_url: ParsedUrl) -> Self {
        Self {
            site_url: Arc::new(parsed_url),
        }
    }

    pub fn parse_api_root_response(
        &self,
        response: WpNetworkResponse,
    ) -> Result<StateFetchedApiRootUrl, FetchApiRootUrlError> {
        match response
            .get_link_header(API_ROOT_LINK_HEADER)
            .into_iter()
            .nth(0)
        {
            Some(url) => Ok(StateFetchedApiRootUrl {
                site_url: self.site_url.clone(),
                api_root_url: ParsedUrl { url }.into(),
            }),
            None => Err(FetchApiRootUrlError::ApiRootLinkHeaderNotFound {
                header_map: response.header_map,
            }),
        }
    }
}

#[derive(Debug)]
pub(super) struct StateFetchedApiRootUrl {
    pub site_url: Arc<ParsedUrl>,
    pub api_root_url: Arc<ParsedUrl>,
}

impl StateFetchedApiRootUrl {
    pub fn parse_api_details_response(
        self,
        response: WpNetworkResponse,
    ) -> Result<UrlDiscoveryAttemptSuccess, UrlDiscoveryAttemptError> {
        match serde_json::from_slice::<WpApiDetails>(&response.body) {
            Ok(api_details) => Ok(UrlDiscoveryAttemptSuccess {
                site_url: self.site_url,
                api_details: api_details.into(),
                api_root_url: self.api_root_url,
            }),
            Err(err) => {
                let e = FetchApiDetailsError::ApiDetailsCouldntBeParsed {
                    reason: err.to_string(),
                    response: response.body_as_string(),
                };
                Err(UrlDiscoveryAttemptError::FetchApiDetailsFailed {
                    site_url: self.site_url,
                    api_root_url: self.api_root_url,
                    error: e,
                })
            }
        }
    }
}

impl From<StateFetchedApiDetails> for UrlDiscoveryAttemptSuccess {
    fn from(state: StateFetchedApiDetails) -> Self {
        UrlDiscoveryAttemptSuccess {
            site_url: state.site_url,
            api_details: state.api_details,
            api_root_url: state.api_root_url,
        }
    }
}

#[derive(Debug)]
pub(super) struct StateFetchedApiDetails {
    pub site_url: Arc<ParsedUrl>,
    pub api_details: Arc<WpApiDetails>,
    pub api_root_url: Arc<ParsedUrl>,
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum FetchApiRootUrlError {
    #[error(
        "Request execution failed!\nStatus Code: '{:?}'.\nResponse: '{}'",
        status_code,
        reason
    )]
    RequestExecutionFailed {
        status_code: Option<u16>,
        reason: String,
    },
    #[error("Api root link header not found in header_map: {:?}", header_map)]
    ApiRootLinkHeaderNotFound { header_map: Arc<WpNetworkHeaderMap> },
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

// TODO: Should be in a central place, used across the code base
#[derive(Debug, uniffi::Object)]
pub struct ParsedUrl {
    url: Url,
}

impl ParsedUrl {
    fn parse(input: &str) -> Result<Self, ParseUrlError> {
        Url::parse(input)
            .map_err(|e| match e {
                url::ParseError::RelativeUrlWithoutBase => ParseUrlError::RelativeUrlWithoutBase,
                _ => ParseUrlError::Generic {
                    reason: e.to_string(),
                },
            })
            .map(|url| Self { url })
    }
}

#[uniffi::export]
impl ParsedUrl {
    pub fn url(&self) -> String {
        self.url.to_string()
    }
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum ParseUrlError {
    #[error("Error while parsing url: {}", reason)]
    Generic { reason: String },
    #[error("Relative URL without a base")]
    RelativeUrlWithoutBase,
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

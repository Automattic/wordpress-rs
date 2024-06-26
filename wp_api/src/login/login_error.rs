use std::sync::Arc;

use crate::{request::WpNetworkHeaderMap, RequestExecutionError};

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum FindApiUrlsError {
    #[error("Api details couldn't be parsed from response: {:?}", response)]
    ApiDetailsCouldntBeParsed { reason: String, response: String },
    #[error("Api root link header not found in header_map: {:?}", header_map)]
    ApiRootLinkHeaderNotFound { header_map: Arc<WpNetworkHeaderMap> },
    #[error("Error while parsing site url: {}", reason)]
    ParseSiteUrlError { reason: String },
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

impl From<url::ParseError> for FindApiUrlsError {
    fn from(value: url::ParseError) -> Self {
        Self::ParseSiteUrlError {
            reason: value.to_string(),
        }
    }
}

impl From<RequestExecutionError> for FindApiUrlsError {
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

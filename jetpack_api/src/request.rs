use std::{fmt::Debug, sync::Arc};

use serde::Deserialize;
use wp_api::{
    request::{WpNetworkHeaderMap, WpNetworkRequest, WpNetworkResponse},
    ParsedRequestError, WpError,
};

use crate::JpApiError;

pub mod endpoint;

pub trait JpParsedRequestError
where
    Self: Sized,
{
    fn try_parse(response: &JpNetworkResponse) -> Option<Self>;
    fn as_parse_error(reason: String, response: String) -> Self;
}

#[uniffi::export(with_foreign)]
#[async_trait::async_trait]
pub trait JetpackRequestExecutor: Send + Sync + Debug {
    async fn execute(
        &self,
        request: Arc<WpNetworkRequest>,
    ) -> Result<JpNetworkResponse, JetpackRequestExecutionError>;
}

#[derive(Debug, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum JetpackRequestExecutionError {
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

#[derive(uniffi::Record)]
pub struct JpNetworkResponse {
    pub body: Vec<u8>,
    pub status_code: u16,
    pub header_map: Arc<WpNetworkHeaderMap>,
}

impl From<WpNetworkResponse> for JpNetworkResponse {
    fn from(value: WpNetworkResponse) -> Self {
        Self {
            body: value.body,
            status_code: value.status_code,
            header_map: value.header_map,
        }
    }
}

impl JpNetworkResponse {
    pub fn body_as_string(&self) -> String {
        String::from_utf8_lossy(&self.body).to_string()
    }

    pub fn parse<'de, T, E>(&'de self) -> Result<T, E>
    where
        T: Deserialize<'de>,
        E: ParsedRequestError,
    {
        if let Some(err) = E::try_parse(&self.body, self.status_code) {
            return Err(err);
        }
        serde_json::from_slice(&self.body)
            .map_err(|err| E::as_parse_error(err.to_string(), self.body_as_string()))
    }

    pub fn parse_with<F, T>(&self, parser: F) -> Result<T, JpApiError>
    where
        F: Fn(&Self) -> Result<T, JpApiError>,
    {
        parser(self)
    }

    fn parse_response_for_errors(&self) -> Result<(), JpApiError> {
        if let Ok(wp_error) = serde_json::from_slice::<WpError>(&self.body) {
            Err(JpApiError::WpError {
                error_code: wp_error.code,
                error_message: wp_error.message,
                status_code: self.status_code,
                response: self.body_as_string(),
            })
        } else {
            let status = http::StatusCode::from_u16(self.status_code).map_err(|_| {
                JpApiError::InvalidHttpStatusCode {
                    status_code: self.status_code,
                }
            })?;
            if status.is_client_error() || status.is_server_error() {
                Err(JpApiError::UnknownError {
                    status_code: self.status_code,
                    response: self.body_as_string(),
                })
            } else {
                Ok(())
            }
        }
    }
}

impl Debug for JpNetworkResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = format!(
            indoc::indoc! {"
                WpNetworkResponse {{
                    status_code: '{}',
                    header_map: '{:?}',
                    body: '{}'
                }}
                "},
            self.status_code,
            self.header_map,
            self.body_as_string()
        );
        s.pop(); // Remove the new line at the end
        write!(f, "{}", s)
    }
}

use std::{collections::HashMap, fmt::Debug};

use endpoint::ApiEndpointUrl;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{api_error::RequestExecutionError, WpApiError, WpAuthentication};

use self::endpoint::WpEndpointUrl;

pub mod endpoint;

const CONTENT_TYPE_JSON: &str = "application/json";
const LINK_HEADER_KEY: &str = "Link";

#[derive(Debug)]
struct InnerRequestBuilder {
    authentication: WpAuthentication,
}

impl InnerRequestBuilder {
    fn new(authentication: WpAuthentication) -> Self {
        Self { authentication }
    }

    fn get(&self, url: ApiEndpointUrl) -> WpNetworkRequest {
        WpNetworkRequest {
            method: RequestMethod::GET,
            url: url.into(),
            header_map: self.header_map(),
            body: None,
        }
    }

    fn post<T>(&self, url: ApiEndpointUrl, json_body: &T) -> WpNetworkRequest
    where
        T: ?Sized + Serialize,
    {
        WpNetworkRequest {
            method: RequestMethod::POST,
            url: url.into(),
            header_map: self.header_map_for_post_request(),
            body: serde_json::to_vec(json_body).ok(),
        }
    }

    fn delete(&self, url: ApiEndpointUrl) -> WpNetworkRequest {
        WpNetworkRequest {
            method: RequestMethod::DELETE,
            url: url.into(),
            header_map: self.header_map(),
            body: None,
        }
    }

    fn header_map(&self) -> HashMap<String, String> {
        let mut header_map = HashMap::new();
        header_map.insert(
            http::header::ACCEPT.to_string(),
            CONTENT_TYPE_JSON.to_string(),
        );
        match self.authentication {
            WpAuthentication::None => None,
            WpAuthentication::AuthorizationHeader { ref token } => {
                header_map.insert("Authorization".to_string(), format!("Basic {}", token))
            }
        };
        header_map
    }

    fn header_map_for_post_request(&self) -> HashMap<String, String> {
        let mut header_map = self.header_map();
        header_map.insert(
            http::header::CONTENT_TYPE.to_string(),
            CONTENT_TYPE_JSON.to_string(),
        );
        header_map
    }
}

#[uniffi::export(with_foreign)]
#[async_trait::async_trait]
pub trait RequestExecutor: Send + Sync + Debug {
    async fn execute(
        &self,
        request: WpNetworkRequest,
    ) -> Result<WpNetworkResponse, RequestExecutionError>;
}

// Has custom `Debug` trait implementation
#[derive(uniffi::Record)]
pub struct WpNetworkRequest {
    pub method: RequestMethod,
    pub url: WpEndpointUrl,
    // TODO: We probably want to implement a specific type for these headers instead of using a
    // regular HashMap.
    //
    // It could be something similar to `reqwest`'s [`header`](https://docs.rs/reqwest/latest/reqwest/header/index.html)
    // module.
    pub header_map: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
}

impl WpNetworkRequest {
    pub fn body_as_string(&self) -> Option<String> {
        self.body.as_ref().map(|b| body_as_string(b))
    }
}

impl Debug for WpNetworkRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = format!(
            indoc::indoc! {"
                WpNetworkRequest {{
                    method: '{:?}',
                    url: '{:?}',
                    header_map: '{:?}',
                    body: '{:?}'
                }}
                "},
            self.method,
            self.url,
            self.header_map,
            self.body_as_string()
        );
        s.pop(); // Remove the new line at the end
        write!(f, "{}", s)
    }
}

// Has custom `Debug` trait implementation
#[derive(uniffi::Record)]
pub struct WpNetworkResponse {
    pub body: Vec<u8>,
    pub status_code: u16,
    // TODO: We probably want to implement a specific type for these headers instead of using a
    // regular HashMap.
    //
    // It could be something similar to `reqwest`'s [`header`](https://docs.rs/reqwest/latest/reqwest/header/index.html)
    // module.
    pub header_map: Option<HashMap<String, String>>,
}

impl WpNetworkResponse {
    pub fn get_link_header(&self, name: &str) -> Option<Url> {
        self.header_map
            .as_ref()
            .map(|h_map| h_map.get(LINK_HEADER_KEY))?
            .and_then(|link_header| parse_link_header::parse_with_rel(link_header).ok())
            .and_then(|link_map| {
                link_map
                    .get(name)
                    .and_then(|link| Url::parse(link.raw_uri.as_str()).ok())
            })
    }

    pub fn body_as_string(&self) -> String {
        body_as_string(&self.body)
    }

    pub fn parse<'de, T: Deserialize<'de>>(&'de self) -> Result<T, WpApiError> {
        self.parse_response_for_generic_errors()?;
        serde_json::from_slice(&self.body).map_err(|err| WpApiError::ParsingError {
            reason: err.to_string(),
            response: self.body_as_string(),
        })
    }

    pub fn parse_with<F, T>(&self, parser: F) -> Result<T, WpApiError>
    where
        F: Fn(&WpNetworkResponse) -> Result<T, WpApiError>,
    {
        parser(self)
    }

    fn parse_response_for_generic_errors(&self) -> Result<(), WpApiError> {
        // TODO: Further parse the response body to include error message
        // TODO: Lots of unwraps to get a basic setup working
        let status = http::StatusCode::from_u16(self.status_code).unwrap();
        if let Ok(rest_error) = serde_json::from_slice(&self.body) {
            Err(WpApiError::RestError {
                rest_error,
                status_code: self.status_code,
                response: self.body_as_string(),
            })
        } else if status.is_client_error() || status.is_server_error() {
            Err(WpApiError::UnknownError {
                status_code: self.status_code,
                response: self.body_as_string(),
            })
        } else {
            Ok(())
        }
    }
}

impl Debug for WpNetworkResponse {
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

#[derive(Debug, Clone, uniffi::Enum)]
pub enum RequestMethod {
    GET,
    POST,
    PUT,
    DELETE,
    HEAD,
}

fn body_as_string(body: &[u8]) -> String {
    String::from_utf8_lossy(body).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[rstest]
    #[case(
        "<http://localhost/wp-json/wp/v2/posts?page=2>; rel=\"next\"",
        None,
        Some("http://localhost/wp-json/wp/v2/posts?page=2")
    )]
    #[case("<http://localhost/wp-json/wp/v2/posts?page=1>; rel=\"prev\", <http://localhost/wp-json/wp/v2/posts?page=3>; rel=\"next\"",
            Some("http://localhost/wp-json/wp/v2/posts?page=1"),
            Some("http://localhost/wp-json/wp/v2/posts?page=3")
        )]
    #[case(
        "<http://localhost/wp-json/wp/v2/posts?page=5>; rel=\"prev\"",
        Some("http://localhost/wp-json/wp/v2/posts?page=5"),
        None
    )]
    fn test_link_header_can_be_parsed(
        #[case] link: &str,
        #[case] expected_prev_link_header: Option<&str>,
        #[case] expected_next_link_header: Option<&str>,
    ) {
        let response = WpNetworkResponse {
            body: Vec::with_capacity(0),
            status_code: 200,
            header_map: Some([("Link".to_string(), link.to_string())].into()),
        };

        assert_eq!(
            expected_prev_link_header.and_then(|s| Url::parse(s).ok()),
            response.get_link_header("prev")
        );
        assert_eq!(
            expected_next_link_header.and_then(|s| Url::parse(s).ok()),
            response.get_link_header("next")
        );
    }
}

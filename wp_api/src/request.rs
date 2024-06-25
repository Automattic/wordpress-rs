use std::{collections::HashMap, fmt::Debug, sync::Arc};

use endpoint::ApiEndpointUrl;
use http::{HeaderMap, HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{api_error::RequestExecutionError, WpApiError, WpAuthentication};

use self::endpoint::WpEndpointUrl;

pub mod endpoint;

const CONTENT_TYPE_JSON: &str = "application/json";
// TODO: It looks like this could be `Link` or `link`
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
    pub header_map: Arc<WpNetworkHeaderMap>,
}

#[derive(Debug, uniffi::Object)]
pub struct WpNetworkHeaderMap {
    inner: HeaderMap,
}

impl WpNetworkHeaderMap {
    pub fn new(header_map: HeaderMap) -> Self {
        Self { inner: header_map }
    }

    // Inserts a key-value pair into the map.
    // If the map did not previously have this key present, then false is returned.
    //
    // Returns an error if a header name or a header value can't be built from the given arguments
    pub fn append_key_value(
        header_map: &mut HeaderMap,
        header_name: String,
        header_value: String,
    ) -> Result<bool, WpNetworkHeaderMapError> {
        if let Ok(header_name) = HeaderName::from_bytes(header_name.as_bytes()) {
            if let Ok(header_value) = HeaderValue::from_str(&header_value) {
                // Using [http::HeaderMap::append] is important here because `insert` will
                // remove any existing values
                Ok(header_map.append(header_name.clone(), header_value))
            } else {
                Err(WpNetworkHeaderMapError::InvalidHeaderValue { header_value })
            }
        } else {
            Err(WpNetworkHeaderMapError::InvalidHeaderName { header_name })
        }
    }

    pub fn build_header_name_value(
        header_name: String,
        header_value: String,
    ) -> Result<(HeaderName, HeaderValue), WpNetworkHeaderMapError> {
        if let Ok(header_name) = HeaderName::from_bytes(header_name.as_bytes()) {
            if let Ok(header_value) = HeaderValue::from_str(&header_value) {
                // Using [http::HeaderMap::append] is important here because `insert` will
                // remove any existing values
                Ok((header_name, header_value))
            } else {
                Err(WpNetworkHeaderMapError::InvalidHeaderValue { header_value })
            }
        } else {
            Err(WpNetworkHeaderMapError::InvalidHeaderName { header_name })
        }
    }
}

#[uniffi::export]
impl WpNetworkHeaderMap {
    #[uniffi::constructor]
    fn from_multi_map(
        hash_map: HashMap<String, Vec<String>>,
    ) -> Result<Self, WpNetworkHeaderMapError> {
        let inner = hash_map
            .into_iter()
            .flat_map(|(header_name, values)| {
                values.into_iter().map(move |header_value| {
                    Self::build_header_name_value(header_name.clone(), header_value)
                })
            })
            .collect::<Result<HeaderMap, WpNetworkHeaderMapError>>()?;
        Ok(Self { inner })
    }

    #[uniffi::constructor]
    fn from_map(hash_map: HashMap<String, String>) -> Result<Self, WpNetworkHeaderMapError> {
        let inner = hash_map
            .into_iter()
            .map(|(header_name, header_value)| {
                Self::build_header_name_value(header_name.clone(), header_value)
            })
            .collect::<Result<HeaderMap, WpNetworkHeaderMapError>>()?;
        Ok(Self { inner })
    }
}

#[derive(Debug, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum WpNetworkHeaderMapError {
    #[error("Invalid header name: {}", header_name)]
    InvalidHeaderName { header_name: String },
    #[error("Invalid header value: {}", header_value)]
    InvalidHeaderValue { header_value: String },
}

impl WpNetworkResponse {
    pub fn get_link_header(&self, name: &str) -> Option<Url> {
        let header_map = &self.header_map.inner;
        let link_header = if let Some(k) = header_map.get(LINK_HEADER_KEY) {
            Some(k)
        } else {
            header_map.get(LINK_HEADER_KEY.to_lowercase())
        }?;
        let link_header = link_header.to_str().ok()?;
        let link_map = parse_link_header::parse_with_rel(link_header).ok()?;
        link_map
            .get(name)
            .and_then(|link| Url::parse(link.raw_uri.as_str()).ok())
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
            header_map: Arc::new(
                WpNetworkHeaderMap::from_map([("Link".to_string(), link.to_string())].into())
                    .unwrap(),
            ),
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

    #[test]
    fn test_header_map_from_map() {
        let hash_map = [
            ("host".to_string(), vec!["x".to_string(), "xy".to_string()]),
            ("host_1".to_string(), vec!["a".to_string()]),
        ]
        .into();
        let result = WpNetworkHeaderMap::from_multi_map(hash_map);
        assert!(result.is_ok());
        let header_map = result.unwrap();
        let values = header_map.inner.get_all("host");
        let mut values_iter = header_map.inner.get_all("host").iter();
        assert_eq!("x", *values_iter.next().unwrap());
        assert_eq!("xy", *values_iter.next().unwrap());
        assert_eq!("a", header_map.inner.get("host_1").unwrap());
    }
}

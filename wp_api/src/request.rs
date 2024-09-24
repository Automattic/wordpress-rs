use std::{collections::HashMap, fmt::Debug, sync::Arc};

use endpoint::ApiEndpointUrl;
use http::{HeaderMap, HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    api_error::{RequestExecutionError, WpError},
    WpApiError, WpAuthentication,
};

use self::endpoint::WpEndpointUrl;

pub mod endpoint;

const CONTENT_TYPE_JSON: &str = "application/json";
const LINK_HEADER_KEY: &str = "Link";

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ParsedResponse<T> {
    pub data: T,
    #[serde(skip)]
    pub header_wp_total: Option<u32>,
    #[serde(skip)]
    pub header_wp_total_pages: Option<u32>,
}

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
            header_map: self.header_map().into(),
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
            header_map: self.header_map_for_post_request().into(),
            body: serde_json::to_vec(json_body)
                .ok()
                .map(|b| Arc::new(WpNetworkRequestBody::new(b))),
        }
    }

    fn delete(&self, url: ApiEndpointUrl) -> WpNetworkRequest {
        WpNetworkRequest {
            method: RequestMethod::DELETE,
            url: url.into(),
            header_map: self.header_map().into(),
            body: None,
        }
    }

    fn header_map(&self) -> WpNetworkHeaderMap {
        let mut header_map = HeaderMap::new();
        header_map.insert(
            http::header::ACCEPT,
            HeaderValue::from_static(CONTENT_TYPE_JSON),
        );
        match self.authentication {
            WpAuthentication::None => (),
            WpAuthentication::AuthorizationHeader { ref token } => {
                let hv = HeaderValue::from_str(&format!("Basic {}", token));
                let hv = hv.expect("It shouldn't be possible to build WpAuthentication::AuthorizationHeader with an invalid token");
                header_map.insert(http::header::AUTHORIZATION, hv);
            }
        };
        header_map.into()
    }

    fn header_map_for_post_request(&self) -> WpNetworkHeaderMap {
        let mut header_map = self.header_map();
        header_map.inner.insert(
            http::header::CONTENT_TYPE,
            HeaderValue::from_static(CONTENT_TYPE_JSON),
        );
        header_map
    }
}

#[uniffi::export(with_foreign)]
#[async_trait::async_trait]
pub trait RequestExecutor: Send + Sync + Debug {
    async fn execute(
        &self,
        request: Arc<WpNetworkRequest>,
    ) -> Result<WpNetworkResponse, RequestExecutionError>;
}

#[derive(uniffi::Object)]
pub struct WpNetworkRequestBody {
    inner: Vec<u8>,
}

impl WpNetworkRequestBody {
    fn new(body: Vec<u8>) -> Self {
        Self { inner: body }
    }
}

#[uniffi::export]
impl WpNetworkRequestBody {
    pub fn contents(&self) -> Vec<u8> {
        self.inner.clone()
    }
}

// Has custom `Debug` trait implementation
#[derive(uniffi::Object)]
pub struct WpNetworkRequest {
    pub(crate) method: RequestMethod,
    pub(crate) url: WpEndpointUrl,
    pub(crate) header_map: Arc<WpNetworkHeaderMap>,
    pub(crate) body: Option<Arc<WpNetworkRequestBody>>,
}

#[uniffi::export]
impl WpNetworkRequest {
    pub fn method(&self) -> RequestMethod {
        self.method.clone()
    }

    pub fn url(&self) -> WpEndpointUrl {
        self.url.clone()
    }

    pub fn header_map(&self) -> Arc<WpNetworkHeaderMap> {
        self.header_map.clone()
    }

    pub fn body(&self) -> Option<Arc<WpNetworkRequestBody>> {
        self.body.clone()
    }

    pub fn body_as_string(&self) -> Option<String> {
        self.body.as_ref().map(|b| body_as_string(&b.inner))
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

#[derive(Debug, Default, Clone, uniffi::Object)]
pub struct WpNetworkHeaderMap {
    inner: HeaderMap,
}

impl WpNetworkHeaderMap {
    pub fn new(header_map: HeaderMap) -> Self {
        Self { inner: header_map }
    }

    // Splits the `header_value` by `,` then parses name & values into `HeaderName` & `HeaderValue`
    fn build_header_name_value(
        header_name: String,
        header_value: String,
    ) -> Vec<Result<(HeaderName, HeaderValue), WpNetworkHeaderMapError>> {
        header_value
            .split(',')
            .filter_map(|x| match x.trim() {
                "" => None,
                y => Some(y),
            })
            .map(|header_value| {
                if let Ok(header_name) = HeaderName::from_bytes(header_name.as_bytes()) {
                    if let Ok(header_value) = HeaderValue::from_str(header_value) {
                        // Using [http::HeaderMap::append] is important here because `insert` will
                        // remove any existing values
                        Ok((header_name, header_value))
                    } else {
                        Err(WpNetworkHeaderMapError::InvalidHeaderValue {
                            header_value: header_value.to_string(),
                        })
                    }
                } else {
                    Err(WpNetworkHeaderMapError::InvalidHeaderName {
                        header_name: header_name.clone(),
                    })
                }
            })
            .collect()
    }

    pub fn as_header_map(&self) -> HeaderMap {
        self.inner.clone()
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
                values.into_iter().flat_map(move |header_value| {
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
            .flat_map(|(header_name, header_value)| {
                Self::build_header_name_value(header_name, header_value)
            })
            .collect::<Result<HeaderMap, WpNetworkHeaderMapError>>()?;
        Ok(Self { inner })
    }

    fn to_map(&self) -> HashMap<String, Vec<String>> {
        let mut header_hashmap = HashMap::new();
        self.inner.iter().for_each(|(k, v)| {
            let v = String::from_utf8_lossy(v.as_bytes()).into_owned();
            header_hashmap
                .entry(k.as_str().to_owned())
                .or_insert_with(Vec::new)
                .push(v)
        });
        header_hashmap
    }
}

impl From<HeaderMap> for WpNetworkHeaderMap {
    fn from(header_map: HeaderMap) -> Self {
        Self::new(header_map)
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
    pub fn get_link_header(&self, name: &str) -> Vec<Url> {
        [
            self.header_map.inner.get_all(LINK_HEADER_KEY),
            self.header_map
                .inner
                .get_all(LINK_HEADER_KEY.to_lowercase()),
        ]
        .into_iter()
        .flatten()
        .flat_map(|link_header| link_header.to_str().ok())
        .flat_map(|link_header_str| parse_link_header::parse_with_rel(link_header_str).ok())
        .flat_map(|link_map| {
            link_map
                .get(name)
                .and_then(|link| Url::parse(link.raw_uri.as_str()).ok())
        })
        .collect()
    }

    pub fn body_as_string(&self) -> String {
        body_as_string(&self.body)
    }

    pub fn parse<'de, T: Deserialize<'de>>(&'de self) -> Result<T, WpApiError> {
        self.parse_response_for_errors()?;
        serde_json::from_slice(&self.body).map_err(|err| WpApiError::ResponseParsingError {
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

    fn parse_response_for_errors(&self) -> Result<(), WpApiError> {
        if let Ok(wp_error) = serde_json::from_slice::<WpError>(&self.body) {
            Err(WpApiError::WpError {
                error_code: wp_error.code,
                error_message: wp_error.message,
                status_code: self.status_code,
                response: self.body_as_string(),
            })
        } else {
            let status = http::StatusCode::from_u16(self.status_code).map_err(|_| {
                WpApiError::InvalidHttpStatusCode {
                    status_code: self.status_code,
                }
            })?;
            if status.is_client_error() || status.is_server_error() {
                Err(WpApiError::UnknownError {
                    status_code: self.status_code,
                    response: self.body_as_string(),
                })
            } else {
                Ok(())
            }
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
            expected_prev_link_header
                .and_then(|s| Url::parse(s).ok())
                .as_ref(),
            response.get_link_header("prev").first(),
            "response headers: {:?}",
            response.header_map.inner
        );
        assert_eq!(
            expected_next_link_header
                .and_then(|s| Url::parse(s).ok())
                .as_ref(),
            response.get_link_header("next").first(),
            "response headers: {:?}",
            response.header_map.inner
        );
    }

    #[test]
    fn test_header_map_from_map() {
        let hash_map = [
            ("Age".to_string(), "1,2".to_string()),
            (
                LINK_HEADER_KEY.to_string(),
                "<https://one.example.com>; rel=\"preconnect\", <https://two.example.com>"
                    .to_string(),
            ),
            ("User-Agent".to_string(), "".to_string()),
        ]
        .into();
        let header_map = WpNetworkHeaderMap::from_map(hash_map).unwrap();
        assert_header_map_values(&header_map, "Age", vec!["1", "2"]);
        assert_header_map_values(
            &header_map,
            LINK_HEADER_KEY,
            vec![
                "<https://one.example.com>; rel=\"preconnect\"",
                "<https://two.example.com>",
            ],
        );
        assert_header_map_values(&header_map, "User-Agent", vec![]);
    }

    #[test]
    fn test_header_map_from_multi_map() {
        let hash_map = [
            ("Age".to_string(), vec!["1".to_string(), "2,3".to_string()]),
            (
                LINK_HEADER_KEY.to_string(),
                vec![
                    "<https://one.example.com>; rel=\"preconnect\", <https://two.example.com>"
                        .to_string(),
                ],
            ),
            ("Retry-After".to_string(), vec!["120".to_string()]),
            ("User-Agent".to_string(), vec![]),
        ]
        .into();
        let header_map = WpNetworkHeaderMap::from_multi_map(hash_map).unwrap();
        assert_header_map_values(&header_map, "Age", vec!["1", "2", "3"]);
        assert_header_map_values(
            &header_map,
            LINK_HEADER_KEY,
            vec![
                "<https://one.example.com>; rel=\"preconnect\"",
                "<https://two.example.com>",
            ],
        );
        assert_header_map_values(&header_map, "Retry-After", vec!["120"]);
        assert_header_map_values(&header_map, "User-Agent", vec![]);
    }

    fn assert_header_map_values(header_map: &WpNetworkHeaderMap, key: &str, values: Vec<&str>) {
        assert_eq!(
            header_map
                .inner
                .get_all(key)
                .iter()
                .map(|h| h.to_str().unwrap())
                .collect::<Vec<&str>>(),
            values
        );
    }
}

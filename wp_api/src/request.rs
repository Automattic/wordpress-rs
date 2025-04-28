use self::endpoint::WpEndpointUrl;
use crate::{
    ParsedUrl, RequestExecutionErrorReason, WpApiError, WpAuthentication,
    api_error::{MediaUploadRequestExecutionError, ParsedRequestError, RequestExecutionError},
    url_query::{FromUrlQueryPairs, UrlQueryPairsMap},
};
use base64::Engine;
use chrono::{DateTime, Utc};
use endpoint::application_passwords_endpoint::ApplicationPasswordsRequestBuilder;
use endpoint::{ApiEndpointUrl, media_endpoint::MediaUploadRequest};
use http::{HeaderMap, HeaderName, HeaderValue};
use regex::Regex;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::str::{FromStr, Utf8Error};
use std::{collections::HashMap, fmt::Debug, sync::Arc};
use url::Url;
use uuid::Uuid;
use wp_localization::{MessageBundle, WpMessages, WpSupportsLocalization};
use wp_localization_macro::WpDeriveLocalizable;

pub mod endpoint;

const CONTENT_TYPE_JSON: &str = "application/json";
const CONTENT_TYPE_MULTIPART: &str = "multipart/form-data";
const HEADER_KEY_WP_TOTAL: &str = "X-WP-Total";
const HEADER_KEY_WP_TOTAL_PAGES: &str = "X-WP-TotalPages";
const MAYBE_HTML_REGEX: &str = r"<\/?[a-z][\s\S]*>";

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ParsedResponse<DataType, ParamsType> {
    pub data: DataType,
    #[serde(skip)]
    pub header_map: Arc<WpNetworkHeaderMap>,
    #[serde(skip)]
    pub next_page_params: Option<ParamsType>,
    #[serde(skip)]
    pub prev_page_params: Option<ParamsType>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PaginationHeaderKey {
    Next,
    Prev,
}

impl PaginationHeaderKey {
    fn as_str(&self) -> &str {
        match self {
            Self::Next => "next",
            Self::Prev => "prev",
        }
    }
}

#[derive(Debug)]
pub struct InnerRequestBuilder {
    authentication: WpAuthentication,
}

impl InnerRequestBuilder {
    pub fn new(authentication: WpAuthentication) -> Self {
        Self { authentication }
    }

    pub fn get(&self, url: ApiEndpointUrl) -> WpNetworkRequest {
        WpNetworkRequest {
            uuid: Uuid::new_v4().into(),
            retry_count: 0,
            method: RequestMethod::GET,
            url: url.into(),
            header_map: self.header_map().into(),
            body: None,
        }
    }

    pub fn post<T>(&self, url: ApiEndpointUrl, json_body: &T) -> WpNetworkRequest
    where
        T: ?Sized + Serialize,
    {
        WpNetworkRequest {
            uuid: Uuid::new_v4().into(),
            retry_count: 0,
            method: RequestMethod::POST,
            url: url.into(),
            header_map: self.header_map_for_post_request().into(),
            body: serde_json::to_vec(json_body)
                .ok()
                .map(|b| Arc::new(WpNetworkRequestBody::new(b))),
        }
    }

    pub fn delete(&self, url: ApiEndpointUrl) -> WpNetworkRequest {
        WpNetworkRequest {
            uuid: Uuid::new_v4().into(),
            retry_count: 0,
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
            WpAuthentication::Bearer { ref token } => {
                let hv = HeaderValue::from_str(&format!("Bearer {}", token));
                let hv = hv.expect("It shouldn't be possible to build WpAuthentication::Bearer with an invalid token");
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

    async fn upload_media(
        &self,
        media_upload_request: Arc<MediaUploadRequest>,
    ) -> Result<WpNetworkResponse, MediaUploadRequestExecutionError>;

    async fn sleep(&self, millis: u64);
}

#[derive(uniffi::Object)]
pub struct WpNetworkRequestBody {
    inner: Vec<u8>,
}

impl WpNetworkRequestBody {
    pub fn new(body: Vec<u8>) -> Self {
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
    pub(crate) uuid: String,
    pub(crate) retry_count: u8,
    pub(crate) method: RequestMethod,
    pub(crate) url: WpEndpointUrl,
    pub(crate) header_map: Arc<WpNetworkHeaderMap>,
    pub(crate) body: Option<Arc<WpNetworkRequestBody>>,
}

#[uniffi::export]
impl WpNetworkRequest {
    /// Unique identifier for this request – used for external record-keeping
    pub fn request_id(&self) -> String {
        self.uuid.clone()
    }

    /// Clones the request and increments the retry count
    pub fn clone_with_incremented_retry_count(&self) -> Self {
        Self {
            uuid: self.uuid.clone(),
            retry_count: self.retry_count + 1,
            method: self.method.clone(),
            url: self.url.clone(),
            header_map: Arc::clone(&self.header_map),
            body: self.body.clone(),
        }
    }

    /// How many times this request has this request been attempted?
    pub fn retry_count(&self) -> u8 {
        self.retry_count
    }

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
        self.body
            .as_ref()
            .map(|b| request_or_response_body_as_string(&b.inner))
    }

    pub fn has_http_authentication(&self) -> bool {
        self.header_map.has_http_authentication()
    }

    pub fn adding_http_authentication(&self, username: &str, password: &str) -> Self {
        let encoded_credentials =
            base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", username, password));

        let mut new_header_map = self.header_map.inner.clone();

        new_header_map.insert(
            http::header::AUTHORIZATION,
            format!("Basic {}", encoded_credentials).parse().expect(
                "base64 can only produce ASCII, so this string is guaranteed to be parseable",
            ),
        );

        WpNetworkRequest {
            uuid: self.uuid.clone(),
            retry_count: self.retry_count + 1,
            method: self.method.clone(),
            url: self.url.clone(),
            header_map: Arc::new(new_header_map.into()),
            body: self.body.clone(),
        }
    }
}

impl WpNetworkRequest {
    pub fn get(url: WpEndpointUrl) -> Self {
        Self {
            uuid: Uuid::new_v4().into(),
            retry_count: 0,
            method: RequestMethod::GET,
            url,
            header_map: Arc::new(WpNetworkHeaderMap::default()),
            body: None,
        }
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
    pub response_header_map: Arc<WpNetworkHeaderMap>,
    pub request_url: WpEndpointUrl,
    pub request_header_map: Arc<WpNetworkHeaderMap>,
}

#[derive(Debug, Default, Clone, uniffi::Object)]
pub struct WpNetworkHeaderMap {
    inner: HeaderMap,
}

impl WpNetworkHeaderMap {
    pub fn new(header_map: HeaderMap) -> Self {
        Self { inner: header_map }
    }

    pub fn wp_total(&self) -> Option<u32> {
        self.header_value_as_u32(HEADER_KEY_WP_TOTAL)
    }

    pub fn wp_total_pages(&self) -> Option<u32> {
        self.header_value_as_u32(HEADER_KEY_WP_TOTAL_PAGES)
    }

    pub fn header_value_as_u32(&self, header_name: &str) -> Option<u32> {
        self.inner
            .get(header_name)
            .and_then(|h_v| h_v.to_str().ok())
            .and_then(|h| h.parse().ok())
    }

    // Splits the `header_value` by `,` then parses name & values into `HeaderName` & `HeaderValue`
    fn build_header_name_value(
        header_name: &str,
        header_value: &str,
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
                        header_name: header_name.to_string(),
                    })
                }
            })
            .collect()
    }

    pub fn to_header_map(&self) -> HeaderMap {
        self.inner.clone()
    }

    /// Does this request specify HTTP login details of some kind?
    pub fn has_http_authentication(&self) -> bool {
        self.inner.get(http::header::AUTHORIZATION).is_some()
    }
}

#[uniffi::export]
impl WpNetworkHeaderMap {
    #[uniffi::constructor]
    fn from_multi_map(
        hash_map: HashMap<String, Vec<String>>,
    ) -> Result<Self, WpNetworkHeaderMapError> {
        let inner = hash_map
            .iter()
            .flat_map(|(header_name, values)| {
                values.iter().flat_map(|header_value| {
                    Self::build_header_name_value(header_name, header_value)
                })
            })
            .collect::<Result<HeaderMap, WpNetworkHeaderMapError>>()?;
        Ok(Self { inner })
    }

    #[uniffi::constructor]
    fn from_map(hash_map: HashMap<String, String>) -> Result<Self, WpNetworkHeaderMapError> {
        let inner = hash_map
            .iter()
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

#[derive(Debug, PartialEq, Eq, thiserror::Error, uniffi::Error, WpDeriveLocalizable)]
pub enum WpNetworkHeaderMapError {
    InvalidHeaderName { header_name: String },
    InvalidHeaderValue { header_value: String },
}

impl WpSupportsLocalization for WpNetworkHeaderMapError {
    fn message_bundle(&self) -> MessageBundle {
        match self {
            WpNetworkHeaderMapError::InvalidHeaderName { header_name } => {
                WpMessages::invalid_header_name_error(header_name)
            }
            WpNetworkHeaderMapError::InvalidHeaderValue { header_value } => {
                WpMessages::invalid_header_value_error(header_value)
            }
        }
    }
}

impl WpNetworkResponse {
    pub fn get_link_header(&self, name: &str) -> Vec<Url> {
        [
            self.response_header_map.inner.get_all(http::header::LINK),
            self.response_header_map
                .inner
                .get_all(http::header::LINK.to_string().to_lowercase()),
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

    pub fn get_pagination_header<P: FromUrlQueryPairs>(
        &self,
        header_key: PaginationHeaderKey,
    ) -> Option<P> {
        self.get_link_header(header_key.as_str())
            .first()
            .and_then(|u| {
                P::from_url_query_pairs(UrlQueryPairsMap::new(
                    u.query_pairs().into_iter().collect(),
                ))
            })
    }

    pub fn body_as_string(&self) -> String {
        request_or_response_body_as_string(&self.body)
    }

    pub fn parse<ResponseType, DataType, ParamsType, E>(self) -> Result<ResponseType, E>
    where
        ResponseType: DeserializeOwned,
        ResponseType: From<ParsedResponse<DataType, ParamsType>>,
        ParsedResponse<DataType, ParamsType>: From<ResponseType>,
        ParamsType: FromUrlQueryPairs,
        E: ParsedRequestError,
    {
        if let Some(err) = E::try_parse(&self) {
            return Err(err);
        }

        serde_json::from_slice(&self.body)
            .map_err(|err| E::as_parse_error(err.to_string(), self.body_as_string()))
            .map(|x| {
                let mut parsed_response = ParsedResponse::<DataType, ParamsType>::from(x);
                if ParamsType::supports_pagination() {
                    parsed_response.next_page_params =
                        self.get_pagination_header(PaginationHeaderKey::Next);
                    parsed_response.prev_page_params =
                        self.get_pagination_header(PaginationHeaderKey::Prev);
                }
                parsed_response.header_map = self.response_header_map;
                ResponseType::from(parsed_response)
            })
    }

    pub fn parse_with<F, T>(&self, parser: F) -> Result<T, WpApiError>
    where
        F: Fn(&WpNetworkResponse) -> Result<T, WpApiError>,
    {
        parser(self)
    }

    pub fn is_rate_limit_exceeded(&self) -> bool {
        self.status_code == 429
    }

    pub fn get_retry_after(&self) -> Option<u64> {
        self.response_header_map
            .inner
            .get(http::header::RETRY_AFTER)
            .and_then(|h_v| h_v.to_str().ok())
            .and_then(|s| HttpRetryDuration::from_str(s).ok())
            .map(|d| d.seconds)
    }

    pub fn get_http_auth_method(
        &self,
    ) -> Result<Option<HttpAuthMethod>, HttpAuthMethodParsingError> {
        let header_value = self
            .response_header_map
            .inner
            .get(http::header::WWW_AUTHENTICATE)
            .and_then(|h_v| h_v.to_str().ok());

        if let Some(header_value) = header_value {
            let result = HttpAuthMethod::from_str(header_value);

            return match result {
                Ok(method) => Ok(Some(method)),
                Err(e) => Err(e),
            };
        }

        Ok(None)
    }

    pub fn body_type(&self) -> Result<ResponseBodyType, Utf8Error> {
        std::str::from_utf8(&self.body).map(ResponseBodyType::new)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, uniffi::Enum)]
pub enum ResponseBodyType {
    ValidJson,
    MaybeHtml,
    Other,
}

impl ResponseBodyType {
    pub fn new(body_as_string: impl AsRef<str>) -> Self {
        if is_valid_json(&body_as_string) {
            Self::ValidJson
        } else if is_maybe_html(&body_as_string) {
            Self::MaybeHtml
        } else {
            Self::Other
        }
    }
}

struct HttpRetryDuration {
    seconds: u64,
}

/// Parser based on https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Retry-After
///
impl FromStr for HttpRetryDuration {
    type Err = WpApiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Handle the simple case where the header is a number of seconds
        if let Ok(seconds) = s.parse::<u64>() {
            return Ok(HttpRetryDuration { seconds });
        }

        // Handle the case where the header is a date string
        let parsed =
            DateTime::parse_from_rfc2822(s).map_err(|_| WpApiError::RequestExecutionFailed {
                status_code: None,
                redirects: None,
                reason: RequestExecutionErrorReason::MisconfiguredRateLimitError {},
            })?;

        let now = Utc::now();
        let duration = parsed.signed_duration_since(now);
        let seconds = duration.num_seconds();

        Ok(HttpRetryDuration {
            seconds: seconds as u64,
        })
    }
}

#[derive(PartialEq, Eq, Debug, Clone, uniffi::Record)]
pub struct BasicAuthenticationDetails {
    pub realm: Option<String>,
    pub requires_utf8: bool,
}

#[derive(PartialEq, Eq, Debug, Clone, uniffi::Record)]
pub struct DigestAuthenticationDetails {
    pub realm: Option<String>,
    pub nonce: String,
    pub qop: String,
    pub algorithm: Option<String>,
    pub opaque: String,
    pub requires_utf8: bool,
}

#[derive(PartialEq, Eq, Debug, Clone, uniffi::Enum)]
pub enum HttpAuthMethod {
    Basic(BasicAuthenticationDetails),
    Digest(DigestAuthenticationDetails),
    Other(String, HashMap<String, String>),
}

#[derive(PartialEq, Eq, Debug, Clone, thiserror::Error, uniffi::Error, WpDeriveLocalizable)]
pub enum HttpAuthMethodParsingError {
    MissingNonce,
    MissingQop,
    MissingAlgorithm,
    MissingOpaque,
    Unknown, // Some case we're not handling yet
}

impl WpSupportsLocalization for HttpAuthMethodParsingError {
    fn message_bundle(&self) -> MessageBundle {
        match self {
            HttpAuthMethodParsingError::MissingNonce => {
                WpMessages::http_auth_method_missing_nonce()
            }
            HttpAuthMethodParsingError::MissingQop => WpMessages::http_auth_method_missing_qop(),
            HttpAuthMethodParsingError::MissingAlgorithm => {
                WpMessages::http_auth_method_missing_algorithm()
            }
            HttpAuthMethodParsingError::MissingOpaque => {
                WpMessages::http_auth_method_missing_opaque()
            }
            HttpAuthMethodParsingError::Unknown => WpMessages::http_auth_method_unknown(),
        }
    }
}

/// Parser based on https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/WWW-Authenticate
///
impl FromStr for HttpAuthMethod {
    type Err = HttpAuthMethodParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts = s.splitn(2, ' ').collect::<Vec<&str>>();

        let method = parts[0];

        let mut details = HashMap::new();
        parts[1].split(',').for_each(|part| {
            let parts = part.splitn(2, "=").collect::<Vec<&str>>();
            details.insert(
                parts[0].trim(),
                parts[1].trim_matches('\"').trim_matches('"'),
            );
        });

        if method == "Basic" {
            return Ok(HttpAuthMethod::Basic(BasicAuthenticationDetails {
                realm: details.get("realm").map(|s| s.to_string()),
                requires_utf8: details
                    .get("charset")
                    .map(|s| s.to_string())
                    .unwrap_or_default()
                    .to_ascii_lowercase()
                    .contains("utf-8"),
            }));
        }

        if method == "Digest" {
            return Ok(HttpAuthMethod::Digest(DigestAuthenticationDetails {
                realm: details.get("realm").map(|s| s.to_string()),
                nonce: details
                    .get("nonce")
                    .map(|s| s.to_string())
                    .ok_or(HttpAuthMethodParsingError::MissingNonce)?,
                qop: details
                    .get("qop")
                    .map(|s| s.to_string())
                    .ok_or(HttpAuthMethodParsingError::MissingQop)?,
                algorithm: details.get("algorithm").map(|s| s.to_string()),
                opaque: details
                    .get("opaque")
                    .map(|s| s.to_string())
                    .ok_or(HttpAuthMethodParsingError::MissingOpaque)?,
                requires_utf8: details
                    .get("charset")
                    .map(|s| s.to_string())
                    .unwrap_or_default()
                    .to_ascii_lowercase()
                    .contains("utf-8"),
            }));
        }

        Ok(HttpAuthMethod::Other(
            method.to_string(),
            details
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        ))
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
            self.response_header_map,
            self.body_as_string()
        );
        s.pop(); // Remove the new line at the end
        write!(f, "{}", s)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, uniffi::Enum)]
pub enum RequestMethod {
    GET,
    POST,
    PUT,
    DELETE,
    HEAD,
}

pub fn request_or_response_body_as_string(body: &[u8]) -> String {
    String::from_utf8_lossy(body).to_string()
}

#[derive(Clone, Debug, Eq, PartialEq, uniffi::Record)]
pub struct WpRedirect {
    pub source: String,
    pub destination: String,
}

impl WpRedirect {
    pub fn new(source: String, destination: String) -> Self {
        WpRedirect {
            source,
            destination,
        }
    }
}

pub fn is_maybe_html(value: impl AsRef<str>) -> bool {
    Regex::new(MAYBE_HTML_REGEX)
        .expect("This is a valid regex")
        .is_match(value.as_ref())
}

pub fn is_valid_json(value: impl AsRef<str>) -> bool {
    serde_json::from_str::<serde_json::Value>(value.as_ref()).is_ok()
}

#[uniffi::export]
pub async fn is_valid_authentication(
    request_executor: Arc<dyn RequestExecutor>,
    api_root_url: Arc<ParsedUrl>,
    authentication: WpAuthentication,
) -> bool {
    let request = ApplicationPasswordsRequestBuilder::new(api_root_url, authentication)
        .retrieve_current_with_edit_context()
        .into();
    request_executor.execute(request).await.is_ok()
}

pub mod user_agent {
    #[uniffi::export]
    pub fn default_user_agent(client_specific_postfix: &str) -> String {
        default_user_agent_with_version(client_specific_postfix, version())
    }

    fn default_user_agent_with_version(client_specific_postfix: &str, version: &str) -> String {
        format!(
            "wordpress-rs/{version} (https://github.com/automattic/wordpress-rs);{client_specific_postfix}"
        )
    }

    fn version() -> &'static str {
        crate::WORDPRESS_RS_VERSION
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_default_user_agent() {
            assert_eq!(
                default_user_agent_with_version("reqwest", "1.0"),
                "wordpress-rs/1.0 (https://github.com/automattic/wordpress-rs);reqwest"
            );
        }
    }

    #[cfg(feature = "reqwest-request-executor")]
    pub mod reqwest {
        use super::*;
        use http::HeaderValue;

        const REQWEST_REQUEST_EXECUTOR_USER_AGENT_POST_FIX: &str = "reqwest";

        pub fn user_agent_for_reqwest_request_executor() -> HeaderValue {
            user_agent_for_reqwest_request_executor_with_version(version())
        }

        fn user_agent_for_reqwest_request_executor_with_version(version: &str) -> HeaderValue {
            HeaderValue::from_str(
                default_user_agent_with_version(
                    REQWEST_REQUEST_EXECUTOR_USER_AGENT_POST_FIX,
                    version,
                )
                .as_str(),
            )
            .expect("Reqwest user agent is a valid header value as proven by its unit test")
        }

        #[cfg(test)]
        mod tests {
            use super::*;

            #[test]
            fn test_reqwest_user_agent() {
                assert_eq!(
                    user_agent_for_reqwest_request_executor_with_version("1.0")
                        .to_str()
                        .expect("Reqwest user agent header value can be converted to str"),
                    "wordpress-rs/1.0 (https://github.com/automattic/wordpress-rs);reqwest"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use rstest::*;
    use std::ops::Add;
    use std::time::Duration;

    impl WpNetworkHeaderMap {
        pub fn insert(&mut self, header_name: HeaderName, header_value: String) {
            let value = header_value.parse().expect("Header Value must be ascii");
            self.inner.insert(header_name, value);
        }
    }

    #[rstest]
    #[case(r#"<foo>"#, ResponseBodyType::MaybeHtml)]
    #[case(r#"</foo>"#, ResponseBodyType::MaybeHtml)]
    #[case(r#"{"foo": "bar"}"#, ResponseBodyType::ValidJson)]
    #[case("foo", ResponseBodyType::Other)]
    #[case("", ResponseBodyType::Other)]
    fn test_response_body_type(#[case] input: &str, #[case] expected: ResponseBodyType) {
        assert_eq!(ResponseBodyType::new(input), expected);
    }

    #[rstest]
    #[case(
        "<http://localhost/wp-json/wp/v2/posts?page=2>; rel=\"next\"",
        None,
        Some("http://localhost/wp-json/wp/v2/posts?page=2")
    )]
    #[case(
        "<http://localhost/wp-json/wp/v2/posts?page=1>; rel=\"prev\", <http://localhost/wp-json/wp/v2/posts?page=3>; rel=\"next\"",
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
            response_header_map: Arc::new(
                WpNetworkHeaderMap::from_map([("Link".to_string(), link.to_string())].into())
                    .unwrap(),
            ),
            request_url: WpEndpointUrl("http://example.com".to_string()),
            request_header_map: Arc::new(WpNetworkHeaderMap::default()),
        };
        assert_eq!(
            expected_prev_link_header
                .and_then(|s| Url::parse(s).ok())
                .as_ref(),
            response.get_link_header("prev").first(),
            "response headers: {:?}",
            response.response_header_map.inner
        );
        assert_eq!(
            expected_next_link_header
                .and_then(|s| Url::parse(s).ok())
                .as_ref(),
            response.get_link_header("next").first(),
            "response headers: {:?}",
            response.response_header_map.inner
        );
    }

    #[test]
    fn test_header_map_from_map() {
        let hash_map = [
            ("Age".to_string(), "1,2".to_string()),
            (
                http::header::LINK.to_string(),
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
            http::header::LINK.as_str(),
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
                http::header::LINK.to_string(),
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
            http::header::LINK.as_str(),
            vec![
                "<https://one.example.com>; rel=\"preconnect\"",
                "<https://two.example.com>",
            ],
        );
        assert_header_map_values(&header_map, "Retry-After", vec!["120"]);
        assert_header_map_values(&header_map, "User-Agent", vec![]);
    }

    #[rstest]
    #[case(r#"Basic realm="example", charset="UTF-8""#, HttpAuthMethod::Basic(BasicAuthenticationDetails {
        realm: Some("example".to_string()),
        requires_utf8: true,
    }))]
    #[case(r#"Basic realm="example""#, HttpAuthMethod::Basic(BasicAuthenticationDetails {
        realm: Some("example".to_string()),
        requires_utf8: false,
    }))]
    #[case(r#"Digest realm="example", nonce="123", qop="auth", algorithm="MD5", opaque="123""#, HttpAuthMethod::Digest(DigestAuthenticationDetails {
        realm: Some("example".to_string()),
        nonce: "123".to_string(),
        qop: "auth".to_string(),
        algorithm: Some("MD5".to_string()),
        opaque: "123".to_string(),
        requires_utf8: false,
    }))]
    #[case(r#"Digest realm="secure", nonce="xyz789", qop="auth", algorithm="SHA-256", opaque="abc""#, HttpAuthMethod::Digest(DigestAuthenticationDetails {
        realm: Some("secure".to_string()),
        nonce: "xyz789".to_string(),
        qop: "auth".to_string(),
        algorithm: Some("SHA-256".to_string()),
        opaque: "abc".to_string(),
        requires_utf8: false,
    }))]
    #[case(r#"Digest realm="api", nonce="456def", qop="auth-int", algorithm="SHA-512", opaque="def""#, HttpAuthMethod::Digest(DigestAuthenticationDetails {
        realm: Some("api".to_string()),
        nonce: "456def".to_string(),
        qop: "auth-int".to_string(),
        algorithm: Some("SHA-512".to_string()),
        opaque: "def".to_string(),
        requires_utf8: false,
    }))]
    #[case(r#"Digest realm="test", nonce="789ghi", qop="auth", algorithm="SHA3-256", opaque="ghi", charset="UTF-8""#, HttpAuthMethod::Digest(DigestAuthenticationDetails {
        realm: Some("test".to_string()),
        nonce: "789ghi".to_string(),
        qop: "auth".to_string(),
        algorithm: Some("SHA3-256".to_string()),
        opaque: "ghi".to_string(),
        requires_utf8: true,
    }))]

    fn assert_get_http_auth_message_parses_valid_values(
        #[case] header_value: &str,
        #[case] expected_value: HttpAuthMethod,
    ) {
        let mut header_map: WpNetworkHeaderMap = WpNetworkHeaderMap::new(HeaderMap::new());
        header_map.insert(http::header::WWW_AUTHENTICATE, header_value.to_string());

        let response = WpNetworkResponse {
            body: Vec::with_capacity(0),
            status_code: 401,
            response_header_map: Arc::new(header_map),
            request_url: WpEndpointUrl("http://example.com".to_string()),
            request_header_map: Arc::new(WpNetworkHeaderMap::default()),
        };

        assert_eq!(response.get_http_auth_method(), Ok(Some(expected_value)));
    }

    #[rstest]
    #[case("120", 120)]
    #[case( Utc::now().add(Duration::from_secs(121)).to_rfc2822(), 120)]
    fn assert_get_http_retry_duration_parses_valid_values(
        #[case] header_value: String,
        #[case] expected_value: u64,
    ) {
        let mut header_map: WpNetworkHeaderMap = WpNetworkHeaderMap::new(HeaderMap::new());
        header_map.insert(http::header::RETRY_AFTER, header_value);

        let response = WpNetworkResponse {
            body: Vec::with_capacity(0),
            status_code: 429,
            response_header_map: Arc::new(header_map),
            request_url: WpEndpointUrl("http://example.com".to_string()),
            request_header_map: Arc::new(WpNetworkHeaderMap::default()),
        };

        assert_eq!(response.get_retry_after(), Some(expected_value));
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

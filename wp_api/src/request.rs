use std::{collections::HashMap, fmt::Debug};

use http::HeaderMap;
use serde::Deserialize;
use url::Url;

use crate::login::WpApiDetails;
use crate::WpApiError;

use self::endpoint::WpEndpointUrl;

pub mod endpoint;
pub mod plugins_request_builder;
pub mod users_request_builder;

const LINK_HEADER_KEY: &str = "Link";

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
#[derive(uniffi::Object)]
pub struct WpNetworkResponse {
    body: Vec<u8>,
    status_code: u16,
    // TODO: We probably want to implement a specific type for these headers instead of using a
    // regular HashMap.
    //
    // It could be something similar to `reqwest`'s [`header`](https://docs.rs/reqwest/latest/reqwest/header/index.html)
    // module.
    headers: HeaderMap,
}

#[uniffi::export]
impl WpNetworkResponse {
    #[uniffi::constructor]
    pub fn new(
        body: Vec<u8>,
        status_code: u16,
        header_map: Option<HashMap<String, String>>,
    ) -> Self {
        let headers: HeaderMap = header_map
            .and_then(|m| (&m).try_into().ok())
            .unwrap_or_default();

        Self {
            body,
            status_code,
            headers,
        }
    }

    pub fn parse_api_details_response(&self) -> Result<WpApiDetails, WpApiError> {
        serde_json::from_slice(&self.body).map_err(|err| WpApiError::ParsingError {
            reason: err.to_string(),
            response: self.body_as_string(),
        })
    }
}

impl WpNetworkResponse {
    pub fn get_link_header(&self, name: &str) -> Option<Url> {
        self.headers
            .get(LINK_HEADER_KEY)
            .and_then(|v| v.to_str().ok())
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
            self.headers,
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

#[macro_export]
macro_rules! add_uniffi_exported_parser {
    ($fn_name:ident, $return_type: ty) => {
        #[uniffi::export]
        pub fn $fn_name(response: &WpNetworkResponse) -> Result<$return_type, WpApiError> {
            response.parse::<$return_type>()
        }
    };
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
        let response = WpNetworkResponse::new(
            Vec::with_capacity(0),
            200,
            Some([("Link".to_string(), link.to_string())].into()),
        );

        assert_eq!(
            expected_prev_link_header.and_then(|s| Url::parse(s).ok()),
            response.get_link_header("prev")
        );
        assert_eq!(
            expected_next_link_header.and_then(|s| Url::parse(s).ok()),
            response.get_link_header("next")
        );
    }

    #[rstest]
    fn test_headers_case_insentive() {
        let headers: HashMap<String, String> = [
            ("server".to_string(), "nginx".to_string()),
            ("x-nananana".to_string(), "Batcache-Hit".to_string()),
            (
                "date".to_string(),
                "Thu, 30 May 2024 23:52:17 GMT".to_string(),
            ),
            (
                "content-type".to_string(),
                "text/html; charset=UTF-8".to_string(),
            ),
            (
                "strict-transport-security".to_string(),
                "max-age=31536000".to_string(),
            ),
            ("vary".to_string(), "Accept-Encoding".to_string()),
            (
                "Link".to_string(),
                "<http://localhost/wp-json/wp/v2/posts?page=2>; rel=\"next\"".to_string(),
            ),
        ]
        .into();
        let response = WpNetworkResponse::new(Vec::with_capacity(0), 200, Some(headers));

        assert_eq!(response.headers.get("Server").unwrap(), "nginx");
        assert_eq!(response.headers.get("X-Nananana").unwrap(), "Batcache-Hit");
        assert_eq!(
            response.headers.get("Date").unwrap(),
            "Thu, 30 May 2024 23:52:17 GMT"
        );
        assert_eq!(
            response.headers.get("Content-Type").unwrap(),
            "text/html; charset=UTF-8"
        );
        assert_eq!(
            response.headers.get("link").unwrap(),
            "<http://localhost/wp-json/wp/v2/posts?page=2>; rel=\"next\""
        );
    }
}

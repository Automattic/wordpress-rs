use std::{collections::HashMap, fmt::Debug};

use serde::Deserialize;
use url::Url;

use crate::WPApiError;

use self::endpoint::WpEndpointUrl;

pub mod endpoint;
pub mod plugins_request_builder;
pub mod users_request_builder;

const LINK_HEADER_KEY: &str = "Link";

// Has custom `Debug` trait implementation
#[derive(uniffi::Record)]
pub struct WPNetworkRequest {
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

impl WPNetworkRequest {
    pub fn body_as_string(&self) -> Option<String> {
        self.body.as_ref().map(|b| body_as_string(b))
    }
}

impl Debug for WPNetworkRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = format!(
            indoc::indoc! {"
                WPNetworkRequest {{
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
pub struct WPNetworkResponse {
    pub body: Vec<u8>,
    pub status_code: u16,
    // TODO: We probably want to implement a specific type for these headers instead of using a
    // regular HashMap.
    //
    // It could be something similar to `reqwest`'s [`header`](https://docs.rs/reqwest/latest/reqwest/header/index.html)
    // module.
    pub header_map: Option<HashMap<String, String>>,
}

impl WPNetworkResponse {
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

    pub fn parse<'de, T: Deserialize<'de>>(&'de self) -> Result<T, WPApiError> {
        self.parse_response_for_generic_errors()?;
        serde_json::from_slice(&self.body).map_err(|err| WPApiError::ParsingError {
            reason: err.to_string(),
            response: self.body_as_string(),
        })
    }

    pub fn parse_with<F, T>(&self, parser: F) -> Result<T, WPApiError>
    where
        F: Fn(&WPNetworkResponse) -> Result<T, WPApiError>,
    {
        parser(self)
    }

    fn parse_response_for_generic_errors(&self) -> Result<(), WPApiError> {
        // TODO: Further parse the response body to include error message
        // TODO: Lots of unwraps to get a basic setup working
        let status = http::StatusCode::from_u16(self.status_code).unwrap();
        if let Ok(rest_error) = serde_json::from_slice(&self.body) {
            Err(WPApiError::RestError {
                rest_error,
                status_code: self.status_code,
                response: self.body_as_string(),
            })
        } else if status.is_client_error() || status.is_server_error() {
            Err(WPApiError::UnknownError {
                status_code: self.status_code,
                response: self.body_as_string(),
            })
        } else {
            Ok(())
        }
    }
}

impl Debug for WPNetworkResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = format!(
            indoc::indoc! {"
                WPNetworkResponse {{
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

#[macro_export]
macro_rules! add_uniffi_exported_parser {
    ($fn_name:ident, $return_type: ty) => {
        #[uniffi::export]
        pub fn $fn_name(response: &WPNetworkResponse) -> Result<$return_type, WPApiError> {
            response.parse::<$return_type>()
        }
    };
}


#[cfg(test)]
mod tests {
    use super::*;
    use claim::assert_none;

    #[test]
    fn test_link_header_can_be_parsed_for_first_page() {
        let response = WPNetworkResponse{ body: Vec::with_capacity(0), status_code: 200, header_map: Some(HashMap::from([("Link".to_string(), "<http://localhost/wp-json/wp/v2/posts?page=2>; rel=\"next\"".to_string())])) };
        assert_eq!(response.get_link_header("next").unwrap(), Url::parse("http://localhost/wp-json/wp/v2/posts?page=2").unwrap());
        assert_none!(response.get_link_header("prev"));
    }

    #[test]
    fn test_link_header_can_be_parsed_for_intermediate_pages() {
        let response = WPNetworkResponse{ body: Vec::with_capacity(0), status_code: 200, header_map: Some(HashMap::from([("Link".to_string(), "<http://localhost/wp-json/wp/v2/posts?page=1>; rel=\"prev\", <http://localhost/wp-json/wp/v2/posts?page=3>; rel=\"next\"".to_string())])) };
        assert_eq!(response.get_link_header("prev").unwrap(), Url::parse("http://localhost/wp-json/wp/v2/posts?page=1").unwrap());
        assert_eq!(response.get_link_header("next").unwrap(), Url::parse("http://localhost/wp-json/wp/v2/posts?page=3").unwrap());
    }

    #[test]
    fn test_link_header_can_be_parsed_for_last_page() {
        let response = WPNetworkResponse{ body: Vec::with_capacity(0), status_code: 200, header_map: Some(HashMap::from([("Link".to_string(), "<http://localhost/wp-json/wp/v2/posts?page=5>; rel=\"prev\"".to_string())])) };
        assert_none!(response.get_link_header("next"));
        assert_eq!(response.get_link_header("prev").unwrap(), Url::parse("http://localhost/wp-json/wp/v2/posts?page=5").unwrap());
    }
}

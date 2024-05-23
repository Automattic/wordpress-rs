use std::{collections::HashMap, fmt::Debug};

use url::Url;

use crate::RequestMethod;

pub mod endpoint;

#[derive(Debug, uniffi::Record)]
pub struct WPNetworkRequest {
    pub method: RequestMethod,
    pub url: String,
    // TODO: We probably want to implement a specific type for these headers instead of using a
    // regular HashMap.
    //
    // It could be something similar to `reqwest`'s [`header`](https://docs.rs/reqwest/latest/reqwest/header/index.html)
    // module.
    pub header_map: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
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
        if let Some(headers) = self.header_map.clone() {
            // TODO: This is inefficient
            if headers.contains_key("Link") {
                if let Ok(res) = parse_link_header::parse_with_rel(&headers["Link"]) {
                    if let Some(next) = res.get(name) {
                        if let Ok(url) = Url::parse(next.raw_uri.as_str()) {
                            return Some(url);
                        }
                    }
                }
            }
        }
        None
    }

    pub fn body_as_string(&self) -> String {
        String::from_utf8_lossy(&self.body).to_string()
    }
}

impl Debug for WPNetworkResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
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
        )
    }
}

#![allow(dead_code, unused_variables)]

use std::collections::HashMap;
use url::Url;

pub use api_error::*;
pub use pages::*;
pub use posts::*;

pub mod api_error;
pub mod pages;
pub mod posts;

#[derive(uniffi::Object)]
pub struct WPApiHelper {
    site_url: Url,
    authentication: WPAuthentication,
}

#[uniffi::export]
impl WPApiHelper {
    #[uniffi::constructor]
    pub fn new(site_url: String, authentication: WPAuthentication) -> Self {
        let url = Url::parse(site_url.as_str()).unwrap();

        Self {
            site_url: url,
            authentication,
        }
    }

    pub fn raw_request(&self, url: String) -> WPNetworkRequest {
        let mut header_map = HashMap::new();

        header_map.insert(
            "Authorization".into(),
            format!("Basic {}", self.authentication.auth_token),
        );

        WPNetworkRequest {
            method: RequestMethod::GET,
            url: Url::parse(url.as_str()).unwrap().into(),
            header_map: Some(header_map),
        }
    }

    pub fn post_list_request(&self, params: PostListParams) -> WPNetworkRequest {
        let mut url = self
            .site_url
            .join("/wp-json/wp/v2/posts?context=edit")
            .unwrap();

        let mut header_map = HashMap::new();
        header_map.insert(
            "Authorization".into(),
            format!("Basic {}", self.authentication.auth_token),
        );

        url.query_pairs_mut()
            .append_pair("page", params.page.to_string().as_str());
        url.query_pairs_mut()
            .append_pair("per_page", params.per_page.to_string().as_str());

        WPNetworkRequest {
            method: RequestMethod::GET,
            url: url.into(),
            header_map: Some(header_map),
        }
    }
}

#[derive(Debug, Clone, uniffi::Record)]
// TODO: This will probably become an `enum` where we support multiple authentication types.
pub struct WPAuthentication {
    pub auth_token: String,
}

#[derive(uniffi::Enum)]
pub enum RequestMethod {
    GET,
    POST,
    PUT,
    DELETE,
}

#[derive(uniffi::Record)]
pub struct WPNetworkRequest {
    pub method: RequestMethod,
    pub url: String,
    // TODO: We probably want to implement a specific type for these headers instead of using a
    // regular HashMap.
    //
    // It could be something similar to `reqwest`'s [`header`](https://docs.rs/reqwest/latest/reqwest/header/index.html)
    // module.
    pub header_map: Option<HashMap<String, String>>,
}

#[derive(Debug, uniffi::Record)]
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
    fn as_error(&self) -> Option<WPRestError> {
        if self.status_code >= 400 {
            WPRestError::from_slice(&self.body)
        } else {
            None
        }
    }
}

#[uniffi::export]
pub fn parse_post_list_response(
    response: WPNetworkResponse,
) -> Result<PostListResponse, WPApiError> {
    // TODO: Further parse the response body to include error message
    // TODO: Lots of unwraps to get a basic setup working
    if let Some(endpoint_error) = response.as_error() {
        return Err(WPApiError::EndpointError {
            status_code: response.status_code,
            error: endpoint_error,
        });
    }

    if response.status_code != 200 {
        return Err(WPApiError::UnacceptableStatusCodeError { response: response });
    }

    let parsed: Result<Vec<PostObject>, _> = serde_json::from_slice(&response.body);
    if let Err(err) = parsed {
        return Err(WPApiError::ParsingError {
            reason: err.to_string(),
            response: response,
        });
    }

    let post_list: Vec<PostObject> = parsed.unwrap();

    let mut next_page: Option<String> = None;

    if let Some(link_header) = extract_link_header(&response) {
        if let Ok(res) = parse_link_header::parse_with_rel(link_header.as_str()) {
            if let Some(next) = res.get("next") {
                next_page = Some(next.raw_uri.clone())
            }
        }
    }

    Ok(PostListResponse {
        post_list: Some(post_list),
        next_page,
    })
}

pub fn extract_link_header(response: &WPNetworkResponse) -> Option<String> {
    if let Some(headers) = response.header_map.clone() {
        // TODO: This is inefficient
        if headers.contains_key("Link") {
            return Some(headers["Link"].clone());
        }
    }

    None
}

uniffi::setup_scaffolding!("wp_api");

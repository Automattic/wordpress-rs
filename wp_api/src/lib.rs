#![allow(dead_code, unused_variables)]

use http::header::*;
use std::collections::HashMap;

pub use api_error::*;
pub use login::*;
pub use pages::*;
pub use posts::*;
pub use url::*;

pub mod api_error;
pub mod login;
pub mod pages;
pub mod paginator;
pub mod posts;
pub mod url;

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

        match &self.authentication {
            WPAuthentication::AuthorizationHeader { token } => {
                header_map.insert("Authorization".into(), format!("Basic {}", token));
            }
            WPAuthentication::None => (),
        }

        WPNetworkRequest {
            method: RequestMethod::GET,
            url: Url::parse(url.as_str()).unwrap().into(),
            header_map: Some(header_map),
        }
    }

    pub fn post_list_request(&self, params: PostListParams) -> WPNetworkRequest {
        let mut url = self.site_url.join("/wp-json/wp/v2/posts").unwrap();

        let mut header_map = HashMap::new();

        match &self.authentication {
            WPAuthentication::AuthorizationHeader { token } => {
                header_map.insert("Authorization".into(), format!("Basic {}", token));
            }
            WPAuthentication::None => (),
        }

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

impl WPApiHelper {
    pub fn request<'a, QI>(
        &self,
        route: &String,
        query: QI,
        method: Option<RequestMethod>,
    ) -> Result<WPNetworkRequest, WPApiError>
    where QI: Iterator<Item = &'a QueryItem>
    {
        let method = method.unwrap_or(RequestMethod::GET);

        let mut url = self
            .site_url
            // TODO: The `join` function does not suit our need here. We need an 'append' function.
            .join(format!("wp-json/{}", route).as_str())
            .map_err(|err| WPApiError::RequestEncodingError {
                reason: err.to_string(),
            })?;

        let mut header_map: HashMap<String, String> = HashMap::new();
        header_map.insert("Accept".into(), "application/json".into());

        match &self.authentication {
            WPAuthentication::AuthorizationHeader { token } => {
                header_map.insert("Authorization".into(), format!("Basic {}", token));
            }
            WPAuthentication::None => (),
        }

        for item in query {
            url.query_pairs_mut()
                .append_pair(item.name.as_str(), item.value.as_str());
        }

        Ok(WPNetworkRequest {
            method,
            url: url.into(),
            header_map: Some(header_map),
        })
    }
}

#[derive(Debug, Clone, uniffi::Enum)]
pub enum WPAuthentication {
    AuthorizationHeader { token: String },
    None,
}

#[derive(Debug, uniffi::Record)]
pub struct QueryItem {
    name: String,
    value: String,
}

#[derive(uniffi::Enum)]
pub enum RequestMethod {
    GET,
    POST,
    PUT,
    DELETE,
    HEAD,
}

impl RequestMethod {
    fn allow_body(&self) -> bool {
        matches!(self, RequestMethod::POST | RequestMethod::PUT)
    }
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
    fn headers(&self) -> HeaderMap {
        let mut map = HeaderMap::new();
        if let Some(headers) = &self.header_map {
            for (key, value) in headers {
                map.insert(
                    HeaderName::from_bytes(key.as_bytes()).unwrap(),
                    value.parse().unwrap(),
                );
            }
        }
        map
    }

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
}

#[uniffi::export]
pub fn parse_post_list_response(
    response: WPNetworkResponse,
) -> Result<PostListResponse, WPApiError> {
    // TODO: Further parse the response body to include error message
    // TODO: Lots of unwraps to get a basic setup working
    if let Some(client_error_type) = ClientErrorType::from_status_code(response.status_code) {
        return Err(WPApiError::ClientError {
            error_type: client_error_type,
            status_code: response.status_code,
        });
    }
    let status = http::StatusCode::from_u16(response.status_code).unwrap();
    if status.is_server_error() {
        return Err(WPApiError::ServerError {
            status_code: response.status_code,
        });
    }
    let post_list: Vec<PostObject> =
        serde_json::from_slice(&response.body).map_err(|err| WPApiError::ParsingError {
            reason: err.to_string(),
            response: std::str::from_utf8(&response.body).unwrap().to_string(),
        })?;

    let headers = response.headers();
    let next_page = headers
        .get("Link")
        .map(|f| f.to_str().unwrap_or_default())
        .and_then(|f| get_link_header_rel_url(f, "next"))
        .map(|f| f.into());
    let total = headers
        .get("X-WP-Total")
        .map(|f| f.to_str().unwrap_or_default())
        .and_then(|f| f.parse::<u32>().ok());
    let total_pages = headers
        .get("X-WP-TotalPages")
        .map(|f| f.to_str().unwrap_or_default())
        .and_then(|f| f.parse::<u32>().ok());

    Ok(PostListResponse {
        post_list: Some(post_list),
        next_page,
        total,
        total_pages,
    })
}

#[uniffi::export]
pub fn parse_api_details_response(response: WPNetworkResponse) -> Result<WPAPIDetails, WPApiError> {
    let api_details =
        serde_json::from_slice(&response.body).map_err(|err| WPApiError::ParsingError {
            reason: err.to_string(),
            response: std::str::from_utf8(&response.body).unwrap().to_string(),
        })?;

    Ok(api_details)
}

// TODO: Figure out why we can't expose this method on `WPNetworkResponse` via UniFFI
#[uniffi::export]
pub fn get_link_header(response: &WPNetworkResponse, name: &str) -> Option<WPRestAPIURL> {
    if let Some(url) = response.get_link_header(name) {
        return Some(url.into());
    }

    None
}

fn get_link_header_rel_url(link: &str, name: &str) -> Option<Url> {
    parse_link_header::parse_with_rel(link)
        .unwrap_or_default()
        .get(name)
        .and_then(|f| Url::parse(f.raw_uri.as_str()).ok())
}

uniffi::setup_scaffolding!("wp_api");

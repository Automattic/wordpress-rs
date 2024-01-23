#![allow(dead_code, unused_variables)]

use http::Method;
use reqwest::blocking::Client;
use wp_api::{
    ClientErrorType, PostListParams, PostListResponse, PostObject, WPApiError, WPAuthentication,
};

pub struct WPApi {
    client: Client,
    site_url: String,
    authentication: WPAuthentication,
}

impl WPApi {
    pub fn new(site_url: String, authentication: WPAuthentication) -> Self {
        Self {
            client: reqwest::blocking::Client::new(),
            site_url,
            authentication,
        }
    }

    pub fn list_posts(
        &self,
        params: Option<PostListParams>,
    ) -> Result<PostListResponse, WPApiError> {
        let url = format!("{}/wp-json/wp/v2/posts?context=edit", self.site_url);
        let response = self
            .client
            .request(Method::GET, url)
            .header(
                "Authorization",
                format!("Basic {}", self.authentication.auth_token),
            )
            .send()
            .unwrap();
        parse_list_posts_response(response)
    }
}

fn parse_list_posts_response(
    response: reqwest::blocking::Response,
) -> Result<PostListResponse, WPApiError> {
    let status_code = response.status().as_u16();
    // TODO: Further parse the response body to include error message
    if let Some(client_error_type) = ClientErrorType::from_status_code(status_code) {
        return Err(WPApiError::ClientError {
            error_type: client_error_type,
            status_code,
        });
    }
    if response.status().is_server_error() {
        return Err(WPApiError::ServerError { status_code });
    }
    let body = response.text().unwrap();
    let post_list: Vec<PostObject> = serde_json::from_str(&body).or_else(|err| {
        Err(WPApiError::ParsingError {
            reason: err.to_string(),
            response: body,
        })
    })?;
    Ok(PostListResponse {
        post_list: Some(post_list),
    })
}

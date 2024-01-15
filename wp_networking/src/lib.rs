#![allow(dead_code, unused_variables)]
use std::{collections::HashMap, sync::Arc};

use reqwest::{blocking::Client, header::HeaderMap};
use wp_api::{
    PageListParams, PageListResponse, PostCreateParams, PostCreateResponse, PostDeleteParams,
    PostDeleteResponse, PostListParams, PostListResponse, PostObject, PostRetrieveParams,
    PostRetrieveResponse, PostUpdateParams, PostUpdateResponse, WPApiError, WPApiInterface,
    WPAuthentication, WPNetworkRequest, WPNetworkResponse, WPNetworkingInterface,
};

pub fn add_custom(left: i32, right: i32) -> i32 {
    left + right
}

pub fn combine_strings(a: String, b: String) -> String {
    format!("{}-{}", a, b)
}

pub fn panic_from_rust() {
    std::fs::read_to_string("doesnt_exist.txt").unwrap();
}

pub fn wp_api(site_url: String, authentication: WPAuthentication) -> Arc<dyn WPApiInterface> {
    Arc::new(WPApi {
        site_url,
        authentication,
        networking_interface: Arc::new(WPNetworking::default()),
    })
}

pub fn wp_api_with_custom_networking(
    site_url: String,
    authentication: WPAuthentication,
    networking_interface: Arc<dyn WPNetworkingInterface>,
) -> Arc<dyn WPApiInterface> {
    Arc::new(WPApi {
        site_url,
        authentication,
        networking_interface,
    })
}

struct WPNetworking {
    client: Client,
}

impl Default for WPNetworking {
    fn default() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

impl WPNetworkingInterface for WPNetworking {
    fn request(&self, request: WPNetworkRequest) -> wp_api::WPNetworkResponse {
        let method = match request.method {
            wp_api::RequestMethod::GET => reqwest::Method::GET,
            wp_api::RequestMethod::POST => reqwest::Method::POST,
            wp_api::RequestMethod::PUT => reqwest::Method::PUT,
            wp_api::RequestMethod::DELETE => reqwest::Method::DELETE,
        };

        let request_headers: HeaderMap = (&request.header_map.unwrap()).try_into().unwrap();

        // TODO: Error handling
        let response = self
            .client
            .request(method, request.url)
            .headers(request_headers)
            .send()
            .unwrap();
        WPNetworkResponse {
            status: Arc::new(response.status()),
            body: response.text().unwrap().as_bytes().to_vec(),
        }
    }
}

struct WPApi {
    site_url: String,
    authentication: WPAuthentication,
    networking_interface: Arc<dyn WPNetworkingInterface>,
}

impl WPApiInterface for WPApi {
    fn list_posts(&self, params: Option<PostListParams>) -> Result<PostListResponse, WPApiError> {
        let mut header_map = HashMap::new();
        // TODO: Authorization headers should be generated through its type not like a cave man
        header_map.insert(
            "Authorization".into(),
            format!("Basic {}", self.authentication.auth_token).into(),
        );

        let response = self.networking_interface.request(WPNetworkRequest {
            method: wp_api::RequestMethod::GET,
            // TODO: Centralize the endpoints
            url: format!("{}/wp-json/wp/v2/posts?context=edit", self.site_url).into(),
            header_map: Some(header_map),
        });
        parse_list_posts_response(&response)
    }

    fn create_post(&self, params: Option<PostCreateParams>) -> PostCreateResponse {
        todo!()
    }

    fn retrieve_post(
        &self,
        post_id: u32,
        params: Option<PostRetrieveParams>,
    ) -> PostRetrieveResponse {
        todo!()
    }

    fn update_post(&self, post_id: u32, params: Option<PostUpdateParams>) -> PostUpdateResponse {
        todo!()
    }

    fn delete_post(&self, post_id: u32, params: Option<PostDeleteParams>) -> PostDeleteResponse {
        todo!()
    }

    fn list_pages(&self, params: Option<PageListParams>) -> PageListResponse {
        todo!()
    }
}

fn parse_list_posts_response(response: &WPNetworkResponse) -> Result<PostListResponse, WPApiError> {
    let post_list: Vec<PostObject> = serde_json::from_slice(&response.body).or_else(|err| {
        Err(WPApiError::InvalidResponseError {
            reason: err.to_string(),
            response: std::str::from_utf8(&response.body).unwrap().to_string(),
        })
    })?;
    Ok(PostListResponse {
        post_list: Some(post_list),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add_custom(2, 2);
        assert_eq!(result, 4);
    }

    #[test]
    fn test_combine_strings() {
        let result = combine_strings("this".into(), "that".into());
        assert_eq!(result, "this-that");
    }

    #[test]
    #[should_panic]
    fn test_panic_from_rust() {
        panic_from_rust()
    }
}

uniffi::include_scaffolding!("wp_networking");

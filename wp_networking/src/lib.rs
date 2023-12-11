#![allow(dead_code, unused_variables)]
use std::sync::Arc;

use reqwest::blocking::Client;
use wp_api::{
    ParsedPostListResponse, WPApiInterface, WPAuthentication, WPNetworkRequest, WPNetworkResponse,
    WPNetworkingInterface,
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

pub fn wp_api(authentication: WPAuthentication) -> Arc<dyn WPApiInterface> {
    Arc::new(WPApi {
        authentication,
        networking_interface: Arc::new(WPNetworking::default()),
    })
}

pub fn wp_api_with_custom_networking(
    authentication: WPAuthentication,
    networking_interface: Arc<dyn WPNetworkingInterface>,
) -> Arc<dyn WPApiInterface> {
    Arc::new(WPApi {
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

        // TODO: Error handling
        let json = self
            .client
            .request(method, request.url)
            .send()
            .unwrap()
            .json()
            .unwrap();

        WPNetworkResponse { json }
    }
}

struct WPApi {
    authentication: WPAuthentication,
    networking_interface: Arc<dyn WPNetworkingInterface>,
}

impl WPApiInterface for WPApi {
    fn list_posts(&self, params: Option<wp_api::PostListParams>) -> ParsedPostListResponse {
        let response = self.networking_interface.request(WPNetworkRequest {
            method: wp_api::RequestMethod::GET,
            // TODO: Correct URL
            url: "".into(),
        });
        serde_json::from_str(response.json.as_str()).unwrap()
    }

    fn create_post(
        &self,
        params: Option<wp_api::PostCreateParams>,
    ) -> wp_api::ParsedPostCreateResponse {
        todo!()
    }

    fn retrieve_post(
        &self,
        post_id: u32,
        params: Option<wp_api::PostRetrieveParams>,
    ) -> wp_api::ParsedPostRetrieveResponse {
        todo!()
    }

    fn update_post(
        &self,
        post_id: u32,
        params: Option<wp_api::PostUpdateParams>,
    ) -> wp_api::ParsedPostUpdateResponse {
        todo!()
    }

    fn delete_post(
        &self,
        post_id: u32,
        params: Option<wp_api::PostDeleteParams>,
    ) -> wp_api::ParsedPostDeleteResponse {
        todo!()
    }

    fn list_pages(&self, params: Option<wp_api::PageListParams>) -> wp_api::ParsedPageListResponse {
        todo!()
    }
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

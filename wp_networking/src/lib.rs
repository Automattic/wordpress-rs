#![allow(dead_code, unused_variables)]
use std::{collections::HashMap, sync::Arc};

use wp_api::{
    ClientErrorType, PageListParams, PageListResponse, PostCreateParams, PostCreateResponse,
    PostDeleteParams, PostDeleteResponse, PostListParams, PostListResponse, PostObject,
    PostRetrieveParams, PostRetrieveResponse, PostUpdateParams, PostUpdateResponse, WPApiError,
    WPApiInterface, WPAuthentication, WPNetworkRequest, WPNetworkResponse, WPNetworkingInterface,
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
    let status_code = response.status.as_u16();
    // TODO: Further parse the response body to include error message
    if let Some(client_error_type) = ClientErrorType::from_status_code(status_code) {
        return Err(WPApiError::ClientError {
            error_type: client_error_type,
            status_code,
        });
    }
    if response.status.is_server_error() {
        return Err(WPApiError::ServerError { status_code });
    }
    let post_list: Vec<PostObject> = serde_json::from_slice(&response.body).or_else(|err| {
        Err(WPApiError::ParsingError {
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

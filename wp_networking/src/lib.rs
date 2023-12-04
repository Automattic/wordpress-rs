#![allow(dead_code, unused_variables)]
use std::sync::Arc;

use wp_api::{PostListRequest, PostListResponse, PostNetworkingInterface};

pub fn add_custom(left: i32, right: i32) -> i32 {
    left + right
}

pub fn combine_strings(a: String, b: String) -> String {
    format!("{}-{}", a, b)
}

pub fn panic_from_rust() {
    std::fs::read_to_string("doesnt_exist.txt").unwrap();
}

pub fn post_networking() -> Arc<dyn PostNetworkingInterface> {
    WPPostNetworking {}
}

struct WPPostNetworking {}

impl PostNetworkingInterface for WPPostNetworking {
    fn list(&self, request: PostListRequest) -> PostListResponse {
        todo!()
    }

    fn create(&self, request: wp_api::PostCreateRequest) -> wp_api::PostCreateResponse {
        todo!()
    }

    fn retrieve(&self, request: wp_api::PostRetrieveRequest) -> wp_api::PostRetrieveResponse {
        todo!()
    }

    fn update(&self, request: wp_api::PostUpdateRequest) -> wp_api::PostUpdateResponse {
        todo!()
    }

    fn delete(&self, request: wp_api::PostDeleteRequest) -> wp_api::PostDeleteResponse {
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

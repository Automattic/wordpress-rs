#![allow(dead_code, unused_variables)]
use std::sync::Arc;

use wp_api::{
    ParsedPostCreateResponse, ParsedPostDeleteResponse, ParsedPostListResponse,
    ParsedPostUpdateResponse, PostResponseParser, PostRetrieveResponse,
};

pub fn post_response_parser() -> Arc<dyn PostResponseParser> {
    Arc::new(WPPostResponseParser {})
}

struct WPPostResponseParser {}

impl PostResponseParser for WPPostResponseParser {
    fn list(&self, response: wp_api::PostListResponse) -> ParsedPostListResponse {
        todo!()
    }

    fn create(&self, response: wp_api::PostCreateResponse) -> ParsedPostCreateResponse {
        todo!()
    }

    fn retrieve(&self, response: PostRetrieveResponse) -> wp_api::ParsedPostRetrieveResponse {
        todo!()
    }

    fn update(&self, response: wp_api::PostUpdateResponse) -> ParsedPostUpdateResponse {
        todo!()
    }

    fn delete(&self, response: wp_api::PostDeleteResponse) -> ParsedPostDeleteResponse {
        todo!()
    }
}

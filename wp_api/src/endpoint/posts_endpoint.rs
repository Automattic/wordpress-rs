use url::Url;

use crate::{ApiBaseUrl, PostDeleteParams, PostId, PostListParams, PostUpdateParams, WPContext};

pub struct PostsEndpoint {
    api_base_url: ApiBaseUrl,
}

impl PostsEndpoint {
    pub fn new(api_base_url: ApiBaseUrl) -> Self {
        Self { api_base_url }
    }

    pub fn list(&self, context: WPContext, params: Option<&PostListParams>) -> Url {
        let mut url = self.api_base_url.by_appending("posts");
        url.query_pairs_mut()
            .append_pair("context", context.as_str());
        if let Some(params) = params {
            url.query_pairs_mut().extend_pairs(params.query_pairs());
        }
        url
    }

    pub fn retrieve(&self, post_id: PostId, context: WPContext) -> Url {
        let mut url = self
            .api_base_url
            .by_extending(["posts", &post_id.to_string()]);
        url.query_pairs_mut()
            .append_pair("context", context.as_str());
        url
    }

    pub fn create(&self) -> Url {
        self.api_base_url.by_appending("posts")
    }

    pub fn update(&self, post_id: PostId, params: &PostUpdateParams) -> Url {
        self.api_base_url
            .by_extending(["posts", &post_id.to_string()])
    }

    pub fn delete(&self, post_id: PostId, params: &PostDeleteParams) -> Url {
        let mut url = self.api_base_url.by_appending("posts");
        url.query_pairs_mut().extend_pairs(params.query_pairs());
        url
    }
}

use crate::{
    posts::{
        PostId, PostListParams, PostUpdateParams, PostWithEditContext,
        SparsePostFieldWithEditContext, SparsePostFieldWithEmbedContext,
        SparsePostFieldWithViewContext,
    },
    SparseField,
};
use wp_derive_request_builder::WpDerivedRequest;

use super::{DerivedRequest, Namespace};

#[derive(WpDerivedRequest)]
enum PostsRequest {
    #[contextual_get(url = "/posts", params = &PostListParams, output = Vec<crate::posts::SparsePost>, filter_by = crate::posts::SparsePostField)]
    List,
    #[contextual_get(url = "/posts/<post_id>", params = &crate::posts::PostRetrieveParams, output = crate::posts::SparsePost, filter_by = crate::posts::SparsePostField)]
    Retrieve,
    #[post(url = "/posts", params = &crate::posts::PostCreateParams, output = crate::posts::PostWithEditContext)]
    Create,
    #[delete(url = "/posts/<post_id>", output = crate::posts::PostDeleteResponse)]
    Delete,
    #[delete(url = "/posts/<post_id>", output = crate::posts::PostWithEditContext)]
    Trash,
    #[post(url = "/posts/<post_id>", params = &PostUpdateParams, output = PostWithEditContext)]
    Update,
}

impl DerivedRequest for PostsRequest {
    fn additional_query_pairs(&self) -> Vec<(&str, String)> {
        match self {
            PostsRequest::Delete => vec![("force", true.to_string())],
            PostsRequest::Trash => vec![("force", false.to_string())],
            _ => vec![],
        }
    }

    fn namespace() -> Namespace {
        Namespace::WpV2
    }
}

super::macros::default_sparse_field_implementation_from_field_name!(SparsePostFieldWithEditContext);
super::macros::default_sparse_field_implementation_from_field_name!(
    SparsePostFieldWithEmbedContext
);
super::macros::default_sparse_field_implementation_from_field_name!(SparsePostFieldWithViewContext);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::endpoint::{
        tests::{fixture_api_base_url, validate_wp_v2_endpoint},
        ApiBaseUrl,
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    fn create_post(endpoint: PostsRequestEndpoint) {
        validate_wp_v2_endpoint(endpoint.create(), "/posts");
    }

    #[rstest]
    fn delete_post(endpoint: PostsRequestEndpoint) {
        validate_wp_v2_endpoint(endpoint.delete(&PostId(54)), "/posts/54?force=true");
    }

    #[rstest]
    fn trash_post(endpoint: PostsRequestEndpoint) {
        validate_wp_v2_endpoint(endpoint.trash(&PostId(54)), "/posts/54?force=false");
    }

    #[fixture]
    fn endpoint(fixture_api_base_url: Arc<ApiBaseUrl>) -> PostsRequestEndpoint {
        PostsRequestEndpoint::new(fixture_api_base_url)
    }
}

use crate::{
    posts::{
        PostId, PostListParams, SparsePostFieldWithEditContext, SparsePostFieldWithEmbedContext,
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
}

impl DerivedRequest for PostsRequest {
    fn namespace() -> Namespace {
        Namespace::WpV2
    }
}

super::macros::default_sparse_field_implementation_from_field_name!(SparsePostFieldWithEditContext);
super::macros::default_sparse_field_implementation_from_field_name!(
    SparsePostFieldWithEmbedContext
);
super::macros::default_sparse_field_implementation_from_field_name!(SparsePostFieldWithViewContext);

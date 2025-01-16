use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::{
    tags::{
        SparseTagFieldWithEditContext, SparseTagFieldWithEmbedContext,
        SparseTagFieldWithViewContext, TagId, TagListParams,
    },
    SparseField,
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum TagsRequest {
    #[contextual_paged(url = "/tags", params = &TagListParams, output = Vec<crate::tags::SparseTag>, filter_by = crate::tags::SparseTagField)]
    List,
    #[contextual_get(url = "/tags/<tag_id>", output = crate::tags::SparseTag, filter_by = crate::tags::SparseTagField)]
    Retrieve,
}

impl DerivedRequest for TagsRequest {
    fn namespace() -> impl AsNamespace {
        WpNamespace::WpV2
    }
}

super::macros::default_sparse_field_implementation_from_field_name!(SparseTagFieldWithEditContext);
super::macros::default_sparse_field_implementation_from_field_name!(SparseTagFieldWithEmbedContext);
super::macros::default_sparse_field_implementation_from_field_name!(SparseTagFieldWithViewContext);

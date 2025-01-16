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
    #[post(url = "/tags", params = &crate::tags::TagCreateParams, output = crate::tags::TagWithEditContext)]
    Create,
    #[delete(url = "/tags/<tag_id>", output = crate::tags::TagDeleteResponse)]
    Delete,
}

impl DerivedRequest for TagsRequest {
    fn additional_query_pairs(&self) -> Vec<(&str, String)> {
        match self {
            // The server always returns an error when `force=false`, so a separate `Trash` action
            // is not implemented.
            Self::Delete => vec![("force", true.to_string())],
            _ => vec![],
        }
    }

    fn namespace() -> impl AsNamespace {
        WpNamespace::WpV2
    }
}

super::macros::default_sparse_field_implementation_from_field_name!(SparseTagFieldWithEditContext);
super::macros::default_sparse_field_implementation_from_field_name!(SparseTagFieldWithEmbedContext);
super::macros::default_sparse_field_implementation_from_field_name!(SparseTagFieldWithViewContext);

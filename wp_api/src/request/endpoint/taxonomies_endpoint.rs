use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::taxonomies::{
    SparseTaxonomyTypeDetailsFieldWithEditContext, SparseTaxonomyTypeDetailsFieldWithEmbedContext,
    SparseTaxonomyTypeDetailsFieldWithViewContext, TaxonomyListParams, TaxonomyType,
};
use crate::SparseField;
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum TaxonomiesRequest {
    #[contextual_get(url = "/taxonomies", params = &TaxonomyListParams, output = crate::taxonomies::SparseTaxonomyTypesResponse)]
    List,
    #[contextual_get(url = "/taxonomies/<taxonomy_type>", output = crate::taxonomies::SparseTaxonomyTypeDetails, filter_by = crate::taxonomies::SparseTaxonomyTypeDetailsField)]
    Retrieve,
}

impl DerivedRequest for TaxonomiesRequest {
    fn namespace() -> impl AsNamespace {
        WpNamespace::WpV2
    }
}

super::macros::default_sparse_field_implementation_from_field_name!(
    SparseTaxonomyTypeDetailsFieldWithEditContext
);
super::macros::default_sparse_field_implementation_from_field_name!(
    SparseTaxonomyTypeDetailsFieldWithEmbedContext
);
super::macros::default_sparse_field_implementation_from_field_name!(
    SparseTaxonomyTypeDetailsFieldWithViewContext
);

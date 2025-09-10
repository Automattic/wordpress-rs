use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::taxonomies::{TaxonomyListParams, TaxonomyType};
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

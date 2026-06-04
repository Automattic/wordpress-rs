use super::{AsNamespace, DerivedRequest, WpNamespace};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum BlockPatternCategoriesRequest {
    #[contextual_get(url = "/block-patterns/categories", output = Vec<crate::block_pattern_categories::SparseBlockPatternCategory>, filter_by = crate::block_pattern_categories::SparseBlockPatternCategoryField)]
    List,
}

impl DerivedRequest for BlockPatternCategoriesRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpNamespace::WpV2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        block_pattern_categories::SparseBlockPatternCategoryFieldWithViewContext,
        request::endpoint::{
            ApiUrlResolver,
            tests::{fixture_wp_org_site_api_url_resolver, validate_wp_v2_endpoint},
        },
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    fn list_block_pattern_categories(endpoint: BlockPatternCategoriesRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.list_with_edit_context(),
            "/block-patterns/categories?context=edit",
        );
        validate_wp_v2_endpoint(
            endpoint.list_with_embed_context(),
            "/block-patterns/categories?context=embed",
        );
        validate_wp_v2_endpoint(
            endpoint.list_with_view_context(),
            "/block-patterns/categories?context=view",
        );
    }

    #[rstest]
    fn filter_list_block_pattern_categories(endpoint: BlockPatternCategoriesRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_view_context(&[
                SparseBlockPatternCategoryFieldWithViewContext::Name,
                SparseBlockPatternCategoryFieldWithViewContext::Label,
            ]),
            "/block-patterns/categories?context=view&_fields=name%2Clabel",
        );
    }

    #[fixture]
    fn endpoint(
        fixture_wp_org_site_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> BlockPatternCategoriesRequestEndpoint {
        BlockPatternCategoriesRequestEndpoint::new(fixture_wp_org_site_api_url_resolver)
    }
}

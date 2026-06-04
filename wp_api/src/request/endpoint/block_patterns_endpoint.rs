use super::{AsNamespace, DerivedRequest, WpNamespace};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum BlockPatternsRequest {
    #[contextual_get(url = "/block-patterns/patterns", output = Vec<crate::block_patterns::SparseBlockPattern>, filter_by = crate::block_patterns::SparseBlockPatternField)]
    List,
}

impl DerivedRequest for BlockPatternsRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpNamespace::WpV2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        block_patterns::SparseBlockPatternFieldWithViewContext,
        request::endpoint::{
            ApiUrlResolver,
            tests::{fixture_wp_org_site_api_url_resolver, validate_wp_v2_endpoint},
        },
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    fn list_block_patterns(endpoint: BlockPatternsRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.list_with_edit_context(),
            "/block-patterns/patterns?context=edit",
        );
        validate_wp_v2_endpoint(
            endpoint.list_with_embed_context(),
            "/block-patterns/patterns?context=embed",
        );
        validate_wp_v2_endpoint(
            endpoint.list_with_view_context(),
            "/block-patterns/patterns?context=view",
        );
    }

    #[rstest]
    fn filter_list_block_patterns(endpoint: BlockPatternsRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.filter_list_with_view_context(&[
                SparseBlockPatternFieldWithViewContext::Name,
                SparseBlockPatternFieldWithViewContext::Title,
            ]),
            "/block-patterns/patterns?context=view&_fields=name%2Ctitle",
        );
    }

    #[fixture]
    fn endpoint(
        fixture_wp_org_site_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> BlockPatternsRequestEndpoint {
        BlockPatternsRequestEndpoint::new(fixture_wp_org_site_api_url_resolver)
    }
}

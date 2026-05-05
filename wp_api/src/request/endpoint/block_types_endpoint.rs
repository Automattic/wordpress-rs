use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::block_types::{BlockTypeName, BlockTypeNamespace};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum BlockTypesRequest {
    #[contextual_get(url = "/block-types", output = Vec<crate::block_types::SparseBlockType>, filter_by = crate::block_types::SparseBlockTypeField)]
    List,
    #[contextual_get(url = "/block-types/<block_type_namespace>", output = Vec<crate::block_types::SparseBlockType>, filter_by = crate::block_types::SparseBlockTypeField)]
    ListByNamespace,
    #[contextual_get(url = "/block-types/<block_type_namespace>/<block_type_name>", output = crate::block_types::SparseBlockType, filter_by = crate::block_types::SparseBlockTypeField)]
    Retrieve,
}

impl DerivedRequest for BlockTypesRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpNamespace::WpV2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::endpoint::{
        ApiUrlResolver,
        tests::{fixture_wp_org_site_api_url_resolver, validate_wp_v2_endpoint},
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    fn list_block_types(endpoint: BlockTypesRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.list_with_edit_context(),
            "/block-types?context=edit",
        );
        validate_wp_v2_endpoint(
            endpoint.list_with_embed_context(),
            "/block-types?context=embed",
        );
        validate_wp_v2_endpoint(
            endpoint.list_with_view_context(),
            "/block-types?context=view",
        );
    }

    #[rstest]
    fn list_block_types_by_namespace(endpoint: BlockTypesRequestEndpoint) {
        let ns = BlockTypeNamespace("core".to_string());
        validate_wp_v2_endpoint(
            endpoint.list_by_namespace_with_edit_context(&ns),
            "/block-types/core?context=edit",
        );
        validate_wp_v2_endpoint(
            endpoint.list_by_namespace_with_embed_context(&ns),
            "/block-types/core?context=embed",
        );
        validate_wp_v2_endpoint(
            endpoint.list_by_namespace_with_view_context(&ns),
            "/block-types/core?context=view",
        );
    }

    #[rstest]
    fn retrieve_block_type(endpoint: BlockTypesRequestEndpoint) {
        let ns = BlockTypeNamespace("core".to_string());
        let name = BlockTypeName("paragraph".to_string());
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_edit_context(&ns, &name),
            "/block-types/core/paragraph?context=edit",
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_embed_context(&ns, &name),
            "/block-types/core/paragraph?context=embed",
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_view_context(&ns, &name),
            "/block-types/core/paragraph?context=view",
        );
    }

    #[fixture]
    fn endpoint(
        fixture_wp_org_site_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> BlockTypesRequestEndpoint {
        BlockTypesRequestEndpoint::new(fixture_wp_org_site_api_url_resolver)
    }
}

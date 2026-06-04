use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::{block_revisions::BlockRevisionId, blocks::BlockId};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum BlockAutosavesRequest {
    #[contextual_get(url = "/blocks/<block_id>/autosaves", output = Vec<crate::block_revisions::SparseBlockRevision>, filter_by = crate::block_revisions::SparseBlockRevisionField)]
    List,
    #[contextual_get(url = "/blocks/<block_id>/autosaves/<block_revision_id>", output = crate::block_revisions::SparseBlockRevision, filter_by = crate::block_revisions::SparseBlockRevisionField)]
    Retrieve,
    #[post(url = "/blocks/<block_id>/autosaves", params = &crate::blocks::BlockCreateParams, output = crate::block_revisions::BlockRevisionWithEditContext)]
    Create,
}

impl DerivedRequest for BlockAutosavesRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpNamespace::WpV2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        block_revisions::BlockRevisionId,
        blocks::BlockId,
        request::endpoint::{
            ApiUrlResolver,
            tests::{fixture_wp_org_site_api_url_resolver, validate_wp_v2_endpoint},
        },
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    fn list_block_autosaves(endpoint: BlockAutosavesRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.list_with_edit_context(&BlockId(42)),
            "/blocks/42/autosaves?context=edit",
        );
        validate_wp_v2_endpoint(
            endpoint.list_with_embed_context(&BlockId(42)),
            "/blocks/42/autosaves?context=embed",
        );
        validate_wp_v2_endpoint(
            endpoint.list_with_view_context(&BlockId(42)),
            "/blocks/42/autosaves?context=view",
        );
    }

    #[rstest]
    fn retrieve_block_autosave(endpoint: BlockAutosavesRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_edit_context(&BlockId(42), &BlockRevisionId(99)),
            "/blocks/42/autosaves/99?context=edit",
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_embed_context(&BlockId(42), &BlockRevisionId(99)),
            "/blocks/42/autosaves/99?context=embed",
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_view_context(&BlockId(42), &BlockRevisionId(99)),
            "/blocks/42/autosaves/99?context=view",
        );
    }

    #[rstest]
    fn create_block_autosave(endpoint: BlockAutosavesRequestEndpoint) {
        validate_wp_v2_endpoint(endpoint.create(&BlockId(42)), "/blocks/42/autosaves");
    }

    #[fixture]
    fn endpoint(
        fixture_wp_org_site_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> BlockAutosavesRequestEndpoint {
        BlockAutosavesRequestEndpoint::new(fixture_wp_org_site_api_url_resolver)
    }
}

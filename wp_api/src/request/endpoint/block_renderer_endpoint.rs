use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::block_renderer::BlockName;
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum BlockRendererRequest {
    #[post(url = "/block-renderer/<block_name>", params = &crate::block_renderer::BlockRendererPostParams, output = crate::block_renderer::BlockRendererResponse)]
    Render,
}

impl DerivedRequest for BlockRendererRequest {
    // The block-renderer schema only defines `rendered` in the `edit` context,
    // so the allowed `context` enum is `["edit"]`. WordPress defaults `context`
    // to `"view"` which is not in the enum, causing a validation error. We must
    // always send `context=edit` explicitly.
    fn additional_query_pairs(&self) -> Vec<(&str, String)> {
        vec![("context", "edit".to_string())]
    }

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
    #[case(BlockName("core/paragraph".to_string()), "/block-renderer/core/paragraph?context=edit")]
    #[case(BlockName("core/latest-posts".to_string()), "/block-renderer/core/latest-posts?context=edit")]
    #[case(BlockName("my-plugin/custom-block".to_string()), "/block-renderer/my-plugin/custom-block?context=edit")]
    fn render_block(
        endpoint: BlockRendererRequestEndpoint,
        #[case] block_name: BlockName,
        #[case] expected_path: &str,
    ) {
        validate_wp_v2_endpoint(endpoint.render(&block_name), expected_path);
    }

    #[fixture]
    fn endpoint(
        fixture_wp_org_site_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> BlockRendererRequestEndpoint {
        BlockRendererRequestEndpoint::new(fixture_wp_org_site_api_url_resolver)
    }
}

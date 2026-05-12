use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::block_renderer::BlockName;
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum BlockRendererRequest {
    #[post(url = "/block-renderer/<block_name>", params = &crate::block_renderer::BlockRendererPostParams, output = crate::block_renderer::BlockRendererWithEditContext)]
    Render,
}

impl DerivedRequest for BlockRendererRequest {
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
    #[case("core/paragraph".parse().unwrap(), "/block-renderer/core/paragraph")]
    #[case("core/latest-posts".parse().unwrap(), "/block-renderer/core/latest-posts")]
    #[case("my-plugin/custom-block".parse().unwrap(), "/block-renderer/my-plugin/custom-block")]
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

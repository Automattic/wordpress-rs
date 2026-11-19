use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace, WpComSiteId,
        reader_blocks::{
            ReaderBlockSiteResponse, ReaderBlockedSitesParams, ReaderBlockedSitesResponse,
        },
    },
};
use wp_derive_request_builder::WpDerivedRequest;

// Every request here needs an account.
#[derive(WpDerivedRequest)]
enum ReaderBlocksRequest {
    // The sites the user blocked.
    #[get(url = "/me/blocks/sites", params = &ReaderBlockedSitesParams, output = ReaderBlockedSitesResponse)]
    ListSites,
    // Blocks a site.
    #[post(url = "/me/block/sites/<wp_com_site_id>/new", output = ReaderBlockSiteResponse)]
    BlockSite,
    // Unblocks a site.
    #[post(url = "/me/block/sites/<wp_com_site_id>/delete", output = ReaderBlockSiteResponse)]
    UnblockSite,
}

impl DerivedRequest for ReaderBlocksRequest {
    fn namespace(&self) -> impl AsNamespace {
        match self {
            Self::ListSites => WpComNamespace::V2,
            // Blocking has no newer version than `rest/v1.1`.
            Self::BlockSite | Self::UnblockSite => WpComNamespace::RestV1_1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        request::endpoint::ApiUrlResolver,
        wp_com::endpoint::tests::{
            fixture_wp_com_api_url_resolver, validate_wp_com_rest_v1_1_endpoint,
            validate_wp_com_v2_endpoint,
        },
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    fn list_sites(endpoint: ReaderBlocksRequestEndpoint) {
        validate_wp_com_v2_endpoint(
            endpoint.list_sites(&ReaderBlockedSitesParams::default()),
            "/me/blocks/sites?",
        );
        validate_wp_com_v2_endpoint(
            endpoint.list_sites(&ReaderBlockedSitesParams {
                page: Some(2),
                per_page: Some(50),
            }),
            "/me/blocks/sites?page=2&per_page=50",
        );
    }

    #[rstest]
    fn block_and_unblock(endpoint: ReaderBlocksRequestEndpoint) {
        validate_wp_com_rest_v1_1_endpoint(
            endpoint.block_site(&WpComSiteId(123)),
            "/me/block/sites/123/new",
        );
        validate_wp_com_rest_v1_1_endpoint(
            endpoint.unblock_site(&WpComSiteId(123)),
            "/me/block/sites/123/delete",
        );
    }

    #[fixture]
    fn endpoint(
        fixture_wp_com_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> ReaderBlocksRequestEndpoint {
        ReaderBlocksRequestEndpoint::new(fixture_wp_com_api_url_resolver)
    }
}

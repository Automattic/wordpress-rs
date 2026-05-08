use super::{AsNamespace, DerivedRequest, WpNamespace};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum BlockDirectoryRequest {
    #[get(url = "/block-directory/search", params = &crate::block_directory::BlockDirectorySearchParams, output = Vec<crate::block_directory::SparseBlockDirectoryItem>, filter_by = crate::block_directory::SparseBlockDirectoryItemField)]
    Search,
}

impl DerivedRequest for BlockDirectoryRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpNamespace::WpV2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        block_directory::{BlockDirectorySearchParams, SparseBlockDirectoryItemField},
        request::endpoint::{
            ApiUrlResolver,
            tests::{fixture_wp_org_site_api_url_resolver, validate_wp_v2_endpoint},
        },
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    fn search_block_directory(endpoint: BlockDirectoryRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.search(&BlockDirectorySearchParams::new("coblocks".to_string())),
            "/block-directory/search?term=coblocks",
        );
    }

    #[rstest]
    fn search_block_directory_with_pagination(endpoint: BlockDirectoryRequestEndpoint) {
        let params = BlockDirectorySearchParams {
            term: "gallery".to_string(),
            page: Some(2),
            per_page: Some(5),
        };
        validate_wp_v2_endpoint(
            endpoint.search(&params),
            "/block-directory/search?term=gallery&page=2&per_page=5",
        );
    }

    #[rstest]
    fn filter_search_block_directory(endpoint: BlockDirectoryRequestEndpoint) {
        validate_wp_v2_endpoint(
            endpoint.filter_search(
                &BlockDirectorySearchParams::new("coblocks".to_string()),
                &[
                    SparseBlockDirectoryItemField::Name,
                    SparseBlockDirectoryItemField::Title,
                ],
            ),
            "/block-directory/search?term=coblocks&_fields=name%2Ctitle",
        );
    }

    #[fixture]
    fn endpoint(
        fixture_wp_org_site_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> BlockDirectoryRequestEndpoint {
        BlockDirectoryRequestEndpoint::new(fixture_wp_org_site_api_url_resolver)
    }
}

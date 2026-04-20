use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::post_statuses::PostStatusSlug;
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum PostStatusesRequest {
    #[contextual_get(url = "/statuses", output = crate::post_statuses::SparsePostStatusesResponse, filter_by = crate::post_statuses::SparsePostStatusField)]
    List,
    #[contextual_get(url = "/statuses/<post_status_slug>", output = crate::post_statuses::SparsePostStatus, filter_by = crate::post_statuses::SparsePostStatusField)]
    Retrieve,
}

impl DerivedRequest for PostStatusesRequest {
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
    fn list_post_statuses(endpoint: PostStatusesRequestEndpoint) {
        validate_wp_v2_endpoint(endpoint.list_with_edit_context(), "/statuses?context=edit");
        validate_wp_v2_endpoint(
            endpoint.list_with_embed_context(),
            "/statuses?context=embed",
        );
        validate_wp_v2_endpoint(endpoint.list_with_view_context(), "/statuses?context=view");
    }

    #[rstest]
    fn retrieve_post_status(endpoint: PostStatusesRequestEndpoint) {
        let status_slug = "publish".into();
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_edit_context(&status_slug),
            "/statuses/publish?context=edit",
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_embed_context(&status_slug),
            "/statuses/publish?context=embed",
        );
        validate_wp_v2_endpoint(
            endpoint.retrieve_with_view_context(&status_slug),
            "/statuses/publish?context=view",
        );
    }

    #[fixture]
    fn endpoint(
        fixture_wp_org_site_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> PostStatusesRequestEndpoint {
        PostStatusesRequestEndpoint::new(fixture_wp_org_site_api_url_resolver)
    }
}

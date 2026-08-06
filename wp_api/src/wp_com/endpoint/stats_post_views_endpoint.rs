use crate::{
    posts::PostId,
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{WpComNamespace, WpComSiteId, stats_post_views::StatsPostViewsResponse},
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum StatsPostViewsRequest {
    #[get(url = "/sites/<wp_com_site_id>/stats/post/<post_id>", output = StatsPostViewsResponse)]
    GetStatsPostViews,
}

impl DerivedRequest for StatsPostViewsRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpComNamespace::RestV1_1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        request::endpoint::ApiUrlResolver,
        wp_com::endpoint::tests::{
            fixture_wp_com_api_url_resolver, validate_wp_com_rest_v1_1_endpoint,
        },
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    #[case::numeric_id(WpComSiteId(12345), PostId(2729), "/sites/12345/stats/post/2729")]
    #[case::large_ids(
        WpComSiteId(229889220),
        PostId(9007199254740991),
        "/sites/229889220/stats/post/9007199254740991"
    )]
    fn get_stats_post_views(
        endpoint: StatsPostViewsRequestEndpoint,
        #[case] site_id: WpComSiteId,
        #[case] post_id: PostId,
        #[case] expected_path: &str,
    ) {
        validate_wp_com_rest_v1_1_endpoint(
            endpoint.get_stats_post_views(&site_id, &post_id),
            expected_path,
        );
    }

    #[fixture]
    fn endpoint(
        fixture_wp_com_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> StatsPostViewsRequestEndpoint {
        StatsPostViewsRequestEndpoint::new(fixture_wp_com_api_url_resolver)
    }
}

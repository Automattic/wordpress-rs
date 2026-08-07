use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace, WpComSiteId,
        stats_post::{StatsPostResponse, StatsPostTarget},
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum StatsPostRequest {
    #[get(url = "/sites/<wp_com_site_id>/stats/post/<stats_post_target>", output = StatsPostResponse)]
    GetStatsPost,
}

impl DerivedRequest for StatsPostRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpComNamespace::RestV1_1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        posts::PostId,
        request::endpoint::ApiUrlResolver,
        wp_com::endpoint::tests::{
            fixture_wp_com_api_url_resolver, validate_wp_com_rest_v1_1_endpoint,
        },
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    #[case::numeric_id(
        WpComSiteId(12345),
        StatsPostTarget::Post { id: PostId(2729) },
        "/sites/12345/stats/post/2729"
    )]
    #[case::large_ids(
        WpComSiteId(229889220),
        StatsPostTarget::Post { id: PostId(9007199254740991) },
        "/sites/229889220/stats/post/9007199254740991"
    )]
    // The API addresses the site's home page as post 0.
    #[case::home_page(
        WpComSiteId(12345),
        StatsPostTarget::HomePage,
        "/sites/12345/stats/post/0"
    )]
    fn get_stats_post(
        endpoint: StatsPostRequestEndpoint,
        #[case] site_id: WpComSiteId,
        #[case] target: StatsPostTarget,
        #[case] expected_path: &str,
    ) {
        validate_wp_com_rest_v1_1_endpoint(
            endpoint.get_stats_post(&site_id, &target),
            expected_path,
        );
    }

    #[fixture]
    fn endpoint(
        fixture_wp_com_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> StatsPostRequestEndpoint {
        StatsPostRequestEndpoint::new(fixture_wp_com_api_url_resolver)
    }
}

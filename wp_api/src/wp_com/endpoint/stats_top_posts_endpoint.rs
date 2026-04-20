use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace, WpComSiteId,
        stats_top_posts::{StatsTopPostsParams, StatsTopPostsResponse},
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum StatsTopPostsRequest {
    #[get(url = "/sites/<wp_com_site_id>/stats/top-posts", params = &StatsTopPostsParams, output = StatsTopPostsResponse)]
    GetStatsTopPosts,
}

impl DerivedRequest for StatsTopPostsRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpComNamespace::RestV1_1
    }
}

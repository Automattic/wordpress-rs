use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace, WpComSiteId,
        stats_video_plays::{StatsVideoPlaysParams, StatsVideoPlaysResponse},
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum StatsVideoPlaysRequest {
    #[get(url = "/sites/<wp_com_site_id>/stats/video-plays", params = &StatsVideoPlaysParams, output = StatsVideoPlaysResponse)]
    GetStatsVideoPlays,
}

impl DerivedRequest for StatsVideoPlaysRequest {
    fn namespace() -> impl AsNamespace {
        WpComNamespace::RestV1_1
    }
}

use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace, WpComSiteId,
        stats_file_downloads::{StatsFileDownloadsParams, StatsFileDownloadsResponse},
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum StatsFileDownloadsRequest {
    #[get(url = "/sites/<wp_com_site_id>/stats/file-downloads", params = &StatsFileDownloadsParams, output = StatsFileDownloadsResponse)]
    GetStatsFileDownloads,
}

impl DerivedRequest for StatsFileDownloadsRequest {
    fn namespace() -> impl AsNamespace {
        WpComNamespace::RestV1_1
    }
}

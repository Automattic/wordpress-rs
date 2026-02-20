use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace, WpComSiteId,
        stats_devices::{StatsDevicesParams, StatsDevicesResponse},
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum StatsDevicesScreensizeRequest {
    #[get(url = "/sites/<wp_com_site_id>/stats/devices/screensize", params = &StatsDevicesParams, output = StatsDevicesResponse)]
    GetStatsDevicesScreensize,
}

impl DerivedRequest for StatsDevicesScreensizeRequest {
    fn namespace() -> impl AsNamespace {
        WpComNamespace::RestV1_1
    }
}

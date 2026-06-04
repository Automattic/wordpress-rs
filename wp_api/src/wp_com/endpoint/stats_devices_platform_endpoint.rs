use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace, WpComSiteId,
        stats_devices::{StatsDevicesParams, StatsDevicesResponse},
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum StatsDevicesPlatformRequest {
    #[get(url = "/sites/<wp_com_site_id>/stats/devices/platform", params = &StatsDevicesParams, output = StatsDevicesResponse)]
    GetStatsDevicesPlatform,
}

impl DerivedRequest for StatsDevicesPlatformRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpComNamespace::RestV1_1
    }
}

use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace, WpComSiteId,
        stats_devices::{StatsDevicesParams, StatsDevicesResponse},
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum StatsDevicesBrowserRequest {
    #[get(url = "/sites/<wp_com_site_id>/stats/devices/browser", params = &StatsDevicesParams, output = StatsDevicesResponse)]
    GetStatsDevicesBrowser,
}

impl DerivedRequest for StatsDevicesBrowserRequest {
    fn namespace() -> impl AsNamespace {
        WpComNamespace::RestV1_1
    }
}

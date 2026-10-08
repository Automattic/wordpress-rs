use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace, WpComSiteId,
        stats_archives::{StatsArchivesParams, StatsArchivesResponse},
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum StatsArchivesRequest {
    #[get(url = "/sites/<wp_com_site_id>/stats/archives", params = &StatsArchivesParams, output = StatsArchivesResponse)]
    GetStatsArchives,
}

impl DerivedRequest for StatsArchivesRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpComNamespace::RestV1_1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        date::WpDateString,
        request::endpoint::ApiUrlResolver,
        wp_com::{
            endpoint::tests::{
                fixture_wp_com_api_url_resolver, validate_wp_com_rest_v1_1_endpoint,
            },
            stats_archives::StatsArchivesPeriod,
        },
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    fn get_stats_archives(endpoint: StatsArchivesRequestEndpoint) {
        validate_wp_com_rest_v1_1_endpoint(
            endpoint.get_stats_archives(
                &WpComSiteId(12345),
                &StatsArchivesParams {
                    period: Some(StatsArchivesPeriod::Day),
                    date: Some(WpDateString::new("2026-10-08".to_string())),
                    start_date: Some(WpDateString::new("2026-10-01".to_string())),
                    max: Some(10),
                    ..Default::default()
                },
            ),
            "/sites/12345/stats/archives?period=day&date=2026-10-08&start_date=2026-10-01&max=10&summarize=1&skip_archives=1",
        );
    }

    #[rstest]
    fn get_stats_archives_with_default_params(endpoint: StatsArchivesRequestEndpoint) {
        validate_wp_com_rest_v1_1_endpoint(
            endpoint.get_stats_archives(&WpComSiteId(12345), &StatsArchivesParams::default()),
            "/sites/12345/stats/archives?summarize=1&skip_archives=1",
        );
    }

    #[fixture]
    fn endpoint(
        fixture_wp_com_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> StatsArchivesRequestEndpoint {
        StatsArchivesRequestEndpoint::new(fixture_wp_com_api_url_resolver)
    }
}

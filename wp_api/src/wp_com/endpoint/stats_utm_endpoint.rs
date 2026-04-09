use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace, WpComSiteId,
        stats_utm::{StatsUtmKeys, StatsUtmParams, StatsUtmResponse},
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum StatsUtmRequest {
    #[get(url = "/sites/<wp_com_site_id>/stats/utm/<stats_utm_keys>", params = &StatsUtmParams, output = StatsUtmResponse)]
    GetStatsUtm,
}

impl DerivedRequest for StatsUtmRequest {
    fn namespace() -> impl AsNamespace {
        WpComNamespace::RestV1_1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        request::endpoint::ApiUrlResolver,
        wp_com::{
            endpoint::tests::{
                fixture_wp_com_api_url_resolver, validate_wp_com_rest_v1_1_endpoint,
            },
            stats_utm::{StatsUtmKey, StatsUtmParams},
        },
    };
    use rstest::*;
    use std::sync::Arc;

    const SITE_ID: u64 = 12345;

    #[rstest]
    #[case(
        vec![StatsUtmKey::UtmSource],
        "/sites/12345/stats/utm/utm_source?query_top_posts=1"
    )]
    #[case(
        vec![StatsUtmKey::UtmSource, StatsUtmKey::UtmMedium],
        "/sites/12345/stats/utm/utm_source,utm_medium?query_top_posts=1"
    )]
    #[case(
        vec![StatsUtmKey::UtmCampaign, StatsUtmKey::UtmSource, StatsUtmKey::UtmMedium],
        "/sites/12345/stats/utm/utm_source,utm_medium,utm_campaign?query_top_posts=1"
    )]
    #[case(
        vec![],
        "/sites/12345/stats/utm/utm_source,utm_medium,utm_campaign?query_top_posts=1"
    )]
    fn get_stats_utm(
        endpoint: StatsUtmRequestEndpoint,
        #[case] keys: Vec<StatsUtmKey>,
        #[case] expected_path: &str,
    ) {
        let site_id = WpComSiteId(SITE_ID);
        let utm_keys = StatsUtmKeys(keys);
        let params = StatsUtmParams::default();
        validate_wp_com_rest_v1_1_endpoint(
            endpoint.get_stats_utm(&site_id, &utm_keys, &params),
            expected_path,
        );
    }

    #[rstest]
    fn get_stats_utm_with_params(endpoint: StatsUtmRequestEndpoint) {
        let site_id = WpComSiteId(SITE_ID);
        let utm_keys = StatsUtmKeys(vec![StatsUtmKey::UtmSource]);
        let params = StatsUtmParams {
            max: Some(10),
            date: Some("2026-03-24".to_string()),
            days: Some(30),
            start_date: Some("2026-02-22".to_string()),
            query_top_posts: false,
        };
        validate_wp_com_rest_v1_1_endpoint(
            endpoint.get_stats_utm(&site_id, &utm_keys, &params),
            "/sites/12345/stats/utm/utm_source?max=10&date=2026-03-24&days=30&start_date=2026-02-22&query_top_posts=0",
        );
    }

    #[fixture]
    fn endpoint(
        fixture_wp_com_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> StatsUtmRequestEndpoint {
        StatsUtmRequestEndpoint::new(fixture_wp_com_api_url_resolver)
    }
}

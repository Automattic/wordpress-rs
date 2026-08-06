use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace, WpComSiteId,
        site_plans::{SitePlansParams, SitePlansResponse},
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum SitePlansRequest {
    #[get(url = "/sites/<wp_com_site_id>/plans", params = &SitePlansParams, output = SitePlansResponse)]
    List,
}

impl DerivedRequest for SitePlansRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpComNamespace::RestV1_3
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        request::endpoint::ApiUrlResolver,
        wp_com::{
            CouponCode,
            endpoint::tests::{
                fixture_wp_com_api_url_resolver, validate_wp_com_rest_v1_3_endpoint,
            },
            language::WPComLanguage,
        },
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    #[case::numeric_id(WpComSiteId(12345), "/sites/12345/plans?")]
    #[case::large_id(WpComSiteId(229889220), "/sites/229889220/plans?")]
    fn list(
        endpoint: SitePlansRequestEndpoint,
        #[case] site_id: WpComSiteId,
        #[case] expected_path: &str,
    ) {
        validate_wp_com_rest_v1_3_endpoint(
            endpoint.list(&site_id, &SitePlansParams::default()),
            expected_path,
        );
    }

    #[rstest]
    fn list_with_locale(endpoint: SitePlansRequestEndpoint) {
        validate_wp_com_rest_v1_3_endpoint(
            endpoint.list(
                &WpComSiteId(12345),
                &SitePlansParams {
                    locale: Some(WPComLanguage::Spanish),
                    ..Default::default()
                },
            ),
            "/sites/12345/plans?locale=es",
        );
    }

    #[rstest]
    fn list_with_coupon_code(endpoint: SitePlansRequestEndpoint) {
        validate_wp_com_rest_v1_3_endpoint(
            endpoint.list(
                &WpComSiteId(12345),
                &SitePlansParams {
                    coupon_code: Some(CouponCode("SPRING25".to_string())),
                    ..Default::default()
                },
            ),
            "/sites/12345/plans?coupon_code=SPRING25",
        );
    }

    #[rstest]
    fn list_with_locale_and_coupon_code(endpoint: SitePlansRequestEndpoint) {
        validate_wp_com_rest_v1_3_endpoint(
            endpoint.list(
                &WpComSiteId(12345),
                &SitePlansParams {
                    locale: Some(WPComLanguage::Japanese),
                    coupon_code: Some(CouponCode("SPRING25".to_string())),
                },
            ),
            "/sites/12345/plans?locale=ja&coupon_code=SPRING25",
        );
    }

    #[fixture]
    fn endpoint(
        fixture_wp_com_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> SitePlansRequestEndpoint {
        SitePlansRequestEndpoint::new(fixture_wp_com_api_url_resolver)
    }
}

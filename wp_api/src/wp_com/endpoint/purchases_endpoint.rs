use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{WpComNamespace, WpComSiteId, purchases::SitePurchase},
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum PurchasesRequest {
    #[get(url = "/sites/<wp_com_site_id>/purchases", output = Vec<SitePurchase>)]
    SitePurchases,
}

impl DerivedRequest for PurchasesRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpComNamespace::RestV1_2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        request::endpoint::ApiUrlResolver,
        wp_com::endpoint::tests::{
            fixture_wp_com_api_url_resolver, validate_wp_com_rest_v1_2_endpoint,
        },
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    #[case::numeric_id(WpComSiteId(12345), "/sites/12345/purchases")]
    #[case::large_id(WpComSiteId(229889220), "/sites/229889220/purchases")]
    fn site_purchases(
        endpoint: PurchasesRequestEndpoint,
        #[case] site_id: WpComSiteId,
        #[case] expected_path: &str,
    ) {
        validate_wp_com_rest_v1_2_endpoint(endpoint.site_purchases(&site_id), expected_path);
    }

    #[fixture]
    fn endpoint(
        fixture_wp_com_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> PurchasesRequestEndpoint {
        PurchasesRequestEndpoint::new(fixture_wp_com_api_url_resolver)
    }
}

use crate::wp_com::domains::SupportedCountries;
use crate::wp_com::me::{DomainContactInformation, WPComUserInfo};
use crate::wp_com::transactions::{RedeemCartParams, TransactionReceipt};
use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::WpComNamespace,
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum MeRequest {
    #[get(url = "/me", output = WPComUserInfo)]
    Get,
    #[get(url = "/me/transactions/supported-countries", output = SupportedCountries)]
    TransactionsSupportedCountries,
    #[get(url = "/me/domain-contact-information", output = DomainContactInformation)]
    DomainContactInformation,
    #[post(url = "/me/transactions", params = &RedeemCartParams, output = TransactionReceipt)]
    RedeemCart,
}

impl DerivedRequest for MeRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpComNamespace::RestV1_1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        request::endpoint::ApiUrlResolver,
        wp_com::endpoint::tests::{
            fixture_wp_com_api_url_resolver, validate_wp_com_rest_v1_1_endpoint,
        },
    };
    use rstest::*;
    use std::sync::Arc;

    #[fixture]
    fn endpoint(fixture_wp_com_api_url_resolver: Arc<dyn ApiUrlResolver>) -> MeRequestEndpoint {
        MeRequestEndpoint::new(fixture_wp_com_api_url_resolver)
    }

    #[rstest]
    fn get(endpoint: MeRequestEndpoint) {
        validate_wp_com_rest_v1_1_endpoint(endpoint.get(), "/me");
    }

    #[rstest]
    fn transactions_supported_countries(endpoint: MeRequestEndpoint) {
        validate_wp_com_rest_v1_1_endpoint(
            endpoint.transactions_supported_countries(),
            "/me/transactions/supported-countries",
        );
    }

    #[rstest]
    fn domain_contact_information(endpoint: MeRequestEndpoint) {
        validate_wp_com_rest_v1_1_endpoint(
            endpoint.domain_contact_information(),
            "/me/domain-contact-information",
        );
    }

    #[rstest]
    fn redeem_cart(endpoint: MeRequestEndpoint) {
        validate_wp_com_rest_v1_1_endpoint(endpoint.redeem_cart(), "/me/transactions");
    }
}

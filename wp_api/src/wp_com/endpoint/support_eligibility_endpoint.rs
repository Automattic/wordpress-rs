use wp_derive_request_builder::WpDerivedRequest;

use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{WpComNamespace, support_eligibility::SupportEligibility},
};

#[derive(WpDerivedRequest)]
enum SupportEligibilityRequest {
    #[get(url = "/mobile-support/eligibility", output = SupportEligibility)]
    GetSupportEligibility,
}

impl DerivedRequest for SupportEligibilityRequest {
    fn namespace() -> impl AsNamespace {
        WpComNamespace::V2
    }
}

use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{WpComNamespace, segments::Segment},
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum SegmentsRequest {
    #[get(url = "/segments", output = Vec<Segment>)]
    List,
}

impl DerivedRequest for SegmentsRequest {
    fn namespace() -> impl AsNamespace {
        WpComNamespace::V2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        request::endpoint::ApiUrlResolver,
        wp_com::endpoint::tests::{fixture_wp_com_api_url_resolver, validate_wp_com_v2_endpoint},
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    fn list(endpoint: SegmentsRequestEndpoint) {
        validate_wp_com_v2_endpoint(endpoint.list(), "/segments");
    }

    #[fixture]
    fn endpoint(
        fixture_wp_com_api_url_resolver: Arc<dyn ApiUrlResolver>,
    ) -> SegmentsRequestEndpoint {
        SegmentsRequestEndpoint::new(fixture_wp_com_api_url_resolver)
    }
}

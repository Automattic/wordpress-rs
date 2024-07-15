use wp_derive_request_builder::WpDerivedRequest;

use crate::wp_site_health_tests::{
    SparseWpSiteHealthTest, SparseWpSiteHealthTestField, WpSiteHealthTest,
};

use super::{DerivedRequest, Namespace};

#[derive(WpDerivedRequest)]
#[Namespace("/wp-site-health/v1")]
#[SparseField(SparseWpSiteHealthTestField)]
enum WpSiteHealthTestsRequest {
    #[get(url = "/tests/background-updates", output = SparseWpSiteHealthTest, filter_by = SparseWpSiteHealthTestField)]
    BackgroundUpdates,
    #[get(url = "/tests/loopback-requests", output = SparseWpSiteHealthTest, filter_by = SparseWpSiteHealthTestField)]
    LoopbackRequests,
    #[get(url = "/tests/https-status", output = SparseWpSiteHealthTest, filter_by = SparseWpSiteHealthTestField)]
    HttpsStatus,
    #[get(url = "/tests/dotorg-communication", output = SparseWpSiteHealthTest, filter_by = SparseWpSiteHealthTestField)]
    DotorgCommunication,
    #[get(url = "/tests/authorization-header", output = SparseWpSiteHealthTest, filter_by = SparseWpSiteHealthTestField)]
    AuthorizationHeader,
}

impl DerivedRequest for WpSiteHealthTestsRequest {
    fn namespace() -> Namespace {
        Namespace::WpSiteHealthV1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::endpoint::{
        tests::{fixture_api_base_url, validate_wp_site_health_endpoint},
        ApiBaseUrl,
    };
    use rstest::*;
    use std::sync::Arc;

    #[rstest]
    #[case(None, "/tests/background-updates")]
    #[case(Some(vec![]), "/tests/background-updates?_fields=")]
    #[case(Some(vec![SparseWpSiteHealthTestField::Actions, SparseWpSiteHealthTestField::Badge]), "/tests/background-updates?_fields=actions%2Cbadge")]
    fn background_updates(
        endpoint: WpSiteHealthTestsRequestEndpoint,
        #[case] sparse_fields: Option<Vec<SparseWpSiteHealthTestField>>,
        #[case] expected_path: &str,
    ) {
        if let Some(sparse_fields) = sparse_fields {
            validate_wp_site_health_endpoint(
                endpoint.filter_background_updates(&sparse_fields),
                expected_path,
            );
        } else {
            validate_wp_site_health_endpoint(endpoint.background_updates(), expected_path);
        }
    }

    #[rstest]
    #[case(None, "/tests/loopback-requests")]
    #[case(Some(vec![SparseWpSiteHealthTestField::Description]), "/tests/loopback-requests?_fields=description")]
    fn loopback_requests(
        endpoint: WpSiteHealthTestsRequestEndpoint,
        #[case] sparse_fields: Option<Vec<SparseWpSiteHealthTestField>>,
        #[case] expected_path: &str,
    ) {
        if let Some(sparse_fields) = sparse_fields {
            validate_wp_site_health_endpoint(
                endpoint.filter_loopback_requests(&sparse_fields),
                expected_path,
            );
        } else {
            validate_wp_site_health_endpoint(endpoint.loopback_requests(), expected_path);
        }
    }

    #[rstest]
    #[case(None, "/tests/https-status")]
    #[case(Some(vec![SparseWpSiteHealthTestField::Label]), "/tests/https-status?_fields=label")]
    fn https_status(
        endpoint: WpSiteHealthTestsRequestEndpoint,
        #[case] sparse_fields: Option<Vec<SparseWpSiteHealthTestField>>,
        #[case] expected_path: &str,
    ) {
        if let Some(sparse_fields) = sparse_fields {
            validate_wp_site_health_endpoint(
                endpoint.filter_https_status(&sparse_fields),
                expected_path,
            );
        } else {
            validate_wp_site_health_endpoint(endpoint.https_status(), expected_path);
        }
    }

    #[rstest]
    #[case(None, "/tests/dotorg-communication")]
    #[case(Some(vec![SparseWpSiteHealthTestField::Status]), "/tests/dotorg-communication?_fields=status")]
    fn dotorg_communication(
        endpoint: WpSiteHealthTestsRequestEndpoint,
        #[case] sparse_fields: Option<Vec<SparseWpSiteHealthTestField>>,
        #[case] expected_path: &str,
    ) {
        if let Some(sparse_fields) = sparse_fields {
            validate_wp_site_health_endpoint(
                endpoint.filter_dotorg_communication(&sparse_fields),
                expected_path,
            );
        } else {
            validate_wp_site_health_endpoint(endpoint.dotorg_communication(), expected_path);
        }
    }

    #[rstest]
    #[case(None, "/tests/authorization-header")]
    #[case(Some(vec![SparseWpSiteHealthTestField::Test]), "/tests/authorization-header?_fields=test")]
    fn authorization_header(
        endpoint: WpSiteHealthTestsRequestEndpoint,
        #[case] sparse_fields: Option<Vec<SparseWpSiteHealthTestField>>,
        #[case] expected_path: &str,
    ) {
        if let Some(sparse_fields) = sparse_fields {
            validate_wp_site_health_endpoint(
                endpoint.filter_authorization_header(&sparse_fields),
                expected_path,
            );
        } else {
            validate_wp_site_health_endpoint(endpoint.authorization_header(), expected_path);
        }
    }

    #[fixture]
    fn endpoint(fixture_api_base_url: Arc<ApiBaseUrl>) -> WpSiteHealthTestsRequestEndpoint {
        WpSiteHealthTestsRequestEndpoint::new(fixture_api_base_url)
    }
}

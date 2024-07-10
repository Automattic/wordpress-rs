use rstest::*;
use rstest_reuse::{self, apply, template};
use serial_test::parallel;
use wp_api::wp_site_health_tests::{SparseWpSiteHealthTest, SparseWpSiteHealthTestField};

use crate::integration_test_common::{api_client, AssertResponse};

pub mod integration_test_common;

#[macro_export]
macro_rules! generate_tests {
    ($ident:ident) => {
        paste::paste! {
            #[tokio::test]
            #[parallel]
            async fn $ident() {
                let t = api_client()
                    .wp_site_health_tests()
                    .$ident()
                    .await
                    .assert_response();
                assert!(!t.test.is_empty());
            }

            #[apply(filter_fields_cases)]
            #[tokio::test]
            #[parallel]
            async fn [<filter_$ident>](#[case] fields: &[SparseWpSiteHealthTestField]) {
                let wp_site_health_test = api_client()
                    .wp_site_health_tests()
                    .[<filter_$ident>](fields)
                    .await
                    .assert_response();
                validate_sparse_wp_site_health_tests_fields(&wp_site_health_test, fields);
            }
        }
    };
}

generate_tests!(background_updates);
generate_tests!(loopback_requests);
generate_tests!(https_status);
generate_tests!(dotorg_communication);
generate_tests!(authorization_header);

fn validate_sparse_wp_site_health_tests_fields(
    wp_site_health_test: &SparseWpSiteHealthTest,
    fields: &[SparseWpSiteHealthTestField],
) {
    let field_included = |field| {
        // If "fields" is empty the server will return all fields
        fields.is_empty() || fields.contains(&field)
    };
    assert_eq!(
        wp_site_health_test.actions.is_some(),
        field_included(SparseWpSiteHealthTestField::Actions)
    );
    assert_eq!(
        wp_site_health_test.badge.is_some(),
        field_included(SparseWpSiteHealthTestField::Badge)
    );
    assert_eq!(
        wp_site_health_test.description.is_some(),
        field_included(SparseWpSiteHealthTestField::Description)
    );
    assert_eq!(
        wp_site_health_test.label.is_some(),
        field_included(SparseWpSiteHealthTestField::Label)
    );
    assert_eq!(
        wp_site_health_test.status.is_some(),
        field_included(SparseWpSiteHealthTestField::Status)
    );
    assert_eq!(
        wp_site_health_test.test.is_some(),
        field_included(SparseWpSiteHealthTestField::Test)
    );
}

#[template]
#[rstest]
#[case(&[])]
#[case(&[SparseWpSiteHealthTestField::Actions])]
#[case(&[SparseWpSiteHealthTestField::Badge])]
#[case(&[SparseWpSiteHealthTestField::Description])]
#[case(&[SparseWpSiteHealthTestField::Label])]
#[case(&[SparseWpSiteHealthTestField::Status])]
#[case(&[SparseWpSiteHealthTestField::Test])]
#[case(&[SparseWpSiteHealthTestField::Actions, SparseWpSiteHealthTestField::Badge])]
#[case(&[SparseWpSiteHealthTestField::Label, SparseWpSiteHealthTestField::Status, SparseWpSiteHealthTestField::Test])]
fn filter_fields_cases(#[case] fields: &[SparseWpSiteHealthTestField]) {}

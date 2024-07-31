use rstest::*;
use serial_test::parallel;
use wp_api::wp_site_health_tests::{
    SparseWpSiteHealthDirectorySizes, SparseWpSiteHealthDirectorySizesField,
    SparseWpSiteHealthTest, SparseWpSiteHealthTestField,
};

use wp_api_integration_tests::{api_client, AssertResponse};

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
generate_tests!(page_cache);

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

#[tokio::test]
#[parallel]
async fn directory_sizes() {
    api_client()
        .wp_site_health_tests()
        .directory_sizes()
        .await
        .assert_response();
}

#[rstest]
#[case(&[])]
#[case(&[SparseWpSiteHealthDirectorySizesField::DatabaseSize])]
#[case(&[SparseWpSiteHealthDirectorySizesField::FontsSize])]
#[case(&[SparseWpSiteHealthDirectorySizesField::PluginsSize])]
#[case(&[SparseWpSiteHealthDirectorySizesField::ThemesSize])]
#[case(&[SparseWpSiteHealthDirectorySizesField::TotalSize])]
#[case(&[SparseWpSiteHealthDirectorySizesField::UploadsSize])]
#[case(&[SparseWpSiteHealthDirectorySizesField::WordpressSize])]
#[case(&[SparseWpSiteHealthDirectorySizesField::Raw])]
#[case(&[SparseWpSiteHealthDirectorySizesField::DatabaseSize, SparseWpSiteHealthDirectorySizesField::WordpressSize])]
#[tokio::test]
#[parallel]
async fn filter_directory_sizes(#[case] fields: &[SparseWpSiteHealthDirectorySizesField]) {
    let directory_sizes = api_client()
        .wp_site_health_tests()
        .filter_directory_sizes(fields)
        .await
        .assert_response();
    validate_sparse_wp_site_health_directory_sizes_fields(&directory_sizes, fields);
}

fn validate_sparse_wp_site_health_directory_sizes_fields(
    wp_site_health_test: &SparseWpSiteHealthDirectorySizes,
    fields: &[SparseWpSiteHealthDirectorySizesField],
) {
    let field_included = |field| {
        // If "fields" is empty the server will return all fields
        fields.is_empty() || fields.contains(&field)
    };
    assert_eq!(
        wp_site_health_test.database_size.is_some(),
        field_included(SparseWpSiteHealthDirectorySizesField::DatabaseSize)
    );

    assert_eq!(
        wp_site_health_test.fonts_size.is_some(),
        field_included(SparseWpSiteHealthDirectorySizesField::FontsSize)
    );
    assert_eq!(
        wp_site_health_test.plugins_size.is_some(),
        field_included(SparseWpSiteHealthDirectorySizesField::PluginsSize)
    );
    assert_eq!(
        wp_site_health_test.themes_size.is_some(),
        field_included(SparseWpSiteHealthDirectorySizesField::ThemesSize)
    );
    assert_eq!(
        wp_site_health_test.total_size.is_some(),
        field_included(SparseWpSiteHealthDirectorySizesField::TotalSize)
    );
    assert_eq!(
        wp_site_health_test.uploads_size.is_some(),
        field_included(SparseWpSiteHealthDirectorySizesField::UploadsSize)
    );
    assert_eq!(
        wp_site_health_test.wordpress_size.is_some(),
        field_included(SparseWpSiteHealthDirectorySizesField::WordpressSize)
    );
    assert_eq!(
        wp_site_health_test.raw.is_some(),
        field_included(SparseWpSiteHealthDirectorySizesField::Raw)
    );
}

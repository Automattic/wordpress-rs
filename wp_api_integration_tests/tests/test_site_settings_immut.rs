use rstest::*;
use rstest_reuse::{self, apply, template};
use serial_test::parallel;
use wp_api::site_settings::{
    SparseSiteSettingsFieldWithEditContext, SparseSiteSettingsFieldWithEmbedContext,
    SparseSiteSettingsFieldWithViewContext,
};
use wp_api_integration_tests::{api_client, AssertResponse, FIRST_USER_EMAIL};

#[rstest]
#[tokio::test]
#[parallel]
async fn retrieve_site_settings_with_edit_context() {
    let site_settings = api_client()
        .site_settings()
        .retrieve_with_edit_context()
        .await
        .assert_response()
        .data;
    assert_eq!(FIRST_USER_EMAIL, site_settings.email);
}

#[rstest]
#[tokio::test]
#[parallel]
async fn retrieve_site_settings_with_embed_context() {
    let site_settings = api_client()
        .site_settings()
        .retrieve_with_embed_context()
        .await
        .assert_response()
        .data;
    assert_eq!(FIRST_USER_EMAIL, site_settings.email);
}

#[rstest]
#[tokio::test]
#[parallel]
async fn retrieve_site_settings_with_view_context() {
    let site_settings = api_client()
        .site_settings()
        .retrieve_with_view_context()
        .await
        .assert_response()
        .data;
    assert_eq!(FIRST_USER_EMAIL, site_settings.email);
}

mod filter {
    use super::*;

    wp_api::generate_sparse_site_settings_field_with_edit_context_test_cases!();
    wp_api::generate_sparse_site_settings_field_with_embed_context_test_cases!();
    wp_api::generate_sparse_site_settings_field_with_view_context_test_cases!();

    #[apply(sparse_site_settings_field_with_edit_context_test_cases)]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_post_types_with_edit_context(
        #[case] fields: &[SparseSiteSettingsFieldWithEditContext],
    ) {
        let p = api_client()
            .site_settings()
            .filter_retrieve_with_edit_context(fields)
            .await
            .assert_response()
            .data;
        p.assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_site_settings_field_with_embed_context_test_cases)]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_post_types_with_embed_context(
        #[case] fields: &[SparseSiteSettingsFieldWithEmbedContext],
    ) {
        let p = api_client()
            .site_settings()
            .filter_retrieve_with_embed_context(fields)
            .await
            .assert_response()
            .data;
        p.assert_that_instance_fields_nullability_match_provided_fields(fields);
    }

    #[apply(sparse_site_settings_field_with_view_context_test_cases)]
    #[tokio::test]
    #[parallel]
    async fn filter_retrieve_post_types_with_view_context(
        #[case] fields: &[SparseSiteSettingsFieldWithViewContext],
    ) {
        let p = api_client()
            .site_settings()
            .filter_retrieve_with_view_context(fields)
            .await
            .assert_response()
            .data;
        p.assert_that_instance_fields_nullability_match_provided_fields(fields);
    }
}

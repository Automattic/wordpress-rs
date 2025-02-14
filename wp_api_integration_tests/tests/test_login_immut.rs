use rstest::rstest;
use serial_test::parallel;
use std::sync::Arc;
use wp_api::login::login_client::WpLoginClient;
use wp_api_integration_tests::AsyncWpNetworking;

const LOCALHOST_AUTH_URL: &str = "http://localhost/wp-admin/authorize-application.php";
const AUTOMATTIC_WIDGETS_AUTH_URL: &str =
    "https://automatticwidgets.wpcomstaging.com/wp-admin/authorize-application.php";
const OPTIONAL_HTTPS_AUTH_URL: &str =
    "https://optional-https.wpmt.co/wp-admin/authorize-application.php";
const VANILLA_WP_AUTH_URL: &str = "https://vanilla.wpmt.co/wp-admin/authorize-application.php";

#[rstest]
#[case("http://localhost", LOCALHOST_AUTH_URL)]
#[case("http://localhost/wp-admin", LOCALHOST_AUTH_URL)]
#[case("http://localhost/wp-admin.php", LOCALHOST_AUTH_URL)]
#[case("http://localhost/wp-admin/", LOCALHOST_AUTH_URL)]
#[case("http://localhost/wp-json", LOCALHOST_AUTH_URL)]
#[case(
    "https://automatticwidgets.wpcomstaging.com/",
    AUTOMATTIC_WIDGETS_AUTH_URL
)]
#[case(
    "https://automatticwidgets.wpcomstaging.com/wp-admin",
    AUTOMATTIC_WIDGETS_AUTH_URL
)]
#[case(
    "https://automatticwidgets.wpcomstaging.com/wp-admin.php",
    AUTOMATTIC_WIDGETS_AUTH_URL
)]
#[case(
    "https://automatticwidgets.wpcomstaging.com/wp-admin/",
    AUTOMATTIC_WIDGETS_AUTH_URL
)]
#[case(
    "https://automatticwidgets.wpcomstaging.com/wp-json",
    AUTOMATTIC_WIDGETS_AUTH_URL
)]
#[case("automatticwidgets.wpcomstaging.com/ ", AUTOMATTIC_WIDGETS_AUTH_URL)]
#[case("vanilla.wpmt.co", VANILLA_WP_AUTH_URL)]
#[case("http://vanilla.wpmt.co", VANILLA_WP_AUTH_URL)]
#[case("http://optional-https.wpmt.co", OPTIONAL_HTTPS_AUTH_URL)]
#[case("https://optional-https.wpmt.co", OPTIONAL_HTTPS_AUTH_URL)]
#[case(
    "https://わぷー.wpmt.co",
    "https://xn--39j4bws.wpmt.co/wp-admin/authorize-application.php"
)]
#[case(
    "https://jetpack.wpmt.co",
    "https://jetpack.wpmt.co/wp-admin/authorize-application.php"
)]
#[case(
    "http://wordpress-1315525-4803651.cloudwaysapps.com",
    VANILLA_WP_AUTH_URL
)]
#[case(
    "https://aggressive-caching.wpmt.co",
    "https://aggressive-caching.wpmt.co/wp-admin/authorize-application.php"
)] // Returns gzip responses, may not always include Link header
#[tokio::test]
#[parallel]
async fn test_login_flow(#[case] site_url: &str, #[case] expected_auth_url: &str) {
    let client = WpLoginClient::new(Arc::new(AsyncWpNetworking::default()));
    let result = client.api_discovery(site_url.to_string()).await;
    assert!(
        result.is_successful(),
        "Auto discovery failed: {:#?}",
        result
    );

    let successful_attempt = result
        .find_successful()
        .expect("Already verified that auto discovery is successful");
    assert_eq!(
        successful_attempt
            .api_discovery_result
            .clone()
            .expect("Already verified that auto discovery is successful")
            .api_details
            .find_application_passwords_authentication_url(),
        Some(expected_auth_url.to_string())
    );

    let is_wordpress_site = &successful_attempt.is_wordpress_site;
    assert!(
        is_wordpress_site.is_successful(),
        "'{site_url}' is incorrectly marked as non-WordPress site: {:#?}",
        result
    );
    assert!(is_wordpress_site.api_link_header_result.is_ok());

    // We can't do a reasonable assertion of `is_wordpress_site.fetch_wp_json_result` because not
    // all of the test cases will return a parseable JSON from `/wp-json`.
    //
    // assert!(is_wordpress_site.fetch_wp_json_result.is_ok());
    //
    // ---
    //
    // We can't do a reasonable assertion of `is_wordpress_site.parse_html_result` because each
    // test site has a different configuration, so we have a mixed results of
    // `has_wordpress_generator_meta_tag`, `mentions_wp_content` & `mentions_wp_includes` fields.
    //
    // assert_eq!(
    //     is_wordpress_site.parse_html_result,
    //     Ok(IsWordPressSiteParseHtmlResult {
    //         has_wordpress_generator_meta_tag: true,
    //         mentions_wp_content: true,
    //         mentions_wp_includes: true
    //     })
    // );
}

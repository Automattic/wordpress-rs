use rstest::rstest;
use serial_test::serial;
use std::sync::Arc;
use wp_api::login::WpLoginClient;
use wp_api_integration_tests::{AssertResponse, AsyncWpNetworking};

const LOCALHOST_AUTH_URL: &str = "http://localhost/wp-admin/authorize-application.php";
const AUTOMATTIC_WIDGETS_AUTH_URL: &str =
    "https://automatticwidgets.wpcomstaging.com/wp-admin/authorize-application.php";

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
#[tokio::test]
#[serial]
async fn test_login_flow(#[case] site_url: &str, #[case] expected_auth_url: &str) {
    let client = WpLoginClient::new(Arc::new(AsyncWpNetworking::default()));
    let url_discovery = client
        .api_discovery(site_url.to_string())
        .await
        .assert_response();
    assert_eq!(
        url_discovery
            .api_details
            .find_application_passwords_authentication_url(),
        Some(expected_auth_url.to_string())
    );
}

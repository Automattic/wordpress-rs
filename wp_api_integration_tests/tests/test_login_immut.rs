use rstest::rstest;
use serial_test::parallel;
use std::sync::Arc;
use wp_api::login::{login_client::WpLoginClient, url_discovery::AutoDiscoveryAttemptType};
use wp_api_integration_tests::AsyncWpNetworking;

const LOCALHOST_AUTH_URL: &str = "http://localhost/wp-admin/authorize-application.php";
const AUTOMATTIC_WIDGETS_AUTH_URL: &str =
    "https://automatticwidgets.wpcomstaging.com/wp-admin/authorize-application.php";
const VANILLA_WP_SITE_URL: &str = "https://vanilla.wpmt.co/wp-admin/authorize-application.php";

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
#[case("vanilla.wpmt.co", VANILLA_WP_SITE_URL)]
#[case("http://vanilla.wpmt.co", VANILLA_WP_SITE_URL)]
#[case(
    "https://optional-https.wpmt.co",
    "https://optional-https.wpmt.co/wp-admin/authorize-application.php"
)]
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
    "https://vanilla.wpmt.co/wp-admin/authorize-application.php"
)]
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
    assert_eq!(
        result
            .find_successful()
            .expect("Already verified that auto discovery is successful")
            .result
            .clone()
            .expect("Already verified that auto discovery is successful")
            .api_details
            .find_application_passwords_authentication_url(),
        Some(expected_auth_url.to_string())
    );
}

// TODO: Remove ignore and do a relevant assertion
#[rstest]
#[case("http://localhost")]
#[tokio::test]
#[ignore]
#[parallel]
async fn test_is_wordpress_site(#[case] site_url: &str) {
    let client = WpLoginClient::new(Arc::new(AsyncWpNetworking::default()));
    let result = client
        .is_wordpress_site_discovery(site_url.to_string())
        .await;
    let fetch_wp_json_result = &result
        .get_attempt(&AutoDiscoveryAttemptType::UserInput)
        .unwrap()
        .fetch_wp_json_result;
    dbg!(fetch_wp_json_result);
}

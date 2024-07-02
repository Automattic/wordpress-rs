use integration_test_common::{AssertResponse, AsyncWpNetworking};
use rstest::rstest;
use std::sync::Arc;
use wp_api::login::WpLoginClient;

pub mod integration_test_common;

const LOCALHOST_AUTH_URL: &str = "http://localhost/wp-admin/authorize-application.php";
const ORCHESTREMETROPOLITAIN_AUTH_URL: &str =
    "https://orchestremetropolitain.com/wp-admin/authorize-application.php";

#[rstest]
#[case("http://localhost", LOCALHOST_AUTH_URL)]
#[case("http://localhost/wp-json", LOCALHOST_AUTH_URL)]
#[case("http://localhost/wp-admin.php", LOCALHOST_AUTH_URL)]
#[case("http://localhost/wp-admin", LOCALHOST_AUTH_URL)]
#[case("http://localhost/wp-admin/", LOCALHOST_AUTH_URL)]
#[case("orchestremetropolitain.com/wp-json", ORCHESTREMETROPOLITAIN_AUTH_URL)]
#[case("https://orchestremetropolitain.com", ORCHESTREMETROPOLITAIN_AUTH_URL)]
#[case(
    "https://orchestremetropolitain.com/fr/",
    ORCHESTREMETROPOLITAIN_AUTH_URL
)]
#[case(
    "https://orchestremetropolitain.com/wp-json",
    ORCHESTREMETROPOLITAIN_AUTH_URL
)]
#[tokio::test]
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

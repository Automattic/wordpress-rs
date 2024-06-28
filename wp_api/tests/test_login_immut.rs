use integration_test_common::AsyncWpNetworking;
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
    use wp_api::login::UrlDiscoveryResult;

    let client = WpLoginClient::new(Arc::new(AsyncWpNetworking::default()));
    let state = client.api_discovery(site_url.to_string()).await;
    match state {
        UrlDiscoveryResult::Success { api_details, .. } => {
            assert_eq!(
                api_details.find_application_passwords_authentication_url(),
                Some(expected_auth_url.to_string())
            );
        }
        _ => panic!("Url discovery was unsuccessful: {:?}", state),
    }
}

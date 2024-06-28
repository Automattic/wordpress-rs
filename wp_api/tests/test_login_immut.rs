use integration_test_common::AsyncWpNetworking;
use rstest::rstest;
use std::sync::Arc;
use wp_api::login::WpLoginClient;

pub mod integration_test_common;

#[rstest]
#[case("http://localhost")]
#[case("http://localhost/wp-json")]
#[case("http://localhost/wp-admin.php")]
#[case("https://orchestremetropolitain.com/fr/")]
#[case("https://orchestremetropolitain.com/wp-json")]
//#[case("orchestremetropolitain.com/wp-json")]
// TODO: Theses cases should work, but they don't yet
//#[case("localhost")]
//#[case("http://localhost/wp-admin")]
//#[case("https://orchestremetropolitain.com")]
#[tokio::test]
async fn test_login_flow(#[case] site_url: &str) {
    use wp_api::login::UrlDiscoveryResult;

    let client = WpLoginClient::new(Arc::new(AsyncWpNetworking::default()));
    let state = client.api_discovery(site_url).await;
    match state {
        UrlDiscoveryResult::Success { api_details, .. } => {
            println!("Found api details: {:?}", api_details);
        }
        _ => panic!("Url discovery was unsuccessful: {:?}", state),
    }
}

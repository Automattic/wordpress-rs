use rstest::rstest;
use serial_test::parallel;
use std::sync::Arc;
use wp_api::{
    login::login_client::WpLoginClient,
    middleware::{ApiDiscoveryAuthenticationMiddleware, WpApiMiddleware, WpApiMiddlewarePipeline},
    reqwest_request_executor::ReqwestRequestExecutor,
};

const LOCALHOST_AUTH_URL: &str = "http://localhost/wp-admin/authorize-application.php";
const AUTOMATTIC_WIDGETS_AUTH_URL: &str =
    "https://automatticwidgets.wpcomstaging.com/wp-admin/authorize-application.php";
const OPTIONAL_HTTPS_AUTH_URL: &str =
    "https://optional-https.wpmt.co/wp-admin/authorize-application.php";
const VANILLA_WP_AUTH_URL: &str = "https://vanilla.wpmt.co/wp-admin/authorize-application.php";

#[rstest]
#[case("http://localhost", LOCALHOST_AUTH_URL)]
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
    login_flow_helper(site_url, expected_auth_url, vec![]).await;
}

#[rstest]
#[case(
    "https://basic-auth.wpmt.co",
    "https://basic-auth.wpmt.co/wp-admin/authorize-application.php"
)]
#[tokio::test]
#[parallel]
async fn test_login_flow_with_authentication_middleware(
    #[case] site_url: &str,
    #[case] expected_auth_url: &str,
) {
    login_flow_helper(
        site_url,
        expected_auth_url,
        vec![Arc::new(ApiDiscoveryAuthenticationMiddleware::new(
            // These credentials are safe to check into the repo
            "test@example.com".to_string(),
            "str0ngp4ssw0rd!".to_string(),
        ))],
    )
    .await;
}

async fn login_flow_helper(
    site_url: &str,
    expected_auth_url: &str,
    middlewares: Vec<Arc<dyn WpApiMiddleware>>,
) {
    let client = WpLoginClient::new(
        Arc::new(ReqwestRequestExecutor::new_with_default_timeout(true)),
        Arc::new(WpApiMiddlewarePipeline { middlewares }),
    );

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
}

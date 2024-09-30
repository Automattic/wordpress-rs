use wp_api::{WpApiClient, WpAuthentication, WpLoginCredentials};

use wp_api_integration_tests::*;

use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_cookie_authentication() {
    let login = WpLoginCredentials {
        username: "test@example.com".to_string(),
        password: "strongpassword".to_string(),
    };
    let request_executor = std::sync::Arc::new(AsyncWpNetworking::with_cookie_store());
    let client = WpApiClient::new(
        test_site_url(),
        WpAuthentication::UserAccount { login },
        request_executor,
    );
    let user = client
        .users()
        .retrieve_me_with_edit_context()
        .await
        .assert_response();
    assert_eq!(user.id, FIRST_USER_ID);
}

#[tokio::test]
#[serial]
async fn test_fail_with_incorrect_password() {
    let login = WpLoginCredentials {
        username: "test@example.com".to_string(),
        password: "incorrect".to_string(),
    };
    let request_executor = std::sync::Arc::new(AsyncWpNetworking::with_cookie_store());
    let client = WpApiClient::new(
        test_site_url(),
        WpAuthentication::UserAccount { login },
        request_executor,
    );
    client
        .users()
        .retrieve_me_with_edit_context()
        .await
        .assert_wp_error(wp_api::WpErrorCode::Unauthorized);
}

#[tokio::test]
#[serial]
async fn test_fail_without_cookie_store() {
    let login = WpLoginCredentials {
        username: "test@example.com".to_string(),
        password: "strongpassword".to_string(),
    };
    let request_executor = std::sync::Arc::new(AsyncWpNetworking::default());
    let client = WpApiClient::new(
        test_site_url(),
        WpAuthentication::UserAccount { login },
        request_executor,
    );
    client
        .users()
        .retrieve_me_with_edit_context()
        .await
        .assert_wp_error(wp_api::WpErrorCode::Unauthorized);
}

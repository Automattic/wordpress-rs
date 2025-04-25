use serial_test::parallel;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use wp_api::{
    WpApiClient, WpErrorCode,
    auth::{
        ModifiableAuthenticationProvider, WpAuthentication, WpAuthenticationProvider,
        WpDynamicAuthenticationProvider,
    },
};
use wp_api_integration_tests::{
    AssertResponse, AssertWpError, FIRST_USER_ID, TestCredentials, api_client_with_auth_provider,
};

#[tokio::test]
#[parallel]
async fn test_static_auth_provider() {
    let auth_provider = WpAuthenticationProvider::static_with_username_and_password(
        TestCredentials::instance().admin_username.to_string(),
        TestCredentials::instance().admin_password.to_string(),
    );
    let client: WpApiClient = api_client_with_auth_provider(Arc::new(auth_provider));
    let user = client
        .users()
        .retrieve_me_with_edit_context()
        .await
        .assert_response()
        .data;
    // FIRST_USER_ID is the current user's id
    assert_eq!(FIRST_USER_ID, user.id);
}

#[tokio::test]
#[parallel]
async fn test_dynamic_auth_provider() {
    #[derive(Default)]
    struct DynamicAuthProvider {
        is_authorized: AtomicBool,
    }

    impl WpDynamicAuthenticationProvider for DynamicAuthProvider {
        fn auth(&self) -> WpAuthentication {
            if self.is_authorized.load(Ordering::Relaxed) {
                WpAuthentication::from_username_and_password(
                    TestCredentials::instance().admin_username.to_string(),
                    TestCredentials::instance().admin_password.to_string(),
                )
            } else {
                WpAuthentication::None
            }
        }
    }
    let dynamic_auth_provider = Arc::new(DynamicAuthProvider::default());
    let client: WpApiClient = api_client_with_auth_provider(Arc::new(
        WpAuthenticationProvider::dynamic(dynamic_auth_provider.clone()),
    ));

    // Assert that initial unauthorized request fails
    client
        .users()
        .retrieve_me_with_edit_context()
        .await
        .assert_wp_error(WpErrorCode::Unauthorized);

    // Assert that request succeeds after setting `is_authorized = true`
    dynamic_auth_provider
        .is_authorized
        .store(true, Ordering::Relaxed);
    let user = client
        .users()
        .retrieve_me_with_edit_context()
        .await
        .assert_response()
        .data;
    // FIRST_USER_ID is the current user's id
    assert_eq!(FIRST_USER_ID, user.id);
}

#[tokio::test]
#[parallel]
async fn test_modifiable_auth_provider() {
    let modifiable_auth_provider = Arc::new(ModifiableAuthenticationProvider::new(
        WpAuthentication::None,
    ));
    let auth_provider = WpAuthenticationProvider::modifiable(modifiable_auth_provider.clone());
    let client: WpApiClient = api_client_with_auth_provider(Arc::new(auth_provider));

    // Assert that request fails without authentication
    client
        .users()
        .retrieve_me_with_edit_context()
        .await
        .assert_wp_error(WpErrorCode::Unauthorized);

    // Assert that request succeeds after authentication is modified
    modifiable_auth_provider.set_authentication(WpAuthentication::from_username_and_password(
        TestCredentials::instance().admin_username.to_string(),
        TestCredentials::instance().admin_password.to_string(),
    ));
    let user = client
        .users()
        .retrieve_me_with_edit_context()
        .await
        .assert_response()
        .data;
    // FIRST_USER_ID is the current user's id
    assert_eq!(FIRST_USER_ID, user.id);
}

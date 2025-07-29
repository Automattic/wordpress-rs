use std::sync::RwLock;
use wp_api::auth::{ModifiableAuthenticationProvider, WpDynamicAuthenticationProvider};
use wp_api_integration_tests::prelude::*;

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

#[derive(Debug)]
struct DynamicAuthProvider {
    auth: RwLock<WpAuthentication>,
    refreshed_auth: RwLock<Option<WpAuthentication>>,
}

impl DynamicAuthProvider {
    fn new() -> Self {
        Self {
            auth: RwLock::new(WpAuthentication::None),
            refreshed_auth: RwLock::new(None),
        }
    }

    fn authenticate_with(&self, auth: WpAuthentication) {
        *(self.auth.write().unwrap()) = auth;
    }

    fn allow_refresh_auth(&self, auth: WpAuthentication) {
        *(self.refreshed_auth.write().unwrap()) = Some(auth);
    }
}

#[async_trait::async_trait]
impl WpDynamicAuthenticationProvider for DynamicAuthProvider {
    fn auth(&self) -> WpAuthentication {
        self.auth.read().unwrap().clone()
    }

    async fn refresh(&self) -> bool {
        if let Some(ref auth) = *(self.refreshed_auth.read().unwrap()) {
            *(self.auth.write().unwrap()) = auth.clone();
            return true;
        } else {
            return false;
        }
    }
}

#[tokio::test]
#[parallel]
async fn test_dynamic_auth_provider() {
    let dynamic_auth_provider = Arc::new(DynamicAuthProvider::new());
    let client: WpApiClient = api_client_with_auth_provider(Arc::new(
        WpAuthenticationProvider::dynamic(dynamic_auth_provider.clone()),
    ));

    // Assert that initial unauthorized request fails
    client
        .users()
        .retrieve_me_with_edit_context()
        .await
        .assert_wp_error(WpErrorCode::Unauthorized);

    // Assert that request succeeds after providing a valid authentication
    dynamic_auth_provider.authenticate_with(WpAuthentication::from_username_and_password(
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

#[tokio::test]
#[parallel]
async fn test_refresh_dynamic_auth_provider() {
    let dynamic_auth_provider = Arc::new(DynamicAuthProvider::new());
    let client: WpApiClient = api_client_with_auth_provider(Arc::new(
        WpAuthenticationProvider::dynamic(dynamic_auth_provider.clone()),
    ));

    // Assert that initial unauthorized request fails
    client
        .users()
        .retrieve_me_with_edit_context()
        .await
        .assert_wp_error(WpErrorCode::Unauthorized);

    // Set the refreshed authentication
    dynamic_auth_provider.allow_refresh_auth(WpAuthentication::from_username_and_password(
        TestCredentials::instance().admin_username.to_string(),
        TestCredentials::instance().admin_password.to_string(),
    ));

    // Assert that request succeeds after refresh
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

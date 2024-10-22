use futures::lock::Mutex;
use std::sync::Arc;
use wp_api::RequestExecutionError;
use wp_api::{
    request::{RequestExecutor, WpNetworkRequest, WpNetworkResponse},
    WpApiClient, WpAuthentication, WpLoginCredentials,
};
use wp_api_integration_tests::*;

use serial_test::serial;

#[derive(Debug)]
struct TrackRequestExecutor<T: RequestExecutor> {
    requests: Mutex<Vec<Arc<WpNetworkRequest>>>,
    inner: T,
}

impl<T: RequestExecutor> TrackRequestExecutor<T> {
    fn new(request_executor: T) -> Self {
        Self {
            requests: vec![].into(),
            inner: request_executor,
        }
    }

    async fn wp_login_requests_count(&self) -> usize {
        self.requests
            .lock()
            .await
            .iter()
            .filter(|req| req.url().0.contains("/wp-login.php"))
            .count()
    }

    async fn rest_nonce_requests_count(&self) -> usize {
        self.requests
            .lock()
            .await
            .iter()
            .filter(|req| req.url().0.contains("action=rest-nonce"))
            .count()
    }
}

#[async_trait::async_trait]
impl<T: RequestExecutor> RequestExecutor for TrackRequestExecutor<T> {
    async fn execute(
        &self,
        request: Arc<WpNetworkRequest>,
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        self.requests.lock().await.push(request.clone());

        self.inner.execute(request).await
    }
}

#[tokio::test]
#[serial]
async fn test_cookie_authentication() {
    let login = WpLoginCredentials {
        username: TestCredentials::instance().admin_username.to_string(),
        password: TestCredentials::instance()
            .admin_account_password
            .to_string(),
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
        username: TestCredentials::instance().admin_username.to_string(),
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
        username: TestCredentials::instance().admin_username.to_string(),
        password: TestCredentials::instance()
            .admin_account_password
            .to_string(),
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

#[tokio::test]
#[serial_test::parallel]
async fn test_nonce_is_reused_across_requests() {
    let login = WpLoginCredentials {
        username: TestCredentials::instance().admin_username.to_string(),
        password: TestCredentials::instance()
            .admin_account_password
            .to_string(),
    };
    let request_executor = TrackRequestExecutor::new(AsyncWpNetworking::with_cookie_store());
    let request_executor = std::sync::Arc::new(request_executor);
    let request_executor_clone = request_executor.clone();
    let client = WpApiClient::new(
        test_site_url(),
        WpAuthentication::UserAccount { login },
        request_executor,
    );

    futures::future::join_all((0..100).map(|_| client.users().retrieve_me_with_edit_context()))
        .await;

    assert_eq!(request_executor_clone.wp_login_requests_count().await, 1);
    assert_eq!(request_executor_clone.rest_nonce_requests_count().await, 1);
}

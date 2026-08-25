//! Regression test for [#1366]: self-hosted sites with *plain* permalinks
//! advertise the REST API root as `…/index.php?rest_route=/` rather than
//! `…/wp-json/`. Before the fix, every endpoint URL was built by path-extending
//! the discovered root and silently collapsed to the API index.
//!
//! This flips the shared integration-test server to plain permalinks for the
//! duration of the test, then restores the original structure as its last step
//! (the same pattern the other `_mut` tests use with `RestoreServer::db()`).
//! It is `#[serial]` because it mutates a *global* server setting.
//!
//! [#1366]: https://github.com/Automattic/wordpress-rs/issues/1366

use std::sync::Arc;
use wp_api::{
    EmptyAppNotifier,
    api_client::{WpApiClient, WpApiClientDelegate},
    auth::{WpAuthentication, WpAuthenticationProvider},
    login::login_client::WpLoginClient,
    middleware::WpApiMiddlewarePipeline,
    request::endpoint::WpOrgSiteApiUrlResolver,
    reqwest_request_executor::ReqwestRequestExecutor,
};
use wp_api_integration_tests::prelude::{AssertResponse, TestCredentials, serial};

#[tokio::test]
#[serial]
async fn login_and_fetch_users_me_on_plain_permalinks_site() {
    let creds = TestCredentials::instance();

    // Capture the server's current (date-based) permalink structure so we can put
    // it back at the end, then switch the site to "Plain".
    let original_permalink_structure = wp_cli::get_permalink_structure();
    wp_cli::set_permalink_structure("");

    let executor = Arc::new(ReqwestRequestExecutor::default());
    let login_client = WpLoginClient::new(
        executor.clone(),
        Arc::new(WpApiMiddlewarePipeline::default()),
    );

    let discovery = login_client
        .api_discovery(creds.site_url.to_string(), None)
        .await;
    let success = discovery
        .combined_result()
        .expect("API discovery should succeed on the plain-permalinks site");

    // Sanity-check that the discovered root really is the rest_route form —
    // otherwise the test isn't exercising the bug it was added to cover.
    let api_root = success.api_root_url.url();
    assert!(
        api_root.contains("rest_route="),
        "expected the discovered API root to be the `?rest_route=…` form, got `{api_root}`",
    );

    let client = WpApiClient::new(
        Arc::new(WpOrgSiteApiUrlResolver::new(success.api_root_url.clone())),
        WpApiClientDelegate {
            auth_provider: Arc::new(WpAuthenticationProvider::static_with_auth(
                WpAuthentication::from_username_and_password(
                    creds.admin_username.to_string(),
                    creds.admin_password.to_string(),
                ),
            )),
            request_executor: executor,
            middleware_pipeline: Arc::new(WpApiMiddlewarePipeline::default()),
            app_notifier: Arc::new(EmptyAppNotifier),
            language_provider: None,
        },
    );

    let user = client
        .users()
        .retrieve_me_with_edit_context()
        .await
        .assert_response()
        .data;
    assert_eq!(user.id.0, 1, "admin user should have id 1");

    // Restore the original permalink structure for subsequent tests.
    wp_cli::set_permalink_structure(&original_permalink_structure);
}

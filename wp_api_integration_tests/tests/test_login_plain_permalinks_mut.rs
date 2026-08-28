//! Regression tests for [#1366]: self-hosted sites with *plain* permalinks
//! advertise the REST API root as `…/index.php?rest_route=/` rather than
//! `…/wp-json/`. Before the fix, every endpoint URL was built by path-extending
//! the discovered root and silently collapsed to the API index.
//!
//! Each test flips the shared integration-test server to plain permalinks for
//! the duration of the test via a `PlainPermalinks` guard that restores the
//! original structure on drop — including on unwind, so a failed assertion can't
//! leave the shared server mis-configured. They are `#[serial]` because they
//! mutate a *global* server setting.
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
    users::UserId,
};
use wp_api_integration_tests::prelude::{AssertResponse, TestCredentials, serial};

/// Flips the shared server to plain (`?rest_route=`) permalinks and restores the
/// original structure on drop — including on unwind, so a failed assertion can't
/// leave the shared server mis-configured for the rest of the run.
struct PlainPermalinks(String);

impl PlainPermalinks {
    fn set() -> Self {
        let original = wp_cli::get_permalink_structure();
        wp_cli::set_permalink_structure("");
        Self(original)
    }
}

impl Drop for PlainPermalinks {
    fn drop(&mut self) {
        wp_cli::set_permalink_structure(&self.0);
    }
}

#[tokio::test]
#[serial]
async fn login_and_fetch_users_me_on_plain_permalinks_site() {
    // Switch the shared server to "Plain" permalinks; `_plain` restores the
    // original structure on drop, even if an assertion below panics.
    let _plain = PlainPermalinks::set();

    let executor = Arc::new(ReqwestRequestExecutor::default());
    let client = discover_and_build_admin_client(executor).await;

    let user = client
        .users()
        .retrieve_me_with_edit_context()
        .await
        .assert_response()
        .data;
    assert_eq!(user.id.0, 1, "admin user should have id 1");
}

/// Companion to the `/users/me` check above, but through a *parameterized*
/// route (`/wp/v2/users/<id>`): fetch a real object addressed by its numeric id
/// on a plain-permalinks site, proving ID-bearing endpoints resolve and
/// round-trip over the `?rest_route=` form. This is the case the REST index
/// can't self-document — it publishes a `self` href only for routes without
/// path parameters — so it's verified against a live server here rather than
/// against the index's self-links.
#[tokio::test]
#[serial]
async fn fetch_object_by_id_on_plain_permalinks_site() {
    let _plain = PlainPermalinks::set();

    let executor = Arc::new(ReqwestRequestExecutor::default());
    let client = discover_and_build_admin_client(executor).await;

    // `/wp/v2/users/1` — the admin, addressed by numeric id (a real object),
    // over the `?rest_route=%2Fwp%2Fv2%2Fusers%2F1` form.
    let user = client
        .users()
        .retrieve_with_edit_context(&UserId(1))
        .await
        .assert_response()
        .data;
    assert_eq!(user.id.0, 1, "retrieving user 1 should return user 1");
}

/// Discovers the (rest_route) API root on the now-plain-permalinks site and
/// builds an admin-authenticated client on it. Asserts the discovered root
/// really is the `?rest_route=…` form, so a test can't pass against the wrong
/// permalink structure. The caller is responsible for flipping the permalink
/// structure to Plain beforehand and restoring it afterwards.
async fn discover_and_build_admin_client(executor: Arc<ReqwestRequestExecutor>) -> WpApiClient {
    let creds = TestCredentials::instance();
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

    WpApiClient::new(
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
        },
    )
}

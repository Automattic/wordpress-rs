//! End-to-end regression test for [#1366]: self-hosted sites with plain
//! permalinks advertise the REST API root as `…/index.php?rest_route=/` rather
//! than `…/wp-json`. Before the fix, every endpoint URL was built by
//! path-extending the discovered root and silently collapsed to the API index.
//!
//! Requires a dedicated WordPress instance (see `make
//! start-plain-permalinks-test-server`); the test self-skips when its
//! credentials file is absent so it doesn't break the default suite.
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
use wp_api_integration_tests::prelude::AssertResponse;

const CREDENTIALS_PATH: &str = "../test_credentials_plain_permalinks.json";

struct PlainPermalinksCredentials {
    site_url: String,
    admin_username: String,
    admin_password: String,
}

impl PlainPermalinksCredentials {
    fn load() -> Option<Self> {
        let raw = std::fs::read_to_string(CREDENTIALS_PATH).ok()?;
        let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
        Some(Self {
            site_url: v.get("site_url")?.as_str()?.to_string(),
            admin_username: v.get("admin_username")?.as_str()?.to_string(),
            admin_password: v.get("admin_password")?.as_str()?.to_string(),
        })
    }
}

#[tokio::test]
async fn login_and_fetch_users_me_on_plain_permalinks_site() {
    let Some(creds) = PlainPermalinksCredentials::load() else {
        eprintln!(
            "Skipping: {CREDENTIALS_PATH} not found. Run \
             `make start-plain-permalinks-test-server` to bring up the \
             dedicated WordPress instance for this test."
        );
        return;
    };

    let executor = Arc::new(ReqwestRequestExecutor::default());
    let login_client = WpLoginClient::new(
        executor.clone(),
        Arc::new(WpApiMiddlewarePipeline::default()),
    );

    let discovery = login_client
        .api_discovery(creds.site_url.clone(), None)
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
                    creds.admin_username,
                    creds.admin_password,
                ),
            )),
            request_executor: executor,
            middleware_pipeline: Arc::new(WpApiMiddlewarePipeline::default()),
            app_notifier: Arc::new(EmptyAppNotifier),
        },
    );

    let user = client
        .users()
        .retrieve_me_with_edit_context()
        .await
        .assert_response()
        .data;
    assert_eq!(user.id.0, 1, "admin user should have id 1");
}

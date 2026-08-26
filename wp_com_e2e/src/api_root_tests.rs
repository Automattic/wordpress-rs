use libtest_mimic::{Failed, Trial};
use std::sync::Arc;
use std::time::Duration;
use wp_api::{
    EmptyAppNotifier,
    prelude::*,
    reqwest_request_executor::ReqwestRequestExecutor,
    wp_com::{WpComBaseUrl, endpoint::WpComDotOrgApiUrlResolver},
};

use crate::context::TestContext;

const SITE_SLUG: &str = "mobile.blog";
const EXPECTED_SITE_URL: &str = "http://mobiledotblog.wordpress.com";

pub fn tests(ctx: Arc<TestContext>) -> Vec<Trial> {
    let resolver = Arc::new(WpComDotOrgApiUrlResolver::new(
        SITE_SLUG.to_string(),
        WpComBaseUrl::Production,
    ));

    let client = WpApiClient::new(
        resolver.clone(),
        WpApiClientDelegate {
            auth_provider: Arc::new(WpAuthenticationProvider::static_with_auth(
                WpAuthentication::None,
            )),
            request_executor: Arc::new(ReqwestRequestExecutor::new(false, Duration::from_secs(60))),
            middleware_pipeline: Arc::new(WpApiMiddlewarePipeline::default()),
            app_notifier: Arc::new(EmptyAppNotifier),
            language_provider: None,
        },
    );

    // Fetch once during test collection so each trial reports an individual
    // result rather than re-fetching the API root four times.
    let details = ctx.runtime.block_on(async {
        client
            .api_root()
            .get()
            .await
            .map_err(|e| e.to_string())
            .map(|r| r.data)
    });

    let mut trials = vec![];

    trials.push(Trial::test("api_root::fetch_wpcom", {
        let details = details.clone();
        move || {
            let details = details.map_err(Failed::from)?;
            if details.url != EXPECTED_SITE_URL {
                return Err(
                    format!("expected url `{EXPECTED_SITE_URL}`, got `{}`", details.url).into(),
                );
            }
            Ok(())
        }
    }));

    let route_cases: &[(&str, &str, &str, bool)] = &[
        ("wp_v2_posts", "/wp/v2", "posts", true),
        ("wp_v2_media", "/wp/v2", "media", true),
        (
            "block_editor_settings",
            "/wp-block-editor/v1",
            "settings",
            true,
        ),
        ("wp_v2_fake_endpoint", "/wp/v2", "fake-endpoint", false),
    ];

    for (name, namespace, endpoint, expected) in route_cases {
        let resolver = resolver.clone();
        let details = details.clone();
        let namespace = namespace.to_string();
        let endpoint = endpoint.to_string();
        let expected = *expected;
        trials.push(Trial::test(
            format!("api_root::has_route_for_endpoint::{name}"),
            move || {
                let details = details.map_err(Failed::from)?;
                let actual = details.has_route_for_endpoint(
                    resolver.as_ref(),
                    namespace.clone(),
                    endpoint.clone(),
                );
                if actual != expected {
                    return Err(format!(
                        "has_route_for_endpoint(`{namespace}`, `{endpoint}`) = {actual}, expected {expected}"
                    )
                    .into());
                }
                Ok(())
            },
        ));
    }

    trials
}

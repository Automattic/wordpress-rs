//! Replays responses recorded from Cloudflare (`test-data/cloudflare-responses`)
//! through a local socket, so they reach the real `reqwest` executor and API
//! client the way a live Cloudflare response would.
//!
//! These tests don't need the WordPress test server.

use std::{path::PathBuf, sync::Mutex};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use wp_api::{
    cloudflare::CloudflareErrorKind,
    media::MediaCreateParams,
    middleware::{RetryAfterMiddleware, WpApiMiddleware},
    parsed_url::ParsedUrl,
};
use wp_api_integration_tests::prelude::*;

/// Serves one recorded response to every request, and keeps each request's
/// head so tests can check what the client sent.
struct ReplayServer {
    url: ParsedUrl,
    request_heads: Arc<Mutex<Vec<String>>>,
}

impl ReplayServer {
    async fn start(file_name: &str) -> Self {
        let (head, body) = recorded_response(file_name);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = ParsedUrl::parse(&format!(
            "http://{}/wp-json",
            listener.local_addr().unwrap()
        ))
        .unwrap();
        let request_heads = Arc::new(Mutex::new(Vec::new()));
        let heads = request_heads.clone();
        tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    return;
                };
                let request_head = respond(stream, &head, &body).await;
                heads.lock().unwrap().push(request_head);
            }
        });
        Self { url, request_heads }
    }

    fn api_client(&self, middlewares: Vec<Arc<dyn WpApiMiddleware>>) -> WpApiClient {
        WpApiClient::new(
            Arc::new(WpOrgSiteApiUrlResolver::new(Arc::new(self.url.clone()))),
            WpApiClientDelegate {
                auth_provider: Arc::new(
                    WpAuthenticationProvider::static_with_username_and_password(
                        "user".to_string(),
                        "password".to_string(),
                    ),
                ),
                request_executor: Arc::new(ReqwestRequestExecutor::default()),
                middleware_pipeline: Arc::new(WpApiMiddlewarePipeline { middlewares }),
                app_notifier: Arc::new(EmptyAppNotifier),
                language_provider: None,
            },
        )
    }

    fn request_heads(&self) -> Vec<String> {
        self.request_heads.lock().unwrap().clone()
    }
}

/// Splits a recorded response into its head (status line and headers, without
/// the blank line) and its body.
fn recorded_response(file_name: &str) -> (String, Vec<u8>) {
    let path: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "..",
        "test-data",
        "cloudflare-responses",
        file_name,
    ]
    .iter()
    .collect();
    let raw = std::fs::read(&path).unwrap();
    let separator = raw.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
    (
        String::from_utf8(raw[..separator].to_vec()).unwrap(),
        raw[separator + 4..].to_vec(),
    )
}

/// Reads the whole request before answering, as Cloudflare's edge does, then
/// sends the recorded response and closes the connection.
async fn respond(mut stream: TcpStream, head: &str, body: &[u8]) -> String {
    let mut request = Vec::new();
    let mut buffer = [0u8; 64 * 1024];
    let head_end = loop {
        let n = stream.read(&mut buffer).await.unwrap();
        assert!(n > 0, "The client closed the connection mid-request");
        request.extend_from_slice(&buffer[..n]);
        if let Some(i) = request.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
    };
    let request_head = String::from_utf8_lossy(&request[..head_end]).to_lowercase();
    let is_body_complete = |request: &[u8]| {
        let body = &request[head_end..];
        if let Some(length) = request_head
            .lines()
            .find_map(|l| l.strip_prefix("content-length:"))
        {
            body.len() >= length.trim().parse::<usize>().unwrap()
        } else if request_head.contains("transfer-encoding: chunked") {
            body.ends_with(b"0\r\n\r\n")
        } else {
            true
        }
    };
    while !is_body_complete(&request) {
        let n = stream.read(&mut buffer).await.unwrap();
        assert!(n > 0, "The client closed the connection mid-body");
        request.extend_from_slice(&buffer[..n]);
    }

    let mut response = format!(
        "{head}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    response.extend_from_slice(body);
    stream.write_all(&response).await.unwrap();
    stream.shutdown().await.unwrap();
    request_head
}

fn cloudflare_error(error: WpApiError) -> (Option<u32>, CloudflareErrorKind, Option<String>) {
    match error {
        WpApiError::RequestExecutionFailed {
            reason:
                RequestExecutionErrorReason::CloudflareError {
                    cloudflare_code,
                    kind,
                    ray_id,
                    ..
                },
            ..
        } => (cloudflare_code, kind, ray_id),
        other => panic!("Expected a Cloudflare error, got {other:#?}"),
    }
}

/// A JSON API request that Cloudflare answers with its own error.
#[rstest]
#[case::json_origin_timeout("cf-522-json.http", Some(522), CloudflareErrorKind::OriginTimeout)]
#[case::json_origin_ssl("cf-526-json.http", Some(526), CloudflareErrorKind::OriginSsl)]
#[case::json_origin_dns(
    "cf-1016-json.http",
    Some(1016),
    CloudflareErrorKind::OriginUnreachable
)]
#[case::json_firewall_rule("cf-1020-json.http", Some(1020), CloudflareErrorKind::AccessDenied)]
#[case::text_origin_timeout(
    "cf-522-plain-text.http",
    Some(522),
    CloudflareErrorKind::OriginTimeout
)]
#[case::text_firewall_rule(
    "cf-1020-plain-text.http",
    Some(1020),
    CloudflareErrorKind::AccessDenied
)]
#[case::text_direct_ip("cf-1003-plain-text.http", Some(1003), CloudflareErrorKind::Other)]
#[case::html_bad_gateway("cf-502-html.http", None, CloudflareErrorKind::OriginError)]
#[case::waf_block_page("cf-waf-block-page.http", None, CloudflareErrorKind::AccessDenied)]
#[case::challenge("cf-challenge.http", None, CloudflareErrorKind::Challenge)]
#[tokio::test]
async fn json_request_answered_by_cloudflare(
    #[case] file_name: &str,
    #[case] code: Option<u32>,
    #[case] kind: CloudflareErrorKind,
) {
    let server = ReplayServer::start(file_name).await;
    let error = server
        .api_client(vec![])
        .users()
        .retrieve_me_with_edit_context()
        .await
        .unwrap_err();

    let (actual_code, actual_kind, ray_id) = cloudflare_error(error);
    assert_eq!(actual_code, code);
    assert_eq!(actual_kind, kind);
    assert!(ray_id.is_some());
}

/// Cloudflare only sends its JSON error body when the request carries an
/// `Accept-Encoding` header, alongside `Accept: application/json`. A client
/// without it gets the plain-text body.
#[tokio::test]
async fn json_request_asks_for_cloudflares_json_error_body() {
    let server = ReplayServer::start("cf-522-json.http").await;
    let _ = server
        .api_client(vec![])
        .users()
        .retrieve_me_with_edit_context()
        .await;

    let heads = server.request_heads();
    assert_eq!(heads.len(), 1);
    assert!(
        heads[0].contains("\r\naccept: application/json"),
        "{}",
        heads[0]
    );
    assert!(heads[0].contains("\r\naccept-encoding: "), "{}", heads[0]);
}

/// The CMM-2500 case: an upload over the zone's body size limit.
#[tokio::test]
async fn upload_rejected_by_cloudflare_as_too_large() {
    let server = ReplayServer::start("cf-413-html.http").await;
    let error = server
        .api_client(vec![])
        .media()
        .create(&MediaCreateParams {
            file_path: MEDIA_TEST_FILE_PATH.to_string(),
            ..Default::default()
        })
        .await
        .unwrap_err();

    let (code, kind, ray_id) = cloudflare_error(error);
    assert_eq!(code, None);
    assert_eq!(kind, CloudflareErrorKind::RequestTooLarge);
    assert_eq!(ray_id.as_deref(), Some("a51300505b32e411"));
}

/// A Cloudflare 1015 with `Retry-After` is still retried by
/// `RetryAfterMiddleware` and ends the way it always has.
#[tokio::test]
async fn cloudflare_rate_limit_still_goes_through_retry_after_middleware() {
    let server = ReplayServer::start("cf-1015-json.http").await;
    let error = server
        .api_client(vec![Arc::new(RetryAfterMiddleware::new(1, 0))])
        .users()
        .retrieve_me_with_edit_context()
        .await
        .unwrap_err();

    assert!(
        matches!(
            error,
            WpApiError::RequestExecutionFailed {
                reason: RequestExecutionErrorReason::MisconfiguredRateLimitError,
                ..
            }
        ),
        "{error:#?}"
    );
    assert_eq!(server.request_heads().len(), 2);
}

/// Without the middleware, the same response reaches the caller as Cloudflare's.
#[tokio::test]
async fn cloudflare_rate_limit_without_retry_middleware() {
    let server = ReplayServer::start("cf-1015-json.http").await;
    let error = server
        .api_client(vec![])
        .users()
        .retrieve_me_with_edit_context()
        .await
        .unwrap_err();

    match error {
        WpApiError::RequestExecutionFailed {
            status_code,
            reason:
                RequestExecutionErrorReason::CloudflareError {
                    kind, retry_after, ..
                },
            ..
        } => {
            assert_eq!(status_code, Some(429));
            assert_eq!(kind, CloudflareErrorKind::RateLimited);
            assert_eq!(retry_after, Some(10));
        }
        other => panic!("Expected a Cloudflare error, got {other:#?}"),
    }
}

/// The site's own WordPress error, which reached the app through Cloudflare.
#[tokio::test]
async fn wordpress_error_behind_cloudflare_is_unchanged() {
    let server = ReplayServer::start("origin-wp-error-behind-cloudflare.http").await;
    let error = server
        .api_client(vec![])
        .users()
        .retrieve_me_with_edit_context()
        .await
        .unwrap_err();

    assert!(
        matches!(
            error,
            WpApiError::WpError {
                error_code: WpErrorCode::Unauthorized,
                status_code: 401,
                ..
            }
        ),
        "{error:#?}"
    );
}

/// The client sends credentials, so today a 403 without `WWW-Authenticate` reads as the
/// credentials being rejected. That's unchanged.
#[rstest]
#[case::nginx_403("origin-nginx-403-behind-cloudflare.http")]
#[case::plain_text_403("origin-plain-forbidden-behind-cloudflare.http")]
#[tokio::test]
async fn forbidden_from_the_site_behind_cloudflare_is_unchanged(#[case] file_name: &str) {
    let server = ReplayServer::start(file_name).await;
    let error = server
        .api_client(vec![])
        .users()
        .retrieve_me_with_edit_context()
        .await
        .unwrap_err();

    assert!(
        matches!(
            error,
            WpApiError::RequestExecutionFailed {
                reason: RequestExecutionErrorReason::HttpAuthenticationRejectedError {
                    method: None,
                    ..
                },
                ..
            }
        ),
        "{error:#?}"
    );
}

#[tokio::test]
async fn http_authentication_behind_cloudflare_is_unchanged() {
    let server = ReplayServer::start("origin-basic-auth-401-behind-cloudflare.http").await;
    let error = server
        .api_client(vec![])
        .users()
        .retrieve_me_with_edit_context()
        .await
        .unwrap_err();

    assert!(
        matches!(
            error,
            WpApiError::RequestExecutionFailed {
                reason: RequestExecutionErrorReason::HttpAuthenticationRejectedError {
                    method: Some(_),
                    ..
                },
                ..
            }
        ),
        "{error:#?}"
    );
}

#[rstest]
#[case::origin_502_behind_cloudflare("origin-apache-502-behind-cloudflare.http", 502)]
#[case::nginx_413_without_cloudflare("origin-nginx-413.http", 413)]
#[tokio::test]
async fn other_errors_from_the_site_are_unchanged(#[case] file_name: &str, #[case] status: u32) {
    let server = ReplayServer::start(file_name).await;
    let error = server
        .api_client(vec![])
        .users()
        .retrieve_me_with_edit_context()
        .await
        .unwrap_err();

    assert!(
        matches!(error, WpApiError::UnknownError { status_code, .. } if status_code == status),
        "{error:#?}"
    );
}

/// Sends every request to one URL, so a live Cloudflare error page can stand in
/// for an API endpoint.
struct FixedUrlResolver(Arc<ParsedUrl>);

impl ApiUrlResolver for FixedUrlResolver {
    fn resolve(&self, _: String, _: Vec<String>) -> Arc<ParsedUrl> {
        self.0.clone()
    }

    fn route_path(&self, namespace: String, endpoint_path: String) -> String {
        format!("{namespace}/{endpoint_path}")
    }
}

fn live_api_client(url: &str) -> WpApiClient {
    WpApiClient::new(
        Arc::new(FixedUrlResolver(Arc::new(ParsedUrl::parse(url).unwrap()))),
        WpApiClientDelegate {
            auth_provider: Arc::new(WpAuthenticationProvider::none()),
            request_executor: Arc::new(ReqwestRequestExecutor::default()),
            middleware_pipeline: Arc::new(WpApiMiddlewarePipeline::default()),
            app_notifier: Arc::new(EmptyAppNotifier),
            language_provider: None,
        },
    )
}

/// Against live Cloudflare: its error preview path, which answers like a real
/// error. Checks that the real client gets, and recognises, the JSON body.
///
/// `cargo test -p wp_api_integration_tests --test test_cloudflare_errors -- --ignored`
#[rstest]
#[case::origin_timeout(522, CloudflareErrorKind::OriginTimeout)]
#[case::firewall_rule(1020, CloudflareErrorKind::AccessDenied)]
#[tokio::test]
#[ignore = "Sends requests to developers.cloudflare.com"]
async fn live_cloudflare_error(#[case] code: u32, #[case] kind: CloudflareErrorKind) {
    let error = live_api_client(&format!(
        "https://developers.cloudflare.com/cdn-cgi/error/{code}"
    ))
    .users()
    .retrieve_me_with_edit_context()
    .await
    .unwrap_err();

    let WpApiError::RequestExecutionFailed {
        reason:
            RequestExecutionErrorReason::CloudflareError {
                cloudflare_code,
                kind: actual_kind,
                ray_id,
                ..
            },
        ..
    } = error
    else {
        panic!("Expected a Cloudflare error, got {error:#?}");
    };
    assert_eq!(cloudflare_code, Some(code));
    assert_eq!(actual_kind, kind);
    assert!(ray_id.is_some());
}

/// Against live Cloudflare: requesting one of its IP addresses directly (1003),
/// which has no zone and so is always plain text.
#[tokio::test]
#[ignore = "Sends a request to a Cloudflare IP address"]
async fn live_cloudflare_direct_ip_access() {
    let error = live_api_client("http://104.21.20.53/wp-json/wp/v2/users/me")
        .users()
        .retrieve_me_with_edit_context()
        .await
        .unwrap_err();

    let (code, kind, _) = cloudflare_error(error);
    assert_eq!(code, Some(1003));
    assert_eq!(kind, CloudflareErrorKind::Other);
}

/// The Cloudflare zone set up to produce each error for real, named by
/// `WPRS_CLOUDFLARE_TEST_ZONE` (e.g. `wpmtcf.co`). See
/// `test-data/cloudflare-responses/README.md` for what each host needs.
fn test_zone() -> String {
    std::env::var("WPRS_CLOUDFLARE_TEST_ZONE")
        .expect("Set WPRS_CLOUDFLARE_TEST_ZONE to the Cloudflare test zone")
}

/// Real failures between Cloudflare and a site's server, one host per error.
///
/// `WPRS_CLOUDFLARE_TEST_ZONE=wpmtcf.co cargo test -p wp_api_integration_tests
/// --test test_cloudflare_errors live_zone -- --ignored`
#[rstest]
#[case::origin_error("cf-520", 520, CloudflareErrorKind::OriginError)]
#[case::origin_down("cf-521", 521, CloudflareErrorKind::OriginDown)]
#[case::origin_timeout("cf-522", 522, CloudflareErrorKind::OriginTimeout)]
#[case::ssl_handshake("cf-525", 525, CloudflareErrorKind::OriginSsl)]
#[case::invalid_certificate("cf-526", 526, CloudflareErrorKind::OriginSsl)]
#[case::disallowed_origin_ip("cf-523", 1002, CloudflareErrorKind::OriginUnreachable)]
#[case::origin_dns("cf-1016", 1016, CloudflareErrorKind::OriginUnreachable)]
#[tokio::test]
#[ignore = "Needs the Cloudflare test zone"]
async fn live_zone_origin_failure(
    #[case] host: &str,
    #[case] code: u32,
    #[case] kind: CloudflareErrorKind,
) {
    // A 522 takes Cloudflare about 20 seconds; the default 10 second timeout
    // would end the request first.
    let client = WpApiClient::new(
        Arc::new(WpOrgSiteApiUrlResolver::new(Arc::new(
            ParsedUrl::parse(&format!("https://{host}.{}/wp-json", test_zone())).unwrap(),
        ))),
        WpApiClientDelegate {
            auth_provider: Arc::new(WpAuthenticationProvider::none()),
            request_executor: Arc::new(ReqwestRequestExecutor::new(
                false,
                std::time::Duration::from_secs(60),
            )),
            middleware_pipeline: Arc::new(WpApiMiddlewarePipeline::default()),
            app_notifier: Arc::new(EmptyAppNotifier),
            language_provider: None,
        },
    );
    let error = client
        .users()
        .retrieve_me_with_edit_context()
        .await
        .unwrap_err();

    let (actual_code, actual_kind, ray_id) = cloudflare_error(error);
    assert_eq!(actual_code, Some(code));
    assert_eq!(actual_kind, kind);
    assert!(ray_id.is_some());
}

/// A WAF rule's block action sends Cloudflare's HTML block page even when the
/// request asks for JSON, and a challenge sends its HTML challenge page.
#[rstest]
#[case::waf_block("/cf-test/block", CloudflareErrorKind::AccessDenied)]
#[case::managed_challenge("/cf-test/challenge", CloudflareErrorKind::Challenge)]
#[case::js_challenge("/cf-test/js-challenge", CloudflareErrorKind::Challenge)]
#[case::interactive_challenge("/cf-test/interactive", CloudflareErrorKind::Challenge)]
#[tokio::test]
#[ignore = "Needs the Cloudflare test zone"]
async fn live_zone_security_rule(#[case] path: &str, #[case] kind: CloudflareErrorKind) {
    let error = live_api_client(&format!("https://cf-rules.{}{path}", test_zone()))
        .users()
        .retrieve_me_with_edit_context()
        .await
        .unwrap_err();

    let (code, actual_kind, ray_id) = cloudflare_error(error);
    assert_eq!(code, None);
    assert_eq!(actual_kind, kind);
    assert!(ray_id.is_some());
}

#[tokio::test]
#[ignore = "Needs the Cloudflare test zone"]
async fn live_zone_rate_limit() {
    let client = live_api_client(&format!(
        "https://cf-rules.{}/cf-test/ratelimit",
        test_zone()
    ));
    // The rule allows 2 requests per 10 seconds.
    for _ in 0..5 {
        let error = client
            .users()
            .retrieve_me_with_edit_context()
            .await
            .unwrap_err();
        if let WpApiError::RequestExecutionFailed {
            reason:
                RequestExecutionErrorReason::CloudflareError {
                    cloudflare_code,
                    kind,
                    retry_after,
                    ..
                },
            ..
        } = error
        {
            assert_eq!(cloudflare_code, Some(1015));
            assert_eq!(kind, CloudflareErrorKind::RateLimited);
            assert!(retry_after.is_some());
            return;
        }
    }
    panic!("Cloudflare never rate-limited the requests");
}

/// The CMM-2500 case for real: an upload over the Free plan's 100 MB limit.
///
/// Cloudflare usually answers 413, but sometimes 502 with no code, after the
/// client has sent the whole body. Either way it's Cloudflare's own page.
#[tokio::test]
#[ignore = "Needs the Cloudflare test zone, and sends up to 150 MB"]
async fn live_zone_upload_too_large() {
    let path = std::env::temp_dir().join("wprs-cloudflare-150mb.mp4");
    std::fs::File::create(&path)
        .unwrap()
        .set_len(150 * 1024 * 1024)
        .unwrap();

    let error = live_api_client(&format!(
        "https://testing.{}/wp-json/wp/v2/media",
        test_zone()
    ))
    .media()
    .create(&MediaCreateParams {
        file_path: path.to_string_lossy().into_owned(),
        ..Default::default()
    })
    .await
    .unwrap_err();
    std::fs::remove_file(&path).unwrap();

    let (code, kind, ray_id) = cloudflare_error(error);
    assert_eq!(code, None);
    assert!(
        matches!(
            kind,
            CloudflareErrorKind::RequestTooLarge | CloudflareErrorKind::OriginError
        ),
        "{kind:?}"
    );
    assert!(ray_id.is_some());
}

/// What the login screen shows. Discovery sends no `Accept` header, so reqwest
/// sends `*/*` and Cloudflare answers with its branded HTML page, not JSON.
#[rstest]
#[case::challenge(
    "cf-challenge-all",
    "Your site's Cloudflare bot protection is blocking the app."
)]
#[case::waf_block(
    "cf-block-all",
    "A Cloudflare security rule on your site blocked this request."
)]
#[case::origin_down("cf-521", "Cloudflare reports that your site's server is down.")]
#[case::origin_timeout(
    "cf-522",
    "Cloudflare reports that your site's server isn't responding."
)]
#[case::origin_dns("cf-1016", "Cloudflare can't reach your site's server.")]
#[case::disallowed_origin_ip("cf-523", "Cloudflare can't reach your site's server.")]
#[tokio::test]
#[ignore = "Needs the Cloudflare test zone"]
async fn live_zone_login(#[case] host: &str, #[case] expected: &str) {
    use wp_api::login::login_client::WpLoginClient;
    use wp_localization::{WpLocale, WpLocalizable};

    let client = WpLoginClient::new_with_default_middleware_pipeline(Arc::new(
        ReqwestRequestExecutor::new(false, std::time::Duration::from_secs(60)),
    ));
    let failure = client
        .api_discovery(format!("https://{host}.{}", test_zone()), None)
        .await
        .combined_result()
        .expect_err("Login was expected to fail")
        .clone();

    assert_eq!(failure.localize(Some(WpLocale::from("en-US"))), expected);
}

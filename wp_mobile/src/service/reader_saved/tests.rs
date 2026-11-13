use super::*;
use crate::testing::EmptyAppNotifier;
use async_trait::async_trait;
use futures::channel::oneshot;
use rstest::rstest;
use serde_json::{Value, json};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use wp_api::posts::PostId;
use wp_api::prelude::*;
use wp_api::request::{NetworkRequestAccessor, RequestContext, WpMultipartFormRequest};
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::me::WpComUserId;
use wp_api::wp_com::reader_saved::{ReaderFeedId, ReaderFeedItemId};

pub(super) const KEYS_URL: &str = "https://public-api.wordpress.com/wpcom/v2/read/saved/keys";
pub(super) const NEW_URL: &str = "https://public-api.wordpress.com/wpcom/v2/read/saved/new";
pub(super) const DELETE_URL: &str = "https://public-api.wordpress.com/wpcom/v2/read/saved/delete";
pub(super) const LIST_URL: &str = "https://public-api.wordpress.com/rest/v1.2/read/saved";

pub(super) enum Step {
    Respond(u32, Value),
    /// Waits for the test to release it, so the test can act while the request is in flight.
    Hold(oneshot::Receiver<()>, u32, Value),
    /// The device is offline, so the request never left.
    Offline,
    /// The request timed out, so the server may have applied it.
    TimedOut,
}

#[derive(Debug, Clone)]
pub(super) struct Sent {
    pub(super) method: RequestMethod,
    pub(super) url: String,
    pub(super) body: Option<Value>,
}

impl Sent {
    /// The request in short: `keys`, `list`, `bulk`, or `new` or `delete` with its body.
    pub(super) fn short(&self) -> String {
        let endpoint = self.url.split('?').next().unwrap_or_default();
        let name = endpoint.rsplit('/').next().unwrap_or_default();
        match (name, &self.body) {
            ("new" | "delete", Some(body)) => {
                let mut body = body.clone();
                body.as_object_mut().unwrap().remove("source");
                format!("{name} {body}")
            }
            ("saved", _) => "list".to_string(),
            _ => name.to_string(),
        }
    }
}

/// Plays back a script of responses, and records every request.
#[derive(Debug, Default)]
pub(super) struct ScriptedExecutor {
    steps: Mutex<VecDeque<StepBox>>,
    sent: Mutex<Vec<Sent>>,
}

pub(super) struct StepBox(Step);

impl std::fmt::Debug for StepBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Step")
    }
}

impl ScriptedExecutor {
    pub(super) fn push(&self, step: Step) {
        self.steps.lock().unwrap().push_back(StepBox(step));
    }

    pub(super) fn sent(&self) -> Vec<Sent> {
        self.sent.lock().unwrap().clone()
    }

    pub(super) fn sent_short(&self) -> Vec<String> {
        self.sent().iter().map(Sent::short).collect()
    }
}

#[async_trait]
impl RequestExecutor for ScriptedExecutor {
    async fn execute(
        &self,
        request: Arc<WpNetworkRequest>,
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        self.sent.lock().unwrap().push(Sent {
            method: request.method(),
            url: request.url().0,
            body: request
                .body_as_string()
                .map(|body| serde_json::from_str(&body).unwrap()),
        });

        let step = self.steps.lock().unwrap().pop_front();
        let (status_code, body) = match step.expect("Unexpected request").0 {
            Step::Respond(status_code, body) => (status_code, body),
            Step::Hold(release, status_code, body) => {
                release.await.expect("The test dropped the release");
                (status_code, body)
            }
            Step::Offline => {
                return Err(RequestExecutionError::RequestExecutionFailed {
                    status_code: None,
                    redirects: None,
                    reason: RequestExecutionErrorReason::DeviceIsOfflineError {
                        error_message: "The Internet connection appears to be offline.".to_string(),
                    },
                    request_url: request.url().0,
                    request_method: request.method(),
                });
            }
            Step::TimedOut => {
                return Err(RequestExecutionError::RequestExecutionFailed {
                    status_code: None,
                    redirects: None,
                    reason: RequestExecutionErrorReason::HttpTimeoutError,
                    request_url: request.url().0,
                    request_method: request.method(),
                });
            }
        };

        Ok(WpNetworkResponse {
            body: body.to_string().into_bytes(),
            status_code,
            response_header_map: Default::default(),
            request_url: request.url(),
            request_method: request.method(),
            request_header_map: Default::default(),
        })
    }

    async fn upload(
        &self,
        _request: Arc<WpMultipartFormRequest>,
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        unimplemented!("The saved posts API doesn't upload")
    }

    async fn sleep(&self, _: u64) {}

    fn cancel(&self, _: Arc<RequestContext>) {}
}

pub(super) fn delegate(executor: Arc<dyn RequestExecutor>) -> WpApiClientDelegate {
    WpApiClientDelegate {
        auth_provider: Arc::new(WpAuthenticationProvider::none()),
        request_executor: executor,
        middleware_pipeline: Arc::new(WpApiMiddlewarePipeline::default()),
        app_notifier: Arc::new(EmptyAppNotifier),
        language_provider: None,
    }
}

pub(super) fn service_with(
    source: ReaderSavedSource,
) -> (ReaderSavedService, Arc<ScriptedExecutor>) {
    let executor = Arc::new(ScriptedExecutor::default());
    let service = ReaderSavedService::new(
        delegate(executor.clone()),
        source,
        storage::migrated_in_memory_cache(),
        WpComUserId(1),
    )
    .unwrap();
    (service, executor)
}

pub(super) fn service() -> (ReaderSavedService, Arc<ScriptedExecutor>) {
    service_with(ReaderSavedSource::Android)
}

pub(super) fn identity(post_id: u64) -> ReaderPostSaveIdentity {
    ReaderPostSaveIdentity {
        id: post_id,
        site_id: Some(WpComSiteId(123)),
        feed_id: None,
        feed_item_id: None,
        is_external: false,
        url: None,
    }
}

pub(super) fn jetpack_identity(post_id: u64, feed_item_id: u64) -> ReaderPostSaveIdentity {
    ReaderPostSaveIdentity {
        feed_id: Some(ReaderFeedId(5)),
        feed_item_id: Some(ReaderFeedItemId(feed_item_id)),
        ..identity(post_id)
    }
}

pub(super) fn rss_identity(feed_item_id: u64) -> ReaderPostSaveIdentity {
    ReaderPostSaveIdentity {
        id: feed_item_id,
        site_id: Some(WpComSiteId(5)),
        feed_id: Some(ReaderFeedId(5)),
        feed_item_id: Some(ReaderFeedItemId(feed_item_id)),
        is_external: true,
        url: None,
    }
}

pub(super) fn unsaveable_identity() -> ReaderPostSaveIdentity {
    ReaderPostSaveIdentity {
        site_id: None,
        ..identity(1)
    }
}

pub(super) fn post_key(post_id: i64) -> ReaderSavedKey {
    ReaderSavedKey::Post {
        blog_id: WpComSiteId(123),
        post_id: PostId(post_id),
    }
}

pub(super) fn feed_key(feed_item_id: u64) -> ReaderSavedKey {
    ReaderSavedKey::FeedItem {
        feed_item_id: ReaderFeedItemId(feed_item_id),
    }
}

pub(super) fn post_json(post_id: u64) -> Value {
    json!({ "blog_id": 123, "post_id": post_id })
}

pub(super) fn feed_json(feed_item_id: u64) -> Value {
    json!({ "feed_item_id": feed_item_id })
}

pub(super) fn keys_response(keys: &[Value]) -> Value {
    let entries: Vec<_> = keys
        .iter()
        .enumerate()
        .map(|(index, key)| {
            let mut entry = key.clone();
            entry["id"] = json!(index + 1);
            entry["date_saved"] = json!("2026-11-02T10:15:00+00:00");
            entry
        })
        .collect();
    json!({ "keys": entries, "total": keys.len() })
}

/// A `new` or `delete` response. Only `new` sends `resolved`; `delete` ignores it.
pub(super) fn write_response(saved: bool, key: Value, total: u64) -> Value {
    let resolved = if key.get("url").is_some() {
        "url"
    } else if key.get("feed_item_id").is_some() {
        "feed_item"
    } else {
        "post"
    };
    json!({ "saved": saved, "key": key, "total": total, "resolved": resolved })
}

pub(super) fn bulk_response(keys: &[Value], total: u64) -> Value {
    let results: Vec<_> = keys
        .iter()
        .map(|key| json!({ "key": key, "saved": true }))
        .collect();
    json!({ "results": results, "total": total })
}

pub(super) fn listed_post(post_id: u64) -> Value {
    json!({ "ID": post_id, "site_ID": 123, "is_external": false, "title": format!("Post {post_id}") })
}

pub(super) fn listed_rss_post(feed_item_id: u64) -> Value {
    json!({
        "ID": feed_item_id,
        "site_ID": 5,
        "feed_ID": 5,
        "feed_item_ID": feed_item_id,
        "is_external": true,
    })
}

pub(super) fn error_response() -> Value {
    json!({ "code": "post_not_found", "message": "Post not found.", "data": { "status": 404 } })
}

pub(super) fn cap_response() -> Value {
    json!({ "code": "saved_limit_reached", "message": "You can save up to 1000 posts." })
}

/// Syncs, with the server holding `keys` and nothing pending.
pub(super) async fn synced(
    service: &ReaderSavedService,
    executor: &ScriptedExecutor,
    keys: &[Value],
) {
    executor.push(Step::Respond(200, keys_response(keys)));
    service.sync().await.unwrap();
}

fn status(service: &ReaderSavedService, post: ReaderPostSaveIdentity) -> ReaderSavedStatus {
    service.status(post).unwrap()
}

fn count(service: &ReaderSavedService) -> Option<u64> {
    service.saved_count().unwrap()
}

fn is_pending(outcome: &ReaderSavedOutcome) -> bool {
    matches!(outcome, ReaderSavedOutcome::Pending { .. })
}

use ReaderSavedStatus::{NotSaved, Saved, Unknown};

// Syncing

#[tokio::test]
async fn a_post_is_unknown_before_a_sync() {
    let (service, _) = service();
    assert_eq!(status(&service, identity(1)), Unknown);
    assert_eq!(count(&service), None);
}

#[tokio::test]
async fn a_sync_loads_the_keys() {
    let (service, executor) = service();
    synced(&service, &executor, &[post_json(1), feed_json(2)]).await;

    assert_eq!(status(&service, identity(1)), Saved);
    assert_eq!(status(&service, rss_identity(2)), Saved);
    assert_eq!(status(&service, identity(3)), NotSaved);
    assert_eq!(count(&service), Some(2));

    let sent = executor.sent();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].method, RequestMethod::GET);
    assert!(sent[0].url.starts_with(KEYS_URL), "{}", sent[0].url);
}

#[tokio::test]
async fn a_failed_sync_reports_the_error_and_keeps_the_keys() {
    let (service, executor) = service();
    synced(&service, &executor, &[post_json(1)]).await;

    executor.push(Step::Respond(500, error_response()));

    assert!(matches!(
        service.sync().await,
        Err(ReaderSavedError::Api(_))
    ));
    assert_eq!(status(&service, identity(1)), Saved);
}

#[tokio::test]
async fn a_sync_that_cannot_reach_the_server_reports_it() {
    let (service, executor) = service();
    executor.push(Step::Offline);

    assert!(matches!(
        service.sync().await,
        Err(ReaderSavedError::Api(_))
    ));
    assert_eq!(status(&service, identity(1)), Unknown);
}

#[tokio::test]
async fn a_sync_with_a_malformed_response_fails_rather_than_emptying_the_list() {
    let (service, executor) = service();
    synced(&service, &executor, &[post_json(1)]).await;

    executor.push(Step::Respond(
        200,
        json!({ "keys": [{ "id": 1 }], "total": 1 }),
    ));

    assert!(service.sync().await.is_err());
    assert_eq!(status(&service, identity(1)), Saved);
}

/// A sync sends what's pending before it reads the keys, so the read has it.
#[tokio::test]
async fn a_sync_sends_what_is_pending_before_it_reads() {
    let (service, executor) = service();
    executor.push(Step::Offline);
    let outcome = service.save(identity(1)).await.unwrap();
    assert!(is_pending(&outcome), "{outcome:?}");

    executor.push(Step::Respond(200, write_response(true, post_json(1), 1)));
    synced(&service, &executor, &[post_json(1)]).await;

    assert_eq!(
        executor.sent_short()[1..],
        [r#"new {"blog_id":123,"post_id":1}"#, "keys"]
    );
    assert_eq!(status(&service, identity(1)), Saved);
    assert_eq!(count(&service), Some(1));
}

// Status

#[tokio::test]
async fn a_jetpack_post_is_saved_under_either_key() {
    let (service, executor) = service();
    synced(&service, &executor, &[feed_json(2)]).await;

    assert_eq!(status(&service, jetpack_identity(1, 2)), Saved);
}

#[tokio::test]
async fn a_post_without_ids_is_not_saved() {
    let (service, _) = service();
    assert_eq!(status(&service, unsaveable_identity()), NotSaved);
}

#[tokio::test]
async fn status_does_not_touch_the_network() {
    let (service, executor) = service();
    service.status(identity(1)).unwrap();
    service.saved_count().unwrap();
    assert!(executor.sent().is_empty());
}

// Saving

#[rstest]
#[case::android(ReaderSavedSource::Android, "android")]
#[case::ios(ReaderSavedSource::Ios, "ios")]
#[tokio::test]
async fn save_sends_the_key_and_source(#[case] source: ReaderSavedSource, #[case] name: &str) {
    let (service, executor) = service_with(source);
    executor.push(Step::Respond(200, write_response(true, post_json(1), 1)));

    let outcome = service.save(identity(1)).await.unwrap();

    assert_eq!(outcome, ReaderSavedOutcome::Synced { key: post_key(1) });
    let sent = executor.sent();
    assert_eq!(sent[0].method, RequestMethod::POST);
    assert!(sent[0].url.starts_with(NEW_URL), "{}", sent[0].url);
    assert_eq!(
        sent[0].body,
        Some(json!({ "blog_id": 123, "post_id": 1, "source": name }))
    );
}

#[tokio::test]
async fn saving_an_rss_post_sends_its_feed_item() {
    let (service, executor) = service();
    executor.push(Step::Respond(200, write_response(true, feed_json(7), 1)));

    service.save(rss_identity(7)).await.unwrap();

    assert_eq!(
        executor.sent()[0].body,
        Some(json!({ "feed_item_id": 7, "source": "android" }))
    );
}

#[tokio::test]
async fn a_post_without_ids_cannot_be_saved_and_sends_nothing() {
    let (service, executor) = service();

    let result = service.save(unsaveable_identity()).await;

    assert!(matches!(result, Err(ReaderSavedError::PostCannotBeSaved)));
    assert!(executor.sent().is_empty());
}

#[tokio::test]
async fn a_save_shows_while_the_request_is_in_flight() {
    let (service, executor) = service();
    synced(&service, &executor, &[]).await;
    let (release, held) = oneshot::channel();
    executor.push(Step::Hold(held, 200, write_response(true, post_json(1), 1)));

    let (result, ()) = futures::join!(service.save(identity(1)), async {
        assert_eq!(status(&service, identity(1)), Saved);
        assert_eq!(count(&service), Some(1));
        release.send(()).unwrap();
    });

    result.unwrap();
    assert_eq!(status(&service, identity(1)), Saved);
}

/// The account's save cap is the one forbidden answer that won't pass.
#[tokio::test]
async fn a_save_refused_at_the_cap_is_undone_and_reported() {
    let (service, executor) = service();
    synced(&service, &executor, &[]).await;
    executor.push(Step::Respond(403, cap_response()));

    let result = service.save(identity(1)).await;

    let Err(ReaderSavedError::Api(error)) = result else {
        panic!("Expected an API error, got {result:?}");
    };
    assert!(error.to_string().contains("1000"), "{error}");
    assert_eq!(status(&service, identity(1)), NotSaved);
    assert_eq!(count(&service), Some(0));
}

/// Any other forbidden answer, such as an expired token, may pass, so the save waits to be sent
/// again.
#[tokio::test]
async fn a_save_forbidden_for_another_reason_stays_queued() {
    let (service, executor) = service();
    synced(&service, &executor, &[]).await;
    executor.push(Step::Respond(
        403,
        json!({ "code": "rest_forbidden", "message": "Sorry, you are not allowed to do that." }),
    ));

    let outcome = service.save(identity(1)).await.unwrap();

    assert!(is_pending(&outcome), "{outcome:?}");
    assert_eq!(status(&service, identity(1)), Saved);
    executor.push(Step::Respond(200, write_response(true, post_json(1), 1)));
    synced(&service, &executor, &[post_json(1)]).await;
    assert_eq!(executor.sent().len(), 4, "sent again by the sync");
}

#[tokio::test]
async fn a_save_that_cannot_reach_the_server_is_queued() {
    let (service, executor) = service();
    synced(&service, &executor, &[]).await;
    executor.push(Step::Offline);

    let outcome = service.save(identity(1)).await.unwrap();

    assert!(is_pending(&outcome), "{outcome:?}");
    assert_eq!(status(&service, identity(1)), Saved);
    assert_eq!(count(&service), Some(1));
}

/// The server answered, so it may have applied the save: a sync says.
#[tokio::test]
async fn a_save_whose_answer_cannot_be_read_waits_for_a_sync() {
    let (service, executor) = service();
    synced(&service, &executor, &[]).await;
    executor.push(Step::Respond(200, json!({ "unexpected": true })));

    let outcome = service.save(identity(1)).await.unwrap();

    assert!(is_pending(&outcome), "{outcome:?}");
    assert_eq!(status(&service, identity(1)), Saved);
}

#[tokio::test]
async fn a_save_answered_with_a_server_error_waits_for_a_sync() {
    let (service, executor) = service();
    synced(&service, &executor, &[]).await;
    executor.push(Step::Respond(502, json!({})));

    let outcome = service.save(identity(1)).await.unwrap();

    assert!(is_pending(&outcome), "{outcome:?}");
    assert_eq!(status(&service, identity(1)), Saved);
}

/// The timed-out save isn't sent again until the keys say the server doesn't have it.
#[tokio::test]
async fn a_save_that_times_out_is_sent_again_once_a_sync_says_the_server_lacks_it() {
    let (service, executor) = service();
    synced(&service, &executor, &[]).await;
    executor.push(Step::TimedOut);

    let outcome = service.save(identity(1)).await.unwrap();
    assert!(is_pending(&outcome), "{outcome:?}");
    assert_eq!(status(&service, identity(1)), Saved);

    executor.push(Step::Respond(200, keys_response(&[])));
    executor.push(Step::Respond(200, write_response(true, post_json(1), 1)));
    service.sync().await.unwrap();

    assert_eq!(
        executor.sent_short()[1..],
        [
            r#"new {"blog_id":123,"post_id":1}"#,
            "keys",
            r#"new {"blog_id":123,"post_id":1}"#
        ]
    );
    assert_eq!(status(&service, identity(1)), Saved);
    assert_eq!(count(&service), Some(1));
}

#[tokio::test]
async fn a_save_stored_under_another_key_shows_under_that_key() {
    let (service, executor) = service();
    synced(&service, &executor, &[]).await;
    executor.push(Step::Respond(200, write_response(true, post_json(1), 1)));

    let outcome = service.save(rss_identity(7)).await.unwrap();

    assert_eq!(outcome, ReaderSavedOutcome::Synced { key: post_key(1) });
    assert_eq!(status(&service, identity(1)), Saved);
    assert_eq!(status(&service, rss_identity(7)), Saved);
}

// Saving URLs

#[tokio::test]
async fn save_url_sends_the_page_details() {
    let (service, executor) = service();
    executor.push(Step::Respond(
        200,
        write_response(true, json!({ "url": "https://example.com/page" }), 1),
    ));

    service
        .save_url(
            "https://example.com/page".to_string(),
            Some(ReaderSavedPageDetails {
                title: Some("A page".to_string()),
                ..Default::default()
            }),
        )
        .await
        .unwrap();

    assert_eq!(
        executor.sent()[0].body,
        Some(json!({
            "url": "https://example.com/page",
            "source": "android",
            "title": "A page",
        }))
    );
}

/// The details last until the save is answered, so a retry within the launch sends them too.
#[tokio::test]
async fn a_url_save_sent_again_still_sends_the_page_details() {
    let (service, executor) = service();
    executor.push(Step::Offline);
    service
        .save_url(
            "https://example.com/page".to_string(),
            Some(ReaderSavedPageDetails {
                title: Some("A page".to_string()),
                ..Default::default()
            }),
        )
        .await
        .unwrap();

    executor.push(Step::Respond(
        200,
        write_response(true, json!({ "url": "https://example.com/page" }), 1),
    ));
    executor.push(Step::Respond(200, keys_response(&[])));
    service.sync().await.unwrap();

    assert_eq!(executor.sent()[0].body, executor.sent()[1].body);
}

#[tokio::test]
async fn a_url_of_a_wp_com_post_returns_the_post_key() {
    let (service, executor) = service();
    synced(&service, &executor, &[]).await;
    executor.push(Step::Respond(200, write_response(true, post_json(1), 1)));

    let outcome = service
        .save_url("https://example.com/post".to_string(), None)
        .await
        .unwrap();

    assert_eq!(outcome, ReaderSavedOutcome::Synced { key: post_key(1) });
    assert_eq!(status(&service, identity(1)), Saved);
}

#[tokio::test]
async fn a_saved_url_can_be_unsaved_with_another_spelling() {
    let (service, executor) = service();
    synced(&service, &executor, &[]).await;
    let page = json!({ "url": "https://example.com/page" });
    executor.push(Step::Respond(200, write_response(true, page.clone(), 1)));
    executor.push(Step::Respond(200, write_response(false, page, 0)));

    let ReaderSavedOutcome::Synced { key } = service
        .save_url("https://example.com/page/?utm_source=x".to_string(), None)
        .await
        .unwrap()
    else {
        panic!("Expected the save to sync");
    };
    assert_eq!(count(&service), Some(1));

    service.unsave_key(key).await.unwrap();

    assert_eq!(count(&service), Some(0));
    assert_eq!(executor.sent().len(), 3);
}

#[rstest]
#[case::empty_url(ReaderSavedKey::Url { url: String::new() })]
#[case::zero_blog(ReaderSavedKey::Post { blog_id: WpComSiteId(0), post_id: PostId(1) })]
#[case::zero_feed_item(ReaderSavedKey::FeedItem { feed_item_id: ReaderFeedItemId(0) })]
#[tokio::test]
async fn a_key_the_api_cant_take_is_refused_before_it_becomes_a_row(#[case] key: ReaderSavedKey) {
    let (service, executor) = service();

    let unsave = service.unsave_key(key.clone()).await;
    let rows = service.rows_for(std::slice::from_ref(&key)).unwrap();
    if let ReaderSavedKey::Url { url } = key {
        let save = service.save_url(url, None).await;
        assert!(matches!(save, Err(ReaderSavedError::PostCannotBeSaved)));
    }

    assert!(matches!(unsave, Err(ReaderSavedError::PostCannotBeSaved)));
    assert!(executor.sent().is_empty());
    assert!(rows.is_empty());
    assert!(service.saved_list_items().unwrap().is_empty());
}

#[tokio::test]
async fn a_post_with_a_zero_id_cannot_be_saved_or_unsaved() {
    let (service, executor) = service();
    let zero = ReaderPostSaveIdentity {
        site_id: Some(WpComSiteId(0)),
        ..identity(1)
    };

    assert!(matches!(
        service.save(zero.clone()).await,
        Err(ReaderSavedError::PostCannotBeSaved)
    ));
    assert!(matches!(
        service.unsave(zero).await,
        Err(ReaderSavedError::PostCannotBeSaved)
    ));
    assert!(executor.sent().is_empty());
}

// Unsaving

#[tokio::test]
async fn unsave_sends_the_key_and_source() {
    let (service, executor) = service();
    synced(&service, &executor, &[post_json(1)]).await;
    executor.push(Step::Respond(200, write_response(false, post_json(1), 0)));

    let outcome = service.unsave(identity(1)).await.unwrap();

    assert_eq!(outcome, ReaderSavedOutcome::Synced { key: post_key(1) });
    let sent = executor.sent();
    let request = sent.last().unwrap();
    assert!(request.url.starts_with(DELETE_URL), "{}", request.url);
    assert_eq!(request.method, RequestMethod::POST);
    assert_eq!(
        request.body,
        Some(json!({ "blog_id": 123, "post_id": 1, "source": "android" }))
    );
    assert_eq!(status(&service, identity(1)), NotSaved);
}

/// The keys may be out of date, so every key the post may be saved under is sent.
#[tokio::test]
async fn unsave_sends_every_key_the_post_may_be_saved_under() {
    let (service, executor) = service();
    synced(&service, &executor, &[feed_json(2)]).await;
    // The feed item's row first. The post isn't saved under its blog and post IDs, so deleting
    // those does nothing.
    executor.push(Step::Respond(200, write_response(false, feed_json(2), 0)));
    executor.push(Step::Respond(200, write_response(false, post_json(1), 0)));

    service.unsave(jetpack_identity(1, 2)).await.unwrap();

    assert_eq!(
        executor.sent_short()[1..],
        [
            r#"delete {"feed_item_id":2}"#,
            r#"delete {"blog_id":123,"post_id":1}"#
        ]
    );
    assert_eq!(status(&service, jetpack_identity(1, 2)), NotSaved);
    assert_eq!(count(&service), Some(0));
}

#[tokio::test]
async fn unsave_removes_every_key_that_matched() {
    let (service, executor) = service();
    synced(&service, &executor, &[post_json(1), feed_json(2)]).await;
    executor.push(Step::Respond(200, write_response(false, post_json(1), 1)));
    executor.push(Step::Respond(200, write_response(false, feed_json(2), 0)));

    service.unsave(jetpack_identity(1, 2)).await.unwrap();

    assert_eq!(executor.sent().len(), 3);
    assert_eq!(status(&service, jetpack_identity(1, 2)), NotSaved);
    assert_eq!(count(&service), Some(0));
}

/// Before the first sync nothing says which key the post is saved under, and missing one leaves it
/// saved.
#[tokio::test]
async fn unsave_before_a_sync_sends_every_key() {
    let (service, executor) = service();
    executor.push(Step::Respond(200, write_response(false, post_json(1), 0)));
    executor.push(Step::Respond(200, write_response(false, feed_json(2), 0)));

    service.unsave(jetpack_identity(1, 2)).await.unwrap();

    assert_eq!(
        executor.sent_short(),
        [
            r#"delete {"blog_id":123,"post_id":1}"#,
            r#"delete {"feed_item_id":2}"#
        ]
    );
    assert_eq!(status(&service, jetpack_identity(1, 2)), NotSaved);
}

/// The keys say the post isn't saved, but they may be out of date, so the unsave is still sent.
#[tokio::test]
async fn unsaving_a_post_the_keys_say_is_not_saved_still_sends_it() {
    let (service, executor) = service();
    synced(&service, &executor, &[]).await;
    executor.push(Step::Respond(200, write_response(false, post_json(1), 0)));

    let outcome = service.unsave(identity(1)).await.unwrap();

    assert_eq!(outcome, ReaderSavedOutcome::Synced { key: post_key(1) });
    assert_eq!(
        executor.sent_short()[1..],
        [r#"delete {"blog_id":123,"post_id":1}"#]
    );
}

#[tokio::test]
async fn a_post_without_ids_cannot_be_unsaved_and_sends_nothing() {
    let (service, executor) = service();

    let result = service.unsave(unsaveable_identity()).await;

    assert!(matches!(result, Err(ReaderSavedError::PostCannotBeSaved)));
    assert!(executor.sent().is_empty());
}

#[tokio::test]
async fn an_unsave_shows_while_the_request_is_in_flight() {
    let (service, executor) = service();
    synced(&service, &executor, &[post_json(1)]).await;
    let (release, held) = oneshot::channel();
    executor.push(Step::Hold(
        held,
        200,
        write_response(false, post_json(1), 0),
    ));

    let (result, ()) = futures::join!(service.unsave(identity(1)), async {
        assert_eq!(status(&service, identity(1)), NotSaved);
        assert_eq!(count(&service), Some(0));
        release.send(()).unwrap();
    });

    result.unwrap();
}

#[tokio::test]
async fn unsaving_a_post_under_two_keys_shows_straight_away() {
    let (service, executor) = service();
    synced(&service, &executor, &[post_json(1), feed_json(2)]).await;
    let (release, held) = oneshot::channel();
    executor.push(Step::Hold(
        held,
        200,
        write_response(false, post_json(1), 1),
    ));
    executor.push(Step::Respond(200, write_response(false, feed_json(2), 0)));

    let (result, ()) = futures::join!(service.unsave(jetpack_identity(1, 2)), async {
        assert_eq!(status(&service, jetpack_identity(1, 2)), NotSaved);
        assert_eq!(count(&service), Some(0));
        release.send(()).unwrap();
    });

    result.unwrap();
}

/// An unsave tapped offline waits on the device, and the next sync sends it.
#[tokio::test]
async fn an_unsave_that_cannot_reach_the_server_is_sent_by_the_next_sync() {
    let (service, executor) = service();
    synced(&service, &executor, &[post_json(1)]).await;
    executor.push(Step::Offline);

    let outcome = service.unsave(identity(1)).await.unwrap();
    assert!(is_pending(&outcome), "{outcome:?}");
    assert_eq!(status(&service, identity(1)), NotSaved);
    assert_eq!(count(&service), Some(0));

    executor.push(Step::Respond(200, write_response(false, post_json(1), 0)));
    synced(&service, &executor, &[]).await;

    assert_eq!(
        executor.sent_short()[1..],
        [
            r#"delete {"blog_id":123,"post_id":1}"#,
            r#"delete {"blog_id":123,"post_id":1}"#,
            "keys"
        ]
    );
    assert_eq!(status(&service, identity(1)), NotSaved);
}

// Cancellation: Kotlin cancels a call by dropping its future. Swift doesn't.

/// The tap is on the device the moment it's made, so cancelling the call that waits to send it
/// doesn't undo it: the call holding the lane sends it.
#[tokio::test]
async fn a_save_cancelled_while_waiting_its_turn_is_still_sent() {
    let (service, executor) = service();
    synced(&service, &executor, &[]).await;
    let (release, held) = oneshot::channel();
    executor.push(Step::Hold(held, 200, write_response(true, post_json(2), 1)));
    executor.push(Step::Respond(200, write_response(true, post_json(1), 2)));

    let mut earlier = Box::pin(service.save(identity(2)));
    let mut save = Box::pin(service.save(identity(1)));
    assert!(futures::poll!(earlier.as_mut()).is_pending());
    assert!(futures::poll!(save.as_mut()).is_pending());
    drop(save);
    assert_eq!(status(&service, identity(1)), Saved);

    release.send(()).unwrap();
    earlier.await.unwrap();

    assert_eq!(executor.sent().len(), 3);
    assert_eq!(status(&service, identity(1)), Saved);
    assert_eq!(count(&service), Some(2));
}

/// Its request went out, so the server may have applied it: it waits for a sync.
#[tokio::test]
async fn a_save_cancelled_after_it_is_sent_waits_for_a_sync() {
    let (service, executor) = service();
    synced(&service, &executor, &[]).await;
    let (_release, held) = oneshot::channel::<()>();
    executor.push(Step::Hold(held, 200, write_response(true, post_json(1), 1)));

    let mut save = Box::pin(service.save(identity(1)));
    assert!(futures::poll!(save.as_mut()).is_pending());
    drop(save);

    assert_eq!(status(&service, identity(1)), Saved);
    assert_eq!(count(&service), Some(1));

    // The server applied it, and a sync says so: nothing is sent again.
    synced(&service, &executor, &[post_json(1)]).await;
    assert_eq!(executor.sent().len(), 3);
    assert_eq!(status(&service, identity(1)), Saved);
}

/// The delete of the post key went out, and the feed item's waits for the next call that sends.
#[tokio::test]
async fn a_cancelled_unsave_keeps_both_keys_unsaved() {
    let (service, executor) = service();
    synced(&service, &executor, &[post_json(1), feed_json(2)]).await;
    let (_release, held) = oneshot::channel::<()>();
    executor.push(Step::Hold(
        held,
        200,
        write_response(false, post_json(1), 1),
    ));

    let mut unsave = Box::pin(service.unsave(jetpack_identity(1, 2)));
    assert!(futures::poll!(unsave.as_mut()).is_pending());
    drop(unsave);

    assert_eq!(status(&service, identity(1)), NotSaved);
    assert_eq!(status(&service, rss_identity(2)), NotSaved);
    assert_eq!(count(&service), Some(0));

    // The server didn't get the post key's delete. The sync sends the feed item's, and then the
    // post key's again, now the keys say it's needed.
    executor.push(Step::Respond(200, write_response(false, feed_json(2), 1)));
    executor.push(Step::Respond(200, keys_response(&[post_json(1)])));
    executor.push(Step::Respond(200, write_response(false, post_json(1), 0)));
    service.sync().await.unwrap();

    assert_eq!(
        executor.sent_short()[1..],
        [
            r#"delete {"blog_id":123,"post_id":1}"#,
            r#"delete {"feed_item_id":2}"#,
            "keys",
            r#"delete {"blog_id":123,"post_id":1}"#
        ]
    );
    assert_eq!(status(&service, jetpack_identity(1, 2)), NotSaved);
    assert_eq!(count(&service), Some(0));
}

/// A feed item stops resolving once it's pruned, so the server can't find a save the device knows
/// is there.
#[tokio::test]
async fn a_feed_item_the_server_cannot_find_is_an_error_when_it_was_saved_there() {
    let (service, executor) = service();
    synced(&service, &executor, &[feed_json(2)]).await;
    executor.push(Step::Respond(404, error_response()));

    assert!(service.unsave(rss_identity(2)).await.is_err());

    assert_eq!(status(&service, rss_identity(2)), Saved);
}

/// Before a sync, so the delete is sent.
#[tokio::test]
async fn a_feed_item_the_server_cannot_find_counts_as_unsaved_when_it_was_not_saved_there() {
    let (service, executor) = service();
    executor.push(Step::Respond(404, error_response()));

    service.unsave(rss_identity(2)).await.unwrap();

    assert_eq!(executor.sent().len(), 1);
    assert_eq!(status(&service, rss_identity(2)), NotSaved);
}

#[tokio::test]
async fn a_refused_unsave_is_undone_and_reported() {
    let (service, executor) = service();
    synced(&service, &executor, &[post_json(1)]).await;
    executor.push(Step::Respond(400, error_response()));

    assert!(service.unsave(identity(1)).await.is_err());

    assert_eq!(status(&service, identity(1)), Saved);
    assert_eq!(count(&service), Some(1));
}

#[tokio::test]
async fn a_partly_refused_unsave_tries_every_key_and_keeps_the_one_refused() {
    let (service, executor) = service();
    synced(&service, &executor, &[post_json(1), feed_json(2)]).await;
    executor.push(Step::Respond(400, error_response()));
    executor.push(Step::Respond(200, write_response(false, feed_json(2), 1)));

    assert!(service.unsave(jetpack_identity(1, 2)).await.is_err());

    assert_eq!(executor.sent().len(), 3);
    assert_eq!(status(&service, jetpack_identity(1, 2)), Saved);
    assert_eq!(status(&service, rss_identity(2)), NotSaved);
    assert_eq!(count(&service), Some(1));
}

/// A refusal is kept only for a tap waiting to hear it, so one a sync meets, with no tap waiting,
/// isn't kept for the life of the service.
#[tokio::test]
async fn a_refusal_no_tap_waits_for_is_not_kept() {
    let (service, executor) = service();
    synced(&service, &executor, &[]).await;
    executor.push(Step::Offline);
    service.save(identity(1)).await.unwrap();

    executor.push(Step::Respond(400, error_response()));
    executor.push(Step::Respond(200, keys_response(&[])));
    service.sync().await.unwrap();

    assert_eq!(status(&service, identity(1)), NotSaved);
    assert!(service.refusals.lock().unwrap().is_empty());
    assert!(service.waiting.lock().unwrap().is_empty());
}

/// A tap cancelled while it waits stops waiting, so a refusal of its row isn't kept either.
#[tokio::test]
async fn a_cancelled_tap_stops_waiting() {
    let (service, executor) = service();
    synced(&service, &executor, &[]).await;
    let (_release, held) = oneshot::channel::<()>();
    executor.push(Step::Hold(held, 200, write_response(true, post_json(1), 1)));

    let mut save = Box::pin(service.save(identity(1)));
    assert!(futures::poll!(save.as_mut()).is_pending());
    assert_eq!(service.waiting.lock().unwrap().len(), 1);
    drop(save);

    assert!(service.waiting.lock().unwrap().is_empty());
}

// One lane: a sync and a tap never overlap on the network

/// The sync waits for the unsave in flight, then reads, so its read includes the unsave.
#[tokio::test]
async fn a_sync_waits_for_a_write_in_flight() {
    let (service, executor) = service();
    synced(&service, &executor, &[post_json(1)]).await;
    let (release, held) = oneshot::channel();
    executor.push(Step::Hold(
        held,
        200,
        write_response(false, post_json(1), 0),
    ));
    executor.push(Step::Respond(200, keys_response(&[])));

    let mut unsave = Box::pin(service.unsave(identity(1)));
    let mut sync = Box::pin(service.sync());
    assert!(futures::poll!(unsave.as_mut()).is_pending());
    assert!(futures::poll!(sync.as_mut()).is_pending());
    assert_eq!(executor.sent().len(), 2, "the sync hasn't read yet");
    assert_eq!(status(&service, identity(1)), NotSaved);

    release.send(()).unwrap();
    let (unsave, sync) = futures::join!(unsave, sync);
    unsave.unwrap();
    sync.unwrap();

    assert_eq!(
        executor.sent_short(),
        ["keys", r#"delete {"blog_id":123,"post_id":1}"#, "keys"]
    );
    assert_eq!(status(&service, identity(1)), NotSaved);
}

/// The server applies a post's writes in the order it receives them, and the unsave goes once the
/// save is answered.
#[tokio::test]
async fn a_write_waits_for_the_posts_earlier_write_to_finish() {
    let (service, executor) = service();
    synced(&service, &executor, &[]).await;
    let (release, held) = oneshot::channel();
    executor.push(Step::Hold(held, 200, write_response(true, post_json(1), 1)));
    executor.push(Step::Respond(200, write_response(false, post_json(1), 0)));

    let mut save = Box::pin(service.save(identity(1)));
    let mut unsave = Box::pin(service.unsave(identity(1)));
    assert!(futures::poll!(save.as_mut()).is_pending());
    assert!(futures::poll!(unsave.as_mut()).is_pending());
    assert_eq!(executor.sent().len(), 2, "only the save has been sent");
    assert_eq!(status(&service, identity(1)), NotSaved);

    release.send(()).unwrap();
    let (save, unsave) = futures::join!(save, unsave);
    save.unwrap();
    unsave.unwrap();

    let sent = executor.sent();
    assert!(sent[1].url.starts_with(NEW_URL), "{}", sent[1].url);
    assert!(sent[2].url.starts_with(DELETE_URL), "{}", sent[2].url);
    assert_eq!(sent.len(), 3);
    assert_eq!(status(&service, identity(1)), NotSaved);
    assert_eq!(count(&service), Some(0));
}

/// The cancelled save may have reached the server, so its row is in doubt, and the unsave waits
/// for a sync to say what the server has.
#[tokio::test]
async fn an_unsave_behind_a_cancelled_save_waits_for_a_sync() {
    let (service, executor) = service();
    synced(&service, &executor, &[]).await;
    let (_release, held) = oneshot::channel::<()>();
    executor.push(Step::Hold(held, 200, write_response(true, post_json(1), 1)));

    let mut save = Box::pin(service.save(identity(1)));
    let mut unsave = Box::pin(service.unsave(identity(1)));
    assert!(futures::poll!(save.as_mut()).is_pending());
    assert!(futures::poll!(unsave.as_mut()).is_pending());

    drop(save);
    let outcome = unsave.await.unwrap();

    assert!(is_pending(&outcome), "{outcome:?}");
    assert_eq!(executor.sent().len(), 2);
    assert_eq!(status(&service, identity(1)), NotSaved);

    // The server applied the save, and the sync sends the unsave.
    executor.push(Step::Respond(200, keys_response(&[post_json(1)])));
    executor.push(Step::Respond(200, write_response(false, post_json(1), 0)));
    service.sync().await.unwrap();
    assert_eq!(executor.sent().len(), 4);
    assert_eq!(status(&service, identity(1)), NotSaved);
    assert_eq!(count(&service), Some(0));
}

/// A save the server stores under another key, with an unsave queued behind it. The answer moves
/// the row to the stored key, so the unsave goes to that key alone.
#[tokio::test]
async fn a_queued_unsave_also_unsaves_the_key_the_save_was_stored_under() {
    let (service, executor) = service();
    synced(&service, &executor, &[]).await;
    let (release, held) = oneshot::channel();
    executor.push(Step::Hold(held, 200, write_response(true, post_json(1), 1)));
    executor.push(Step::Respond(200, write_response(false, post_json(1), 0)));

    let mut save = Box::pin(service.save(rss_identity(7)));
    let mut unsave = Box::pin(service.unsave(rss_identity(7)));
    assert!(futures::poll!(save.as_mut()).is_pending());
    assert!(futures::poll!(unsave.as_mut()).is_pending());
    assert_eq!(status(&service, rss_identity(7)), NotSaved);

    release.send(()).unwrap();
    let (save, unsave) = futures::join!(save, unsave);
    save.unwrap();
    unsave.unwrap();

    assert_eq!(
        executor.sent_short()[1..],
        [
            r#"new {"feed_item_id":7}"#,
            r#"delete {"blog_id":123,"post_id":1}"#
        ]
    );
    assert_eq!(status(&service, rss_identity(7)), NotSaved);
    assert_eq!(status(&service, identity(1)), NotSaved);
    assert_eq!(count(&service), Some(0));
}

#[tokio::test]
async fn unsave_key_also_unsaves_the_key_the_server_stores_the_post_under() {
    let (service, executor) = service();
    synced(&service, &executor, &[]).await;
    executor.push(Step::Respond(200, write_response(true, post_json(1), 1)));
    executor.push(Step::Respond(200, write_response(false, post_json(1), 0)));
    // The feed item can also be a separate older save, so it's sent too.
    executor.push(Step::Respond(200, write_response(false, post_json(1), 0)));
    service.save(rss_identity(7)).await.unwrap();

    service.unsave_key(feed_key(7)).await.unwrap();

    assert_eq!(
        executor.sent_short()[1..],
        [
            r#"new {"feed_item_id":7}"#,
            r#"delete {"blog_id":123,"post_id":1}"#,
            r#"delete {"feed_item_id":7}"#
        ]
    );
    assert_eq!(status(&service, rss_identity(7)), NotSaved);
    assert_eq!(count(&service), Some(0));
}

// The observer

#[derive(Default)]
pub(super) struct CountingObserver {
    count: AtomicUsize,
}

impl CountingObserver {
    pub(super) fn count(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }
}

impl ReaderSavedObserver for CountingObserver {
    fn saved_posts_changed(&self) {
        self.count.fetch_add(1, Ordering::SeqCst);
    }
}

pub(super) fn observed(service: &ReaderSavedService) -> Arc<CountingObserver> {
    let observer = Arc::new(CountingObserver::default());
    service.set_observer(Some(observer.clone()));
    observer
}

#[tokio::test]
async fn the_observer_hears_about_a_sync() {
    let (service, executor) = service();
    let observer = observed(&service);

    synced(&service, &executor, &[]).await;

    assert_eq!(observer.count(), 1);
}

#[tokio::test]
async fn the_observer_does_not_hear_about_a_failed_sync() {
    let (service, executor) = service();
    let observer = observed(&service);
    executor.push(Step::Respond(500, error_response()));

    assert!(service.sync().await.is_err());

    assert_eq!(observer.count(), 0);
}

#[tokio::test]
async fn the_observer_hears_when_a_save_starts_and_finishes() {
    let (service, executor) = service();
    let observer = observed(&service);
    executor.push(Step::Respond(200, write_response(true, post_json(1), 1)));

    service.save(identity(1)).await.unwrap();

    assert_eq!(observer.count(), 2);
}

#[tokio::test]
async fn the_observer_hears_about_a_rollback() {
    let (service, executor) = service();
    let observer = observed(&service);
    executor.push(Step::Respond(400, error_response()));

    assert!(service.save(identity(1)).await.is_err());

    assert_eq!(observer.count(), 2);
}

/// A cancelled write changes nothing the apps show: the tap stands.
#[tokio::test]
async fn the_observer_hears_only_the_tap_of_a_cancelled_write() {
    let (service, executor) = service();
    let observer = observed(&service);
    let (_release, held) = oneshot::channel::<()>();
    executor.push(Step::Hold(held, 200, write_response(true, post_json(1), 1)));

    let mut save = Box::pin(service.save(identity(1)));
    assert!(futures::poll!(save.as_mut()).is_pending());
    drop(save);

    assert_eq!(observer.count(), 1);
}

#[tokio::test]
async fn the_observer_stops_hearing_once_removed() {
    let (service, executor) = service();
    let observer = observed(&service);
    service.set_observer(None);

    synced(&service, &executor, &[]).await;

    assert_eq!(observer.count(), 0);
}

/// Apps read the state back from inside the callback, which would deadlock if it ran with a lock
/// held.
pub(super) struct ReadingObserver {
    service: Mutex<Option<Arc<ReaderSavedService>>>,
    statuses: Mutex<Vec<ReaderSavedStatus>>,
}

impl ReaderSavedObserver for ReadingObserver {
    fn saved_posts_changed(&self) {
        let service = self.service.lock().unwrap().clone().unwrap();
        self.statuses
            .lock()
            .unwrap()
            .push(service.status(identity(1)).unwrap());
    }
}

#[tokio::test]
async fn the_observer_can_read_the_state_back() {
    let (service, executor) = service();
    let service = Arc::new(service);
    let observer = Arc::new(ReadingObserver {
        service: Mutex::new(Some(service.clone())),
        statuses: Mutex::default(),
    });
    service.set_observer(Some(observer.clone()));
    executor.push(Step::Respond(200, keys_response(&[])));
    executor.push(Step::Respond(200, write_response(true, post_json(1), 1)));
    executor.push(Step::Respond(200, keys_response(&[post_json(1)])));
    executor.push(Step::Respond(200, json!({ "posts": [listed_post(1)] })));

    service.sync().await.unwrap();
    service.save(identity(1)).await.unwrap();
    service.clone().saved_posts().refresh().await.unwrap();

    assert_eq!(
        *observer.statuses.lock().unwrap(),
        vec![NotSaved, Saved, Saved, Saved, Saved]
    );

    // Breaks the reference cycle between the service and its observer.
    service.set_observer(None);
}

// Whole flows

#[tokio::test]
async fn a_full_session_stays_consistent() {
    let (service, executor) = service();

    synced(&service, &executor, &[post_json(1), feed_json(2)]).await;
    assert_eq!(count(&service), Some(2));

    executor.push(Step::Respond(200, write_response(true, post_json(3), 3)));
    service.save(identity(3)).await.unwrap();

    executor.push(Step::Respond(200, write_response(false, feed_json(2), 2)));
    service.unsave(rss_identity(2)).await.unwrap();

    // The keys read after both writes.
    synced(&service, &executor, &[post_json(3), post_json(1)]).await;

    assert_eq!(status(&service, identity(1)), Saved);
    assert_eq!(status(&service, rss_identity(2)), NotSaved);
    assert_eq!(status(&service, identity(3)), Saved);
    assert_eq!(count(&service), Some(2));
    assert_eq!(executor.sent().len(), 4);
}

#[path = "scenario_tests.rs"]
mod scenarios;

// Which errors may have been applied, and which are refusals

pub(super) fn execution_failed(reason: RequestExecutionErrorReason) -> WpApiError {
    WpApiError::RequestExecutionFailed {
        status_code: None,
        redirects: None,
        reason,
        request_url: String::new(),
        request_method: RequestMethod::POST,
    }
}

pub(super) fn answered(status_code: u32) -> WpApiError {
    answered_with(status_code, "x")
}

pub(super) fn answered_with(status_code: u32, code: &str) -> WpApiError {
    WpApiError::WpError {
        error_code: WpErrorCode::CustomError(code.to_string()),
        error_message: String::new(),
        status_code,
        response: String::new(),
        request_url: String::new(),
        request_method: RequestMethod::POST,
    }
}

pub(super) fn unknown(status_code: u32) -> WpApiError {
    WpApiError::UnknownError {
        status_code,
        response: String::new(),
        request_url: String::new(),
        request_method: RequestMethod::POST,
    }
}

pub(super) fn cloudflare(status_code: u32) -> WpApiError {
    execution_failed(RequestExecutionErrorReason::CloudflareError {
        status_code,
        cloudflare_code: None,
        kind: wp_api::cloudflare::CloudflareErrorKind::OriginTimeout,
        ray_id: None,
        retry_after: None,
    })
}

#[rstest]
#[case::timeout(execution_failed(RequestExecutionErrorReason::HttpTimeoutError), true)]
#[case::lost_mid_request(execution_failed(RequestExecutionErrorReason::HttpError { reason: String::new() }), true)]
#[case::generic(execution_failed(RequestExecutionErrorReason::GenericError { error_message: String::new() }), true)]
#[case::cancelled(execution_failed(RequestExecutionErrorReason::CancellationError), true)]
#[case::unreadable_answer(WpApiError::ResponseParsingError { reason: String::new(), response: String::new(), request_url: String::new(), request_method: RequestMethod::POST }, true)]
#[case::server_error(answered(500), true)]
#[case::bad_gateway(unknown(502), true)]
#[case::gateway_timeout(unknown(504), true)]
#[case::invalid_status(WpApiError::InvalidHttpStatusCode { status_code: 503, request_url: String::new(), request_method: RequestMethod::POST }, true)]
#[case::cloudflare_timeout(cloudflare(524), true)]
#[case::cloudflare_block(cloudflare(403), false)]
#[case::offline(execution_failed(RequestExecutionErrorReason::DeviceIsOfflineError { error_message: String::new() }), false)]
#[case::no_such_host(execution_failed(RequestExecutionErrorReason::NonExistentSiteError { error_message: None, suggested_action: None }), false)]
#[case::refused_connection(execution_failed(RequestExecutionErrorReason::ConnectionError { reason: String::new() }), false)]
#[case::tls(execution_failed(RequestExecutionErrorReason::InvalidSslError { reason: InvalidSslErrorReason::GenericSslError }), false)]
#[case::forbidden(execution_failed(RequestExecutionErrorReason::HttpForbiddenError { hostname: String::new() }), false)]
#[case::rate_limited(
    execution_failed(RequestExecutionErrorReason::MisconfiguredRateLimitError),
    false
)]
#[case::bad_request(answered(400), false)]
#[case::not_found(unknown(404), false)]
fn a_failed_request_is_in_doubt_only_if_it_may_have_been_applied(
    #[case] error: WpApiError,
    #[case] in_doubt: bool,
) {
    assert_eq!(may_have_been_applied(&error), in_doubt);
}

#[rstest]
#[case::bad_request(answered(400), true)]
#[case::not_found(unknown(404), true)]
#[case::at_the_cap(answered_with(403, "saved_limit_reached"), true)]
#[case::forbidden(answered_with(403, "rest_forbidden"), false)]
#[case::unauthorized(answered(401), false)]
#[case::request_timeout(answered(408), false)]
#[case::rate_limited(answered(429), false)]
#[case::server_error(answered(500), false)]
#[case::offline(execution_failed(RequestExecutionErrorReason::DeviceIsOfflineError { error_message: String::new() }), false)]
fn only_an_answer_that_will_not_pass_is_a_refusal(
    #[case] error: WpApiError,
    #[case] refused: bool,
) {
    assert_eq!(is_refused(&error), refused);
}

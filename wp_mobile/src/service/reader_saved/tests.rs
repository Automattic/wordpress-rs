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
use wp_api::wp_com::reader_saved::{ReaderFeedId, ReaderFeedItemId};

const KEYS_URL: &str = "https://public-api.wordpress.com/wpcom/v2/read/saved/keys";
const NEW_URL: &str = "https://public-api.wordpress.com/wpcom/v2/read/saved/new";
const DELETE_URL: &str = "https://public-api.wordpress.com/wpcom/v2/read/saved/delete";
const BULK_URL: &str = "https://public-api.wordpress.com/wpcom/v2/read/saved/bulk";
const LIST_URL: &str = "https://public-api.wordpress.com/rest/v1.2/read/saved";

enum Step {
    Respond(u32, Value),
    /// Waits for the test to release it, so the test can act while the request is in flight.
    Hold(oneshot::Receiver<()>, u32, Value),
    /// The device is offline, so the request never left.
    Offline,
    /// The request timed out, so the server may have applied it.
    TimedOut,
}

#[derive(Debug, Clone)]
struct Sent {
    method: RequestMethod,
    url: String,
    body: Option<Value>,
}

/// Plays back a script of responses, and records every request.
#[derive(Debug, Default)]
struct ScriptedExecutor {
    steps: Mutex<VecDeque<StepBox>>,
    sent: Mutex<Vec<Sent>>,
}

struct StepBox(Step);

impl std::fmt::Debug for StepBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Step")
    }
}

impl ScriptedExecutor {
    fn push(&self, step: Step) {
        self.steps.lock().unwrap().push_back(StepBox(step));
    }

    fn sent(&self) -> Vec<Sent> {
        self.sent.lock().unwrap().clone()
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

fn service_with(source: ReaderSavedSource) -> (ReaderSavedService, Arc<ScriptedExecutor>) {
    let executor = Arc::new(ScriptedExecutor::default());
    let service = ReaderSavedService::new(
        WpApiClientDelegate {
            auth_provider: Arc::new(WpAuthenticationProvider::none()),
            request_executor: executor.clone(),
            middleware_pipeline: Arc::new(WpApiMiddlewarePipeline::default()),
            app_notifier: Arc::new(EmptyAppNotifier),
            language_provider: None,
        },
        source,
    );
    (service, executor)
}

fn service() -> (ReaderSavedService, Arc<ScriptedExecutor>) {
    service_with(ReaderSavedSource::Android)
}

fn identity(post_id: u64) -> ReaderPostSaveIdentity {
    ReaderPostSaveIdentity {
        id: post_id,
        site_id: Some(WpComSiteId(123)),
        feed_id: None,
        feed_item_id: None,
        is_external: false,
        url: None,
    }
}

fn jetpack_identity(post_id: u64, feed_item_id: u64) -> ReaderPostSaveIdentity {
    ReaderPostSaveIdentity {
        feed_id: Some(ReaderFeedId(5)),
        feed_item_id: Some(ReaderFeedItemId(feed_item_id)),
        ..identity(post_id)
    }
}

fn rss_identity(feed_item_id: u64) -> ReaderPostSaveIdentity {
    ReaderPostSaveIdentity {
        id: feed_item_id,
        site_id: Some(WpComSiteId(5)),
        feed_id: Some(ReaderFeedId(5)),
        feed_item_id: Some(ReaderFeedItemId(feed_item_id)),
        is_external: true,
        url: None,
    }
}

fn unsaveable_identity() -> ReaderPostSaveIdentity {
    ReaderPostSaveIdentity {
        site_id: None,
        ..identity(1)
    }
}

fn post_key(post_id: i64) -> ReaderSavedKey {
    ReaderSavedKey::Post {
        blog_id: WpComSiteId(123),
        post_id: PostId(post_id),
    }
}

fn feed_key(feed_item_id: u64) -> ReaderSavedKey {
    ReaderSavedKey::FeedItem {
        feed_item_id: ReaderFeedItemId(feed_item_id),
    }
}

fn bulk_post_key(post_id: i64) -> ReaderSavedBulkKey {
    ReaderSavedBulkKey::Post {
        blog_id: WpComSiteId(123),
        post_id: PostId(post_id),
    }
}

fn post_json(post_id: u64) -> Value {
    json!({ "blog_id": 123, "post_id": post_id })
}

fn feed_json(feed_item_id: u64) -> Value {
    json!({ "feed_item_id": feed_item_id })
}

fn keys_response(keys: &[Value]) -> Value {
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
fn write_response(saved: bool, key: Value, total: u64) -> Value {
    let resolved = if key.get("url").is_some() {
        "url"
    } else if key.get("feed_item_id").is_some() {
        "feed_item"
    } else {
        "post"
    };
    json!({ "saved": saved, "key": key, "total": total, "resolved": resolved })
}

fn listed_post(post_id: u64) -> Value {
    json!({ "ID": post_id, "site_ID": 123, "is_external": false, "title": format!("Post {post_id}") })
}

fn listed_rss_post(feed_item_id: u64) -> Value {
    json!({
        "ID": feed_item_id,
        "site_ID": 5,
        "feed_ID": 5,
        "feed_item_ID": feed_item_id,
        "is_external": true,
    })
}

fn error_response() -> Value {
    json!({ "code": "post_not_found", "message": "Post not found.", "data": { "status": 404 } })
}

async fn refreshed(service: &ReaderSavedService, executor: &ScriptedExecutor, keys: &[Value]) {
    executor.push(Step::Respond(200, keys_response(keys)));
    service.refresh().await.unwrap();
}

// Refreshing

#[tokio::test]
async fn a_post_is_unknown_before_a_refresh() {
    let (service, _) = service();
    assert_eq!(service.status(identity(1)), ReaderSavedStatus::Unknown);
    assert_eq!(service.saved_count(), None);
}

#[tokio::test]
async fn a_refresh_loads_the_keys() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[post_json(1), feed_json(2)]).await;

    assert_eq!(service.status(identity(1)), ReaderSavedStatus::Saved);
    assert_eq!(service.status(rss_identity(2)), ReaderSavedStatus::Saved);
    assert_eq!(service.status(identity(3)), ReaderSavedStatus::NotSaved);
    assert_eq!(service.saved_count(), Some(2));

    let sent = executor.sent();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].method, RequestMethod::GET);
    assert!(sent[0].url.starts_with(KEYS_URL), "{}", sent[0].url);
}

#[tokio::test]
async fn a_failed_refresh_reports_the_error_and_keeps_the_keys() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[post_json(1)]).await;

    executor.push(Step::Respond(500, error_response()));

    assert!(matches!(
        service.refresh().await,
        Err(ReaderSavedError::Api(_))
    ));
    assert_eq!(service.status(identity(1)), ReaderSavedStatus::Saved);
}

#[tokio::test]
async fn a_refresh_that_cannot_reach_the_server_reports_it() {
    let (service, executor) = service();
    executor.push(Step::Offline);

    assert!(matches!(
        service.refresh().await,
        Err(ReaderSavedError::Api(_))
    ));
    assert_eq!(service.status(identity(1)), ReaderSavedStatus::Unknown);
}

#[tokio::test]
async fn a_refresh_with_a_malformed_response_fails_rather_than_emptying_the_list() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[post_json(1)]).await;

    executor.push(Step::Respond(
        200,
        json!({ "keys": [{ "id": 1 }], "total": 1 }),
    ));

    assert!(service.refresh().await.is_err());
    assert_eq!(service.status(identity(1)), ReaderSavedStatus::Saved);
}

// Status

#[tokio::test]
async fn a_jetpack_post_is_saved_under_either_key() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[feed_json(2)]).await;

    assert_eq!(
        service.status(jetpack_identity(1, 2)),
        ReaderSavedStatus::Saved
    );
}

#[tokio::test]
async fn a_post_without_ids_is_not_saved() {
    let (service, _) = service();
    assert_eq!(
        service.status(unsaveable_identity()),
        ReaderSavedStatus::NotSaved
    );
}

#[tokio::test]
async fn status_does_not_touch_the_network() {
    let (service, executor) = service();
    service.status(identity(1));
    service.saved_count();
    service.recent_saves();
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

    let key = service.save(identity(1)).await.unwrap();

    assert_eq!(key, post_key(1));
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
    refreshed(&service, &executor, &[]).await;
    let (release, held) = oneshot::channel();
    executor.push(Step::Hold(held, 200, write_response(true, post_json(1), 1)));

    let (result, ()) = futures::join!(service.save(identity(1)), async {
        assert_eq!(service.status(identity(1)), ReaderSavedStatus::Saved);
        assert_eq!(service.saved_count(), Some(1));
        release.send(()).unwrap();
    });

    result.unwrap();
    assert_eq!(service.status(identity(1)), ReaderSavedStatus::Saved);
}

#[tokio::test]
async fn a_refused_save_is_undone_and_reported() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[]).await;
    executor.push(Step::Respond(
        403,
        json!({ "code": "saved_limit_reached", "message": "You can save up to 1000 posts." }),
    ));

    let result = service.save(identity(1)).await;

    let Err(ReaderSavedError::Api(error)) = result else {
        panic!("Expected an API error");
    };
    assert!(error.to_string().contains("1000"), "{error}");
    assert_eq!(service.status(identity(1)), ReaderSavedStatus::NotSaved);
    assert_eq!(service.saved_count(), Some(0));
}

#[tokio::test]
async fn a_save_that_cannot_reach_the_server_is_undone() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[]).await;
    executor.push(Step::Offline);

    assert!(service.save(identity(1)).await.is_err());

    assert_eq!(service.status(identity(1)), ReaderSavedStatus::NotSaved);
    assert!(service.recent_saves().is_empty());
}

/// The server answered, so it may have applied the save: it stands until a read can say.
#[tokio::test]
async fn a_save_whose_answer_cannot_be_read_stands_until_a_read() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[]).await;
    executor.push(Step::Respond(200, json!({ "unexpected": true })));

    assert!(service.save(identity(1)).await.is_err());

    assert_eq!(service.status(identity(1)), ReaderSavedStatus::Saved);
}

#[tokio::test]
async fn a_save_that_times_out_stands_until_a_read() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[]).await;
    executor.push(Step::TimedOut);

    assert!(service.save(identity(1)).await.is_err());

    assert_eq!(service.status(identity(1)), ReaderSavedStatus::Saved);
    refreshed(&service, &executor, &[]).await;
    assert_eq!(
        service.status(identity(1)),
        ReaderSavedStatus::Saved,
        "a refresh straight away may predate it"
    );
}

#[tokio::test]
async fn a_save_stored_under_another_key_shows_under_that_key() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[]).await;
    executor.push(Step::Respond(200, write_response(true, post_json(1), 1)));

    let key = service.save(rss_identity(7)).await.unwrap();

    assert_eq!(key, post_key(1));
    assert_eq!(service.status(identity(1)), ReaderSavedStatus::Saved);
    assert_eq!(service.recent_saves(), vec![post_key(1)]);
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

#[tokio::test]
async fn a_url_of_a_wp_com_post_returns_the_post_key() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[]).await;
    executor.push(Step::Respond(200, write_response(true, post_json(1), 1)));

    let key = service
        .save_url("https://example.com/post".to_string(), None)
        .await
        .unwrap();

    assert_eq!(key, post_key(1));
    assert_eq!(service.status(identity(1)), ReaderSavedStatus::Saved);
}

#[tokio::test]
async fn a_saved_url_can_be_unsaved_with_another_spelling() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[]).await;
    let page = json!({ "url": "https://example.com/page" });
    executor.push(Step::Respond(200, write_response(true, page.clone(), 1)));
    executor.push(Step::Respond(200, write_response(false, page, 0)));

    let key = service
        .save_url("https://example.com/page/?utm_source=x".to_string(), None)
        .await
        .unwrap();
    assert_eq!(service.saved_count(), Some(1));

    service.unsave_key(key).await.unwrap();

    assert_eq!(service.saved_count(), Some(0));
    assert!(service.recent_saves().is_empty());
}

// Unsaving

#[tokio::test]
async fn unsave_sends_the_key_and_source() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[post_json(1)]).await;
    executor.push(Step::Respond(200, write_response(false, post_json(1), 0)));

    service.unsave(identity(1)).await.unwrap();

    let sent = executor.sent();
    let request = sent.last().unwrap();
    assert!(request.url.starts_with(DELETE_URL), "{}", request.url);
    assert_eq!(request.method, RequestMethod::POST);
    assert_eq!(
        request.body,
        Some(json!({ "blog_id": 123, "post_id": 1, "source": "android" }))
    );
    assert_eq!(service.status(identity(1)), ReaderSavedStatus::NotSaved);
}

#[tokio::test]
async fn unsave_sends_every_key_the_post_may_be_saved_under() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[feed_json(2)]).await;
    // The post isn't saved under its blog and post IDs: deleting those does nothing.
    executor.push(Step::Respond(200, write_response(false, post_json(1), 1)));
    executor.push(Step::Respond(200, write_response(false, feed_json(2), 0)));

    service.unsave(jetpack_identity(1, 2)).await.unwrap();

    let bodies: Vec<_> = executor.sent()[1..]
        .iter()
        .map(|sent| sent.body.clone())
        .collect();
    assert_eq!(
        bodies,
        vec![
            Some(json!({ "blog_id": 123, "post_id": 1, "source": "android" })),
            Some(json!({ "feed_item_id": 2, "source": "android" })),
        ]
    );
    assert_eq!(
        service.status(jetpack_identity(1, 2)),
        ReaderSavedStatus::NotSaved
    );
    assert_eq!(service.saved_count(), Some(0));
}

#[tokio::test]
async fn unsave_removes_every_key_that_matched() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[post_json(1), feed_json(2)]).await;
    executor.push(Step::Respond(200, write_response(false, post_json(1), 1)));
    executor.push(Step::Respond(200, write_response(false, feed_json(2), 0)));

    service.unsave(jetpack_identity(1, 2)).await.unwrap();

    assert_eq!(executor.sent().len(), 3);
    assert_eq!(
        service.status(jetpack_identity(1, 2)),
        ReaderSavedStatus::NotSaved
    );
    assert_eq!(service.saved_count(), Some(0));
}

/// Before the first refresh nothing says which key the post is saved under, and missing one
/// leaves it saved.
#[tokio::test]
async fn unsave_before_a_refresh_sends_every_key() {
    let (service, executor) = service();
    executor.push(Step::Respond(200, write_response(false, post_json(1), 0)));
    executor.push(Step::Respond(200, write_response(false, feed_json(2), 0)));

    service.unsave(jetpack_identity(1, 2)).await.unwrap();

    let bodies: Vec<_> = executor
        .sent()
        .iter()
        .map(|sent| sent.body.clone())
        .collect();
    assert_eq!(
        bodies,
        vec![
            Some(json!({ "blog_id": 123, "post_id": 1, "source": "android" })),
            Some(json!({ "feed_item_id": 2, "source": "android" })),
        ]
    );
    assert_eq!(
        service.status(jetpack_identity(1, 2)),
        ReaderSavedStatus::NotSaved
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
    refreshed(&service, &executor, &[post_json(1)]).await;
    let (release, held) = oneshot::channel();
    executor.push(Step::Hold(
        held,
        200,
        write_response(false, post_json(1), 0),
    ));

    let (result, ()) = futures::join!(service.unsave(identity(1)), async {
        assert_eq!(service.status(identity(1)), ReaderSavedStatus::NotSaved);
        assert_eq!(service.saved_count(), Some(0));
        release.send(()).unwrap();
    });

    result.unwrap();
}

#[tokio::test]
async fn unsaving_a_post_under_two_keys_shows_straight_away() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[post_json(1), feed_json(2)]).await;
    let (release, held) = oneshot::channel();
    executor.push(Step::Hold(
        held,
        200,
        write_response(false, post_json(1), 1),
    ));
    executor.push(Step::Respond(200, write_response(false, feed_json(2), 0)));

    let (result, ()) = futures::join!(service.unsave(jetpack_identity(1, 2)), async {
        assert_eq!(
            service.status(jetpack_identity(1, 2)),
            ReaderSavedStatus::NotSaved
        );
        assert_eq!(service.saved_count(), Some(0));
        release.send(()).unwrap();
    });

    result.unwrap();
}

// Cancellation: Kotlin cancels a call by dropping its future. Swift doesn't.

#[tokio::test]
async fn a_save_cancelled_before_it_is_sent_is_undone() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[]).await;
    let (_release, held) = oneshot::channel::<()>();
    executor.push(Step::Hold(held, 200, write_response(true, post_json(2), 1)));

    let mut earlier = Box::pin(service.save(identity(2)));
    let mut save = Box::pin(service.save(identity(1)));
    assert!(futures::poll!(earlier.as_mut()).is_pending());
    assert!(futures::poll!(save.as_mut()).is_pending());
    assert_eq!(service.status(identity(1)), ReaderSavedStatus::Saved);

    drop(save);

    assert_eq!(executor.sent().len(), 2, "only the earlier save was sent");
    assert_eq!(service.status(identity(1)), ReaderSavedStatus::NotSaved);
    assert_eq!(service.recent_saves(), vec![post_key(2)]);
}

/// Its request went out, so the server may have applied it.
#[tokio::test]
async fn a_save_cancelled_after_it_is_sent_stands_until_a_read() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[]).await;
    let (_release, held) = oneshot::channel::<()>();
    executor.push(Step::Hold(held, 200, write_response(true, post_json(1), 1)));

    let mut save = Box::pin(service.save(identity(1)));
    assert!(futures::poll!(save.as_mut()).is_pending());
    drop(save);

    assert_eq!(service.status(identity(1)), ReaderSavedStatus::Saved);
    assert_eq!(service.saved_count(), Some(1));
}

#[tokio::test]
async fn a_cancelled_unsave_is_undone_for_the_keys_not_yet_sent() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[post_json(1), feed_json(2)]).await;
    let (_release, held) = oneshot::channel::<()>();
    executor.push(Step::Hold(
        held,
        200,
        write_response(false, post_json(1), 1),
    ));

    let mut unsave = Box::pin(service.unsave(jetpack_identity(1, 2)));
    assert!(futures::poll!(unsave.as_mut()).is_pending());
    assert_eq!(
        service.status(jetpack_identity(1, 2)),
        ReaderSavedStatus::NotSaved
    );

    drop(unsave);

    // The post key's delete went out and stands; the feed item's never did.
    assert_eq!(service.status(identity(1)), ReaderSavedStatus::NotSaved);
    assert_eq!(service.status(rss_identity(2)), ReaderSavedStatus::Saved);
    assert_eq!(service.saved_count(), Some(1));
}

#[tokio::test]
async fn a_cancelled_import_stands_until_a_read() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[]).await;
    let (_release, held) = oneshot::channel::<()>();
    executor.push(Step::Hold(held, 200, bulk_response(&[post_json(1)], 1)));

    let mut import = Box::pin(service.import(vec![bulk_post_key(1)]));
    assert!(futures::poll!(import.as_mut()).is_pending());
    assert_eq!(service.status(identity(1)), ReaderSavedStatus::Saved);

    drop(import);

    assert_eq!(service.status(identity(1)), ReaderSavedStatus::Saved);
}

/// A feed item stops resolving once it's pruned, so the server can't find a save the device knows
/// is there.
#[tokio::test]
async fn a_feed_item_the_server_cannot_find_is_an_error_when_it_was_saved_there() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[feed_json(2)]).await;
    executor.push(Step::Respond(404, error_response()));

    assert!(service.unsave(rss_identity(2)).await.is_err());

    assert_eq!(service.status(rss_identity(2)), ReaderSavedStatus::Saved);
}

#[tokio::test]
async fn a_feed_item_the_server_cannot_find_counts_as_unsaved_when_it_was_not_saved_there() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[]).await;
    executor.push(Step::Respond(404, error_response()));

    service.unsave(rss_identity(2)).await.unwrap();

    assert_eq!(service.status(rss_identity(2)), ReaderSavedStatus::NotSaved);
}

#[tokio::test]
async fn a_failed_unsave_is_undone_and_reported() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[post_json(1)]).await;
    executor.push(Step::Respond(400, error_response()));

    assert!(service.unsave(identity(1)).await.is_err());

    assert_eq!(service.status(identity(1)), ReaderSavedStatus::Saved);
    assert_eq!(service.saved_count(), Some(1));
}

#[tokio::test]
async fn a_partly_failed_unsave_tries_every_key_and_keeps_the_one_that_failed() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[post_json(1), feed_json(2)]).await;
    executor.push(Step::Respond(400, error_response()));
    executor.push(Step::Respond(200, write_response(false, feed_json(2), 1)));

    assert!(service.unsave(jetpack_identity(1, 2)).await.is_err());

    assert_eq!(executor.sent().len(), 3);
    assert_eq!(
        service.status(jetpack_identity(1, 2)),
        ReaderSavedStatus::Saved
    );
    assert_eq!(service.status(rss_identity(2)), ReaderSavedStatus::NotSaved);
    assert_eq!(service.saved_count(), Some(1));
}

// Stale reads, through the whole service

#[tokio::test]
async fn a_stale_refresh_cannot_bring_back_an_unsaved_post() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[post_json(1)]).await;
    executor.push(Step::Respond(200, write_response(false, post_json(1), 0)));
    service.unsave(identity(1)).await.unwrap();

    refreshed(&service, &executor, &[post_json(1)]).await;

    assert_eq!(service.status(identity(1)), ReaderSavedStatus::NotSaved);
    assert_eq!(service.saved_count(), Some(0));
}

#[tokio::test]
async fn a_stale_refresh_cannot_remove_a_saved_post() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[]).await;
    executor.push(Step::Respond(200, write_response(true, post_json(1), 1)));
    service.save(identity(1)).await.unwrap();

    refreshed(&service, &executor, &[]).await;

    assert_eq!(service.status(identity(1)), ReaderSavedStatus::Saved);
    assert_eq!(service.saved_count(), Some(1));
}

#[tokio::test]
async fn a_refresh_landing_during_a_slow_unsave_cannot_undo_it() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[post_json(1)]).await;
    let (release, held) = oneshot::channel();
    executor.push(Step::Hold(
        held,
        200,
        write_response(false, post_json(1), 0),
    ));
    executor.push(Step::Respond(200, keys_response(&[post_json(1)])));

    let (result, ()) = futures::join!(service.unsave(identity(1)), async {
        service.refresh().await.unwrap();
        assert_eq!(service.status(identity(1)), ReaderSavedStatus::NotSaved);
        release.send(()).unwrap();
    });

    result.unwrap();
    assert_eq!(service.status(identity(1)), ReaderSavedStatus::NotSaved);
}

#[tokio::test]
async fn a_refresh_started_before_a_write_cannot_undo_it_when_it_lands() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[post_json(1)]).await;
    let (release, held) = oneshot::channel();
    executor.push(Step::Hold(held, 200, keys_response(&[post_json(1)])));
    executor.push(Step::Respond(200, write_response(false, post_json(1), 0)));

    let (refresh, ()) = futures::join!(service.refresh(), async {
        service.unsave(identity(1)).await.unwrap();
        release.send(()).unwrap();
    });

    refresh.unwrap();
    assert_eq!(service.status(identity(1)), ReaderSavedStatus::NotSaved);
}

#[tokio::test]
async fn a_slow_older_refresh_cannot_replace_a_newer_one() {
    let (service, executor) = service();
    let (release, held) = oneshot::channel();
    executor.push(Step::Hold(held, 200, keys_response(&[post_json(1)])));
    executor.push(Step::Respond(200, keys_response(&[post_json(2)])));

    let (older, ()) = futures::join!(service.refresh(), async {
        service.refresh().await.unwrap();
        release.send(()).unwrap();
    });

    older.unwrap();
    assert_eq!(service.status(identity(1)), ReaderSavedStatus::NotSaved);
    assert_eq!(service.status(identity(2)), ReaderSavedStatus::Saved);
}

/// The server applies a post's writes in the order it receives them, so the unsave isn't sent
/// until the save has been answered.
#[tokio::test]
async fn a_write_waits_for_the_posts_earlier_write_to_finish() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[]).await;
    let (release, held) = oneshot::channel();
    executor.push(Step::Hold(held, 200, write_response(true, post_json(1), 1)));
    executor.push(Step::Respond(200, write_response(false, post_json(1), 0)));

    let mut save = Box::pin(service.save(identity(1)));
    let mut unsave = Box::pin(service.unsave(identity(1)));
    assert!(futures::poll!(save.as_mut()).is_pending());
    assert!(futures::poll!(unsave.as_mut()).is_pending());
    assert_eq!(executor.sent().len(), 2, "only the save has been sent");
    assert_eq!(service.status(identity(1)), ReaderSavedStatus::NotSaved);

    release.send(()).unwrap();
    let (save, unsave) = futures::join!(save, unsave);
    save.unwrap();
    unsave.unwrap();

    let sent = executor.sent();
    assert!(sent[1].url.starts_with(NEW_URL), "{}", sent[1].url);
    assert!(sent[2].url.starts_with(DELETE_URL), "{}", sent[2].url);
    assert_eq!(service.status(identity(1)), ReaderSavedStatus::NotSaved);
    assert_eq!(service.saved_count(), Some(0));
    assert!(service.recent_saves().is_empty());
}

#[tokio::test]
async fn a_cancelled_write_lets_the_posts_next_write_go() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[]).await;
    let (_release, held) = oneshot::channel::<()>();
    executor.push(Step::Hold(held, 200, write_response(true, post_json(1), 1)));
    executor.push(Step::Respond(200, write_response(false, post_json(1), 0)));

    let mut save = Box::pin(service.save(identity(1)));
    let mut unsave = Box::pin(service.unsave(identity(1)));
    assert!(futures::poll!(save.as_mut()).is_pending());
    assert!(futures::poll!(unsave.as_mut()).is_pending());

    drop(save);
    unsave.await.unwrap();

    assert_eq!(executor.sent().len(), 3);
    assert_eq!(service.status(identity(1)), ReaderSavedStatus::NotSaved);
}

/// A save the server stores under another key, with an unsave queued behind it: the unsave also
/// sends the key the save was stored under.
#[tokio::test]
async fn a_queued_unsave_also_unsaves_the_key_the_save_was_stored_under() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[]).await;
    let (release, held) = oneshot::channel();
    executor.push(Step::Hold(held, 200, write_response(true, post_json(1), 1)));
    // The feed item resolves to the post, which the delete removes; the post's own delete then
    // finds nothing.
    executor.push(Step::Respond(200, write_response(false, post_json(1), 0)));
    executor.push(Step::Respond(200, write_response(false, post_json(1), 0)));

    let mut save = Box::pin(service.save(rss_identity(7)));
    let mut unsave = Box::pin(service.unsave(rss_identity(7)));
    assert!(futures::poll!(save.as_mut()).is_pending());
    assert!(futures::poll!(unsave.as_mut()).is_pending());
    assert_eq!(service.status(rss_identity(7)), ReaderSavedStatus::NotSaved);

    release.send(()).unwrap();
    let (save, unsave) = futures::join!(save, unsave);
    save.unwrap();
    unsave.unwrap();

    let bodies: Vec<_> = executor.sent()[2..]
        .iter()
        .map(|sent| sent.body.clone())
        .collect();
    assert_eq!(
        bodies,
        vec![
            Some(json!({ "feed_item_id": 7, "source": "android" })),
            Some(json!({ "blog_id": 123, "post_id": 1, "source": "android" })),
        ]
    );
    assert_eq!(service.status(rss_identity(7)), ReaderSavedStatus::NotSaved);
    assert_eq!(service.status(identity(1)), ReaderSavedStatus::NotSaved);
    assert_eq!(service.saved_count(), Some(0));
    assert!(service.recent_saves().is_empty());
}

#[tokio::test]
async fn unsave_key_also_unsaves_the_key_the_server_stores_the_post_under() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[]).await;
    executor.push(Step::Respond(200, write_response(true, post_json(1), 1)));
    // The feed item resolves to the post, which the delete removes; the post's own delete then
    // finds nothing.
    executor.push(Step::Respond(200, write_response(false, post_json(1), 0)));
    executor.push(Step::Respond(200, write_response(false, post_json(1), 0)));
    service.save(rss_identity(7)).await.unwrap();

    service.unsave_key(feed_key(7)).await.unwrap();

    let bodies: Vec<_> = executor.sent()[2..]
        .iter()
        .map(|sent| sent.body.clone())
        .collect();
    assert_eq!(
        bodies,
        vec![
            Some(json!({ "feed_item_id": 7, "source": "android" })),
            Some(json!({ "blog_id": 123, "post_id": 1, "source": "android" })),
        ]
    );
    assert_eq!(service.status(rss_identity(7)), ReaderSavedStatus::NotSaved);
    assert_eq!(service.saved_count(), Some(0));
}

// The saved list

#[tokio::test]
async fn list_posts_sends_the_params() {
    let (service, executor) = service();
    executor.push(Step::Respond(200, json!({ "posts": [] })));

    service
        .list_posts(ReaderSavedPostsParams {
            page_handle: Some("abc".to_string()),
            number: Some(20),
            ..Default::default()
        })
        .await
        .unwrap();

    let sent = executor.sent();
    assert_eq!(sent[0].method, RequestMethod::GET);
    assert!(sent[0].url.starts_with(LIST_URL), "{}", sent[0].url);
    assert!(sent[0].url.contains("page_handle=abc"), "{}", sent[0].url);
    assert!(sent[0].url.contains("number=20"), "{}", sent[0].url);
}

#[tokio::test]
async fn list_posts_returns_the_page() {
    let (service, executor) = service();
    executor.push(Step::Respond(
        200,
        json!({ "posts": [listed_post(1)], "found": 7, "next_page_handle": "next" }),
    ));

    let page = service
        .list_posts(ReaderSavedPostsParams::default())
        .await
        .unwrap();

    assert_eq!(page.posts.len(), 1);
    assert_eq!(page.found, Some(7));
    assert_eq!(page.next_page_handle.as_deref(), Some("next"));
}

#[tokio::test]
async fn list_posts_hides_a_post_just_unsaved() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[post_json(1), post_json(2)]).await;
    executor.push(Step::Respond(200, write_response(false, post_json(1), 1)));
    service.unsave(identity(1)).await.unwrap();
    executor.push(Step::Respond(
        200,
        json!({ "posts": [listed_post(1), listed_post(2)], "next_page_handle": "next" }),
    ));

    let page = service
        .list_posts(ReaderSavedPostsParams::default())
        .await
        .unwrap();

    assert_eq!(page.posts.len(), 1);
    assert_eq!(page.posts[0].identity.id, 2);
    assert_eq!(page.next_page_handle.as_deref(), Some("next"));
}

#[tokio::test]
async fn listing_a_hidden_post_does_not_mark_it_saved() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[post_json(1)]).await;
    executor.push(Step::Respond(200, write_response(false, post_json(1), 0)));
    service.unsave(identity(1)).await.unwrap();
    executor.push(Step::Respond(200, json!({ "posts": [listed_post(1)] })));

    service
        .list_posts(ReaderSavedPostsParams::default())
        .await
        .unwrap();

    assert_eq!(service.status(identity(1)), ReaderSavedStatus::NotSaved);
}

#[tokio::test]
async fn the_saved_list_does_not_change_a_post_s_status() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[]).await;
    executor.push(Step::Respond(
        200,
        json!({ "posts": [listed_post(1), listed_rss_post(2)] }),
    ));

    service
        .list_posts(ReaderSavedPostsParams::default())
        .await
        .unwrap();

    // It doesn't say which key the post is saved under, so the keys decide.
    assert_eq!(service.status(identity(1)), ReaderSavedStatus::NotSaved);
    assert_eq!(service.status(rss_identity(2)), ReaderSavedStatus::NotSaved);
}

#[tokio::test]
async fn a_short_page_can_still_have_a_handle() {
    let (service, executor) = service();
    executor.push(Step::Respond(
        200,
        json!({ "posts": [], "next_page_handle": "next" }),
    ));

    let page = service
        .list_posts(ReaderSavedPostsParams::default())
        .await
        .unwrap();

    assert!(page.posts.is_empty());
    assert_eq!(page.next_page_handle.as_deref(), Some("next"));
}

#[tokio::test]
async fn a_failed_list_reports_the_error() {
    let (service, executor) = service();
    executor.push(Step::Respond(
        400,
        json!({ "error": "invalid_page_handle", "message": "The page_handle is not valid." }),
    ));

    let result = service.list_posts(ReaderSavedPostsParams::default()).await;

    assert!(matches!(result, Err(ReaderSavedError::Api(_))));
}

// Importing

fn bulk_response(keys: &[Value], total: u64) -> Value {
    let results: Vec<_> = keys
        .iter()
        .map(|key| json!({ "key": key, "saved": true }))
        .collect();
    json!({ "results": results, "total": total })
}

#[tokio::test]
async fn importing_nothing_sends_nothing() {
    let (service, executor) = service();

    assert!(service.import(vec![]).await.unwrap().is_empty());

    assert!(executor.sent().is_empty());
}

#[tokio::test]
async fn import_sends_the_keys_and_source() {
    let (service, executor) = service_with(ReaderSavedSource::Ios);
    executor.push(Step::Respond(200, bulk_response(&[post_json(1)], 1)));

    service.import(vec![bulk_post_key(1)]).await.unwrap();

    let sent = executor.sent();
    assert_eq!(sent[0].method, RequestMethod::POST);
    assert!(sent[0].url.starts_with(BULK_URL), "{}", sent[0].url);
    assert_eq!(
        sent[0].body,
        Some(json!({ "keys": [{ "blog_id": 123, "post_id": 1 }], "source": "ios" }))
    );
}

#[rstest]
#[case::one_full_batch(100, vec![100])]
#[case::one_over(101, vec![1, 100])]
#[case::two_full_batches(200, vec![100, 100])]
#[case::many(250, vec![50, 100, 100])]
#[tokio::test]
async fn import_splits_keys_into_batches_oldest_first(
    #[case] count: u64,
    #[case] batch_sizes: Vec<usize>,
) {
    let (service, executor) = service();
    let keys: Vec<_> = (1..=count).map(|id| bulk_post_key(id as i64)).collect();

    // Newest first, so the last keys go first.
    let mut remaining = count;
    for size in &batch_sizes {
        let first = remaining - *size as u64 + 1;
        let batch: Vec<_> = (first..=remaining).map(post_json).collect();
        executor.push(Step::Respond(200, bulk_response(&batch, count)));
        remaining -= *size as u64;
    }

    let results = service.import(keys).await.unwrap();

    let sent = executor.sent();
    let sizes: Vec<_> = sent
        .iter()
        .map(|request| {
            request.body.as_ref().unwrap()["keys"]
                .as_array()
                .unwrap()
                .len()
        })
        .collect();
    assert_eq!(sizes, batch_sizes);

    // The oldest keys go first, so the newest end up at the top of the list.
    assert_eq!(
        sent[0].body.as_ref().unwrap()["keys"]
            .as_array()
            .unwrap()
            .last(),
        Some(&post_json(count))
    );

    // Results come back in the order the keys were given.
    let expected: Vec<_> = (1..=count)
        .map(|id| Some(bulk_post_key(id as i64)))
        .collect();
    let keys: Vec<_> = results.into_iter().map(|result| result.key).collect();
    assert_eq!(keys, expected);
}

/// The device already had them, so they don't need showing above the list.
#[tokio::test]
async fn imported_posts_are_not_recent_saves() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[]).await;
    executor.push(Step::Respond(
        200,
        bulk_response(&[post_json(1), post_json(2), post_json(3)], 3),
    ));

    service
        .import(vec![bulk_post_key(1), bulk_post_key(2), bulk_post_key(3)])
        .await
        .unwrap();

    assert!(service.recent_saves().is_empty());
    assert_eq!(service.status(identity(1)), ReaderSavedStatus::Saved);
}

#[tokio::test]
async fn imported_posts_show_as_saved() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[]).await;
    executor.push(Step::Respond(
        200,
        bulk_response(&[post_json(1), post_json(2)], 2),
    ));

    service
        .import(vec![bulk_post_key(1), bulk_post_key(2)])
        .await
        .unwrap();

    assert_eq!(service.status(identity(1)), ReaderSavedStatus::Saved);
    assert_eq!(service.status(identity(2)), ReaderSavedStatus::Saved);
    assert_eq!(service.saved_count(), Some(2));
}

#[tokio::test]
async fn imported_posts_show_as_saved_while_the_request_is_in_flight() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[]).await;
    let (release, held) = oneshot::channel();
    executor.push(Step::Hold(held, 200, bulk_response(&[post_json(1)], 1)));

    let (result, ()) = futures::join!(service.import(vec![bulk_post_key(1)]), async {
        assert_eq!(service.status(identity(1)), ReaderSavedStatus::Saved);
        release.send(()).unwrap();
    });

    result.unwrap();
}

#[tokio::test]
async fn a_stale_refresh_after_an_import_cannot_remove_the_posts() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[]).await;
    executor.push(Step::Respond(200, bulk_response(&[post_json(1)], 1)));
    service.import(vec![bulk_post_key(1)]).await.unwrap();

    refreshed(&service, &executor, &[]).await;

    assert_eq!(service.status(identity(1)), ReaderSavedStatus::Saved);
}

#[tokio::test]
async fn a_rejected_key_is_a_result_and_is_undone() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[]).await;
    executor.push(Step::Respond(
        200,
        json!({
            "results": [
                { "key": post_json(1), "saved": true },
                { "key": post_json(2), "saved": false, "error": "saved_limit_reached" },
                { "key": { "post_id": 5 }, "saved": false, "error": "invalid_input" },
            ],
            "total": 1,
        }),
    ));

    let results = service
        .import(vec![
            bulk_post_key(1),
            bulk_post_key(2),
            ReaderSavedBulkKey::Post {
                blog_id: WpComSiteId(0),
                post_id: PostId(5),
            },
        ])
        .await
        .unwrap();

    assert_eq!(results.len(), 3);
    assert_eq!(results[1].error.as_deref(), Some("saved_limit_reached"));
    assert_eq!(results[2].key, None);
    assert_eq!(service.status(identity(1)), ReaderSavedStatus::Saved);
    assert_eq!(service.status(identity(2)), ReaderSavedStatus::NotSaved);
    assert_eq!(service.saved_count(), Some(1));
}

#[tokio::test]
async fn an_imported_feed_item_can_be_stored_as_a_post() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[]).await;
    executor.push(Step::Respond(200, bulk_response(&[post_json(1)], 1)));

    service
        .import(vec![ReaderSavedBulkKey::FeedItem {
            feed_item_id: ReaderFeedItemId(7),
        }])
        .await
        .unwrap();

    // The feed item the post was saved by still reads as saved, under the key the server stored.
    assert_eq!(service.status(identity(1)), ReaderSavedStatus::Saved);
    assert_eq!(service.status(rss_identity(7)), ReaderSavedStatus::Saved);
    assert_eq!(service.saved_count(), Some(1));
}

#[tokio::test]
async fn a_failed_batch_stops_the_import_and_undoes_its_keys() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[]).await;
    let keys: Vec<_> = (1..=150).map(bulk_post_key).collect();
    let first_batch: Vec<_> = (101..=150).map(post_json).collect();
    executor.push(Step::Respond(200, bulk_response(&first_batch, 50)));
    executor.push(Step::Respond(400, error_response()));

    let result = service.import(keys).await;

    assert!(matches!(result, Err(ReaderSavedError::Api(_))));
    assert_eq!(executor.sent().len(), 3);
    assert_eq!(service.status(identity(150)), ReaderSavedStatus::Saved);
    assert_eq!(service.status(identity(101)), ReaderSavedStatus::Saved);
    assert_eq!(service.status(identity(100)), ReaderSavedStatus::NotSaved);
    assert_eq!(service.status(identity(1)), ReaderSavedStatus::NotSaved);
    assert_eq!(service.saved_count(), Some(50));
}

#[tokio::test]
async fn a_failed_first_batch_saves_nothing() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[]).await;
    executor.push(Step::Offline);

    assert!(service.import(vec![bulk_post_key(1)]).await.is_err());

    assert_eq!(service.status(identity(1)), ReaderSavedStatus::NotSaved);
    assert!(service.recent_saves().is_empty());
}

/// Results are matched to keys by position, so with one left out from the middle, every result
/// after it would land on the wrong key.
#[tokio::test]
async fn a_result_missing_from_the_middle_fails_the_batch_and_matches_nothing() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[]).await;
    // Sent newest first, as 3, 2, 1. The server leaves out 2's result.
    executor.push(Step::Respond(
        200,
        bulk_response(&[post_json(3), post_json(1)], 2),
    ));

    let result = service
        .import(vec![bulk_post_key(3), bulk_post_key(2), bulk_post_key(1)])
        .await;

    assert!(matches!(
        result,
        Err(ReaderSavedError::UnexpectedImportResults {
            sent: 3,
            received: 2
        })
    ));
    // The server may have saved any of them, so they stand until a read can say.
    for post in 1..=3 {
        assert_eq!(service.status(identity(post)), ReaderSavedStatus::Saved);
    }

    // Post 2 isn't taken to be stored as post 1, so unsaving it leaves post 1 alone.
    executor.push(Step::Respond(200, write_response(false, post_json(2), 2)));
    service.unsave(identity(2)).await.unwrap();
    let unsaves: Vec<_> = executor.sent()[2..]
        .iter()
        .map(|sent| sent.body.clone())
        .collect();
    assert_eq!(
        unsaves,
        vec![Some(
            json!({ "blog_id": 123, "post_id": 2, "source": "android" })
        )]
    );
    assert_eq!(service.status(identity(1)), ReaderSavedStatus::Saved);
}

#[tokio::test]
async fn a_result_missing_from_the_end_fails_the_batch_and_stops_the_import() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[]).await;
    let keys: Vec<_> = (1..=150).map(bulk_post_key).collect();
    // The newest batch, 101 to 150, goes first and comes back without 150's result.
    let short: Vec<_> = (101..=149).map(post_json).collect();
    executor.push(Step::Respond(200, bulk_response(&short, 49)));

    let result = service.import(keys).await;

    assert!(matches!(
        result,
        Err(ReaderSavedError::UnexpectedImportResults {
            sent: 50,
            received: 49
        })
    ));
    assert_eq!(executor.sent().len(), 2, "the older batch isn't sent");
    assert_eq!(service.status(identity(150)), ReaderSavedStatus::Saved);
    assert_eq!(service.status(identity(101)), ReaderSavedStatus::Saved);
    assert_eq!(service.status(identity(100)), ReaderSavedStatus::NotSaved);
}

#[tokio::test]
async fn too_many_results_fail_the_batch() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[]).await;
    executor.push(Step::Respond(
        200,
        bulk_response(&[post_json(1), post_json(2)], 2),
    ));

    let result = service.import(vec![bulk_post_key(1)]).await;

    assert!(matches!(
        result,
        Err(ReaderSavedError::UnexpectedImportResults {
            sent: 1,
            received: 2
        })
    ));
    assert_eq!(service.status(identity(1)), ReaderSavedStatus::Saved);
    assert_eq!(service.status(identity(2)), ReaderSavedStatus::NotSaved);
}

// The observer

#[derive(Default)]
struct CountingObserver {
    count: AtomicUsize,
}

impl ReaderSavedObserver for CountingObserver {
    fn saved_posts_changed(&self) {
        self.count.fetch_add(1, Ordering::SeqCst);
    }
}

fn observed(service: &ReaderSavedService) -> Arc<CountingObserver> {
    let observer = Arc::new(CountingObserver::default());
    service.set_observer(Some(observer.clone()));
    observer
}

#[tokio::test]
async fn the_observer_hears_about_a_refresh() {
    let (service, executor) = service();
    let observer = observed(&service);

    refreshed(&service, &executor, &[]).await;

    assert_eq!(observer.count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn the_observer_does_not_hear_about_a_failed_refresh() {
    let (service, executor) = service();
    let observer = observed(&service);
    executor.push(Step::Respond(500, error_response()));

    assert!(service.refresh().await.is_err());

    assert_eq!(observer.count.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn the_observer_hears_when_a_save_starts_and_finishes() {
    let (service, executor) = service();
    let observer = observed(&service);
    executor.push(Step::Respond(200, write_response(true, post_json(1), 1)));

    service.save(identity(1)).await.unwrap();

    assert_eq!(observer.count.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn the_observer_hears_about_a_rollback() {
    let (service, executor) = service();
    let observer = observed(&service);
    executor.push(Step::Respond(400, error_response()));

    assert!(service.save(identity(1)).await.is_err());

    assert_eq!(observer.count.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn the_observer_hears_nothing_from_a_list_that_changes_nothing() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[post_json(1)]).await;
    let observer = observed(&service);
    executor.push(Step::Respond(200, json!({ "posts": [listed_post(1)] })));

    service
        .list_posts(ReaderSavedPostsParams::default())
        .await
        .unwrap();

    assert_eq!(observer.count.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn the_observer_hears_about_a_cancelled_write() {
    let (service, executor) = service();
    let observer = observed(&service);
    let (_release, held) = oneshot::channel::<()>();
    executor.push(Step::Hold(held, 200, write_response(true, post_json(1), 1)));

    let mut save = Box::pin(service.save(identity(1)));
    assert!(futures::poll!(save.as_mut()).is_pending());
    drop(save);

    assert_eq!(observer.count.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn the_observer_stops_hearing_once_removed() {
    let (service, executor) = service();
    let observer = observed(&service);
    service.set_observer(None);

    refreshed(&service, &executor, &[]).await;

    assert_eq!(observer.count.load(Ordering::SeqCst), 0);
}

/// Apps read the state back from inside the callback, which would deadlock if it ran with a lock
/// held.
struct ReadingObserver {
    service: Mutex<Option<Arc<ReaderSavedService>>>,
    statuses: Mutex<Vec<ReaderSavedStatus>>,
}

impl ReaderSavedObserver for ReadingObserver {
    fn saved_posts_changed(&self) {
        let service = self.service.lock().unwrap().clone().unwrap();
        self.statuses
            .lock()
            .unwrap()
            .push(service.status(identity(1)));
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
    executor.push(Step::Respond(200, json!({ "posts": [listed_post(1)] })));

    service.refresh().await.unwrap();
    service.save(identity(1)).await.unwrap();
    service
        .list_posts(ReaderSavedPostsParams::default())
        .await
        .unwrap();

    assert_eq!(
        *observer.statuses.lock().unwrap(),
        vec![
            ReaderSavedStatus::NotSaved,
            ReaderSavedStatus::Saved,
            ReaderSavedStatus::Saved,
            ReaderSavedStatus::Saved,
        ]
    );

    // Breaks the reference cycle between the service and its observer.
    service.set_observer(None);
}

// Whole flows

#[tokio::test]
async fn a_full_session_stays_consistent() {
    let (service, executor) = service();

    refreshed(&service, &executor, &[post_json(1), feed_json(2)]).await;
    assert_eq!(service.saved_count(), Some(2));

    executor.push(Step::Respond(200, write_response(true, post_json(3), 3)));
    service.save(identity(3)).await.unwrap();
    assert_eq!(service.recent_saves(), vec![post_key(3)]);

    executor.push(Step::Respond(200, write_response(false, feed_json(2), 2)));
    service.unsave(rss_identity(2)).await.unwrap();

    // A list from before both writes.
    executor.push(Step::Respond(
        200,
        json!({ "posts": [listed_post(1), listed_rss_post(2)], "found": 2 }),
    ));
    let page = service
        .list_posts(ReaderSavedPostsParams::default())
        .await
        .unwrap();
    assert_eq!(page.posts.len(), 1);
    assert_eq!(page.posts[0].identity.id, 1);

    // Keys from before both writes.
    refreshed(&service, &executor, &[post_json(1), feed_json(2)]).await;

    assert_eq!(service.status(identity(1)), ReaderSavedStatus::Saved);
    assert_eq!(service.status(rss_identity(2)), ReaderSavedStatus::NotSaved);
    assert_eq!(service.status(identity(3)), ReaderSavedStatus::Saved);
    assert_eq!(service.saved_count(), Some(2));
}

#[path = "scenario_tests.rs"]
mod scenarios;

// An import takes one place in line

/// The unsave is tapped while the import's first batch is in flight, and post 1 is in the second.
#[tokio::test]
async fn an_unsave_tapped_during_an_import_waits_for_every_batch() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[post_json(1)]).await;
    // 101 keys, newest first: post 1 is in the second, newest batch.
    let keys: Vec<_> = (1..=101).map(bulk_post_key).collect();
    let (release, held) = oneshot::channel();
    executor.push(Step::Hold(held, 200, bulk_response(&[post_json(101)], 2)));
    let newest: Vec<_> = (1..=100).map(post_json).collect();
    executor.push(Step::Respond(200, bulk_response(&newest, 101)));
    executor.push(Step::Respond(200, write_response(false, post_json(1), 100)));

    let mut import = Box::pin(service.import(keys));
    assert!(futures::poll!(import.as_mut()).is_pending());
    let mut unsave = Box::pin(service.unsave(identity(1)));
    assert!(futures::poll!(unsave.as_mut()).is_pending());
    assert_eq!(service.status(identity(1)), ReaderSavedStatus::NotSaved);

    release.send(()).unwrap();
    let (imported, unsaved) = futures::join!(import, unsave);
    imported.unwrap();
    unsaved.unwrap();

    let sent: Vec<_> = executor
        .sent()
        .iter()
        .map(|sent| sent.url.rsplit('/').next().unwrap().to_string())
        .collect();
    assert_eq!(sent, ["keys", "bulk", "bulk", "delete"]);
    assert_eq!(service.status(identity(1)), ReaderSavedStatus::NotSaved);
    assert_eq!(service.saved_count(), Some(100));
}

/// Cancelled while its first batch is in flight: that batch may have been applied, the second was
/// never sent, and the tap waiting behind the import goes ahead.
#[tokio::test]
async fn a_cancelled_import_keeps_the_batch_sent_and_lets_a_waiting_tap_go() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[]).await;
    let keys: Vec<_> = (1..=150).map(bulk_post_key).collect();
    let (_release, held) = oneshot::channel::<()>();
    let first: Vec<_> = (101..=150).map(post_json).collect();
    executor.push(Step::Hold(held, 200, bulk_response(&first, 50)));
    executor.push(Step::Respond(200, write_response(true, post_json(500), 51)));

    let mut import = Box::pin(service.import(keys));
    assert!(futures::poll!(import.as_mut()).is_pending());
    let mut save = Box::pin(service.save(identity(500)));
    assert!(futures::poll!(save.as_mut()).is_pending());
    assert_eq!(service.status(identity(100)), ReaderSavedStatus::NotSaved);

    drop(import);
    save.await.unwrap();

    assert_eq!(service.status(identity(150)), ReaderSavedStatus::Saved);
    assert_eq!(service.status(identity(1)), ReaderSavedStatus::NotSaved);
    assert_eq!(service.status(identity(500)), ReaderSavedStatus::Saved);
    assert_eq!(executor.sent().len(), 3);
}

/// The first batch's total includes its own 50 saves, and the second batch's 100 are still to
/// come.
#[tokio::test]
async fn the_count_during_an_import_counts_each_save_once() {
    let (service, executor) = service();
    let existing: Vec<_> = (1001..=1010).map(post_json).collect();
    refreshed(&service, &executor, &existing).await;
    let keys: Vec<_> = (1..=150).map(bulk_post_key).collect();
    let first: Vec<_> = (101..=150).map(post_json).collect();
    executor.push(Step::Respond(200, bulk_response(&first, 60)));
    let (release, held) = oneshot::channel();
    let second: Vec<_> = (1..=100).map(post_json).collect();
    executor.push(Step::Hold(held, 200, bulk_response(&second, 160)));

    let (result, ()) = futures::join!(service.import(keys), async {
        tokio::task::yield_now().await;
        assert_eq!(service.saved_count(), Some(160));
        release.send(()).unwrap();
    });
    result.unwrap();
    assert_eq!(service.saved_count(), Some(160));
}

// Which errors may have been applied

fn execution_failed(reason: RequestExecutionErrorReason) -> WpApiError {
    WpApiError::RequestExecutionFailed {
        status_code: None,
        redirects: None,
        reason,
        request_url: String::new(),
        request_method: RequestMethod::POST,
    }
}

fn answered(status_code: u32) -> WpApiError {
    WpApiError::WpError {
        error_code: WpErrorCode::CustomError("x".to_string()),
        error_message: String::new(),
        status_code,
        response: String::new(),
        request_url: String::new(),
        request_method: RequestMethod::POST,
    }
}

fn unknown(status_code: u32) -> WpApiError {
    WpApiError::UnknownError {
        status_code,
        response: String::new(),
        request_url: String::new(),
        request_method: RequestMethod::POST,
    }
}

fn cloudflare(status_code: u32) -> WpApiError {
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
fn a_failed_write_stands_only_if_it_may_have_been_applied(
    #[case] error: WpApiError,
    #[case] stands: bool,
) {
    assert_eq!(may_have_been_applied(&error), stands);
}

#[tokio::test]
async fn a_save_answered_with_a_server_error_stands_until_a_read() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[]).await;
    executor.push(Step::Respond(502, json!({})));

    assert!(service.save(identity(1)).await.is_err());

    assert_eq!(service.status(identity(1)), ReaderSavedStatus::Saved);
}

// The count after a refresh during an import

/// 150 keys: the first batch (50) is answered with a total of 50. The second (100) times out, but
/// the server applied it. A refresh straight after reads all 150.
#[tokio::test]
async fn the_count_after_a_refresh_that_has_a_timed_out_import_batch() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[]).await;
    let keys: Vec<_> = (1..=150).map(bulk_post_key).collect();
    let first: Vec<_> = (101..=150).map(post_json).collect();
    executor.push(Step::Respond(200, bulk_response(&first, 50)));
    executor.push(Step::TimedOut);

    assert!(service.import(keys).await.is_err());
    assert_eq!(service.saved_count(), Some(150), "before the refresh");

    let all: Vec<_> = (1..=150).map(post_json).collect();
    refreshed(&service, &executor, &all).await;
    assert_eq!(service.saved_count(), Some(150), "the server holds 150");
}

/// The same with the second batch still in flight: the server applied it, and a refresh lands
/// before its answer.
#[tokio::test]
async fn the_count_while_an_import_batch_a_refresh_already_has_is_in_flight() {
    let (service, executor) = service();
    refreshed(&service, &executor, &[]).await;
    let keys: Vec<_> = (1..=150).map(bulk_post_key).collect();
    let first: Vec<_> = (101..=150).map(post_json).collect();
    executor.push(Step::Respond(200, bulk_response(&first, 50)));
    let (release, held) = oneshot::channel();
    let second: Vec<_> = (1..=100).map(post_json).collect();
    executor.push(Step::Hold(held, 200, bulk_response(&second, 150)));
    let all: Vec<_> = (1..=150).map(post_json).collect();

    let (result, during) = futures::join!(service.import(keys), async {
        tokio::task::yield_now().await;
        assert_eq!(service.saved_count(), Some(150));
        executor.push(Step::Respond(200, keys_response(&all)));
        service.refresh().await.unwrap();
        let during = service.saved_count();
        release.send(()).unwrap();
        during
    });
    result.unwrap();
    assert_eq!(service.saved_count(), Some(150), "after the answer");
    assert_eq!(
        during,
        Some(150),
        "while the second batch's answer is on its way"
    );
}

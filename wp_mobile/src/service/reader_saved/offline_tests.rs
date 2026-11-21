//! Getting saved posts ready to read with no connection.

use super::super::storage::migrated_in_memory_cache;
use super::super::tests::{
    ScriptedExecutor, SentInShort, Step, delegate, error_response, identity, keys_response,
    listed_post, post_json, post_key,
};
use super::*;
use crate::service::reader_saved::{ReaderSavedListItem, ReaderSavedStatus};
use serde_json::{Value, json};
use std::sync::Mutex;
use std::time::SystemTime;
use wp_api::auth::{WpAuthentication, WpAuthenticationProvider};
use wp_api::prelude::WpApiClientDelegate;
use wp_api::wp_com::me::WpComUserId;
use wp_api::wp_com::reader_saved::ReaderSavedSource;
use wp_mobile_cache::WpApiCache;
use wp_mobile_cache::repository::wordpress_com_accounts::ReaderOwner;

/// A reader's saved posts, the server they talk to, and the device's clock.
struct Reader {
    service: Arc<ReaderSavedService>,
    executor: Arc<ScriptedExecutor>,
    clock: Arc<Mutex<SystemTime>>,
}

impl Reader {
    fn signed_in() -> Self {
        Self::new(
            ReaderOwner::Account {
                user_id: WpComUserId(1),
            },
            migrated_in_memory_cache(),
            |delegate| delegate,
        )
    }

    fn signed_out() -> Self {
        Self::new(
            ReaderOwner::SignedOut,
            migrated_in_memory_cache(),
            |delegate| delegate,
        )
    }

    fn new(
        owner: ReaderOwner,
        cache: Arc<WpApiCache>,
        configure: impl FnOnce(WpApiClientDelegate) -> WpApiClientDelegate,
    ) -> Self {
        let executor = Arc::new(ScriptedExecutor::default());
        // 2026-11-12, after every date the tests' server gives.
        let clock = Arc::new(Mutex::new(
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_794_441_600),
        ));
        let time = clock.clone();
        let service = ReaderSavedService::for_reader_with_clock(
            configure(delegate(executor.clone())),
            ReaderSavedSource::Android,
            cache,
            owner,
            Arc::new(move || *time.lock().unwrap()),
        )
        .unwrap();
        Self {
            service: Arc::new(service),
            executor,
            clock,
        }
    }

    fn advance(&self, by: Duration) {
        *self.clock.lock().unwrap() += by;
    }

    fn respond(&self, status: u32, body: Value) {
        self.executor.push(Step::Respond(status, body));
    }

    /// The server has these saves, and its list sends these posts as its only page.
    fn server_has(&self, keys: &[Value], listed: &[Value]) {
        self.respond(200, keys_response(keys));
        self.respond(200, json!({ "posts": listed }));
    }

    async fn download(&self, include_images: bool) -> ReaderOfflineReport {
        self.service
            .download_for_offline(include_images)
            .await
            .unwrap()
    }

    async fn list(&self) -> Vec<ReaderSavedListItem> {
        self.service
            .clone()
            .saved_posts()
            .load_items()
            .await
            .unwrap()
    }

    async fn state(&self, post_id: i64) -> ReaderOfflineState {
        self.list()
            .await
            .into_iter()
            .find(|item| item.key == post_key(post_id))
            .expect("the post is in the saved list")
            .offline
    }

    /// The requests sent since the last call, as the last part of each address.
    fn sent(&self) -> Vec<String> {
        self.executor.sent_short()
    }

    fn urls(&self) -> Vec<String> {
        self.executor
            .sent()
            .into_iter()
            .map(|sent| sent.url)
            .collect()
    }

    /// Keeps images in a new directory, with room for `max_bytes`.
    fn store_images_in(&self, max_bytes: u64) -> tempfile::TempDir {
        let directory = tempfile::tempdir().unwrap();
        self.service
            .set_offline_image_store(directory.path().to_str().unwrap().to_string(), max_bytes)
            .unwrap();
        directory
    }
}

fn ready(has_images: bool) -> ReaderOfflineState {
    ReaderOfflineState::Ready { has_images }
}

/// A post as the API sends one on its own, with content.
fn full_post(post_id: u64, content: &str) -> Value {
    json!({
        "ID": post_id,
        "site_ID": 123,
        "is_external": false,
        "title": format!("Post {post_id}"),
        "content": content,
    })
}

/// A post an app has in full, to save with its details.
fn post_with(post_id: u64, fields: Value) -> ReaderPost {
    let mut post = full_post(post_id, "");
    post.as_object_mut()
        .unwrap()
        .extend(fields.as_object().unwrap().clone());
    serde_json::from_value(post).unwrap()
}

/// An image's answer. The scripted server sends no `Content-Type`, so the address says it's an
/// image, and the body's length is `bytes`.
fn image(bytes: usize) -> Value {
    // A JSON string is written with its two quotes.
    json!("x".repeat(bytes - 2))
}

fn files_in(directory: &tempfile::TempDir) -> Vec<String> {
    let mut names: Vec<_> = std::fs::read_dir(directory.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_str().unwrap().to_string())
        .collect();
    names.sort();
    names
}

fn empty_report() -> ReaderOfflineReport {
    ReaderOfflineReport {
        ready: 0,
        with_images: 0,
        failed: 0,
        not_downloaded: 0,
        image_bytes: 0,
        stopped_because: None,
    }
}

// Content

#[tokio::test]
async fn a_post_saved_with_its_details_is_ready_and_one_saved_by_its_ids_is_not() {
    let reader = Reader::signed_out();
    reader
        .service
        .save_with_details(post_with(1, json!({ "content": "<p>One</p>" })))
        .await
        .unwrap();
    reader.service.save(identity(2)).await.unwrap();

    assert_eq!(reader.state(1).await, ready(false));
    assert_eq!(reader.state(2).await, ReaderOfflineState::NotDownloaded);
}

#[tokio::test]
async fn a_download_loads_the_saved_list_until_every_save_has_its_content() {
    let reader = Reader::signed_in();
    reader.respond(
        200,
        keys_response(&[post_json(1), post_json(2), post_json(3)]),
    );
    reader.respond(
        200,
        json!({ "posts": [listed_post(3)], "next_page_handle": "second" }),
    );
    reader.respond(
        200,
        json!({ "posts": [listed_post(2), listed_post(1)], "next_page_handle": "third" }),
    );

    let report = reader.download(false).await;

    assert_eq!(
        report,
        ReaderOfflineReport {
            ready: 3,
            ..empty_report()
        }
    );
    // The third page isn't loaded: every save has its content.
    assert_eq!(reader.sent(), ["keys", "list", "list"]);
    let second_page = &reader.urls()[2];
    assert!(second_page.contains("page_handle=second"), "{second_page}");
    assert!(second_page.contains("number=40"), "{second_page}");
    for post in 1..=3 {
        assert_eq!(reader.state(post).await, ready(false));
    }
}

#[tokio::test]
async fn a_save_the_list_does_not_send_is_fetched_on_its_own() {
    let reader = Reader::signed_in();
    reader.server_has(&[post_json(1), post_json(2)], &[listed_post(2)]);
    reader.respond(200, full_post(1, "<p>One</p>"));

    let report = reader.download(false).await;

    assert_eq!(report.ready, 2);
    assert_eq!(reader.sent(), ["keys", "list", "1"]);
    assert!(reader.urls()[2].contains("/read/sites/123/posts/1"));
    // It's in the saved list now, with what was fetched.
    let list = reader.list().await;
    let fetched = list
        .iter()
        .find(|item| item.key == post_key(1))
        .expect("the fetched save is listed");
    assert_eq!(fetched.offline, ready(false));
    let post = fetched.post.as_ref().unwrap();
    assert_eq!(post.title.as_deref(), Some("Post 1"));
    assert_eq!(post.content.as_deref(), Some("<p>One</p>"));
}

#[tokio::test]
async fn a_save_whose_content_cannot_be_fetched_is_tried_again_an_hour_later() {
    let reader = Reader::signed_in();
    reader.server_has(&[post_json(1)], &[]);
    reader.respond(404, error_response());

    let report = reader.download(false).await;
    assert_eq!(
        report,
        ReaderOfflineReport {
            failed: 1,
            ..empty_report()
        }
    );

    // Straight away, it isn't asked for again.
    reader.server_has(&[post_json(1)], &[]);
    reader.download(false).await;
    assert_eq!(reader.sent(), ["keys", "list", "1", "keys", "list"]);

    reader.advance(RETRY_AFTER);
    reader.server_has(&[post_json(1)], &[]);
    reader.respond(200, full_post(1, "<p>One</p>"));
    let report = reader.download(false).await;
    assert_eq!(report.ready, 1);
    assert_eq!(report.failed, 0);
    assert_eq!(reader.state(1).await, ready(false));
}

#[tokio::test]
async fn a_save_is_given_up_on_after_five_answers() {
    let reader = Reader::signed_in();
    for _ in 0..MAX_ATTEMPTS {
        reader.server_has(&[post_json(1)], &[]);
        reader.respond(404, error_response());
        assert_eq!(reader.download(false).await.failed, 1);
        reader.advance(RETRY_AFTER);
    }

    reader.server_has(&[post_json(1)], &[]);
    let report = reader.download(false).await;

    assert_eq!(report.failed, 1);
    assert_eq!(reader.sent().len(), 3 * MAX_ATTEMPTS as usize + 2);
}

#[tokio::test]
async fn retrying_a_failed_save_fetches_it_now() {
    let reader = Reader::signed_in();
    reader.server_has(&[post_json(1)], &[]);
    reader.respond(404, error_response());
    reader.download(false).await;

    // It fails again, and says so, though a save that failed isn't in the saved list.
    reader.respond(404, error_response());
    let state = reader.service.retry_offline(post_key(1)).await.unwrap();
    assert!(
        matches!(state, ReaderOfflineState::Failed { .. }),
        "{state:?}"
    );

    reader.executor.push(Step::Offline);
    let unreachable = reader.service.retry_offline(post_key(1)).await;
    assert!(
        matches!(
            unreachable,
            Err(ReaderSavedError::OfflineUnreachable { .. })
        ),
        "{unreachable:?}"
    );

    reader.respond(200, full_post(1, "<p>One</p>"));
    let state = reader.service.retry_offline(post_key(1)).await.unwrap();
    assert_eq!(state, ready(false));

    // A post that isn't saved has nothing to retry.
    let state = reader.service.retry_offline(post_key(9)).await.unwrap();
    assert_eq!(state, ReaderOfflineState::NotDownloaded);
    assert_eq!(reader.sent().len(), 6);
}

/// WordPress.com's answer on `rest/v1.x` to a token it doesn't accept.
fn sign_in_rejected() -> Value {
    json!({ "error": "invalid_token", "message": "The OAuth2 token is invalid." })
}

#[tokio::test]
async fn a_rejected_sign_in_does_not_count_against_a_save() {
    let reader = Reader::signed_in();
    reader.server_has(&[post_json(1)], &[]);
    reader.respond(404, error_response());
    reader.download(false).await;

    reader.respond(400, sign_in_rejected());
    let rejected = reader.service.retry_offline(post_key(1)).await;
    assert!(
        matches!(&rejected, Err(ReaderSavedError::Api(error)) if error.to_string().contains("The OAuth2 token is invalid.")),
        "{rejected:?}"
    );

    // A download that meets it stops there, as it does when the list's request is rejected.
    reader.server_has(&[post_json(1)], &[]);
    reader.respond(400, sign_in_rejected());
    let report = reader.download(false).await;
    assert!(report.stopped_because.is_some(), "{report:?}");
    assert_eq!((report.failed, report.not_downloaded), (0, 1));

    // Signed in again, it's fetched straight away.
    reader.server_has(&[post_json(1)], &[]);
    reader.respond(200, full_post(1, "<p>One</p>"));
    assert_eq!(reader.download(false).await.ready, 1);
}

#[tokio::test]
async fn no_connection_stops_a_download_without_counting_against_a_save() {
    let reader = Reader::signed_in();
    reader.executor.push(Step::Offline);
    let report = reader.download(true).await;
    assert!(report.stopped_because.is_some(), "{report:?}");

    // The list answers, and the post's own request doesn't.
    reader.server_has(&[post_json(1)], &[]);
    reader.executor.push(Step::Offline);
    let report = reader.download(false).await;
    assert!(report.stopped_because.is_some(), "{report:?}");
    assert_eq!(report.not_downloaded, 1);
    assert_eq!(report.failed, 0);

    // So it's asked for again as soon as there's a connection.
    reader.server_has(&[post_json(1)], &[]);
    reader.respond(200, full_post(1, "<p>One</p>"));
    assert_eq!(reader.download(false).await.ready, 1);
}

#[tokio::test]
async fn a_saved_post_that_cannot_be_read_is_still_a_save_and_says_why() {
    let reader = Reader::signed_in();
    let unreadable = json!({
        "ID": 2,
        "site_ID": 123,
        "is_external": false,
        "date": "2026-11-01T00:00:00+91:00",
    });
    reader.server_has(&[post_json(1), post_json(2)], &[listed_post(1), unreadable]);

    let report = reader.download(false).await;

    assert_eq!(report.ready, 1);
    assert_eq!(report.failed, 1);
    // It was just tried, by the list, so it isn't fetched on its own yet.
    assert_eq!(reader.sent(), ["keys", "list"]);
    assert_eq!(reader.state(1).await, ready(false));
    let ReaderOfflineState::Failed { reason } = reader.state(2).await else {
        panic!("the unreadable post didn't fail");
    };
    assert!(reason.starts_with("The post couldn't be read"), "{reason}");
    assert_eq!(
        reader.service.status(identity(2)).unwrap(),
        ReaderSavedStatus::Saved
    );
}

/// The server has posts `1..=count` saved, post `count` the newest save, and its list sends none of
/// them. Each is then fetched on its own, newest save first: `count` down to 1.
fn server_has_saves_the_list_does_not_send(reader: &Reader, count: u64) {
    let keys: Vec<Value> = (1..=count)
        .rev()
        .map(|post| {
            let mut key = post_json(post);
            key["id"] = json!(post);
            key["date_saved"] = json!(format!(
                "2026-11-01T{:02}:{:02}:00+00:00",
                post / 60,
                post % 60
            ));
            key
        })
        .collect();
    reader.respond(200, json!({ "keys": keys, "total": count }));
    reader.respond(200, json!({ "posts": [] }));
}

/// The posts fetched on their own, in the order they were asked for.
fn fetched_posts(sent: &[String]) -> Vec<u64> {
    sent.iter()
        .filter(|name| !["keys", "list"].contains(&name.as_str()))
        .map(|name| name.parse().expect("a post's own request ends with its ID"))
        .collect()
}

/// Every save's offline state, by post: `None` for a save the saved list doesn't show, which is
/// one with no content.
async fn states(reader: &Reader, count: u64) -> Vec<(u64, Option<ReaderOfflineState>)> {
    let list = reader.list().await;
    (1..=count)
        .map(|post| {
            let item = list.iter().find(|item| item.key == post_key(post as i64));
            (post, item.map(|item| item.offline.clone()))
        })
        .collect()
}

#[tokio::test]
async fn one_download_fills_in_every_save_the_list_does_not_send() {
    let reader = Reader::signed_in();
    server_has_saves_the_list_does_not_send(&reader, 120);
    for post in (1..=120).rev() {
        reader.respond(200, full_post(post, &format!("<p>{post}</p>")));
    }

    let report = reader.download(false).await;

    assert_eq!(
        report,
        ReaderOfflineReport {
            ready: 120,
            ..empty_report()
        }
    );
    // Each once, newest save first.
    assert_eq!(
        fetched_posts(&reader.sent()),
        (1..=120).rev().collect::<Vec<_>>()
    );
    // Each save has its own post's content.
    let list = reader.list().await;
    for post in 1..=120 {
        let item = list
            .iter()
            .find(|item| item.key == post_key(post))
            .expect("every save is listed");
        assert_eq!(item.offline, ready(false), "{post}");
        let content = item.post.as_ref().and_then(|post| post.content.clone());
        assert_eq!(content, Some(format!("<p>{post}</p>")));
    }
}

#[tokio::test]
async fn a_download_counts_the_saves_the_server_refuses_and_leaves_them_for_later() {
    let reader = Reader::signed_in();
    server_has_saves_the_list_does_not_send(&reader, 120);
    // Every twelfth post can't be had now: ten of them.
    let refused = |post: u64| post.is_multiple_of(12);
    for post in (1..=120).rev() {
        if refused(post) {
            reader.respond(404, error_response());
        } else {
            reader.respond(200, full_post(post, "<p>Text</p>"));
        }
    }

    let report = reader.download(false).await;

    assert_eq!(
        report,
        ReaderOfflineReport {
            ready: 110,
            failed: 10,
            ..empty_report()
        }
    );
    // Each was asked for once, refused or not.
    assert_eq!(fetched_posts(&reader.sent()).len(), 120);
    // The refused ones have no content, and the report counts them as failed, not as waiting.
    for (post, state) in states(&reader, 120).await {
        let expected = (!refused(post)).then(|| ready(false));
        assert_eq!(state, expected, "{post}");
    }

    // Straight away, the refused ones aren't asked for again.
    let before = reader.sent().len();
    server_has_saves_the_list_does_not_send(&reader, 120);
    assert_eq!(reader.download(false).await, report);
    assert!(fetched_posts(&reader.sent()[before..]).is_empty());

    // An hour later they are, and only they.
    let before = reader.sent().len();
    reader.advance(RETRY_AFTER);
    server_has_saves_the_list_does_not_send(&reader, 120);
    for post in (1..=120).rev().filter(|post| refused(*post)) {
        reader.respond(200, full_post(post, "<p>Text</p>"));
    }
    let report = reader.download(false).await;
    assert_eq!(report.ready, 120);
    assert_eq!(report.failed, 0);
    assert_eq!(
        fetched_posts(&reader.sent()[before..]),
        [120, 108, 96, 84, 72, 60, 48, 36, 24, 12]
    );
}

#[tokio::test]
async fn a_download_that_loses_the_connection_stops_and_keeps_what_it_fetched() {
    let reader = Reader::signed_in();
    server_has_saves_the_list_does_not_send(&reader, 120);
    // Posts 120 down to 51 arrive, then the connection goes.
    for post in (51..=120).rev() {
        reader.respond(200, full_post(post, "<p>Text</p>"));
    }
    reader.executor.push(Step::Offline);

    let report = reader.download(false).await;

    assert!(report.stopped_because.is_some(), "{report:?}");
    assert_eq!(
        report,
        ReaderOfflineReport {
            ready: 70,
            not_downloaded: 50,
            stopped_because: report.stopped_because.clone(),
            ..empty_report()
        }
    );
    // Nothing was asked for after the request that got no answer.
    assert_eq!(
        fetched_posts(&reader.sent()),
        (50..=120).rev().collect::<Vec<_>>()
    );
    // The rest have no content, and the report counts them as waiting, not as failed.
    for (post, state) in states(&reader, 120).await {
        let expected = (post > 50).then(|| ready(false));
        assert_eq!(state, expected, "{post}");
    }

    // The next download asks only for the rest, and finishes.
    let before = reader.sent().len();
    server_has_saves_the_list_does_not_send(&reader, 120);
    for post in (1..=50).rev() {
        reader.respond(200, full_post(post, "<p>Text</p>"));
    }
    let report = reader.download(false).await;
    assert_eq!(
        report,
        ReaderOfflineReport {
            ready: 120,
            ..empty_report()
        }
    );
    assert_eq!(
        fetched_posts(&reader.sent()[before..]),
        (1..=50).rev().collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn a_signed_out_readers_save_is_fetched_without_the_saved_list() {
    let reader = Reader::signed_out();
    reader.service.save(identity(1)).await.unwrap();
    reader.respond(200, full_post(1, "<p>One</p>"));

    let report = reader.download(false).await;

    assert_eq!(report.ready, 1);
    assert_eq!(reader.sent(), ["1"]);
    assert_eq!(reader.state(1).await, ready(false));
}

#[tokio::test]
async fn a_page_saved_by_its_address_that_the_list_never_sends_fails_without_a_request() {
    let reader = Reader::signed_in();
    let key = json!({ "url": "https://example.com/page" });
    reader.server_has(&[key], &[]);

    let report = reader.download(false).await;

    assert_eq!(report.failed, 1);
    assert_eq!(reader.sent(), ["keys", "list"]);
}

#[tokio::test]
async fn the_pending_count_is_what_the_server_does_not_have() {
    let signed_out = Reader::signed_out();
    signed_out.service.save(identity(1)).await.unwrap();
    signed_out.service.save(identity(2)).await.unwrap();
    // No server has a signed-out reader's saves.
    assert_eq!(signed_out.service.pending_count().unwrap(), 2);
    signed_out.service.unsave(identity(1)).await.unwrap();
    assert_eq!(signed_out.service.pending_count().unwrap(), 1);

    let reader = Reader::signed_in();
    reader.executor.push(Step::Offline);
    reader.service.save(identity(1)).await.unwrap();
    assert_eq!(reader.service.pending_count().unwrap(), 1);
    // A sync sends the save, then reads the keys, which have it.
    reader.respond(
        200,
        json!({ "saved": true, "key": post_json(1), "total": 1, "resolved": "post" }),
    );
    reader.respond(200, keys_response(&[post_json(1)]));
    reader.service.sync().await.unwrap();
    assert_eq!(reader.service.pending_count().unwrap(), 0);
}

#[tokio::test]
async fn signed_out_a_save_is_kept_and_an_unsave_is_removed() {
    let reader = Reader::signed_out();
    let reason = |outcome: crate::service::reader_saved::ReaderSavedOutcome| match outcome {
        crate::service::reader_saved::ReaderSavedOutcome::Pending { reason } => reason,
        other => panic!("expected a pending outcome, got {other:?}"),
    };

    let saved = reader.service.save(identity(1)).await.unwrap();
    let unsaved = reader.service.unsave(identity(1)).await.unwrap();

    assert!(reason(saved).contains("kept on this device"));
    assert!(reason(unsaved).contains("removed from this device"));
    assert_eq!(
        crate::service::reader_saved::reader_saved_error_message(&ReaderSavedError::TooManySaves),
        "You've reached the limit of saved posts."
    );
}

// Images

const TWO_IMAGES: &str = concat!(
    r#"<p>Text</p><img src="https://example.com/a.jpg" srcset="https://example.com/a-2x.jpg 2x">"#,
    r#"<img src="https://example.com/b.png">"#,
    r#"<img src="https://pixel.wp.com/b.gif?host=example.com" width="1" height="1">"#,
);

async fn save_with_images(reader: &Reader, post_id: u64, content: &str) {
    reader
        .service
        .save_with_details(post_with(post_id, json!({ "content": content })))
        .await
        .unwrap();
}

#[tokio::test]
async fn images_are_not_stored_until_the_app_says_where_and_asks() {
    let reader = Reader::signed_out();
    save_with_images(&reader, 1, TWO_IMAGES).await;

    // Asked, with nowhere to keep them.
    assert_eq!(reader.download(true).await.with_images, 0);
    // Somewhere to keep them, and not asked.
    let directory = reader.store_images_in(1_000);
    assert_eq!(reader.download(false).await.with_images, 0);

    assert_eq!(reader.sent(), Vec::<String>::new());
    assert_eq!(files_in(&directory), Vec::<String>::new());
}

#[tokio::test]
async fn a_download_stores_a_posts_images_and_points_its_content_at_them() {
    let reader = Reader::signed_out();
    reader
        .service
        .save_with_details(post_with(
            1,
            json!({ "content": TWO_IMAGES, "featured_image": "https://example.com/c.jpg" }),
        ))
        .await
        .unwrap();
    let directory = reader.store_images_in(1_000);
    for bytes in [10, 20, 30] {
        reader.respond(200, image(bytes));
    }

    let report = reader.download(true).await;

    assert_eq!(
        report,
        ReaderOfflineReport {
            ready: 1,
            with_images: 1,
            image_bytes: 60,
            ..empty_report()
        }
    );
    // The tracking pixel isn't fetched.
    assert_eq!(
        reader.urls(),
        [
            "https://example.com/a.jpg",
            "https://example.com/b.png",
            "https://example.com/c.jpg",
        ]
    );
    assert_eq!(reader.state(1).await, ready(true));
    assert_eq!(reader.service.offline_image_bytes().unwrap(), 60);

    let offline = reader
        .service
        .offline_post(post_key(1), "reader-file://".to_string())
        .unwrap()
        .unwrap();
    let names = files_in(&directory);
    assert_eq!(names.len(), 3);
    assert!(
        names.iter().all(|name| is_stored_file_name(name)),
        "{names:?}"
    );
    let file = |url: &str| {
        offline
            .images
            .iter()
            .find(|image| image.url == url)
            .map(|image| image.file_name.clone())
            .unwrap()
    };
    assert!(file("https://example.com/a.jpg").ends_with(".jpg"));
    assert!(file("https://example.com/b.png").ends_with(".png"));
    assert_eq!(
        offline.post.content.as_deref(),
        Some(
            format!(
                concat!(
                    r#"<p>Text</p><img src="reader-file://{}">"#,
                    r#"<img src="reader-file://{}">"#,
                    r#"<img src="https://pixel.wp.com/b.gif?host=example.com" width="1" height="1">"#,
                ),
                file("https://example.com/a.jpg"),
                file("https://example.com/b.png"),
            )
            .as_str()
        )
    );
    assert_eq!(
        offline.post.featured_image,
        Some(format!(
            "reader-file://{}",
            file("https://example.com/c.jpg")
        ))
    );

    // Nothing is fetched twice.
    assert_eq!(reader.download(true).await.with_images, 1);
    assert_eq!(reader.sent().len(), 3);
}

#[tokio::test]
async fn an_image_the_server_does_not_have_does_not_hold_its_post_back() {
    let reader = Reader::signed_out();
    save_with_images(&reader, 1, TWO_IMAGES).await;
    let directory = reader.store_images_in(1_000);
    reader.respond(404, json!("Not found"));
    reader.respond(200, image(20));

    let report = reader.download(true).await;

    assert_eq!(report.with_images, 1);
    assert_eq!(files_in(&directory).len(), 1);
    let offline = reader
        .service
        .offline_post(post_key(1), String::new())
        .unwrap()
        .unwrap();
    assert!(
        offline
            .post
            .content
            .unwrap()
            .contains(r#"src="https://example.com/a.jpg""#),
        "an image that isn't stored keeps its address"
    );
    // And it isn't asked for again.
    reader.download(true).await;
    assert_eq!(reader.sent().len(), 2);
}

#[tokio::test]
async fn an_image_the_server_cannot_send_now_is_asked_for_again() {
    let reader = Reader::signed_out();
    save_with_images(&reader, 1, TWO_IMAGES).await;
    let _directory = reader.store_images_in(1_000);
    reader.respond(503, json!("Busy"));
    reader.respond(200, image(20));

    assert_eq!(reader.download(true).await.with_images, 0);
    assert_eq!(reader.state(1).await, ready(false));

    reader.respond(200, image(10));
    assert_eq!(reader.download(true).await.with_images, 1);
    assert_eq!(
        reader.urls(),
        [
            "https://example.com/a.jpg",
            "https://example.com/b.png",
            "https://example.com/a.jpg",
        ]
    );
}

#[tokio::test]
async fn a_private_sites_image_whose_sign_in_is_rejected_is_asked_for_again() {
    let reader = Reader::signed_in();
    let mut private = full_post(
        1,
        r#"<img src="https://private.files.wordpress.com/a.jpg">"#,
    );
    private["site_is_private"] = json!(true);
    reader.server_has(&[post_json(1)], std::slice::from_ref(&private));
    let _directory = reader.store_images_in(1_000);
    reader.respond(401, sign_in_rejected());

    assert_eq!(reader.download(true).await.with_images, 0);
    assert_eq!(reader.state(1).await, ready(false));

    reader.server_has(&[post_json(1)], &[private]);
    reader.respond(200, image(10));
    assert_eq!(reader.download(true).await.with_images, 1);
}

#[tokio::test]
async fn no_connection_stops_storing_images() {
    let reader = Reader::signed_out();
    save_with_images(&reader, 1, TWO_IMAGES).await;
    let _directory = reader.store_images_in(1_000);
    reader.executor.push(Step::Offline);

    let report = reader.download(true).await;

    assert!(report.stopped_because.is_some());
    assert_eq!(report.with_images, 0);
    assert_eq!(reader.sent().len(), 1, "the second image isn't tried");
}

/// With the server just heard from, a host that can't be reached is that host's doing: the image
/// is done without. Without that, it may be the device, so the download stops.
#[tokio::test]
async fn an_image_whose_host_is_gone_is_done_without_once_the_device_is_known_to_be_online() {
    let reader = Reader::signed_in();
    let listed = full_post(1, r#"<img src="https://gone.example/a.jpg">"#);
    reader.server_has(&[post_json(1)], &[listed]);
    let _directory = reader.store_images_in(1_000);
    reader.executor.push(Step::Unreachable);

    let report = reader.download(true).await;

    assert_eq!(report.stopped_because, None);
    assert_eq!(report.with_images, 1);

    let signed_out = Reader::signed_out();
    save_with_images(&signed_out, 1, r#"<img src="https://gone.example/a.jpg">"#).await;
    let _directory = signed_out.store_images_in(1_000);
    signed_out.executor.push(Step::Unreachable);
    let report = signed_out.download(true).await;
    assert!(report.stopped_because.is_some());
    assert_eq!(report.with_images, 0);
}

#[tokio::test]
async fn only_a_private_sites_images_are_asked_for_as_the_account() {
    let bearer = |delegate: WpApiClientDelegate| WpApiClientDelegate {
        auth_provider: Arc::new(WpAuthenticationProvider::static_with_auth(
            WpAuthentication::Bearer {
                token: "secret".to_string(),
            },
        )),
        ..delegate
    };
    let reader = Reader::new(
        ReaderOwner::Account {
            user_id: WpComUserId(1),
        },
        migrated_in_memory_cache(),
        bearer,
    );
    let image_at = |host: &str| format!(r#"<img src="https://{host}/a.jpg">"#);
    let mut private = full_post(
        1,
        &format!(
            "{}{}",
            image_at("private.files.wordpress.com"),
            image_at("elsewhere.example")
        ),
    );
    private["site_is_private"] = json!(true);
    let public = full_post(2, &image_at("public.files.wordpress.com"));
    reader.server_has(&[post_json(1), post_json(2)], &[private, public]);
    let _directory = reader.store_images_in(1_000);
    for _ in 0..3 {
        reader.respond(200, image(10));
    }

    assert_eq!(reader.download(true).await.with_images, 2);

    let mut authorization: Vec<_> = reader
        .executor
        .sent()
        .into_iter()
        .filter(|sent| sent.url.ends_with("/a.jpg"))
        .map(|sent| (sent.url, sent.authorization))
        .collect();
    authorization.sort();
    assert_eq!(
        authorization,
        [
            ("https://elsewhere.example/a.jpg".to_string(), None),
            (
                "https://private.files.wordpress.com/a.jpg".to_string(),
                Some("Bearer secret".to_string())
            ),
            ("https://public.files.wordpress.com/a.jpg".to_string(), None),
        ]
    );
}

#[tokio::test]
async fn unsaving_a_post_deletes_its_images() {
    let reader = Reader::signed_out();
    save_with_images(&reader, 1, TWO_IMAGES).await;
    let directory = reader.store_images_in(1_000);
    reader.respond(200, image(10));
    reader.respond(200, image(20));
    reader.download(true).await;
    assert_eq!(files_in(&directory).len(), 2);

    reader.service.unsave(identity(1)).await.unwrap();

    assert_eq!(files_in(&directory), Vec::<String>::new());
    assert_eq!(reader.service.offline_image_bytes().unwrap(), 0);
}

#[tokio::test]
async fn two_posts_share_one_copy_of_an_image_until_neither_is_saved() {
    let reader = Reader::signed_out();
    let content = r#"<img src="https://example.com/a.jpg">"#;
    save_with_images(&reader, 1, content).await;
    save_with_images(&reader, 2, content).await;
    let directory = reader.store_images_in(1_000);
    reader.respond(200, image(10));

    let report = reader.download(true).await;

    assert_eq!(report.with_images, 2);
    assert_eq!(report.image_bytes, 10);
    assert_eq!(reader.sent().len(), 1);
    assert_eq!(files_in(&directory).len(), 1);

    reader.service.unsave(identity(1)).await.unwrap();
    assert_eq!(files_in(&directory).len(), 1, "post 2 still has it");
    reader.service.unsave(identity(2)).await.unwrap();
    assert_eq!(files_in(&directory), Vec::<String>::new());
}

#[tokio::test]
async fn the_oldest_saves_images_go_to_make_room_for_a_newer_saves() {
    let reader = Reader::signed_out();
    let directory = reader.store_images_in(100);
    save_with_images(&reader, 1, r#"<img src="https://example.com/old.jpg">"#).await;
    reader.respond(200, image(60));
    assert_eq!(reader.download(true).await.with_images, 1);
    let old_file = files_in(&directory);

    reader.advance(Duration::from_secs(60));
    save_with_images(&reader, 2, r#"<img src="https://example.com/new.jpg">"#).await;
    reader.respond(200, image(60));
    let report = reader.download(true).await;

    assert_eq!(report.with_images, 1);
    assert_eq!(report.ready, 2, "the older post keeps its text");
    assert_eq!(report.image_bytes, 60);
    assert_eq!(reader.state(2).await, ready(true));
    assert_eq!(reader.state(1).await, ready(false));
    let files = files_in(&directory);
    assert_eq!(files.len(), 1);
    assert_ne!(files, old_file);

    // The older post's image is tried once more, doesn't fit, and takes nothing from the newer.
    reader.respond(200, image(60));
    let report = reader.download(true).await;
    assert_eq!(report.image_bytes, 60);
    assert_eq!(reader.state(2).await, ready(true));
    assert_eq!(files_in(&directory), files);
}

#[tokio::test]
async fn an_older_saves_images_are_kept_when_removing_them_would_not_make_room() {
    let reader = Reader::signed_out();
    let directory = reader.store_images_in(100);
    save_with_images(&reader, 1, r#"<img src="https://example.com/oldest.jpg">"#).await;
    reader.respond(200, image(30));
    assert_eq!(reader.download(true).await.with_images, 1);
    let oldest_file = files_in(&directory);

    reader.advance(Duration::from_secs(60));
    save_with_images(&reader, 2, r#"<img src="https://example.com/middle.jpg">"#).await;
    reader.advance(Duration::from_secs(60));
    save_with_images(&reader, 3, r#"<img src="https://example.com/newest.jpg">"#).await;
    reader.respond(200, image(60));
    // 60 and 50 don't fit in 100, even with the oldest post's 30 removed.
    reader.respond(200, image(50));
    let report = reader.download(true).await;

    assert_eq!(report.image_bytes, 90);
    assert_eq!(reader.state(3).await, ready(true));
    assert_eq!(reader.state(2).await, ready(false));
    assert_eq!(reader.state(1).await, ready(true));
    let files = files_in(&directory);
    assert_eq!(files.len(), 2);
    assert!(files.contains(&oldest_file[0]), "{files:?}");
    // Newest save first.
    let images: Vec<_> = reader
        .service
        .cache
        .execute(|connection| {
            ReaderOfflineRepository::select_posts_for_images(&*connection, reader.service.account)
        })
        .unwrap()
        .into_iter()
        .map(|post| post.images)
        .collect();
    assert_eq!(
        images,
        [
            DbReaderOfflineImages::Stored,
            DbReaderOfflineImages::NoRoom,
            DbReaderOfflineImages::Stored
        ]
    );
}

/// The posts with images are the newest ones: once one doesn't fit, no older post's are fetched.
/// An older post that has none to fetch is complete all the same.
#[tokio::test]
async fn once_room_runs_out_older_posts_are_not_fetched_and_one_with_no_images_is_still_complete() {
    let reader = Reader::signed_out();
    let directory = reader.store_images_in(100);
    save_with_images(&reader, 1, "<p>No images here.</p>").await;
    reader.advance(Duration::from_secs(60));
    save_with_images(&reader, 2, r#"<img src="https://example.com/small.jpg">"#).await;
    reader.advance(Duration::from_secs(60));
    save_with_images(&reader, 3, r#"<img src="https://example.com/big.jpg">"#).await;
    reader.advance(Duration::from_secs(60));
    save_with_images(&reader, 4, r#"<img src="https://example.com/newest.jpg">"#).await;
    reader.respond(200, image(60));
    reader.respond(200, image(80));

    let report = reader.download(true).await;

    assert_eq!(
        reader.urls(),
        [
            "https://example.com/newest.jpg",
            "https://example.com/big.jpg"
        ],
        "the small image of an older post isn't fetched, though it would fit"
    );
    assert_eq!(report.image_bytes, 60);
    assert_eq!(report.with_images, 2);
    assert_eq!(reader.state(4).await, ready(true));
    assert_eq!(reader.state(3).await, ready(false));
    assert_eq!(reader.state(2).await, ready(false));
    assert_eq!(reader.state(1).await, ready(true));
    assert_eq!(files_in(&directory).len(), 1);
}

#[tokio::test]
async fn an_image_bigger_than_all_the_room_is_done_without() {
    let reader = Reader::signed_out();
    let directory = reader.store_images_in(50);
    save_with_images(&reader, 1, TWO_IMAGES).await;
    reader.respond(200, image(80));
    reader.respond(200, image(20));

    let report = reader.download(true).await;

    assert_eq!(report.with_images, 1);
    assert_eq!(report.image_bytes, 20);
    assert_eq!(files_in(&directory).len(), 1);
}

#[tokio::test]
async fn an_answer_that_is_not_an_image_is_not_stored() {
    let reader = Reader::signed_out();
    let directory = reader.store_images_in(1_000);
    // No `Content-Type`, and an address that doesn't end like an image's.
    save_with_images(
        &reader,
        1,
        r#"<img src="https://example.com/image.php?id=1">"#,
    )
    .await;
    reader.respond(200, json!("<html>Sign in</html>"));

    let report = reader.download(true).await;

    assert_eq!(report.with_images, 1, "the post is whole without it");
    assert_eq!(files_in(&directory), Vec::<String>::new());
}

#[tokio::test]
async fn a_download_deletes_stored_files_no_post_has_and_nothing_else() {
    let reader = Reader::signed_out();
    let directory = reader.store_images_in(1_000);
    let stray = format!("{FILE_PREFIX}{:032x}.jpg", 7);
    for name in [stray.as_str(), "notes.txt", "rdr-short.jpg", "rdr-zz.jpg"] {
        std::fs::write(directory.path().join(name), b"x").unwrap();
    }

    reader.download(true).await;

    assert_eq!(
        files_in(&directory),
        ["notes.txt", "rdr-short.jpg", "rdr-zz.jpg"]
    );
}

#[tokio::test]
async fn removing_offline_images_deletes_them_and_stops_storing_them() {
    let reader = Reader::signed_out();
    save_with_images(&reader, 1, TWO_IMAGES).await;
    let directory = reader.store_images_in(1_000);
    reader.respond(200, image(10));
    reader.respond(200, image(20));
    reader.download(true).await;

    reader.service.remove_offline_images().unwrap();

    assert_eq!(files_in(&directory), Vec::<String>::new());
    assert_eq!(reader.state(1).await, ready(false));
    assert_eq!(reader.download(true).await.with_images, 0);
    assert_eq!(reader.sent().len(), 2, "nothing more is fetched");
}

#[rstest::rstest]
#[case("rdr-0123456789abcdef0123456789abcdef.jpg", true)]
#[case("rdr-0123456789ABCDEF0123456789abcdef.webp", true)]
#[case("rdr-0123456789abcdef0123456789abcdef", false)]
#[case("rdr-0123456789abcdef0123456789abcdef.", false)]
#[case("rdr-0123456789abcdef0123456789abcde.jpg", false)]
#[case("rdr-0123456789abcdef0123456789abcdeg.jpg", false)]
#[case("0123456789abcdef0123456789abcdef.jpg", false)]
#[case("rdr-0123456789abcdef0123456789abcdef.j/pg", false)]
#[case("../rdr-0123456789abcdef0123456789abcdef.jpg", false)]
fn only_a_file_this_library_named_is_its_own(#[case] name: &str, #[case] expected: bool) {
    assert_eq!(is_stored_file_name(name), expected);
}

#[test]
fn a_posts_offline_copy_is_none_for_a_post_that_is_not_saved() {
    let reader = Reader::signed_out();
    assert_eq!(
        reader
            .service
            .offline_post(post_key(1), String::new())
            .unwrap(),
        None
    );
}

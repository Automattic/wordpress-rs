//! What the service keeps in the cache: across a relaunch, through an import, and as the saved list.

use super::super::tests::{
    ScriptedExecutor, Step, bulk_response, cap_response, delegate, error_response, feed_json,
    identity, keys_response, post_json, post_key, rss_identity, synced, unsaveable_identity,
    write_response,
};
use super::*;
use crate::service::reader_saved::ReaderSavedStatus::{NotSaved, Saved, Unknown};
use crate::service::reader_saved::{ReaderSavedOutcome, ReaderSavedStatus};
use futures::channel::oneshot;
use serde_json::{Value, json};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::me::WpComUserId;
use wp_api::wp_com::reader_posts::{ReaderFeaturedMedia, ReaderPost, ReaderPostTag};
use wp_api::wp_com::reader_saved::{ReaderFeedId, ReaderSavedSource};
use wp_mobile_cache::HookAction;

/// A device: its cache, which outlives each launch of the app, and its wall clock.
struct Device {
    cache: Arc<WpApiCache>,
    clock: Arc<Mutex<SystemTime>>,
}

impl Device {
    fn new() -> Self {
        Self::with_cache(migrated_in_memory_cache())
    }

    fn with_cache(cache: Arc<WpApiCache>) -> Self {
        Self {
            cache,
            // 2026-11-12, after every date the tests' server gives.
            clock: Arc::new(Mutex::new(
                SystemTime::UNIX_EPOCH + Duration::from_secs(1_794_441_600),
            )),
        }
    }

    /// Starts the app: a new service, on the same cache, for user 1.
    fn launch(&self) -> (Arc<ReaderSavedService>, Arc<ScriptedExecutor>) {
        self.launch_as(1)
    }

    fn launch_as(&self, user_id: u64) -> (Arc<ReaderSavedService>, Arc<ScriptedExecutor>) {
        let executor = Arc::new(ScriptedExecutor::default());
        let clock = self.clock.clone();
        let service = ReaderSavedService::with_clock(
            delegate(executor.clone()),
            ReaderSavedSource::Android,
            self.cache.clone(),
            WpComUserId(user_id),
            Arc::new(move || *clock.lock().unwrap()),
        )
        .unwrap();
        (Arc::new(service), executor)
    }

    fn advance(&self, by: Duration) {
        *self.clock.lock().unwrap() += by;
    }

    /// A device whose cache is a database file, as the apps' is.
    fn on_file(path: &std::path::Path) -> Self {
        Self::with_cache(open_file_cache(path))
    }

    /// Quits the app: the cache is closed, and opened again from its file.
    fn reopen_file(&mut self, path: &std::path::Path) {
        self.cache = open_file_cache(path);
    }
}

fn open_file_cache(path: &std::path::Path) -> Arc<WpApiCache> {
    let cache = WpApiCache::new(Some(path.to_str().unwrap().to_string())).unwrap();
    cache.perform_migrations().unwrap();
    Arc::new(cache)
}

async fn saved(service: &ReaderSavedService, executor: &ScriptedExecutor, post_id: u64) {
    executor.push(Step::Respond(
        200,
        write_response(true, post_json(post_id), 1),
    ));
    let outcome = service.save(identity(post_id)).await.unwrap();
    assert!(matches!(outcome, ReaderSavedOutcome::Synced { .. }));
}

fn listed(post_id: u64, date_saved: &str) -> Value {
    json!({
        "ID": post_id,
        "site_ID": 123,
        "is_external": false,
        "title": format!("Post {post_id}"),
        "date_saved": date_saved,
    })
}

fn page(posts: &[Value], next_page_handle: Option<&str>) -> Value {
    json!({ "posts": posts, "next_page_handle": next_page_handle })
}

fn url_identity(url: &str) -> ReaderPostSaveIdentity {
    ReaderPostSaveIdentity {
        id: 9,
        site_id: Some(WpComSiteId(0)),
        feed_id: Some(ReaderFeedId(0)),
        feed_item_id: None,
        is_external: true,
        url: Some(url.to_string()),
    }
}

fn local_save(identity: ReaderPostSaveIdentity, title: &str) -> ReaderLocalSave {
    ReaderLocalSave {
        identity,
        date_saved: Some("2026-11-01T00:00:00Z".parse().unwrap()),
        title: Some(title.to_string()),
        excerpt: None,
        featured_image: None,
        site_name: Some("A site".to_string()),
        site_url: None,
    }
}

fn titles(items: &[ReaderSavedListItem]) -> Vec<Option<String>> {
    items
        .iter()
        .map(|item| item.post.as_ref().and_then(|post| post.title.clone()))
        .collect()
}

fn keys_of(items: &[ReaderSavedListItem]) -> Vec<ReaderSavedKey> {
    items.iter().map(|item| item.key.clone()).collect()
}

async fn shown(service: &Arc<ReaderSavedService>) -> Vec<ReaderSavedKey> {
    keys_of(&service.clone().saved_posts().load_items().await.unwrap())
}

fn status(service: &ReaderSavedService, post: ReaderPostSaveIdentity) -> ReaderSavedStatus {
    service.status(post).unwrap()
}

fn hook(table: DbTable) -> UpdateHook {
    UpdateHook {
        action: HookAction::Insert,
        db_name: "main".to_string(),
        table,
        row_id: 1,
    }
}

// Relaunching

#[tokio::test]
async fn the_saved_keys_answer_status_after_a_relaunch_before_any_sync() {
    let device = Device::new();
    let (service, executor) = device.launch();
    synced(&service, &executor, &[post_json(1), post_json(2)]).await;
    drop(service);

    let (service, executor) = device.launch();

    assert_eq!(status(&service, identity(1)), Saved);
    assert_eq!(status(&service, identity(3)), NotSaved);
    assert_eq!(service.saved_count().unwrap(), Some(2));
    assert!(executor.sent().is_empty());
}

#[tokio::test]
async fn a_save_answered_just_before_a_relaunch_reads_saved_after_it() {
    let device = Device::new();
    let (service, executor) = device.launch();
    synced(&service, &executor, &[]).await;
    saved(&service, &executor, 1).await;
    drop(service);

    let (service, executor) = device.launch();
    assert_eq!(status(&service, identity(1)), Saved);
    assert_eq!(shown(&service).await, vec![post_key(1)]);
    assert_eq!(service.saved_count().unwrap(), Some(1));

    synced(&service, &executor, &[post_json(1)]).await;
    assert_eq!(status(&service, identity(1)), Saved);
    assert_eq!(shown(&service).await, vec![post_key(1)]);
}

/// The app is killed with the save in flight. #1729 gave it up at the next launch and let a read
/// after 60s decide; now the sync at launch says the server lacks it, and it's sent again.
#[tokio::test]
async fn a_save_in_flight_when_the_app_quit_is_sent_again_at_the_next_launch() {
    let device = Device::new();
    let (service, executor) = device.launch();
    synced(&service, &executor, &[]).await;
    let (_release, held) = oneshot::channel();
    executor.push(Step::Hold(held, 200, write_response(true, post_json(1), 1)));
    let mut save = Box::pin(service.save(identity(1)));
    assert!(futures::poll!(&mut save).is_pending());
    drop(save);
    drop(service);

    let (service, executor) = device.launch();
    assert_eq!(status(&service, identity(1)), Saved);
    let items = service.clone().saved_posts().load_items().await.unwrap();
    assert_eq!(keys_of(&items), vec![post_key(1)]);
    assert!(items[0].is_pending);

    executor.push(Step::Respond(200, keys_response(&[])));
    executor.push(Step::Respond(200, write_response(true, post_json(1), 1)));
    service.sync().await.unwrap();

    assert_eq!(
        executor.sent_short(),
        ["keys", r#"new {"blog_id":123,"post_id":1}"#]
    );
    let items = service.clone().saved_posts().load_items().await.unwrap();
    assert!(!items[0].is_pending);
}

#[tokio::test]
async fn a_post_saved_by_its_ids_is_found_by_its_url_after_a_relaunch() {
    let device = Device::new();
    let (service, executor) = device.launch();
    let mut with_url = post_json(1);
    with_url["url"] = json!("https://example.com/post-1");
    synced(&service, &executor, &[with_url, feed_json(7)]).await;
    drop(service);

    let (service, _) = device.launch();

    assert_eq!(
        status(&service, url_identity("https://example.com/post-1/")),
        Saved
    );
    assert_eq!(status(&service, rss_identity(7)), Saved);
}

#[tokio::test]
async fn everything_survives_closing_and_reopening_the_database_file() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("app.sqlite");
    let mut device = Device::on_file(&path);
    let (service, executor) = device.launch();
    executor.push(Step::Respond(
        200,
        keys_response(&[post_json(1), post_json(3)]),
    ));
    executor.push(Step::Respond(
        200,
        page(
            &[
                listed(3, "2026-11-03T00:00:00Z"),
                listed(1, "2026-11-01T00:00:00Z"),
            ],
            None,
        ),
    ));
    service.clone().saved_posts().refresh().await.unwrap();
    saved(&service, &executor, 2).await;
    executor.push(Step::Respond(200, write_response(false, post_json(3), 2)));
    service.unsave(identity(3)).await.unwrap();
    // A save the device couldn't send yet.
    device.advance(Duration::from_secs(1));
    executor.push(Step::Offline);
    service.save(identity(4)).await.unwrap();
    drop((service, executor));

    device.reopen_file(&path);
    let (service, executor) = device.launch();

    assert_eq!(status(&service, identity(1)), Saved);
    assert_eq!(status(&service, identity(2)), Saved);
    assert_eq!(status(&service, identity(3)), NotSaved);
    assert_eq!(status(&service, identity(4)), Saved);
    let items = service.clone().saved_posts().load_items().await.unwrap();
    assert_eq!(keys_of(&items), vec![post_key(4), post_key(2), post_key(1)]);
    assert_eq!(
        items.iter().map(|item| item.is_pending).collect::<Vec<_>>(),
        [true, false, false]
    );
    assert!(executor.sent().is_empty());
}

// Signing out

#[tokio::test]
async fn removing_the_account_removes_what_the_service_stored() {
    let device = Device::new();
    let (service, executor) = device.launch();
    executor.push(Step::Respond(200, keys_response(&[post_json(1)])));
    executor.push(Step::Respond(
        200,
        page(&[listed(1, "2026-11-01T00:00:00Z")], None),
    ));
    service.clone().saved_posts().refresh().await.unwrap();
    let (other, other_executor) = device.launch_as(2);
    synced(&other, &other_executor, &[post_json(1)]).await;
    drop(service);

    assert!(
        device
            .cache
            .remove_wordpress_com_account(WpComUserId(1))
            .unwrap()
    );

    let (service, _) = device.launch();
    assert_eq!(status(&service, identity(1)), Unknown);
    assert!(shown(&service).await.is_empty());
    let (other, _) = device.launch_as(2);
    assert_eq!(status(&other, identity(1)), Saved);
}

// The saved list

#[tokio::test]
async fn the_saved_list_loads_page_by_page_and_survives_a_relaunch() {
    let device = Device::new();
    let (service, executor) = device.launch();
    let collection = service.clone().saved_posts();
    assert!(collection.load_items().await.unwrap().is_empty());
    assert_eq!(collection.list_info().unwrap().has_more, None);

    executor.push(Step::Respond(
        200,
        keys_response(&[post_json(1), post_json(2), post_json(3)]),
    ));
    executor.push(Step::Respond(
        200,
        page(
            &[
                listed(3, "2026-11-03T00:00:00Z"),
                listed(2, "2026-11-02T00:00:00Z"),
            ],
            Some("next"),
        ),
    ));
    let result = collection.refresh().await.unwrap();
    assert_eq!(
        result,
        ReaderSavedPageResult {
            post_count: 2,
            has_more: true
        }
    );

    executor.push(Step::Respond(
        200,
        page(&[listed(1, "2026-11-01T00:00:00Z")], None),
    ));
    let result = collection.load_next_page().await.unwrap();
    assert!(!result.has_more);
    assert!(
        executor
            .sent()
            .last()
            .unwrap()
            .url
            .contains("page_handle=next")
    );

    let items = collection.load_items().await.unwrap();
    assert_eq!(
        titles(&items),
        vec![
            Some("Post 3".to_string()),
            Some("Post 2".to_string()),
            Some("Post 1".to_string())
        ]
    );
    assert_eq!(items[0].key, post_key(3));
    assert_eq!(
        items[0].date_saved,
        Some("2026-11-03T00:00:00Z".parse().unwrap())
    );
    let info = collection.list_info().unwrap();
    assert_eq!(info.state, ListState::Idle);
    assert_eq!(info.has_more, Some(false));
    assert!(info.last_refreshed_at.is_some());
    drop((collection, service));

    // Offline, the whole list is still there.
    let (service, executor) = device.launch();
    assert_eq!(shown(&service).await.len(), 3);
    assert!(executor.sent().is_empty());
}

#[tokio::test]
async fn a_save_shows_first_and_an_unsave_hides_at_once() {
    let device = Device::new();
    let (service, executor) = device.launch();
    let collection = service.clone().saved_posts();
    executor.push(Step::Respond(
        200,
        keys_response(&[post_json(1), post_json(2)]),
    ));
    executor.push(Step::Respond(
        200,
        page(
            &[
                listed(2, "2026-11-02T00:00:00Z"),
                listed(1, "2026-11-01T00:00:00Z"),
            ],
            None,
        ),
    ));
    collection.refresh().await.unwrap();

    saved(&service, &executor, 4).await;
    executor.push(Step::Respond(200, write_response(false, post_json(2), 2)));
    service.unsave(identity(2)).await.unwrap();

    let items = collection.load_items().await.unwrap();
    assert_eq!(keys_of(&items), vec![post_key(4), post_key(1)]);
    assert!(!items[0].is_pending);
    assert_eq!(items[0].post, None, "The saved list hasn't sent it yet");
}

/// #1729 also checked the unsaved post's row in `reader_stream_items`; the list doesn't use that
/// table any more.
#[tokio::test]
async fn a_refresh_keeps_saved_posts_past_the_first_page_and_drops_unsaved_ones() {
    let device = Device::new();
    let (service, executor) = device.launch();
    let collection = service.clone().saved_posts();
    executor.push(Step::Respond(
        200,
        keys_response(&[post_json(1), post_json(2), post_json(3)]),
    ));
    executor.push(Step::Respond(
        200,
        page(
            &[
                listed(3, "2026-11-03T00:00:00Z"),
                listed(2, "2026-11-02T00:00:00Z"),
                listed(1, "2026-11-01T00:00:00Z"),
            ],
            None,
        ),
    ));
    collection.refresh().await.unwrap();

    // Post 2 was unsaved elsewhere, and post 1 is past the first page now.
    executor.push(Step::Respond(
        200,
        keys_response(&[post_json(1), post_json(3)]),
    ));
    executor.push(Step::Respond(
        200,
        page(&[listed(3, "2026-11-03T00:00:00Z")], Some("next")),
    ));
    collection.refresh().await.unwrap();

    assert_eq!(
        keys_of(&collection.load_items().await.unwrap()),
        vec![post_key(3), post_key(1)]
    );
}

#[tokio::test]
async fn a_failed_refresh_is_the_lists_error() {
    let device = Device::new();
    let (service, executor) = device.launch();
    let collection = service.clone().saved_posts();
    executor.push(Step::Respond(500, error_response()));

    assert!(collection.refresh().await.is_err());

    let info = collection.list_info().unwrap();
    assert_eq!(info.state, ListState::Error);
    assert!(info.error_message.is_some());
}

/// Replaces #1729's two `StaleLoadMore` tests: a load more started during a refresh waits for it,
/// then loads the page after the refresh's first page.
#[tokio::test]
async fn a_load_more_during_a_refresh_waits_and_loads_the_page_after_it() {
    let device = Device::new();
    let (service, executor) = device.launch();
    let collection = service.clone().saved_posts();
    executor.push(Step::Respond(200, keys_response(&[post_json(1)])));
    executor.push(Step::Respond(
        200,
        page(&[listed(1, "2026-11-01T00:00:00Z")], Some("old")),
    ));
    collection.refresh().await.unwrap();

    let (release, held) = oneshot::channel();
    executor.push(Step::Hold(
        held,
        200,
        keys_response(&[post_json(1), post_json(2)]),
    ));
    executor.push(Step::Respond(
        200,
        page(&[listed(2, "2026-11-02T00:00:00Z")], Some("new")),
    ));
    executor.push(Step::Respond(
        200,
        page(&[listed(1, "2026-11-01T00:00:00Z")], None),
    ));
    let mut refresh = Box::pin(collection.refresh());
    let mut next = Box::pin(collection.load_next_page());
    assert!(futures::poll!(refresh.as_mut()).is_pending());
    assert!(futures::poll!(next.as_mut()).is_pending());
    release.send(()).unwrap();
    let (refreshed, next) = futures::join!(refresh, next);
    refreshed.unwrap();
    assert!(!next.unwrap().has_more);

    let urls: Vec<_> = executor
        .sent()
        .iter()
        .map(|sent| sent.url.clone())
        .collect();
    assert!(!urls.iter().any(|url| url.contains("page_handle=old")));
    assert!(urls.last().unwrap().contains("page_handle=new"));
    assert_eq!(
        keys_of(&collection.load_items().await.unwrap()),
        vec![post_key(2), post_key(1)]
    );
    let info = collection.list_info().unwrap();
    assert_eq!(info.has_more, Some(false));
    assert_eq!(info.state, ListState::Idle);
}

/// The refused unsave puts the post back, and the refresh that waited for it keeps it.
#[tokio::test]
async fn a_refresh_keeps_a_post_whose_unsave_is_refused_while_it_waits() {
    let device = Device::new();
    let (service, executor) = device.launch();
    let collection = service.clone().saved_posts();
    executor.push(Step::Respond(200, keys_response(&[post_json(1)])));
    executor.push(Step::Respond(
        200,
        page(&[listed(1, "2026-11-01T00:00:00Z")], None),
    ));
    collection.refresh().await.unwrap();

    let (release, held) = oneshot::channel();
    executor.push(Step::Hold(held, 400, error_response()));
    executor.push(Step::Respond(200, keys_response(&[post_json(1)])));
    executor.push(Step::Respond(200, page(&[], None)));
    let (unsaved, ()) = futures::join!(service.unsave(identity(1)), async {
        let mut refresh = Box::pin(collection.refresh());
        assert!(futures::poll!(refresh.as_mut()).is_pending());
        release.send(()).unwrap();
        refresh.await.unwrap();
    });

    assert!(unsaved.is_err(), "The server refused the unsave");
    assert_eq!(
        keys_of(&collection.load_items().await.unwrap()),
        vec![post_key(1)]
    );
}

#[test]
fn the_collection_is_relevant_to_the_reader_saved_tables_only() {
    let device = Device::new();
    let (service, _) = device.launch();
    let collection = service.saved_posts();
    for table in [
        DbTable::ReaderPosts,
        DbTable::ReaderStreams,
        DbTable::ReaderSavedState,
        DbTable::ReaderSavedPosts,
    ] {
        assert!(collection.is_relevant_update(&hook(table)), "{table}");
    }
    for table in [
        DbTable::ReaderStreamItems,
        DbTable::PostsEditContext,
        DbTable::ListMetadataItems,
        DbTable::WordPressComAccounts,
    ] {
        assert!(!collection.is_relevant_update(&hook(table)), "{table}");
    }
}

// The product goal: a saved post is never missing from the list on a quick save then reload

/// Saves a post and reloads the saved list while the save is in flight. The reload waits its turn,
/// and the post shows from the cache the whole time: while it's in flight, once it's answered and
/// reloaded, and after a relaunch.
///
/// #1729 answered the reload while the save was in flight, with reads that didn't have it. That
/// can't happen now: the reload's reads start after the save's answer, and were measured to
/// include it. A read that lagged behind an answered save would drop it; see the report.
#[tokio::test]
async fn a_post_saved_and_reloaded_at_once_shows_in_the_list_throughout() {
    let device = Device::new();
    let (service, executor) = device.launch();
    let collection = service.clone().saved_posts();
    let server_has = |executor: &ScriptedExecutor, posts: &[u64]| {
        let keys: Vec<_> = posts.iter().map(|id| post_json(*id)).collect();
        let listed: Vec<_> = posts
            .iter()
            .map(|id| listed(*id, &format!("2026-11-{:02}T00:00:00Z", id)))
            .collect();
        executor.push(Step::Respond(200, keys_response(&keys)));
        executor.push(Step::Respond(200, page(&listed, None)));
    };
    server_has(&executor, &[1]);
    collection.refresh().await.unwrap();

    // The save is in flight, and the reload waits for it.
    let (release, held) = oneshot::channel();
    executor.push(Step::Hold(held, 200, write_response(true, post_json(4), 2)));
    server_has(&executor, &[4, 1]);
    let mut save = Box::pin(service.save(identity(4)));
    assert!(futures::poll!(&mut save).is_pending());
    let mut reload = Box::pin(collection.refresh());
    assert!(futures::poll!(&mut reload).is_pending());
    let items = collection.load_items().await.unwrap();
    assert!(items[0].is_pending);
    assert_eq!(keys_of(&items), vec![post_key(4), post_key(1)]);

    // Answered, and then the reload reads.
    release.send(()).unwrap();
    let (saved, reloaded) = futures::join!(save, reload);
    saved.unwrap();
    reloaded.unwrap();
    let items = collection.load_items().await.unwrap();
    assert!(!items[0].is_pending);
    assert_eq!(keys_of(&items), vec![post_key(4), post_key(1)]);

    // After a relaunch.
    drop((collection, service));
    let (service, executor) = device.launch();
    let collection = service.clone().saved_posts();
    assert_eq!(
        keys_of(&collection.load_items().await.unwrap()),
        vec![post_key(4), post_key(1)]
    );
    server_has(&executor, &[4, 1]);
    collection.refresh().await.unwrap();
    assert_eq!(
        keys_of(&collection.load_items().await.unwrap()),
        vec![post_key(4), post_key(1)]
    );
}

/// The same when the save can't be sent: offline, then a relaunch, then a reload that still has
/// no network. The post shows throughout, and the first reload online sends it.
#[tokio::test]
async fn a_post_saved_offline_shows_in_the_list_through_a_relaunch_until_it_is_sent() {
    let device = Device::new();
    let (service, executor) = device.launch();
    synced(&service, &executor, &[post_json(1)]).await;
    executor.push(Step::Offline);
    service.save(identity(4)).await.unwrap();
    drop(service);

    let (service, executor) = device.launch();
    let collection = service.clone().saved_posts();
    // The save, then the keys.
    executor.push(Step::Offline);
    executor.push(Step::Offline);
    assert!(collection.refresh().await.is_err());
    // Post 1 is only in the keys, which the list doesn't show until a page has it.
    assert_eq!(
        keys_of(&collection.load_items().await.unwrap()),
        vec![post_key(4)]
    );

    executor.push(Step::Respond(200, write_response(true, post_json(4), 2)));
    executor.push(Step::Respond(
        200,
        keys_response(&[post_json(4), post_json(1)]),
    ));
    executor.push(Step::Respond(
        200,
        page(
            &[
                listed(4, "2026-11-04T00:00:00Z"),
                listed(1, "2026-11-01T00:00:00Z"),
            ],
            None,
        ),
    ));
    collection.refresh().await.unwrap();
    let items = collection.load_items().await.unwrap();
    assert_eq!(keys_of(&items), vec![post_key(4), post_key(1)]);
    assert!(!items[0].is_pending);
}

/// Unsaves a post and reloads the saved list while the unsave is in flight. The collection never
/// shows it.
#[tokio::test]
async fn a_post_unsaved_and_reloaded_at_once_stays_hidden() {
    let device = Device::new();
    let (service, executor) = device.launch();
    let collection = service.clone().saved_posts();
    executor.push(Step::Respond(
        200,
        keys_response(&[post_json(2), post_json(1)]),
    ));
    executor.push(Step::Respond(
        200,
        page(
            &[
                listed(2, "2026-11-02T00:00:00Z"),
                listed(1, "2026-11-01T00:00:00Z"),
            ],
            None,
        ),
    ));
    collection.refresh().await.unwrap();

    let (release, held) = oneshot::channel();
    executor.push(Step::Hold(
        held,
        200,
        write_response(false, post_json(2), 1),
    ));
    executor.push(Step::Respond(200, keys_response(&[post_json(1)])));
    executor.push(Step::Respond(
        200,
        page(&[listed(1, "2026-11-01T00:00:00Z")], None),
    ));
    let mut unsave = Box::pin(service.unsave(identity(2)));
    assert!(futures::poll!(&mut unsave).is_pending());
    let mut reload = Box::pin(collection.refresh());
    assert!(futures::poll!(&mut reload).is_pending());
    assert_eq!(
        keys_of(&collection.load_items().await.unwrap()),
        vec![post_key(1)]
    );

    release.send(()).unwrap();
    let (unsaved, reloaded) = futures::join!(unsave, reload);
    unsaved.unwrap();
    reloaded.unwrap();

    assert_eq!(
        keys_of(&collection.load_items().await.unwrap()),
        vec![post_key(1)]
    );
}

// Importing local saves

/// The server refuses a save past the account's cap. With no tap waiting on it, it stays saved and
/// waits for room, rather than being undone.
#[tokio::test]
async fn an_import_keeps_a_save_past_the_cap_waiting_for_room() {
    let device = Device::new();
    let (service, executor) = device.launch();
    synced(&service, &executor, &[]).await;
    // Oldest first: the page, then posts 1 and 2 in one batch, newest first.
    executor.push(Step::Respond(
        200,
        write_response(true, json!({ "url": "https://example.com/page" }), 1),
    ));
    executor.push(Step::Respond(
        200,
        json!({
            "results": [
                { "key": post_json(2), "saved": false, "error": "saved_limit_reached" },
                { "key": post_json(1), "saved": true },
            ],
            "total": 2,
        }),
    ));

    let report = service
        .import_saves(vec![
            local_save(identity(1), "One"),
            local_save(identity(2), "Two"),
            local_save(identity(1), "One again"),
            local_save(unsaveable_identity(), "Can't"),
            local_save(url_identity("https://example.com/page/"), "Page"),
        ])
        .await
        .unwrap();

    assert_eq!(
        report,
        ReaderSavedImportReport {
            imported: 2,
            rejected: vec![],
            pending: 0,
            waiting_for_room: 1,
            unsaveable: 1,
        }
    );
    let sent = executor.sent();
    assert_eq!(sent.len(), 3, "The keys, then the URL, then one batch");
    assert_eq!(
        sent[2].body.as_ref().unwrap()["keys"],
        json!([post_json(2), post_json(1)])
    );

    assert_eq!(status(&service, identity(2)), Saved);
    assert_eq!(service.saves_waiting_for_room().unwrap(), 1);
    let items = service.clone().saved_posts().load_items().await.unwrap();
    let waiting: Vec<_> = items
        .iter()
        .filter(|item| item.is_waiting_for_room)
        .collect();
    assert_eq!(waiting.len(), 1);
    assert!(waiting[0].is_pending);
    assert_eq!(
        waiting[0].post.as_ref().unwrap().title.as_deref(),
        Some("Two")
    );
}

/// A sync asks once whether there's room, and says the account is full. An unsave goes first and
/// makes room, and the save waiting for it goes next.
#[tokio::test]
async fn a_save_waiting_for_room_is_sent_once_an_unsave_makes_room() {
    let device = Device::new();
    let (service, executor) = device.launch();
    synced(&service, &executor, &[post_json(1)]).await;
    executor.push(Step::Respond(403, cap_response()));
    let report = service
        .import_saves(vec![local_save(identity(2), "Two")])
        .await
        .unwrap();
    assert_eq!(report.waiting_for_room, 1);

    // Still full: one try, then the keys, and no second try after them.
    executor.push(Step::Respond(403, cap_response()));
    executor.push(Step::Respond(200, keys_response(&[post_json(1)])));
    let result = service.sync().await;
    assert!(
        matches!(result, Err(ReaderSavedError::TooManySaves)),
        "{result:?}"
    );
    assert_eq!(
        executor.sent_short()[1..],
        [
            r#"new {"blog_id":123,"post_id":2}"#,
            r#"new {"blog_id":123,"post_id":2}"#,
            "keys",
        ]
    );
    assert_eq!(status(&service, identity(2)), Saved);

    executor.push(Step::Respond(200, write_response(false, post_json(1), 999)));
    executor.push(Step::Respond(200, write_response(true, post_json(2), 1000)));
    service.unsave(identity(1)).await.unwrap();

    assert_eq!(
        executor.sent_short()[4..],
        [
            r#"delete {"blog_id":123,"post_id":1}"#,
            r#"new {"blog_id":123,"post_id":2}"#,
        ]
    );
    assert_eq!(service.saves_waiting_for_room().unwrap(), 0);
    let items = service.clone().saved_posts().load_items().await.unwrap();
    assert_eq!(keys_of(&items), vec![post_key(2)]);
    assert!(!items[0].is_pending && !items[0].is_waiting_for_room);
    synced(&service, &executor, &[post_json(2)]).await;
}

/// Tapping save on a post that's already saved and waiting mustn't make the cap undo it.
#[tokio::test]
async fn saving_a_post_waiting_for_room_leaves_it_waiting() {
    let device = Device::new();
    let (service, executor) = device.launch();
    synced(&service, &executor, &[]).await;
    executor.push(Step::Respond(403, cap_response()));
    service
        .import_saves(vec![local_save(identity(2), "Two")])
        .await
        .unwrap();

    let outcome = service.save(identity(2)).await.unwrap();

    assert!(
        matches!(outcome, ReaderSavedOutcome::Pending { .. }),
        "{outcome:?}"
    );
    assert_eq!(executor.sent().len(), 2, "Nothing more was sent");
    assert_eq!(service.saves_waiting_for_room().unwrap(), 1);
}

/// The server never had it, so there's nothing to unsave there.
#[tokio::test]
async fn unsaving_a_save_waiting_for_room_removes_it_without_sending() {
    let device = Device::new();
    let (service, executor) = device.launch();
    synced(&service, &executor, &[]).await;
    executor.push(Step::Respond(403, cap_response()));
    service
        .import_saves(vec![local_save(rss_identity(7), "Feed")])
        .await
        .unwrap();
    assert_eq!(status(&service, rss_identity(7)), Saved);

    service.unsave(rss_identity(7)).await.unwrap();

    assert_eq!(executor.sent().len(), 2);
    assert_eq!(status(&service, rss_identity(7)), NotSaved);
    assert!(shown(&service).await.is_empty());
    assert_eq!(service.saves_waiting_for_room().unwrap(), 0);
    synced(&service, &executor, &[]).await;
}

#[tokio::test]
async fn an_import_that_cannot_reach_the_server_is_sent_by_a_later_sync() {
    let device = Device::new();
    let (service, executor) = device.launch();
    synced(&service, &executor, &[]).await;
    executor.push(Step::Offline);

    let report = service
        .import_saves(vec![local_save(identity(1), "One")])
        .await
        .unwrap();
    assert_eq!((report.imported, report.pending), (0, 1));
    assert_eq!(status(&service, identity(1)), Saved);

    executor.push(Step::Respond(200, write_response(true, post_json(1), 1)));
    synced(&service, &executor, &[post_json(1)]).await;
    let items = service.clone().saved_posts().load_items().await.unwrap();
    assert_eq!(keys_of(&items), vec![post_key(1)]);
    assert!(!items[0].is_pending);
}

/// An import is a list of saves, so a second one of the same list, even after a relaunch, sends
/// nothing, and one with more posts sends only those.
#[tokio::test]
async fn importing_again_sends_only_what_is_new() {
    let device = Device::new();
    let (service, executor) = device.launch();
    synced(&service, &executor, &[]).await;
    executor.push(Step::Respond(200, write_response(true, post_json(1), 1)));
    service
        .import_saves(vec![local_save(identity(1), "One")])
        .await
        .unwrap();

    drop(service);
    let (service, executor) = device.launch();
    let report = service
        .import_saves(vec![local_save(identity(1), "One")])
        .await
        .unwrap();
    assert_eq!((report.imported, report.pending), (1, 0));
    assert!(executor.sent().is_empty());

    executor.push(Step::Respond(200, write_response(true, post_json(2), 2)));
    let report = service
        .import_saves(vec![
            local_save(identity(2), "Two"),
            local_save(identity(1), "One"),
        ])
        .await
        .unwrap();
    assert_eq!(report.imported, 2);
    assert_eq!(
        executor.sent_short(),
        [r#"new {"blog_id":123,"post_id":2}"#]
    );
}

/// An import is an add, as a tap of save is.
#[tokio::test]
async fn importing_a_post_the_user_unsaved_saves_it_again() {
    let device = Device::new();
    let (service, executor) = device.launch();
    synced(&service, &executor, &[post_json(1)]).await;
    executor.push(Step::Respond(200, write_response(false, post_json(1), 0)));
    service.unsave(identity(1)).await.unwrap();
    assert_eq!(status(&service, identity(1)), NotSaved);

    executor.push(Step::Respond(200, write_response(true, post_json(1), 1)));
    let report = service
        .import_saves(vec![local_save(identity(1), "One")])
        .await
        .unwrap();

    assert_eq!(report.imported, 1);
    assert_eq!(status(&service, identity(1)), Saved);
    assert_eq!(
        executor.sent_short()[2..],
        [r#"new {"blog_id":123,"post_id":1}"#]
    );
}

/// #1729 failed the import, and it ran again. Now a rate limit leaves the save queued.
#[tokio::test]
async fn a_url_save_that_is_rate_limited_during_import_is_sent_by_a_later_sync() {
    let device = Device::new();
    let (service, executor) = device.launch();
    synced(&service, &executor, &[]).await;
    executor.push(Step::Respond(
        429,
        json!({ "code": "rate_limit_exceeded", "message": "Slow down.", "data": { "status": 429 } }),
    ));

    let report = service
        .import_saves(vec![local_save(url_identity("https://example.com/a"), "A")])
        .await
        .unwrap();
    assert_eq!((report.imported, report.pending), (0, 1));
    assert!(report.rejected.is_empty());

    executor.push(Step::Respond(
        200,
        write_response(true, json!({ "url": "https://example.com/a" }), 1),
    ));
    executor.push(Step::Respond(
        200,
        keys_response(&[json!({ "url": "https://example.com/a" })]),
    ));
    service.sync().await.unwrap();
    assert_eq!(executor.sent().len(), 4);
    assert!(!service.clone().saved_posts().load_items().await.unwrap()[0].is_pending);
}

/// Refused for something other than the cap, an imported save is undone, as a tap's is.
#[tokio::test]
async fn a_url_the_server_refuses_is_undone_and_reported() {
    let device = Device::new();
    let (service, executor) = device.launch();
    synced(&service, &executor, &[]).await;
    executor.push(Step::Respond(
        400,
        json!({ "code": "invalid_url", "message": "Invalid URL.", "data": { "status": 400 } }),
    ));

    let report = service
        .import_saves(vec![local_save(url_identity("https://example.com/a"), "A")])
        .await
        .unwrap();

    let key = ReaderSavedKey::Url {
        url: "https://example.com/a".to_string(),
    };
    assert_eq!(report.rejected, vec![key]);
    assert_eq!((report.imported, report.waiting_for_room), (0, 0));
    assert!(shown(&service).await.is_empty());
}

/// A single save goes as a save rather than a batch.
#[tokio::test]
async fn two_imports_at_once_send_the_saves_once() {
    let device = Device::new();
    let (service, executor) = device.launch();
    synced(&service, &executor, &[]).await;
    // The first import waits on the server while the second starts.
    let (release, held) = oneshot::channel();
    executor.push(Step::Hold(held, 200, write_response(true, post_json(1), 1)));

    let (first, second, ()) = futures::join!(
        service.import_saves(vec![local_save(identity(1), "One")]),
        service.import_saves(vec![local_save(identity(1), "One")]),
        async { release.send(()).unwrap() },
    );

    assert_eq!(
        [first.unwrap(), second.unwrap()].map(|report| report.imported),
        [1, 1]
    );
    assert_eq!(executor.sent().len(), 2, "The keys, then one save");
}

/// Oldest first, 100 at a time, each batch newest first.
#[tokio::test]
async fn an_import_is_sent_in_batches_oldest_first() {
    let device = Device::new();
    let (service, executor) = device.launch();
    synced(&service, &executor, &[]).await;
    let saves: Vec<_> = (1..=150)
        .rev()
        .map(|id| local_save(identity(id), "A post"))
        .collect();
    let oldest: Vec<_> = (1..=100).rev().map(post_json).collect();
    let newest: Vec<_> = (101..=150).rev().map(post_json).collect();
    executor.push(Step::Respond(200, bulk_response(&oldest, 100)));
    executor.push(Step::Respond(200, bulk_response(&newest, 150)));

    let report = service.import_saves(saves).await.unwrap();

    assert_eq!(report.imported, 150);
    let sent = executor.sent();
    assert_eq!(sent.len(), 3);
    assert_eq!(sent[1].body.as_ref().unwrap()["keys"], json!(oldest));
    assert_eq!(sent[2].body.as_ref().unwrap()["keys"], json!(newest));
    assert_eq!(service.saved_count().unwrap(), Some(150));
}

/// The server sent a result too few, so none can be matched to its key. They stay in doubt, and a
/// sync says what the server has.
#[tokio::test]
async fn a_batch_with_the_wrong_number_of_results_waits_for_a_sync() {
    let device = Device::new();
    let (service, executor) = device.launch();
    synced(&service, &executor, &[]).await;
    executor.push(Step::Respond(200, bulk_response(&[post_json(2)], 1)));

    let result = service
        .import_saves(vec![
            local_save(identity(2), "Two"),
            local_save(identity(1), "One"),
        ])
        .await;

    assert!(matches!(
        result,
        Err(ReaderSavedError::UnexpectedImportResults {
            sent: 2,
            received: 1
        })
    ));
    assert_eq!(status(&service, identity(1)), Saved);
    assert_eq!(status(&service, identity(2)), Saved);

    // The server saved post 2 only, and the sync sends post 1 again.
    executor.push(Step::Respond(200, keys_response(&[post_json(2)])));
    executor.push(Step::Respond(200, write_response(true, post_json(1), 2)));
    service.sync().await.unwrap();
    assert_eq!(
        executor.sent_short()[2..],
        ["keys", r#"new {"blog_id":123,"post_id":1}"#]
    );
    assert_eq!(service.saved_count().unwrap(), Some(2));
}

// Carried over from #1729's last commits

/// The device's copy of a post can be older than the one the saved list stored, so the import only
/// fills in what the list's copy doesn't have.
#[tokio::test]
async fn importing_a_local_save_doesnt_replace_what_the_saved_list_stored() {
    let device = Device::new();
    let (service, executor) = device.launch();
    let collection = service.clone().saved_posts();
    executor.push(Step::Respond(200, keys_response(&[post_json(1)])));
    executor.push(Step::Respond(
        200,
        page(&[listed(1, "2026-11-01T00:00:00Z")], None),
    ));
    collection.refresh().await.unwrap();

    // Post 1 is already saved here, so the import adds nothing to send.
    let mut save = local_save(identity(1), "An older title");
    save.excerpt = Some("An excerpt only the device has".to_string());
    service.import_saves(vec![save]).await.unwrap();

    let items = collection.load_items().await.unwrap();
    let post = items[0].post.as_ref().unwrap();
    assert_eq!(post.title.as_deref(), Some("Post 1"));
    assert_eq!(
        post.excerpt.as_deref(),
        Some("An excerpt only the device has")
    );
    assert_eq!(executor.sent().len(), 2);
}

/// A key with an ID of `0` would be refused, so it isn't imported or sent, as the server would
/// answer it with `invalid_input`.
#[tokio::test]
async fn an_import_leaves_out_a_key_with_a_zero_id() {
    let device = Device::new();
    let (service, executor) = device.launch();
    synced(&service, &executor, &[]).await;
    executor.push(Step::Respond(200, write_response(true, post_json(1), 1)));

    let report = service
        .import_saves(vec![
            local_save(rss_identity(0), "Zero"),
            local_save(identity(1), "One"),
        ])
        .await
        .unwrap();

    assert_eq!((report.imported, report.unsaveable), (1, 1));
    assert_eq!(
        executor.sent_short()[1..],
        [r#"new {"blog_id":123,"post_id":1}"#]
    );
    assert_eq!(shown(&service).await, vec![post_key(1)]);
}

#[derive(Default)]
struct Hooks(Mutex<Vec<DbTable>>);

impl wp_mobile_cache::DatabaseDelegate for Hooks {
    fn did_update(&self, update_hook: UpdateHook) {
        self.0.lock().unwrap().push(update_hook.table);
    }
}

/// The collection reloads when its tables change, so a sync or a list refresh that changes nothing
/// mustn't say they did. Only `reader_posts` is written, because each page stores its posts.
#[tokio::test]
async fn a_sync_and_a_refresh_that_change_nothing_tell_the_saved_list_nothing() {
    let device = Device::new();
    let (service, executor) = device.launch();
    let collection = service.clone().saved_posts();
    let mut with_url = post_json(1);
    with_url["url"] = json!("https://example.com/post-1");
    let keys = keys_response(&[with_url, post_json(2), feed_json(7)]);
    let listed = page(
        &[
            listed(2, "2026-11-02T00:00:00Z"),
            listed(1, "2026-11-01T00:00:00Z"),
        ],
        None,
    );
    executor.push(Step::Respond(200, keys.clone()));
    executor.push(Step::Respond(200, listed.clone()));
    collection.refresh().await.unwrap();

    let hooks = Arc::new(Hooks::default());
    device.cache.start_listening_for_updates(hooks.clone());
    executor.push(Step::Respond(200, keys.clone()));
    service.sync().await.unwrap();
    assert_eq!(*hooks.0.lock().unwrap(), Vec::<DbTable>::new());

    executor.push(Step::Respond(200, keys));
    executor.push(Step::Respond(200, listed));
    collection.refresh().await.unwrap();
    let seen = hooks.0.lock().unwrap().clone();
    device.cache.stop_listening_for_updates();
    assert!(
        seen.iter()
            .all(|table| *table == DbTable::ReaderPosts || *table == DbTable::ReaderStreams),
        "{seen:?}"
    );
}

#[tokio::test]
async fn the_saved_list_keeps_each_posts_site_icon_tags_and_comment_count() {
    let device = Device::new();
    let (service, executor) = device.launch();
    let collection = service.clone().saved_posts();
    executor.push(Step::Respond(
        200,
        keys_response(&[post_json(1), post_json(2)]),
    ));
    let mut post = listed(2, "2026-11-02T00:00:00Z");
    post["site_icon"] =
        json!({ "img": "https://example.com/icon.png", "ico": "https://example.com/favicon.ico" });
    post["tags"] = json!({ "news": { "slug": "news", "name": "News", "post_count": 3 } });
    post["discussion"] = json!({ "comment_count": 5, "comments_open": true });
    executor.push(Step::Respond(200, page(&[post], Some("next"))));
    collection.refresh().await.unwrap();

    // Like an RSS post: no icon of its own, but its feed has an image.
    let mut without_icon = listed(1, "2026-11-01T00:00:00Z");
    without_icon["site_icon"] = json!([]);
    without_icon["tags"] = json!({});
    without_icon["meta"] =
        json!({ "data": { "feed": { "image": "https://example.com/feed.png?w=150&amp;h=150" } } });
    executor.push(Step::Respond(200, page(&[without_icon], None)));
    collection.load_next_page().await.unwrap();

    let sent = executor.sent();
    let [_, first_page, next_page] = &sent[..] else {
        panic!("Expected the keys and two pages, not {sent:?}");
    };
    for request in [first_page, next_page] {
        assert!(request.url.contains("meta=site%2Cfeed"), "{}", request.url);
    }
    drop((collection, service));

    // Offline, the details are still there.
    let (service, _) = device.launch();
    let items = service.saved_posts().load_items().await.unwrap();
    let posts: Vec<_> = items
        .iter()
        .map(|item| item.post.clone().unwrap())
        .collect();
    let [post, without_icon] = &posts[..] else {
        panic!("Expected two posts, not {posts:?}");
    };
    assert_eq!(
        post.site_icon_url.as_deref(),
        Some("https://example.com/icon.png")
    );
    assert_eq!(
        post.tags,
        Some(vec![ReaderPostTag {
            slug: "news".to_string(),
            name: "News".to_string(),
            display_name: None,
            post_count: Some(3),
        }])
    );
    assert_eq!(post.comment_count, Some(5));
    assert_eq!(
        without_icon.site_icon_url.as_deref(),
        Some("https://example.com/feed.png?w=150&h=150")
    );
    assert_eq!(without_icon.tags, Some(vec![]));
    assert_eq!(without_icon.comment_count, None);
}

#[tokio::test]
async fn the_saved_list_shows_an_image_for_a_post_without_a_featured_image() {
    let device = Device::new();
    let (service, executor) = device.launch();
    let collection = service.clone().saved_posts();
    executor.push(Step::Respond(
        200,
        keys_response(&[post_json(1), post_json(2)]),
    ));
    // Like a post whose author set no featured image: the server picks one from the post.
    let mut from_media = listed(2, "2026-11-02T00:00:00Z");
    from_media["featured_image"] = json!("");
    from_media["featured_media"] = json!({
        "uri": "https://example.com/from-post.jpg",
        "width": 2560,
        "height": 1706,
        "type": "image",
    });
    let mut featured = listed(1, "2026-11-01T00:00:00Z");
    featured["featured_image"] = json!("https://example.com/featured.jpg");
    executor.push(Step::Respond(200, page(&[from_media, featured], None)));
    collection.refresh().await.unwrap();

    let items = collection.load_items().await.unwrap();
    let images: Vec<_> = items
        .iter()
        .map(|item| {
            let post = item.post.as_ref().unwrap();
            (post.featured_image.clone(), post.display_image_url.clone())
        })
        .collect();
    assert_eq!(
        images,
        vec![
            (None, Some("https://example.com/from-post.jpg".to_string())),
            (
                Some("https://example.com/featured.jpg".to_string()),
                Some("https://example.com/featured.jpg".to_string())
            ),
        ]
    );
}

#[tokio::test]
async fn a_post_saved_with_only_featured_media_shows_its_image() {
    let device = Device::new();
    let (service, executor) = device.launch();
    let collection = service.clone().saved_posts();
    // As the app builds it: the image it shows, as an image `featured_media`, and no display image.
    let mut post = stream_post(4, "A post");
    post.featured_image = None;
    post.display_image_url = None;
    post.featured_media = Some(ReaderFeaturedMedia {
        uri: Some("https://example.com/from-app.jpg".to_string()),
        media_type: Some("image".to_string()),
        width: None,
        height: None,
    });
    executor.push(Step::Respond(200, write_response(true, post_json(4), 1)));
    service.save_with_details(post).await.unwrap();

    let items = collection.load_items().await.unwrap();
    let post = items[0].post.as_ref().unwrap();
    assert_eq!(post.featured_image, None);
    assert_eq!(
        post.display_image_url.as_deref(),
        Some("https://example.com/from-app.jpg")
    );

    // The saved list's copy, which has none, keeps it.
    executor.push(Step::Respond(200, keys_response(&[post_json(4)])));
    executor.push(Step::Respond(
        200,
        page(&[listed(4, "2026-11-05T00:00:00Z")], None),
    ));
    collection.refresh().await.unwrap();
    let items = collection.load_items().await.unwrap();
    assert_eq!(
        items[0].post.as_ref().unwrap().display_image_url.as_deref(),
        Some("https://example.com/from-app.jpg")
    );
}

/// A post as the app has it when the user taps bookmark: one it loaded from a Reader stream.
fn stream_post(post_id: u64, title: &str) -> ReaderPost {
    serde_json::from_value(json!({
        "ID": post_id,
        "site_ID": 123,
        "is_external": false,
        "title": title,
        "excerpt": "An excerpt",
        "date": "2026-11-01T10:00:00+00:00",
        "author": { "ID": 7, "name": "An author" },
        "featured_image": "https://example.com/image.jpg",
        "site_name": "A site",
        "site_URL": "https://example.com",
        "site_icon": { "img": "https://example.com/icon.png" },
        "tags": { "news": { "slug": "news", "name": "News" } },
        "discussion": { "comment_count": 3, "comments_open": true },
    }))
    .unwrap()
}

#[tokio::test]
async fn a_post_saved_with_its_details_shows_in_full_before_any_refresh() {
    let device = Device::new();
    let (service, executor) = device.launch();
    let collection = service.clone().saved_posts();

    // While the save is in flight, the list already shows the post.
    let (release, held) = oneshot::channel();
    executor.push(Step::Hold(held, 200, write_response(true, post_json(4), 1)));
    let (result, ()) = futures::join!(
        service.save_with_details(stream_post(4, "From the stream")),
        async {
            let items = collection.load_items().await.unwrap();
            assert_eq!(titles(&items), vec![Some("From the stream".to_string())]);
            release.send(()).unwrap();
        }
    );
    assert_eq!(
        result.unwrap(),
        ReaderSavedOutcome::Synced { key: post_key(4) }
    );
    drop((collection, service));

    // Offline, after a relaunch, the row is still complete.
    let (service, executor) = device.launch();
    let collection = service.clone().saved_posts();
    let items = collection.load_items().await.unwrap();
    let [item] = &items[..] else {
        panic!("Expected the saved post, not {items:?}");
    };
    assert!(!item.is_pending, "the server has it");
    assert_eq!(item.key, post_key(4));
    let post = item
        .post
        .clone()
        .expect("The details were stored with the save");
    assert_eq!(post.identity, identity(4));
    assert_eq!(post.title.as_deref(), Some("From the stream"));
    assert_eq!(post.excerpt.as_deref(), Some("An excerpt"));
    assert_eq!(post.author_name.as_deref(), Some("An author"));
    assert_eq!(
        post.featured_image.as_deref(),
        Some("https://example.com/image.jpg")
    );
    assert_eq!(
        post.display_image_url.as_deref(),
        Some("https://example.com/image.jpg")
    );
    assert_eq!(post.site_name.as_deref(), Some("A site"));
    assert_eq!(post.site_url.as_deref(), Some("https://example.com"));
    assert_eq!(
        post.site_icon_url.as_deref(),
        Some("https://example.com/icon.png")
    );
    assert_eq!(
        post.tags.as_ref().map(|tags| tags.len()),
        Some(1),
        "{:?}",
        post.tags
    );
    assert_eq!(post.comment_count, Some(3));
    assert_eq!(post.date, Some("2026-11-01T10:00:00Z".parse().unwrap()));

    // The saved list's copy replaces the app's.
    executor.push(Step::Respond(200, keys_response(&[post_json(4)])));
    let mut from_server = listed(4, "2026-11-05T00:00:00Z");
    from_server["title"] = json!("From the server");
    from_server["discussion"] = json!({ "comment_count": 8, "comments_open": true });
    executor.push(Step::Respond(200, page(&[from_server], None)));
    collection.refresh().await.unwrap();

    let items = collection.load_items().await.unwrap();
    let [item] = &items[..] else {
        panic!("Expected the saved post, not {items:?}");
    };
    assert!(!item.is_pending);
    let post = item.post.clone().unwrap();
    assert_eq!(post.title.as_deref(), Some("From the server"));
    assert_eq!(post.comment_count, Some(8));
    assert_eq!(
        post.author_name.as_deref(),
        Some("An author"),
        "A field the saved list leaves out keeps the app's"
    );
}

#[tokio::test]
async fn details_passed_with_a_save_dont_replace_what_the_server_sent() {
    let device = Device::new();
    let (service, executor) = device.launch();
    let collection = service.clone().saved_posts();
    executor.push(Step::Respond(200, keys_response(&[post_json(1)])));
    executor.push(Step::Respond(
        200,
        page(&[listed(1, "2026-11-01T00:00:00Z")], None),
    ));
    collection.refresh().await.unwrap();
    executor.push(Step::Respond(200, write_response(false, post_json(1), 0)));
    service.unsave(identity(1)).await.unwrap();

    // The app's copy is older than the saved list's.
    executor.push(Step::Respond(200, write_response(true, post_json(1), 1)));
    service
        .save_with_details(stream_post(1, "An older title"))
        .await
        .unwrap();

    let items = collection.load_items().await.unwrap();
    assert_eq!(titles(&items), vec![Some("Post 1".to_string())]);
    let post = items[0].post.clone().unwrap();
    assert_eq!(
        post.author_name.as_deref(),
        Some("An author"),
        "What the server left out is filled in"
    );
}

#[tokio::test]
async fn empty_tags_passed_with_a_save_are_stored_as_unknown() {
    let device = Device::new();
    let (service, executor) = device.launch();
    let collection = service.clone().saved_posts();
    let mut post = stream_post(1, "A post");
    post.tags = Vec::new();
    executor.push(Step::Respond(200, write_response(true, post_json(1), 1)));
    service.save_with_details(post).await.unwrap();

    let items = collection.load_items().await.unwrap();
    assert_eq!(
        items[0].post.as_ref().unwrap().tags,
        None,
        "An app can't tell no tags from tags it doesn't have"
    );
}

#[tokio::test]
async fn empty_tags_passed_with_a_save_dont_clear_the_stored_ones() {
    let device = Device::new();
    let (service, executor) = device.launch();
    let collection = service.clone().saved_posts();
    let mut from_list = listed(1, "2026-11-01T00:00:00Z");
    from_list["tags"] = json!({ "news": { "slug": "news", "name": "News" } });
    executor.push(Step::Respond(200, keys_response(&[post_json(1)])));
    executor.push(Step::Respond(200, page(&[from_list], None)));
    collection.refresh().await.unwrap();

    let mut post = stream_post(1, "A post");
    post.tags = Vec::new();
    executor.push(Step::Respond(200, write_response(true, post_json(1), 1)));
    service.save_with_details(post).await.unwrap();

    let items = collection.load_items().await.unwrap();
    let tags = items[0].post.as_ref().unwrap().tags.as_ref().unwrap();
    let slugs: Vec<_> = tags.iter().map(|tag| tag.slug.as_str()).collect();
    assert_eq!(slugs, vec!["news"]);
}

#[tokio::test]
async fn a_failed_save_with_details_leaves_nothing_in_the_list() {
    let device = Device::new();
    let (service, executor) = device.launch();
    let collection = service.clone().saved_posts();
    executor.push(Step::Respond(404, error_response()));

    assert!(
        service
            .save_with_details(stream_post(4, "A post"))
            .await
            .is_err()
    );

    assert_eq!(collection.load_items().await.unwrap(), vec![]);
    assert_ne!(
        service.status(identity(4)).unwrap(),
        ReaderSavedStatus::Saved
    );
    // The details stay, harmlessly: the list only shows a post through a save.
    let stored: i64 = device.cache.execute(|connection| {
        connection
            .query_row("SELECT COUNT(*) FROM reader_posts", [], |row| row.get(0))
            .unwrap()
    });
    assert_eq!(stored, 1);
}

#[tokio::test]
async fn unsaving_a_post_saved_with_details_removes_it_from_the_list() {
    let device = Device::new();
    let (service, executor) = device.launch();
    let collection = service.clone().saved_posts();
    executor.push(Step::Respond(200, write_response(true, post_json(4), 1)));
    service
        .save_with_details(stream_post(4, "A post"))
        .await
        .unwrap();
    executor.push(Step::Respond(200, write_response(false, post_json(4), 0)));
    service.unsave(identity(4)).await.unwrap();

    assert_eq!(collection.load_items().await.unwrap(), vec![]);
}

#[tokio::test]
async fn a_post_that_cant_be_saved_is_refused_with_its_details() {
    let device = Device::new();
    let (service, executor) = device.launch();
    let mut post = stream_post(4, "A post");
    post.identity = unsaveable_identity();

    assert!(matches!(
        service.save_with_details(post).await,
        Err(ReaderSavedError::PostCannotBeSaved)
    ));
    assert!(executor.sent().is_empty());
}

/// The saved list stored a Jetpack post as its WordPress.com post, with its feed item. Saving its
/// feed item copy with details fills in that row, instead of a second row for the same post that
/// the save would then show.
#[tokio::test]
async fn details_saved_with_a_jetpack_posts_feed_item_copy_fill_in_its_stored_post() {
    let device = Device::new();
    let (service, executor) = device.launch();
    let collection = service.clone().saved_posts();
    executor.push(Step::Respond(200, keys_response(&[post_json(3)])));
    let mut jetpack_post = listed(3, "2026-11-01T00:00:00Z");
    jetpack_post["feed_ID"] = json!(5);
    jetpack_post["feed_item_ID"] = json!(4);
    executor.push(Step::Respond(200, page(&[jetpack_post], None)));
    collection.refresh().await.unwrap();

    let feed_item_copy: ReaderPost = serde_json::from_value(json!({
        "ID": 4,
        "feed_ID": 5,
        "feed_item_ID": 4,
        "is_external": true,
        "title": "From the feed",
        "author": { "name": "A feed author" },
    }))
    .unwrap();
    executor.push(Step::Respond(200, write_response(true, post_json(3), 1)));
    service.save_with_details(feed_item_copy).await.unwrap();

    let stored: i64 = device.cache.execute(|connection| {
        connection
            .query_row("SELECT COUNT(*) FROM reader_posts", [], |row| row.get(0))
            .unwrap()
    });
    assert_eq!(stored, 1, "one row for the post");
    let items = collection.load_items().await.unwrap();
    assert_eq!(titles(&items), vec![Some("Post 3".to_string())]);
    assert_eq!(
        items[0].post.as_ref().unwrap().author_name.as_deref(),
        Some("A feed author"),
        "What the list left out is filled in"
    );
}

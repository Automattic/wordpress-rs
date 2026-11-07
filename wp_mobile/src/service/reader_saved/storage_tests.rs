//! What the service keeps in the cache: across a relaunch, through an import, and as the saved list.

use super::super::tests::{
    ScriptedExecutor, Step, error_response, feed_json, feed_key, identity, keys_response,
    post_json, post_key, rss_identity, unsaveable_identity, write_response,
};
use super::*;
use crate::testing::EmptyAppNotifier;
use futures::channel::oneshot;
use serde_json::{Value, json};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime};
use wp_api::prelude::*;
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::me::WpComUserId;
use wp_api::wp_com::reader_saved::ReaderFeedId;
use wp_mobile_cache::HookAction;

/// Both clocks, driven by hand. A relaunch moves the monotonic clock to a new base, as a real one
/// does, while the wall clock carries on.
struct Clocks {
    instant: Mutex<Instant>,
    wall: Mutex<SystemTime>,
}

impl Clocks {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            instant: Mutex::new(Instant::now()),
            wall: Mutex::new(SystemTime::now()),
        })
    }

    fn advance(&self, by: Duration) {
        *self.instant.lock().unwrap() += by;
        *self.wall.lock().unwrap() += by;
    }

    fn relaunch(&self) {
        *self.instant.lock().unwrap() += Duration::from_secs(5_000);
    }

    fn move_wall_clock_back(&self, by: Duration) {
        *self.wall.lock().unwrap() -= by;
    }
}

struct Device {
    cache: Arc<WpApiCache>,
    clocks: Arc<Clocks>,
}

impl Device {
    fn new() -> Self {
        Self {
            cache: migrated_in_memory_cache(),
            clocks: Clocks::new(),
        }
    }

    /// Starts the app: a new service, on the same cache, for user 1.
    fn launch(&self) -> (Arc<ReaderSavedService>, Arc<ScriptedExecutor>) {
        self.launch_as(1)
    }

    fn launch_as(&self, user_id: u64) -> (Arc<ReaderSavedService>, Arc<ScriptedExecutor>) {
        let executor = Arc::new(ScriptedExecutor::default());
        let instant = self.clocks.clone();
        let wall = self.clocks.clone();
        let service = ReaderSavedService::with_clocks(
            WpApiClientDelegate {
                auth_provider: Arc::new(WpAuthenticationProvider::none()),
                request_executor: executor.clone(),
                middleware_pipeline: Arc::new(WpApiMiddlewarePipeline::default()),
                app_notifier: Arc::new(EmptyAppNotifier),
                language_provider: None,
            },
            wp_api::wp_com::reader_saved::ReaderSavedSource::Android,
            self.cache.clone(),
            WpComUserId(user_id),
            Arc::new(move || *instant.instant.lock().unwrap()),
            Arc::new(move || *wall.wall.lock().unwrap()),
        )
        .unwrap();
        (Arc::new(service), executor)
    }

    fn relaunch(&self) -> (Arc<ReaderSavedService>, Arc<ScriptedExecutor>) {
        self.clocks.relaunch();
        self.launch()
    }

    /// A device whose cache is a database file, as the apps' is.
    fn on_file(path: &std::path::Path) -> Self {
        Self {
            cache: open_file_cache(path),
            clocks: Clocks::new(),
        }
    }

    /// Quits the app and starts it again: the cache is closed, and opened again from its file.
    fn relaunch_from_file(&mut self, path: &std::path::Path) {
        self.clocks.relaunch();
        self.cache = open_file_cache(path);
    }
}

fn open_file_cache(path: &std::path::Path) -> Arc<WpApiCache> {
    let cache = WpApiCache::new(Some(path.to_str().unwrap().to_string())).unwrap();
    cache.perform_migrations().unwrap();
    Arc::new(cache)
}

async fn refreshed(service: &ReaderSavedService, executor: &ScriptedExecutor, keys: &[Value]) {
    executor.push(Step::Respond(200, keys_response(keys)));
    service.refresh().await.unwrap();
}

async fn saved(service: &ReaderSavedService, executor: &ScriptedExecutor, post_id: u64) {
    executor.push(Step::Respond(
        200,
        write_response(true, post_json(post_id), 1),
    ));
    service.save(identity(post_id)).await.unwrap();
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
async fn the_saved_keys_answer_status_after_a_relaunch_before_any_refresh() {
    let device = Device::new();
    let (service, executor) = device.launch();
    refreshed(&service, &executor, &[post_json(1), post_json(2)]).await;
    drop(service);

    let (service, executor) = device.relaunch();

    assert_eq!(service.status(identity(1)), ReaderSavedStatus::Saved);
    assert_eq!(service.status(identity(3)), ReaderSavedStatus::NotSaved);
    assert_eq!(service.saved_count(), Some(2));
    assert!(executor.sent().is_empty());
}

#[tokio::test]
async fn a_save_made_just_before_a_relaunch_wins_over_a_refresh_that_lags() {
    let device = Device::new();
    let (service, executor) = device.launch();
    refreshed(&service, &executor, &[]).await;
    saved(&service, &executor, 1).await;
    drop(service);

    device.clocks.advance(Duration::from_secs(30));
    let (service, executor) = device.relaunch();
    assert_eq!(service.status(identity(1)), ReaderSavedStatus::Saved);
    assert_eq!(service.recent_saves(), vec![post_key(1)]);

    device.clocks.advance(Duration::from_secs(30));
    refreshed(&service, &executor, &[]).await;

    assert_eq!(service.status(identity(1)), ReaderSavedStatus::Saved);
}

#[tokio::test]
async fn a_refresh_late_enough_after_a_relaunch_wins_over_an_earlier_save() {
    let device = Device::new();
    let (service, executor) = device.launch();
    refreshed(&service, &executor, &[]).await;
    saved(&service, &executor, 1).await;
    drop(service);

    device.clocks.advance(Duration::from_secs(150));
    let (service, executor) = device.relaunch();
    refreshed(&service, &executor, &[]).await;

    assert_eq!(service.status(identity(1)), ReaderSavedStatus::NotSaved);
}

#[tokio::test]
async fn a_write_in_flight_when_the_app_quit_is_given_up_on_at_the_next_launch() {
    let device = Device::new();
    let (service, executor) = device.launch();
    refreshed(&service, &executor, &[]).await;
    let (_release, held) = oneshot::channel();
    executor.push(Step::Hold(held, 200, write_response(true, post_json(1), 1)));
    let mut save = Box::pin(service.save(identity(1)));
    assert!(futures::poll!(&mut save).is_pending());

    // The app is killed with the save in flight, so it may have reached the server.
    let (relaunched, executor) = device.relaunch();
    assert_eq!(relaunched.status(identity(1)), ReaderSavedStatus::Saved);

    // Given up on at launch, so it decides the key for twice the usual time.
    device.clocks.advance(Duration::from_secs(200));
    refreshed(&relaunched, &executor, &[]).await;
    assert_eq!(relaunched.status(identity(1)), ReaderSavedStatus::Saved);

    device.clocks.advance(Duration::from_secs(50));
    refreshed(&relaunched, &executor, &[]).await;
    assert_eq!(relaunched.status(identity(1)), ReaderSavedStatus::NotSaved);
}

#[tokio::test]
async fn a_wall_clock_that_moved_back_doesnt_shorten_a_write_that_still_decides_its_key() {
    let device = Device::new();
    let (service, executor) = device.launch();
    refreshed(&service, &executor, &[]).await;
    saved(&service, &executor, 1).await;
    drop(service);

    device
        .clocks
        .move_wall_clock_back(Duration::from_secs(3_600));
    let (service, executor) = device.relaunch();
    refreshed(&service, &executor, &[]).await;

    assert_eq!(service.status(identity(1)), ReaderSavedStatus::Saved);
}

#[tokio::test]
async fn local_state_that_cant_be_read_is_ignored_but_the_keys_still_answer() {
    let device = Device::new();
    let (service, executor) = device.launch();
    refreshed(&service, &executor, &[post_json(1)]).await;
    saved(&service, &executor, 2).await;
    drop(service);
    device.cache.execute(|connection| {
        connection
            .execute("UPDATE reader_saved_state SET local = 'nonsense'", [])
            .unwrap();
    });

    let (service, _) = device.relaunch();

    assert_eq!(service.status(identity(1)), ReaderSavedStatus::Saved);
    assert_eq!(
        service.status(identity(2)),
        ReaderSavedStatus::NotSaved,
        "The save only this device knew about is lost, until a refresh"
    );
    assert!(service.recent_saves().is_empty());
}

#[tokio::test]
async fn a_refresh_stores_each_key_as_the_server_sent_it() {
    let device = Device::new();
    let (service, executor) = device.launch();
    let mut with_url = post_json(1);
    with_url["url"] = json!("https://example.com/post-1");
    refreshed(&service, &executor, &[with_url, feed_json(7)]).await;

    let keys = device
        .cache
        .execute(|connection| ReaderSavedRepository::select_keys(&*connection, service.account))
        .unwrap()
        .unwrap();
    assert_eq!(keys.returned, 2);
    assert_eq!(keys.total, 2);
    assert_eq!(keys.keys.len(), 2);
    let first = &keys.keys[0];
    assert_eq!(first.key, post_key(1));
    assert_eq!(first.save_id, 1);
    assert_eq!(first.date_saved, "2026-11-02T10:15:00Z".parse().unwrap());
    assert_eq!(first.url.as_deref(), Some("https://example.com/post-1"));
    assert_eq!(keys.keys[1].key, feed_key(7));
}

#[tokio::test]
async fn a_post_saved_by_its_ids_is_found_by_its_url_after_a_relaunch() {
    let device = Device::new();
    let (service, executor) = device.launch();
    let mut with_url = post_json(1);
    with_url["url"] = json!("https://example.com/post-1");
    refreshed(&service, &executor, &[with_url]).await;
    drop(service);
    // Even without this device's own state, the stored keys say where the URL is saved.
    device.cache.execute(|connection| {
        connection
            .execute("UPDATE reader_saved_state SET local = NULL", [])
            .unwrap();
    });

    let (service, _) = device.relaunch();

    assert_eq!(
        service.status(url_identity("https://example.com/post-1/")),
        ReaderSavedStatus::Saved
    );
}

#[tokio::test]
async fn an_older_refresh_that_answers_late_doesnt_replace_the_stored_keys() {
    let device = Device::new();
    let (service, executor) = device.launch();
    let (release, held) = oneshot::channel();
    executor.push(Step::Hold(held, 200, keys_response(&[post_json(1)])));
    executor.push(Step::Respond(200, keys_response(&[post_json(2)])));

    let (older, ()) = futures::join!(service.refresh(), async {
        device.clocks.advance(Duration::from_secs(1));
        service.refresh().await.unwrap();
        release.send(()).unwrap();
    });
    older.unwrap();
    drop(service);

    let (service, _) = device.relaunch();
    assert_eq!(service.status(identity(2)), ReaderSavedStatus::Saved);
    assert_eq!(service.status(identity(1)), ReaderSavedStatus::NotSaved);
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
    drop((service, executor));

    device.clocks.advance(Duration::from_secs(10));
    device.relaunch_from_file(&path);
    let (service, executor) = device.launch();

    assert_eq!(service.status(identity(1)), ReaderSavedStatus::Saved);
    assert_eq!(service.status(identity(2)), ReaderSavedStatus::Saved);
    assert_eq!(service.status(identity(3)), ReaderSavedStatus::NotSaved);
    assert_eq!(service.recent_saves(), vec![post_key(2)]);
    let keys: Vec<_> = service
        .saved_posts()
        .load_items()
        .await
        .unwrap()
        .into_iter()
        .map(|item| item.key)
        .collect();
    assert_eq!(keys, vec![post_key(2), post_key(1)]);
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
    refreshed(&other, &other_executor, &[post_json(1)]).await;
    drop(service);

    assert!(
        device
            .cache
            .remove_wordpress_com_account(WpComUserId(1))
            .unwrap()
    );

    let (service, _) = device.launch();
    assert_eq!(service.status(identity(1)), ReaderSavedStatus::Unknown);
    assert!(service.saved_posts().load_items().await.unwrap().is_empty());
    let (other, _) = device.launch_as(2);
    assert_eq!(other.status(identity(1)), ReaderSavedStatus::Saved);
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
    let (service, executor) = device.relaunch();
    let items = service.saved_posts().load_items().await.unwrap();
    assert_eq!(items.len(), 3);
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
    let keys: Vec<_> = items.iter().map(|item| item.key.clone()).collect();
    assert_eq!(keys, vec![post_key(4), post_key(1)]);
    assert!(items[0].is_recent);
    assert_eq!(items[0].post, None, "The saved list hasn't sent it yet");
}

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

    let keys: Vec<_> = collection
        .load_items()
        .await
        .unwrap()
        .into_iter()
        .map(|item| item.key)
        .collect();
    assert_eq!(keys, vec![post_key(3), post_key(1)]);
    let stored: i64 = device.cache.execute(|connection| {
        connection
            .query_row("SELECT COUNT(*) FROM reader_stream_items", [], |row| {
                row.get(0)
            })
            .unwrap()
    });
    assert_eq!(stored, 2, "The unsaved post is removed from the cache");
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

#[tokio::test]
async fn a_page_that_loaded_across_a_refresh_is_discarded() {
    let device = Device::new();
    let (service, executor) = device.launch();
    let collection = service.clone().saved_posts();
    executor.push(Step::Respond(200, keys_response(&[post_json(1)])));
    executor.push(Step::Respond(
        200,
        page(&[listed(1, "2026-11-01T00:00:00Z")], Some("next")),
    ));
    collection.refresh().await.unwrap();

    let (release, held) = oneshot::channel();
    executor.push(Step::Hold(
        held,
        200,
        page(&[listed(9, "2026-10-01T00:00:00Z")], None),
    ));
    executor.push(Step::Respond(200, keys_response(&[post_json(1)])));
    executor.push(Step::Respond(
        200,
        page(&[listed(1, "2026-11-01T00:00:00Z")], Some("next")),
    ));
    let (next, ()) = futures::join!(collection.load_next_page(), async {
        collection.refresh().await.unwrap();
        release.send(()).unwrap();
    });

    assert!(matches!(next, Err(ReaderSavedError::StaleLoadMore)));
    let keys: Vec<_> = collection
        .load_items()
        .await
        .unwrap()
        .into_iter()
        .map(|item| item.key)
        .collect();
    assert_eq!(keys, vec![post_key(1)]);
    assert_eq!(collection.list_info().unwrap().has_more, Some(true));
}

#[tokio::test]
async fn list_posts_stores_the_posts_it_returns() {
    let device = Device::new();
    let (service, executor) = device.launch();
    refreshed(&service, &executor, &[post_json(1)]).await;
    executor.push(Step::Respond(
        200,
        page(&[listed(1, "2026-11-01T00:00:00Z")], Some("next")),
    ));

    service
        .list_posts(ReaderSavedPostsParams::default())
        .await
        .unwrap();

    let collection = service.saved_posts();
    assert_eq!(
        titles(&collection.load_items().await.unwrap()),
        vec![Some("Post 1".to_string())]
    );
    assert_eq!(
        collection.list_info().unwrap().has_more,
        None,
        "Only the collection's own pages move its cursor"
    );
}

#[test]
fn the_collection_is_relevant_to_the_reader_tables_only() {
    let device = Device::new();
    let (service, _) = device.launch();
    let collection = service.saved_posts();
    for table in [
        DbTable::ReaderPosts,
        DbTable::ReaderStreams,
        DbTable::ReaderStreamItems,
        DbTable::ReaderSavedState,
        DbTable::ReaderSavedKeys,
        DbTable::ReaderSavedLocalOnly,
    ] {
        assert!(collection.is_relevant_update(&hook(table)), "{table}");
    }
    for table in [
        DbTable::PostsEditContext,
        DbTable::ListMetadataItems,
        DbTable::WordPressComAccounts,
    ] {
        assert!(!collection.is_relevant_update(&hook(table)), "{table}");
    }
}

// Importing local saves

#[tokio::test]
async fn import_local_saves_imports_once_and_keeps_what_the_server_refused() {
    let device = Device::new();
    let (service, executor) = device.launch();
    refreshed(&service, &executor, &[]).await;
    executor.push(Step::Respond(
        200,
        json!({
            "results": [
                { "key": post_json(1), "saved": true },
                { "key": post_json(2), "saved": false, "error": "saved_limit_reached" },
            ],
            "total": 1,
        }),
    ));
    executor.push(Step::Respond(
        200,
        write_response(true, json!({ "url": "https://example.com/page" }), 2),
    ));

    let report = service
        .import_local_saves(vec![
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
            already_imported: false,
            imported: 2,
            rejected: vec![post_key(2)],
            unsaveable: 1,
        }
    );
    let sent = executor.sent();
    assert_eq!(sent.len(), 3, "The keys once, then the URL");
    assert_eq!(
        sent[1].body.as_ref().unwrap()["keys"],
        json!([post_json(1), post_json(2)])
    );
    assert!(
        service.recent_saves().is_empty(),
        "The device already had them"
    );

    // The refused post stays saved here, and shows in the list.
    assert_eq!(service.local_only_keys(), vec![post_key(2)]);
    assert_eq!(service.status(identity(2)), ReaderSavedStatus::Saved);
    let items = service.clone().saved_posts().load_items().await.unwrap();
    let local: Vec<_> = items.iter().filter(|item| item.is_local_only).collect();
    assert_eq!(local.len(), 1);
    assert_eq!(
        local[0].post.as_ref().unwrap().title.as_deref(),
        Some("Two")
    );

    // A second import sends nothing, even after a relaunch.
    drop((items, service));
    let (service, executor) = device.relaunch();
    let report = service
        .import_local_saves(vec![local_save(identity(3), "Three")])
        .await
        .unwrap();
    assert!(report.already_imported);
    assert!(executor.sent().is_empty());
    assert_eq!(service.local_only_keys(), vec![post_key(2)]);
}

#[tokio::test]
async fn an_import_that_fails_on_the_way_runs_again() {
    let device = Device::new();
    let (service, executor) = device.launch();
    refreshed(&service, &executor, &[]).await;
    executor.push(Step::Offline);

    assert!(
        service
            .import_local_saves(vec![local_save(identity(1), "One")])
            .await
            .is_err()
    );

    executor.push(Step::Respond(
        200,
        json!({ "results": [{ "key": post_json(1), "saved": true }], "total": 1 }),
    ));
    let report = service
        .import_local_saves(vec![local_save(identity(1), "One")])
        .await
        .unwrap();
    assert_eq!(report.imported, 1);
}

#[tokio::test]
async fn a_url_the_server_refuses_is_kept_on_this_device() {
    let device = Device::new();
    let (service, executor) = device.launch();
    refreshed(&service, &executor, &[]).await;
    executor.push(Step::Respond(
        400,
        json!({ "code": "invalid_url", "message": "Invalid URL.", "data": { "status": 400 } }),
    ));

    let report = service
        .import_local_saves(vec![local_save(url_identity("https://example.com/a"), "A")])
        .await
        .unwrap();

    let key = ReaderSavedKey::Url {
        url: "https://example.com/a".to_string(),
    };
    assert_eq!(report.rejected, vec![key.clone()]);
    assert_eq!(service.local_only_keys(), vec![key]);
}

#[tokio::test]
async fn unsaving_a_local_only_save_removes_it() {
    let device = Device::new();
    let (service, executor) = device.launch();
    refreshed(&service, &executor, &[]).await;
    executor.push(Step::Respond(
        200,
        json!({
            "results": [{ "key": json!({ "feed_item_id": 7 }), "saved": false, "error": "saved_limit_reached" }],
            "total": 0,
        }),
    ));
    service
        .import_local_saves(vec![local_save(rss_identity(7), "Feed")])
        .await
        .unwrap();
    assert_eq!(service.status(rss_identity(7)), ReaderSavedStatus::Saved);

    executor.push(Step::Respond(404, error_response()));
    service.unsave(rss_identity(7)).await.unwrap();

    assert_eq!(service.status(rss_identity(7)), ReaderSavedStatus::NotSaved);
    assert!(service.local_only_keys().is_empty());
    assert!(service.saved_posts().load_items().await.unwrap().is_empty());
}

// Found in review

#[tokio::test]
async fn a_load_more_during_a_refresh_is_stale_and_keeps_the_new_cursor() {
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
    executor.push(Step::Hold(held, 200, keys_response(&[post_json(1)])));
    executor.push(Step::Respond(
        200,
        page(&[listed(1, "2026-11-01T00:00:00Z")], Some("new")),
    ));
    let (refreshed, next) = futures::join!(collection.refresh(), async {
        let next = collection.load_next_page().await;
        release.send(()).unwrap();
        next
    });

    refreshed.unwrap();
    assert!(matches!(next, Err(ReaderSavedError::StaleLoadMore)));
    assert!(
        !executor
            .sent()
            .iter()
            .any(|sent| sent.url.contains("page_handle=old")),
        "The load-more didn't fetch the old cursor"
    );
    let info = collection.list_info().unwrap();
    assert_eq!(info.has_more, Some(true));
    assert_eq!(info.state, ListState::Idle);
}

#[tokio::test]
async fn a_url_save_that_is_rate_limited_during_import_is_tried_again() {
    let device = Device::new();
    let (service, executor) = device.launch();
    refreshed(&service, &executor, &[]).await;
    executor.push(Step::Respond(
        429,
        json!({ "code": "rate_limit_exceeded", "message": "Slow down.", "data": { "status": 429 } }),
    ));

    let save = || local_save(url_identity("https://example.com/a"), "A");
    assert!(service.import_local_saves(vec![save()]).await.is_err());
    assert!(service.local_only_keys().is_empty());

    executor.push(Step::Respond(
        200,
        write_response(true, json!({ "url": "https://example.com/a" }), 1),
    ));
    let report = service.import_local_saves(vec![save()]).await.unwrap();
    assert!(!report.already_imported);
    assert_eq!(report.imported, 1);
}

#[tokio::test]
async fn two_imports_at_once_send_the_saves_once() {
    let device = Device::new();
    let (service, executor) = device.launch();
    refreshed(&service, &executor, &[]).await;
    // The first import waits on the server while the second starts.
    let (release, held) = oneshot::channel();
    executor.push(Step::Hold(
        held,
        200,
        json!({ "results": [{ "key": post_json(1), "saved": true }], "total": 1 }),
    ));

    let (first, second, ()) = futures::join!(
        service.import_local_saves(vec![local_save(identity(1), "One")]),
        service.import_local_saves(vec![local_save(identity(1), "One")]),
        async { release.send(()).unwrap() },
    );

    let mut already = [first.unwrap(), second.unwrap()].map(|report| report.already_imported);
    already.sort();
    assert_eq!(already, [false, true]);
    assert_eq!(executor.sent().len(), 2, "The keys, then one import");
}

#[tokio::test]
async fn a_url_no_longer_stored_as_its_post_stays_that_way_after_a_relaunch() {
    let device = Device::new();
    let (service, executor) = device.launch();
    let url = "https://example.com/p";
    let mut with_url = post_json(1);
    with_url["url"] = json!(url);
    refreshed(&service, &executor, &[with_url]).await;

    // The post was deleted, so saving its URL now stores the URL itself.
    executor.push(Step::Respond(
        200,
        write_response(true, json!({ "url": url }), 2),
    ));
    service.save_url(url.to_string(), None).await.unwrap();
    executor.push(Step::Respond(200, write_response(false, post_json(1), 1)));
    service.unsave(identity(1)).await.unwrap();
    assert_eq!(service.status(url_identity(url)), ReaderSavedStatus::Saved);
    drop(service);

    let (service, _) = device.relaunch();

    assert_eq!(service.status(url_identity(url)), ReaderSavedStatus::Saved);
}

#[tokio::test]
async fn a_refresh_keeps_a_post_whose_unsave_is_still_in_flight() {
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
        collection.refresh().await.unwrap();
        release.send(()).unwrap();
    });

    assert!(unsaved.is_err(), "The server refused the unsave");
    let keys: Vec<_> = collection
        .load_items()
        .await
        .unwrap()
        .into_iter()
        .map(|item| item.key)
        .collect();
    assert_eq!(keys, vec![post_key(1)]);
}

// How long the cache takes

/// Times the cache work at the 1,000-key cap, on a database file. Not run by default: run it in a
/// release build with `cargo test -p wp_mobile --release --lib saved_list_timings -- --ignored
/// --nocapture`.
#[tokio::test]
#[ignore]
async fn saved_list_timings() {
    use std::time::Instant as Clock;
    const SAVES: u64 = 1_000;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("app.sqlite");
    let mut device = Device::on_file(&path);
    let (service, executor) = device.launch();
    let keys: Vec<_> = (1..=SAVES).map(post_json).collect();
    let posts: Vec<_> = (1..=SAVES)
        .map(|post_id| {
            let mut post = listed(post_id, "2026-11-01T00:00:00Z");
            post["content"] = json!("<p>".to_string() + &"Some words. ".repeat(400) + "</p>");
            post
        })
        .collect();

    executor.push(Step::Respond(200, keys_response(&keys)));
    let started = Clock::now();
    service.refresh().await.unwrap();
    println!("refresh, {SAVES} keys: {:?}", started.elapsed());

    executor.push(Step::Respond(200, keys_response(&keys)));
    let started = Clock::now();
    service.refresh().await.unwrap();
    println!(
        "refresh again, {SAVES} keys replaced: {:?}",
        started.elapsed()
    );

    executor.push(Step::Respond(200, page(&posts, None)));
    let started = Clock::now();
    service
        .list_posts(ReaderSavedPostsParams::default())
        .await
        .unwrap();
    println!(
        "store {SAVES} listed posts (~5KB each): {:?}",
        started.elapsed()
    );

    executor.push(Step::Respond(
        200,
        write_response(true, post_json(SAVES + 1), SAVES + 1),
    ));
    let started = Clock::now();
    service.save(identity(SAVES + 1)).await.unwrap();
    println!("save (two state writes): {:?}", started.elapsed());

    let collection = service.clone().saved_posts();
    let started = Clock::now();
    let items = collection.load_items().await.unwrap();
    println!("load_items, {} items: {:?}", items.len(), started.elapsed());
    drop((collection, service, executor));

    device.relaunch_from_file(&path);
    let started = Clock::now();
    let (service, _) = device.launch();
    println!("launch, loading the state: {:?}", started.elapsed());
    let started = Clock::now();
    for post_id in 1..=SAVES {
        service.status(identity(post_id));
    }
    println!("status x{SAVES}: {:?}", started.elapsed());
    let size = std::fs::metadata(&path).unwrap().len();
    println!("database file: {} KB", size / 1024);
}

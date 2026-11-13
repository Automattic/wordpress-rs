//! The service against a fake saved posts API, one known starting state and one known action at a
//! time, each with the end state written out by hand.
//!
//! [`every_start_and_action`] covers every state a post can be in that we know of, crossed with
//! every action. The tests after it cover sequences the table can't, and the shapes of key the API
//! stores differently: Jetpack posts, URLs and RSS items.
//!
//! The fake answers the way the live API was seen to on 2026-11-04 (see `FakeServer::delete`).
//! Requests are held until a test answers them, so a test decides the order things happen in.

use super::*;
use ReaderSavedStatus::{NotSaved, Saved, Unknown};
use futures::executor::LocalPool;
use futures::future::RemoteHandle;
use futures::task::LocalSpawnExt;
use std::collections::BTreeSet;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, SystemTime};
use wp_api::request::endpoint::WpEndpointUrl;
use wp_mobile_cache::WpApiCache;

// Keys, by short name: `P1` is post 1, `F4` feed item 4, `U9` the URL of post 9.

const U9: &str = "https://example.com/p9";
const U10: &str = "https://example.com/rss10";

/// When the keys say most saves were made.
const SAVED_DATE: &str = "2026-11-02T10:15:00+00:00";
/// When the saves past the 1,000 keys were made: before every key returned.
const OLD_SAVED_DATE: &str = "2026-10-01T00:00:00+00:00";

fn key(name: &str) -> ReaderSavedKey {
    let number = || name[1..].parse::<u64>().unwrap();
    match (name, &name[..1]) {
        ("U9", _) => ReaderSavedKey::Url {
            url: U9.to_string(),
        },
        ("U10", _) => ReaderSavedKey::Url {
            url: U10.to_string(),
        },
        (_, "P") => post_key(number() as i64),
        (_, "F") => feed_key(number()),
        _ => panic!("unknown key {name}"),
    }
}

fn name(key: &ReaderSavedKey) -> String {
    match key.normalized() {
        ReaderSavedKey::Post { post_id, .. } => format!("P{}", post_id.0),
        ReaderSavedKey::FeedItem { feed_item_id } => format!("F{}", feed_item_id.0),
        ReaderSavedKey::Url { url } if url == U9 => "U9".to_string(),
        ReaderSavedKey::Url { url } if url == U10 => "U10".to_string(),
        ReaderSavedKey::Url { url } => panic!("unknown URL {url}"),
    }
}

/// A Jetpack post: post 3, also known by its feed item 4.
fn jetpack() -> ReaderPostSaveIdentity {
    jetpack_identity(3, 4)
}

/// The same Jetpack post, shown as external while its WordPress.com copy fails to load.
fn jetpack_external() -> ReaderPostSaveIdentity {
    rss_identity(4)
}

/// The saved posts API, as far as these tests need it.
#[derive(Debug, Default, Clone)]
struct FakeServer {
    /// Keys saved, by name, as stored.
    saved: BTreeSet<String>,
    /// Saved keys past the 1,000 the keys endpoint returns: the oldest.
    hidden: BTreeSet<String>,
}

impl FakeServer {
    /// The key the API stores a save as: a feed item of a WordPress.com post as the post, and a URL
    /// as the post or feed item it's the URL of.
    fn stored_as(name: &str) -> &str {
        match name {
            "F4" => "P3",
            "U9" => "P9",
            "U10" => "F10",
            name => name,
        }
    }

    fn exists(name: &str) -> bool {
        name != "F99"
    }

    fn total(&self) -> u64 {
        self.saved.len() as u64
    }

    fn has(&self, name: &str) -> bool {
        self.saved.contains(name)
    }

    fn save(&mut self, sent: &str) -> Result<WpNetworkResponse, RequestExecutionError> {
        if !Self::exists(sent) {
            return respond(404, error_response());
        }
        let stored = Self::stored_as(sent);
        self.saved.insert(stored.to_string());
        // A save is the newest, so the keys return it.
        self.hidden.remove(stored);
        respond(200, write_response(true, json_of(stored), self.total()))
    }

    /// As the live API answers: a delete removes the save the key matches and echoes its key. With
    /// nothing saved, a feed item still echoes its post, but a URL echoes itself, and a feed item
    /// that doesn't exist is `post_not_found`.
    fn delete(&mut self, sent: &str) -> Result<WpNetworkResponse, RequestExecutionError> {
        if !Self::exists(sent) {
            return respond(404, error_response());
        }
        let stored = Self::stored_as(sent);
        let echoed = if self.saved.remove(sent) {
            sent
        } else if self.saved.remove(stored) || !sent.starts_with('U') {
            stored
        } else {
            sent
        };
        self.hidden.retain(|name| self.saved.contains(name));
        respond(
            200,
            json!({ "saved": false, "key": json_of(echoed), "total": self.total() }),
        )
    }

    fn date_of(&self, name: &str) -> &'static str {
        if self.hidden.contains(name) {
            OLD_SAVED_DATE
        } else {
            SAVED_DATE
        }
    }

    fn keys_response(&self) -> Value {
        let entries: Vec<_> = self
            .saved
            .difference(&self.hidden)
            .enumerate()
            .map(|(index, name)| {
                let mut entry = json_of(name);
                entry["id"] = json!(index + 1);
                entry["date_saved"] = json!(SAVED_DATE);
                match name.as_str() {
                    "P9" => entry["url"] = json!(U9),
                    "F10" => entry["url"] = json!(U10),
                    _ => {}
                }
                entry
            })
            .collect();
        json!({ "keys": entries, "total": self.total() })
    }

    fn list_response(&self) -> Value {
        let mut posts: Vec<Value> = Vec::new();
        for name in &self.saved {
            let mut post = match name.as_str() {
                "P3" | "F4" => json!({
                    "ID": 3, "site_ID": 123, "feed_ID": 5, "feed_item_ID": 4, "is_external": false
                }),
                name if name.starts_with('P') => listed_post(name[1..].parse().unwrap()),
                name => listed_rss_post(name[1..].parse().unwrap()),
            };
            post["date_saved"] = json!(self.date_of(name));
            if !posts.iter().any(|other| other["ID"] == post["ID"]) {
                posts.push(post);
            }
        }
        json!({ "posts": posts })
    }
}

fn json_of(name: &str) -> Value {
    serde_json::to_value(key(name)).unwrap()
}

fn respond(status_code: u32, body: Value) -> Result<WpNetworkResponse, RequestExecutionError> {
    Ok(WpNetworkResponse {
        body: body.to_string().into_bytes(),
        status_code,
        response_header_map: Default::default(),
        request_url: WpEndpointUrl(String::new()),
        request_method: RequestMethod::POST,
        request_header_map: Default::default(),
    })
}

fn refused() -> Result<WpNetworkResponse, RequestExecutionError> {
    respond(
        400,
        json!({ "code": "invalid_param", "message": "", "data": { "status": 400 } }),
    )
}

type Reply = oneshot::Sender<Result<WpNetworkResponse, RequestExecutionError>>;

/// A request the executor is holding, described as `new P1`, `delete U9`, `keys` or `list`.
struct Held {
    what: String,
    /// Where it is among every request sent.
    index: usize,
    reply: Reply,
}

impl Held {
    fn is_write(&self) -> bool {
        self.what.starts_with("new") || self.what.starts_with("delete")
    }
}

/// Holds every request until the test answers it, unless the device is offline.
#[derive(Default)]
struct HoldingExecutor {
    held: Mutex<Vec<Held>>,
    /// Every request made, in order.
    sent: Mutex<Vec<String>>,
    offline: AtomicBool,
}

impl std::fmt::Debug for HoldingExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HoldingExecutor")
    }
}

#[async_trait]
impl RequestExecutor for HoldingExecutor {
    async fn execute(
        &self,
        request: Arc<WpNetworkRequest>,
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        let url = request.url().0;
        let body: Option<Value> = request
            .body_as_string()
            .map(|body| serde_json::from_str(&body).unwrap());
        let sent_key = || name(&serde_json::from_value(body.clone().unwrap()).unwrap());
        let what = if url.starts_with(KEYS_URL) {
            "keys".to_string()
        } else if url.starts_with(NEW_URL) {
            format!("new {}", sent_key())
        } else if url.starts_with(DELETE_URL) {
            format!("delete {}", sent_key())
        } else if url.starts_with(LIST_URL) {
            "list".to_string()
        } else {
            panic!("unexpected request {url}")
        };
        let index = {
            let mut sent = self.sent.lock().unwrap();
            sent.push(what.clone());
            sent.len() - 1
        };
        if self.offline.load(Ordering::SeqCst) {
            return Err(RequestExecutionError::RequestExecutionFailed {
                status_code: None,
                redirects: None,
                reason: RequestExecutionErrorReason::DeviceIsOfflineError {
                    error_message: "offline".to_string(),
                },
                request_url: url,
                request_method: request.method(),
            });
        }
        let (reply, response) = oneshot::channel();
        self.held.lock().unwrap().push(Held { what, index, reply });
        let mut response = response.await.expect("the test dropped a request");
        if let Ok(response) = &mut response {
            response.request_url = request.url();
            response.request_method = request.method();
        }
        response
    }

    async fn upload(
        &self,
        _request: Arc<WpMultipartFormRequest>,
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        unimplemented!()
    }

    async fn sleep(&self, _: u64) {}

    fn cancel(&self, _: Arc<RequestContext>) {}
}

/// 2026-11-12, after every date the fake gives.
fn launch_time() -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(1_794_441_600)
}

/// The service, its cache, and the fake API, all driven by hand.
struct World {
    pool: LocalPool,
    cache: Arc<WpApiCache>,
    service: Arc<ReaderSavedService>,
    executor: Arc<HoldingExecutor>,
    server: FakeServer,
    /// Each call started, until it's cancelled.
    calls: Vec<Option<RemoteHandle<()>>>,
}

impl World {
    fn new() -> Self {
        let cache = storage::migrated_in_memory_cache();
        let executor = Arc::new(HoldingExecutor::default());
        Self {
            pool: LocalPool::new(),
            service: Self::launch(&cache, &executor),
            cache,
            executor,
            server: FakeServer::default(),
            calls: Vec::new(),
        }
    }

    fn launch(cache: &Arc<WpApiCache>, executor: &Arc<HoldingExecutor>) -> Arc<ReaderSavedService> {
        Arc::new(
            ReaderSavedService::with_clock(
                delegate(executor.clone()),
                ReaderSavedSource::Android,
                cache.clone(),
                WpComUserId(1),
                Arc::new(launch_time),
            )
            .unwrap(),
        )
    }

    /// The app is killed and started again: every call and request in flight is dropped, and the
    /// requests never reach the server.
    fn relaunch(&mut self) {
        for call in &mut self.calls {
            drop(call.take());
        }
        self.pool.run_until_stalled();
        self.executor.held.lock().unwrap().clear();
        self.service = Self::launch(&self.cache, &self.executor);
    }

    fn set_offline(&self, offline: bool) {
        self.executor.offline.store(offline, Ordering::SeqCst);
    }

    /// Starts a call, and runs it until it waits on a request or the lane. Returns its index, to
    /// cancel it.
    fn call<F, T>(&mut self, call: impl FnOnce(Arc<ReaderSavedService>) -> F) -> usize
    where
        F: std::future::Future<Output = T> + 'static,
    {
        let future = call(self.service.clone());
        let handle = self
            .pool
            .spawner()
            .spawn_local_with_handle(async move {
                let _ = future.await;
            })
            .unwrap();
        self.calls.push(Some(handle));
        self.pool.run_until_stalled();
        self.calls.len() - 1
    }

    fn save(&mut self, post: ReaderPostSaveIdentity) -> usize {
        self.call(|service| async move { service.save(post).await })
    }

    fn unsave(&mut self, post: ReaderPostSaveIdentity) -> usize {
        self.call(|service| async move { service.unsave(post).await })
    }

    fn save_url(&mut self, url: &str) -> usize {
        let url = url.to_string();
        self.call(|service| async move { service.save_url(url, None).await })
    }

    fn unsave_key(&mut self, name: &str) -> usize {
        let key = key(name);
        self.call(|service| async move { service.unsave_key(key).await })
    }

    fn start_sync(&mut self) -> usize {
        self.call(|service| async move { service.sync().await })
    }

    fn start_list_refresh(&mut self) -> usize {
        self.call(|service| async move { service.saved_posts().refresh().await })
    }

    /// Cancels a call, as Kotlin does by dropping its future. Any request it had sent is dropped
    /// without an answer.
    fn cancel(&mut self, call: usize) {
        drop(self.calls[call].take());
        self.pool.run_until_stalled();
        self.executor
            .held
            .lock()
            .unwrap()
            .retain(|held| !held.reply.is_canceled());
    }

    fn take_held(&mut self, which: impl Fn(&Held) -> bool) -> Option<Held> {
        let mut held = self.executor.held.lock().unwrap();
        let index = held.iter().position(which)?;
        Some(held.remove(index))
    }

    fn apply(&mut self, what: &str) -> Result<WpNetworkResponse, RequestExecutionError> {
        let (verb, sent) = what.split_once(' ').unwrap();
        match verb {
            "new" => self.server.save(sent),
            _ => self.server.delete(sent),
        }
    }

    /// Answers a request held: a write is applied to the server, a read reads what it holds.
    fn answer(&mut self, held: Held, refuse: bool) {
        let response = if refuse {
            refused()
        } else if held.is_write() {
            self.apply(&held.what)
        } else if held.what == "keys" {
            respond(200, self.server.keys_response())
        } else {
            respond(200, self.server.list_response())
        };
        let _ = held.reply.send(response);
        self.pool.run_until_stalled();
    }

    /// Answers the oldest write held.
    fn answer_next_write(&mut self) -> bool {
        let Some(held) = self.take_held(Held::is_write) else {
            return false;
        };
        self.answer(held, false);
        true
    }

    /// Answers every write, oldest first, including any a write sends once it's answered.
    fn answer_writes(&mut self) {
        while self.answer_next_write() {}
    }

    /// Answers every write, oldest first, refusing those sent from `since` on that are `what`.
    fn answer_writes_refusing(&mut self, what: &str, since: usize) {
        while let Some(held) = self.take_held(Held::is_write) {
            let refuse = held.what == what && held.index >= since;
            self.answer(held, refuse);
        }
    }

    /// Answers every request sent from `since` on, including any they lead to, refusing reads that
    /// `refuse` names.
    fn answer_since(&mut self, since: usize, refuse: &[&str]) {
        while let Some(held) = self.take_held(|held| held.index >= since) {
            let refused = refuse.contains(&held.what.as_str());
            self.answer(held, refused);
        }
    }

    /// Gives up on the oldest write held, as a request that timed out. The server may still apply
    /// it: see [`World::apply`].
    fn time_out_next_write(&mut self) -> String {
        let held = self.take_held(Held::is_write).unwrap();
        let what = held.what.clone();
        let _ = held
            .reply
            .send(Err(RequestExecutionError::RequestExecutionFailed {
                status_code: None,
                redirects: None,
                reason: RequestExecutionErrorReason::HttpTimeoutError,
                request_url: String::new(),
                request_method: RequestMethod::POST,
            }));
        self.pool.run_until_stalled();
        what
    }

    /// A tap of post 1 whose call is cancelled before it's answered. If its request went out,
    /// `applied` says whether it reached the server.
    fn tap_then_cancel(&mut self, save: bool, applied: bool) {
        let before = self.sent_count();
        let call = if save {
            self.save(identity(1))
        } else {
            self.unsave(identity(1))
        };
        if self.sent_count() > before && applied {
            let what = self.executor.sent.lock().unwrap()[before].clone();
            let _ = self.apply(&what);
        }
        self.cancel(call);
    }

    /// Syncs, answering what it sends from what the server holds.
    fn sync(&mut self) {
        let before = self.sent_count();
        self.start_sync();
        self.answer_since(before, &[]);
    }

    /// Refreshes the saved list, answering what it sends from what the server holds.
    fn list_refresh(&mut self) {
        let before = self.sent_count();
        self.start_list_refresh();
        self.answer_since(before, &[]);
    }

    /// Refreshes the saved list, and returns the posts the collection shows, by key.
    fn list_shown(&mut self) -> Vec<String> {
        self.list_refresh();
        self.shown()
    }

    /// What the collection shows, by key, from the cache.
    fn shown(&self) -> Vec<String> {
        self.service
            .saved_list_items()
            .unwrap()
            .iter()
            .map(|item| name(&item.key))
            .collect()
    }

    fn sent_since(&self, before: usize) -> Vec<String> {
        self.executor.sent.lock().unwrap()[before..].to_vec()
    }

    fn sent_count(&self) -> usize {
        self.executor.sent.lock().unwrap().len()
    }

    fn status(&self, post: ReaderPostSaveIdentity) -> ReaderSavedStatus {
        self.service.status(post).unwrap()
    }

    fn url_status(&self, url: &str) -> ReaderSavedStatus {
        self.service
            .status_of(&[ReaderSavedKey::Url {
                url: url.to_string(),
            }])
            .unwrap()
    }

    fn count(&self) -> Option<u64> {
        self.service.saved_count().unwrap()
    }

    /// Whether the server doesn't have what the user wants for any of the post's rows yet.
    fn pending(&self, post: ReaderPostSaveIdentity) -> bool {
        self.service
            .rows_for(&normalized(&reader_saved_keys_for_post(post)))
            .unwrap()
            .iter()
            .any(DbReaderSavedRow::is_pending)
    }
}

// Every start, crossed with every action, for a plain WordPress.com post: post 1.

#[derive(Debug, Clone, Copy, PartialEq)]
enum Start {
    /// No sync yet, and the post isn't saved.
    UnknownUnsaved,
    /// No sync yet, and the post is saved.
    UnknownSaved,
    /// Synced, and not saved.
    KnownUnsaved,
    /// Synced, and saved.
    KnownSaved,
    /// Read as saved, then unsaved on another device.
    SavedButUnsavedElsewhere,
    /// Read as not saved, then saved on another device.
    UnsavedButSavedElsewhere,
    /// Saved on another device between a list refresh's read of the keys and its page, which shows
    /// it.
    ListedButNotInTheKeys,
    /// Saved on this device, and the request hasn't been answered.
    Saving,
    /// Unsaved on this device, and the request hasn't been answered.
    Unsaving,
    /// Saved, then unsaved, on this device: the save is in flight and the unsave waits its turn.
    SaveThenUnsaveQueued,
    /// Saved on this device, and answered.
    JustSaved,
    /// Unsaved on this device, and answered.
    JustUnsaved,
    /// Saved, but older than the 1,000 keys a sync returns: 1,001 saves in all.
    PastTheKeyLimit,
    /// As above, and the saved list has shown it.
    PastTheKeyLimitAndListed,
}

const STARTS: [Start; 14] = [
    Start::UnknownUnsaved,
    Start::UnknownSaved,
    Start::KnownUnsaved,
    Start::KnownSaved,
    Start::SavedButUnsavedElsewhere,
    Start::UnsavedButSavedElsewhere,
    Start::ListedButNotInTheKeys,
    Start::Saving,
    Start::Unsaving,
    Start::SaveThenUnsaveQueued,
    Start::JustSaved,
    Start::JustUnsaved,
    Start::PastTheKeyLimit,
    Start::PastTheKeyLimitAndListed,
];

#[derive(Debug, Clone, Copy, PartialEq)]
enum Action {
    /// Tap save. Every request is answered.
    Save,
    /// Tap save. Earlier requests are answered, and the save is refused.
    SaveRefused,
    /// Tap unsave. Every request is answered.
    Unsave,
    /// Tap unsave. Earlier requests are answered, and the unsave is refused.
    UnsaveRefused,
    /// Tap save, then cancel the call. Its request, if it went out, never reaches the server.
    SaveCancelledAndLost,
    /// Tap save, then cancel the call. Its request, if it went out, reaches the server.
    SaveCancelledButApplied,
    /// Tap unsave, then cancel the call. Its request, if it went out, never reaches the server.
    UnsaveCancelledAndLost,
    /// Sync. What it sends is answered; requests already in flight aren't.
    Sync,
    /// Sync, and the read of the keys is refused.
    SyncFails,
    /// Refresh the saved list, which syncs, then loads the first page.
    ListLoads,
    /// Refresh the saved list, and the page is refused.
    ListFails,
    /// The app is killed, with any request in flight lost, and started again.
    Relaunch,
}

const ACTIONS: [Action; 12] = [
    Action::Save,
    Action::SaveRefused,
    Action::Unsave,
    Action::UnsaveRefused,
    Action::SaveCancelledAndLost,
    Action::SaveCancelledButApplied,
    Action::UnsaveCancelledAndLost,
    Action::Sync,
    Action::SyncFails,
    Action::ListLoads,
    Action::ListFails,
    Action::Relaunch,
];

/// Where each start and action is in its list. Adding one that isn't in the lists fails to compile
/// here, rather than quietly leaving it out of the table.
fn position(start: Start, action: Action) -> (usize, usize) {
    use Action::*;
    use Start::*;
    let start = match start {
        UnknownUnsaved => 0,
        UnknownSaved => 1,
        KnownUnsaved => 2,
        KnownSaved => 3,
        SavedButUnsavedElsewhere => 4,
        UnsavedButSavedElsewhere => 5,
        ListedButNotInTheKeys => 6,
        Saving => 7,
        Unsaving => 8,
        SaveThenUnsaveQueued => 9,
        JustSaved => 10,
        JustUnsaved => 11,
        PastTheKeyLimit => 12,
        PastTheKeyLimitAndListed => 13,
    };
    let action = match action {
        Save => 0,
        SaveRefused => 1,
        Unsave => 2,
        UnsaveRefused => 3,
        SaveCancelledAndLost => 4,
        SaveCancelledButApplied => 5,
        UnsaveCancelledAndLost => 6,
        Sync => 7,
        SyncFails => 8,
        ListLoads => 9,
        ListFails => 10,
        Relaunch => 11,
    };
    (start, action)
}

/// What the user sees afterwards, what the server holds, and the requests the action sent.
#[derive(Debug, PartialEq)]
struct End {
    status: ReaderSavedStatus,
    count: Option<u64>,
    /// The saved list collection shows the post.
    in_list: bool,
    /// The device has a change for the post the server doesn't have yet.
    pending: bool,
    on_server: bool,
    sent: Vec<String>,
}

fn start(start: Start) -> World {
    let mut world = World::new();
    let p1 = || "P1".to_string();
    let past_the_keys = |world: &mut World| {
        for id in 1001..=2000 {
            world.server.saved.insert(format!("P{id}"));
        }
        world.server.saved.insert(p1());
        world.server.hidden.insert(p1());
    };
    match start {
        Start::UnknownUnsaved => {}
        Start::UnknownSaved => {
            world.server.saved.insert(p1());
        }
        Start::KnownUnsaved => world.sync(),
        Start::KnownSaved => {
            world.server.saved.insert(p1());
            world.sync();
        }
        Start::SavedButUnsavedElsewhere => {
            world.server.saved.insert(p1());
            world.sync();
            world.server.saved.remove("P1");
        }
        Start::UnsavedButSavedElsewhere => {
            world.sync();
            world.server.saved.insert(p1());
        }
        Start::ListedButNotInTheKeys => {
            world.start_list_refresh();
            let keys = world.take_held(|held| held.what == "keys").unwrap();
            world.answer(keys, false);
            world.server.saved.insert(p1());
            let list = world.take_held(|held| held.what == "list").unwrap();
            world.answer(list, false);
        }
        Start::Saving => {
            world.sync();
            world.save(identity(1));
        }
        Start::Unsaving => {
            world.server.saved.insert(p1());
            world.sync();
            world.unsave(identity(1));
        }
        Start::SaveThenUnsaveQueued => {
            world.sync();
            world.save(identity(1));
            world.unsave(identity(1));
        }
        Start::JustSaved => {
            world.sync();
            world.save(identity(1));
            world.answer_writes();
        }
        Start::JustUnsaved => {
            world.server.saved.insert(p1());
            world.sync();
            world.unsave(identity(1));
            world.answer_writes();
        }
        Start::PastTheKeyLimit => {
            past_the_keys(&mut world);
            world.sync();
        }
        Start::PastTheKeyLimitAndListed => {
            past_the_keys(&mut world);
            world.list_refresh();
        }
    }
    world
}

fn act(world: &mut World, action: Action) {
    let before = world.sent_count();
    match action {
        Action::Save => {
            world.save(identity(1));
            world.answer_writes();
        }
        Action::SaveRefused => {
            world.save(identity(1));
            world.answer_writes_refusing("new P1", before);
        }
        Action::Unsave => {
            world.unsave(identity(1));
            world.answer_writes();
        }
        Action::UnsaveRefused => {
            world.unsave(identity(1));
            world.answer_writes_refusing("delete P1", before);
        }
        Action::SaveCancelledAndLost => world.tap_then_cancel(true, false),
        Action::SaveCancelledButApplied => world.tap_then_cancel(true, true),
        Action::UnsaveCancelledAndLost => world.tap_then_cancel(false, false),
        Action::Sync => world.sync(),
        Action::SyncFails => {
            world.start_sync();
            world.answer_since(before, &["keys"]);
        }
        Action::ListLoads => world.list_refresh(),
        Action::ListFails => {
            world.start_list_refresh();
            world.answer_since(before, &["list"]);
        }
        Action::Relaunch => world.relaunch(),
    }
}

fn end(
    status: ReaderSavedStatus,
    count: Option<u64>,
    in_list: bool,
    pending: bool,
    on_server: bool,
    sent: &[&str],
) -> End {
    End {
        status,
        count,
        in_list,
        pending,
        on_server,
        sent: sent.iter().map(|what| what.to_string()).collect(),
    }
}

/// Every row: a start, an action, and the end state.
///
/// Rewritten for the outbox from the table #1729 had (`e1b79948`). What changed for every row:
///
/// - **The actions.** `FreshRefresh` and `RefreshFails` are `Sync` and `SyncFails`. The stale and
///   out-of-order reads are gone (`StaleRefresh`, `RefreshesOutOfOrder`, `StaleListLoads`,
///   `LateListLoads`): one lane runs every read after this device's writes, and reads were
///   measured not to lag behind answered writes. `TimePasses` is gone, because no clock decides
///   anything, and `Relaunch` takes its place.
/// - **The columns.** `in_recent_saves` is `in_list`, since recent saves are gone and a save shows
///   in the list itself. `pending` is new.
///
/// A row whose status, count, server or requests differ from #1729's ends with a tag for why:
///
/// - `A`: a list refresh syncs first, so it reads the keys before the page.
/// - `K`: a post's taps collapse into its row, and a tap that matches what the server last said,
///   or a request in flight is about to say, sends nothing.
/// - `C`: a cancelled tap isn't undone. It's on the device, and is sent when the lane is free.
/// - `W`: a sync or a list refresh waits behind the request in flight, so it hasn't sent yet.
/// - `U`: the keys returned every save and don't have the post, so an unsave has nothing to send.
/// - `N`: `Unknown` is gone past the 1,000 keys: a post they don't have reads `NotSaved`.
/// - `L`: the saved list showed the post, which marks it saved, so it reads `Saved`.
/// - `#`: the count is the newest total plus what's pending, so it's off until the next sync.
#[rustfmt::skip]
fn rows() -> Vec<(Start, Action, End)> {
    use Action::*;
    use Start::*;
    let n = None;
    let s = Some;
    let (t, f) = (true, false);
    vec![
        // start                    action                   status    count    list pend server sent
        (UnknownUnsaved,            Save,                    end(Saved,    s(1),    t, f, t, &["new P1"])),
        (UnknownUnsaved,            SaveRefused,             end(Unknown,  n,       f, f, f, &["new P1"])),
        (UnknownUnsaved,            Unsave,                  end(NotSaved, s(0),    f, f, f, &["delete P1"])),
        (UnknownUnsaved,            UnsaveRefused,           end(Unknown,  n,       f, f, f, &["delete P1"])),
        (UnknownUnsaved,            SaveCancelledAndLost,    end(Saved,    n,       t, t, f, &["new P1"])),
        (UnknownUnsaved,            SaveCancelledButApplied, end(Saved,    n,       t, t, t, &["new P1"])),
        (UnknownUnsaved,            UnsaveCancelledAndLost,  end(NotSaved, n,       f, t, f, &["delete P1"])),
        (UnknownUnsaved,            Sync,                    end(NotSaved, s(0),    f, f, f, &["keys"])),
        (UnknownUnsaved,            SyncFails,               end(Unknown,  n,       f, f, f, &["keys"])),
        (UnknownUnsaved,            ListLoads,               end(NotSaved, s(0),    f, f, f, &["keys", "list"])), // A
        (UnknownUnsaved,            ListFails,               end(NotSaved, s(0),    f, f, f, &["keys", "list"])), // A
        (UnknownUnsaved,            Relaunch,                end(Unknown,  n,       f, f, f, &[])),

        (UnknownSaved,              Save,                    end(Saved,    s(1),    t, f, t, &["new P1"])),
        (UnknownSaved,              SaveRefused,             end(Unknown,  n,       f, f, t, &["new P1"])),
        (UnknownSaved,              Unsave,                  end(NotSaved, s(0),    f, f, f, &["delete P1"])),
        (UnknownSaved,              UnsaveRefused,           end(Unknown,  n,       f, f, t, &["delete P1"])),
        (UnknownSaved,              SaveCancelledAndLost,    end(Saved,    n,       t, t, t, &["new P1"])),
        (UnknownSaved,              SaveCancelledButApplied, end(Saved,    n,       t, t, t, &["new P1"])),
        (UnknownSaved,              UnsaveCancelledAndLost,  end(NotSaved, n,       f, t, t, &["delete P1"])),
        (UnknownSaved,              Sync,                    end(Saved,    s(1),    f, f, t, &["keys"])),
        (UnknownSaved,              SyncFails,               end(Unknown,  n,       f, f, t, &["keys"])),
        (UnknownSaved,              ListLoads,               end(Saved,    s(1),    t, f, t, &["keys", "list"])), // A
        (UnknownSaved,              ListFails,               end(Saved,    s(1),    f, f, t, &["keys", "list"])), // A
        (UnknownSaved,              Relaunch,                end(Unknown,  n,       f, f, t, &[])),

        (KnownUnsaved,              Save,                    end(Saved,    s(1),    t, f, t, &["new P1"])),
        (KnownUnsaved,              SaveRefused,             end(NotSaved, s(0),    f, f, f, &["new P1"])),
        (KnownUnsaved,              Unsave,                  end(NotSaved, s(0),    f, f, f, &[])), // U
        (KnownUnsaved,              UnsaveRefused,           end(NotSaved, s(0),    f, f, f, &[])), // U
        (KnownUnsaved,              SaveCancelledAndLost,    end(Saved,    s(1),    t, t, f, &["new P1"])),
        (KnownUnsaved,              SaveCancelledButApplied, end(Saved,    s(1),    t, t, t, &["new P1"])),
        (KnownUnsaved,              UnsaveCancelledAndLost,  end(NotSaved, s(0),    f, f, f, &[])), // U
        (KnownUnsaved,              Sync,                    end(NotSaved, s(0),    f, f, f, &["keys"])),
        (KnownUnsaved,              SyncFails,               end(NotSaved, s(0),    f, f, f, &["keys"])),
        (KnownUnsaved,              ListLoads,               end(NotSaved, s(0),    f, f, f, &["keys", "list"])), // A
        (KnownUnsaved,              ListFails,               end(NotSaved, s(0),    f, f, f, &["keys", "list"])), // A
        (KnownUnsaved,              Relaunch,                end(NotSaved, s(0),    f, f, f, &[])),

        (KnownSaved,                Save,                    end(Saved,    s(1),    t, f, t, &[])), // K
        (KnownSaved,                SaveRefused,             end(Saved,    s(1),    t, f, t, &[])), // K
        (KnownSaved,                Unsave,                  end(NotSaved, s(0),    f, f, f, &["delete P1"])),
        (KnownSaved,                UnsaveRefused,           end(Saved,    s(1),    f, f, t, &["delete P1"])),
        (KnownSaved,                SaveCancelledAndLost,    end(Saved,    s(1),    t, f, t, &[])), // K
        (KnownSaved,                SaveCancelledButApplied, end(Saved,    s(1),    t, f, t, &[])), // K
        (KnownSaved,                UnsaveCancelledAndLost,  end(NotSaved, s(0),    f, t, t, &["delete P1"])),
        (KnownSaved,                Sync,                    end(Saved,    s(1),    f, f, t, &["keys"])),
        (KnownSaved,                SyncFails,               end(Saved,    s(1),    f, f, t, &["keys"])),
        (KnownSaved,                ListLoads,               end(Saved,    s(1),    t, f, t, &["keys", "list"])), // A
        (KnownSaved,                ListFails,               end(Saved,    s(1),    f, f, t, &["keys", "list"])), // A
        (KnownSaved,                Relaunch,                end(Saved,    s(1),    f, f, t, &[])),

        // The device still thinks the server has it, so a save sends nothing until a sync says.
        (SavedButUnsavedElsewhere,  Save,                    end(Saved,    s(1),    t, f, f, &[])), // K
        (SavedButUnsavedElsewhere,  SaveRefused,             end(Saved,    s(1),    t, f, f, &[])), // K
        (SavedButUnsavedElsewhere,  Unsave,                  end(NotSaved, s(0),    f, f, f, &["delete P1"])),
        (SavedButUnsavedElsewhere,  UnsaveRefused,           end(Saved,    s(1),    f, f, f, &["delete P1"])),
        (SavedButUnsavedElsewhere,  SaveCancelledAndLost,    end(Saved,    s(1),    t, f, f, &[])), // K
        (SavedButUnsavedElsewhere,  SaveCancelledButApplied, end(Saved,    s(1),    t, f, f, &[])), // K
        (SavedButUnsavedElsewhere,  UnsaveCancelledAndLost,  end(NotSaved, s(0),    f, t, f, &["delete P1"])),
        (SavedButUnsavedElsewhere,  Sync,                    end(NotSaved, s(0),    f, f, f, &["keys"])),
        (SavedButUnsavedElsewhere,  SyncFails,               end(Saved,    s(1),    f, f, f, &["keys"])),
        (SavedButUnsavedElsewhere,  ListLoads,               end(NotSaved, s(0),    f, f, f, &["keys", "list"])), // A
        (SavedButUnsavedElsewhere,  ListFails,               end(NotSaved, s(0),    f, f, f, &["keys", "list"])), // A
        (SavedButUnsavedElsewhere,  Relaunch,                end(Saved,    s(1),    f, f, f, &[])),

        (UnsavedButSavedElsewhere,  Save,                    end(Saved,    s(1),    t, f, t, &["new P1"])),
        (UnsavedButSavedElsewhere,  SaveRefused,             end(NotSaved, s(0),    f, f, t, &["new P1"])),
        (UnsavedButSavedElsewhere,  Unsave,                  end(NotSaved, s(0),    f, f, t, &[])), // U
        (UnsavedButSavedElsewhere,  UnsaveRefused,           end(NotSaved, s(0),    f, f, t, &[])), // U
        (UnsavedButSavedElsewhere,  SaveCancelledAndLost,    end(Saved,    s(1),    t, t, t, &["new P1"])),
        (UnsavedButSavedElsewhere,  SaveCancelledButApplied, end(Saved,    s(1),    t, t, t, &["new P1"])),
        (UnsavedButSavedElsewhere,  UnsaveCancelledAndLost,  end(NotSaved, s(0),    f, f, t, &[])), // U
        (UnsavedButSavedElsewhere,  Sync,                    end(Saved,    s(1),    f, f, t, &["keys"])),
        (UnsavedButSavedElsewhere,  SyncFails,               end(NotSaved, s(0),    f, f, t, &["keys"])),
        (UnsavedButSavedElsewhere,  ListLoads,               end(Saved,    s(1),    t, f, t, &["keys", "list"])), // A
        (UnsavedButSavedElsewhere,  ListFails,               end(Saved,    s(1),    f, f, t, &["keys", "list"])), // A
        (UnsavedButSavedElsewhere,  Relaunch,                end(NotSaved, s(0),    f, f, t, &[])),

        // The total the count follows predates the page, so it's 0 until a sync.
        (ListedButNotInTheKeys,     Save,                    end(Saved,    s(0),    t, f, t, &[])), // K #
        (ListedButNotInTheKeys,     SaveRefused,             end(Saved,    s(0),    t, f, t, &[])), // K #
        (ListedButNotInTheKeys,     Unsave,                  end(NotSaved, s(0),    f, f, f, &["delete P1"])),
        (ListedButNotInTheKeys,     UnsaveRefused,           end(Saved,    s(0),    t, f, t, &["delete P1"])), // L
        (ListedButNotInTheKeys,     SaveCancelledAndLost,    end(Saved,    s(0),    t, f, t, &[])), // K #
        (ListedButNotInTheKeys,     SaveCancelledButApplied, end(Saved,    s(0),    t, f, t, &[])), // K #
        (ListedButNotInTheKeys,     UnsaveCancelledAndLost,  end(NotSaved, s(0),    f, t, t, &["delete P1"])),
        (ListedButNotInTheKeys,     Sync,                    end(Saved,    s(1),    t, f, t, &["keys"])),
        (ListedButNotInTheKeys,     SyncFails,               end(Saved,    s(0),    t, f, t, &["keys"])), // L
        (ListedButNotInTheKeys,     ListLoads,               end(Saved,    s(1),    t, f, t, &["keys", "list"])), // A L
        (ListedButNotInTheKeys,     ListFails,               end(Saved,    s(1),    t, f, t, &["keys", "list"])), // A L
        (ListedButNotInTheKeys,     Relaunch,                end(Saved,    s(0),    t, f, t, &[])), // L

        // The save in flight is answered before anything else is sent.
        (Saving,                    Save,                    end(Saved,    s(1),    t, f, t, &[])), // K
        (Saving,                    SaveRefused,             end(Saved,    s(1),    t, f, t, &[])), // K
        (Saving,                    Unsave,                  end(NotSaved, s(0),    f, f, f, &["delete P1"])),
        (Saving,                    UnsaveRefused,           end(Saved,    s(1),    t, f, t, &["delete P1"])),
        (Saving,                    SaveCancelledAndLost,    end(Saved,    s(1),    t, t, f, &[])),
        (Saving,                    SaveCancelledButApplied, end(Saved,    s(1),    t, t, f, &[])),
        (Saving,                    UnsaveCancelledAndLost,  end(NotSaved, s(0),    f, t, f, &[])), // C
        (Saving,                    Sync,                    end(Saved,    s(1),    t, t, f, &[])), // W
        (Saving,                    SyncFails,               end(Saved,    s(1),    t, t, f, &[])), // W
        (Saving,                    ListLoads,               end(Saved,    s(1),    t, t, f, &[])), // W
        (Saving,                    ListFails,               end(Saved,    s(1),    t, t, f, &[])), // W
        // The save stays in doubt, and the next sync says whether to send it again.
        (Saving,                    Relaunch,                end(Saved,    s(1),    t, t, f, &[])),

        (Unsaving,                  Save,                    end(Saved,    s(1),    t, f, t, &["new P1"])),
        (Unsaving,                  SaveRefused,             end(NotSaved, s(0),    f, f, f, &["new P1"])),
        (Unsaving,                  Unsave,                  end(NotSaved, s(0),    f, f, f, &[])), // K
        (Unsaving,                  UnsaveRefused,           end(NotSaved, s(0),    f, f, f, &[])), // K
        // The delete in flight is still unanswered, and the row matches the server's last word.
        (Unsaving,                  SaveCancelledAndLost,    end(Saved,    s(1),    t, f, t, &[])), // C
        (Unsaving,                  SaveCancelledButApplied, end(Saved,    s(1),    t, f, t, &[])), // C
        (Unsaving,                  UnsaveCancelledAndLost,  end(NotSaved, s(0),    f, t, t, &[])),
        (Unsaving,                  Sync,                    end(NotSaved, s(0),    f, t, t, &[])), // W
        (Unsaving,                  SyncFails,               end(NotSaved, s(0),    f, t, t, &[])), // W
        (Unsaving,                  ListLoads,               end(NotSaved, s(0),    f, t, t, &[])), // W
        (Unsaving,                  ListFails,               end(NotSaved, s(0),    f, t, t, &[])), // W
        (Unsaving,                  Relaunch,                end(NotSaved, s(0),    f, t, t, &[])),

        // The save's answer settles the row, and only what still differs is sent.
        (SaveThenUnsaveQueued,      Save,                    end(Saved,    s(1),    t, f, t, &[])), // K
        (SaveThenUnsaveQueued,      SaveRefused,             end(Saved,    s(1),    t, f, t, &[])), // K
        (SaveThenUnsaveQueued,      Unsave,                  end(NotSaved, s(0),    f, f, f, &["delete P1"])), // K
        // Both unsave taps are the row's one unsave, and the refusal undoes it.
        (SaveThenUnsaveQueued,      UnsaveRefused,           end(Saved,    s(1),    t, f, t, &["delete P1"])), // K
        (SaveThenUnsaveQueued,      SaveCancelledAndLost,    end(Saved,    s(1),    t, t, f, &[])), // C
        (SaveThenUnsaveQueued,      SaveCancelledButApplied, end(Saved,    s(1),    t, t, f, &[])), // C
        (SaveThenUnsaveQueued,      UnsaveCancelledAndLost,  end(NotSaved, s(0),    f, t, f, &[])),
        (SaveThenUnsaveQueued,      Sync,                    end(NotSaved, s(0),    f, t, f, &[])), // W
        (SaveThenUnsaveQueued,      SyncFails,               end(NotSaved, s(0),    f, t, f, &[])), // W
        (SaveThenUnsaveQueued,      ListLoads,               end(NotSaved, s(0),    f, t, f, &[])), // W
        (SaveThenUnsaveQueued,      ListFails,               end(NotSaved, s(0),    f, t, f, &[])), // W
        (SaveThenUnsaveQueued,      Relaunch,                end(NotSaved, s(0),    f, t, f, &[])),

        (JustSaved,                 Save,                    end(Saved,    s(1),    t, f, t, &[])), // K
        (JustSaved,                 SaveRefused,             end(Saved,    s(1),    t, f, t, &[])), // K
        (JustSaved,                 Unsave,                  end(NotSaved, s(0),    f, f, f, &["delete P1"])),
        (JustSaved,                 UnsaveRefused,           end(Saved,    s(1),    t, f, t, &["delete P1"])),
        (JustSaved,                 SaveCancelledAndLost,    end(Saved,    s(1),    t, f, t, &[])), // K
        (JustSaved,                 SaveCancelledButApplied, end(Saved,    s(1),    t, f, t, &[])), // K
        (JustSaved,                 UnsaveCancelledAndLost,  end(NotSaved, s(0),    f, t, t, &["delete P1"])),
        (JustSaved,                 Sync,                    end(Saved,    s(1),    t, f, t, &["keys"])),
        (JustSaved,                 SyncFails,               end(Saved,    s(1),    t, f, t, &["keys"])),
        (JustSaved,                 ListLoads,               end(Saved,    s(1),    t, f, t, &["keys", "list"])), // A
        (JustSaved,                 ListFails,               end(Saved,    s(1),    t, f, t, &["keys", "list"])), // A
        (JustSaved,                 Relaunch,                end(Saved,    s(1),    t, f, t, &[])),

        (JustUnsaved,               Save,                    end(Saved,    s(1),    t, f, t, &["new P1"])),
        (JustUnsaved,               SaveRefused,             end(NotSaved, s(0),    f, f, f, &["new P1"])),
        (JustUnsaved,               Unsave,                  end(NotSaved, s(0),    f, f, f, &[])), // K
        (JustUnsaved,               UnsaveRefused,           end(NotSaved, s(0),    f, f, f, &[])), // K
        (JustUnsaved,               SaveCancelledAndLost,    end(Saved,    s(1),    t, t, f, &["new P1"])),
        (JustUnsaved,               SaveCancelledButApplied, end(Saved,    s(1),    t, t, t, &["new P1"])),
        (JustUnsaved,               UnsaveCancelledAndLost,  end(NotSaved, s(0),    f, f, f, &[])), // K
        (JustUnsaved,               Sync,                    end(NotSaved, s(0),    f, f, f, &["keys"])),
        (JustUnsaved,               SyncFails,               end(NotSaved, s(0),    f, f, f, &["keys"])),
        (JustUnsaved,               ListLoads,               end(NotSaved, s(0),    f, f, f, &["keys", "list"])), // A
        (JustUnsaved,               ListFails,               end(NotSaved, s(0),    f, f, f, &["keys", "list"])), // A
        (JustUnsaved,               Relaunch,                end(NotSaved, s(0),    f, f, f, &[])),

        (PastTheKeyLimit,           Save,                    end(Saved,    s(1001), t, f, t, &["new P1"])),
        (PastTheKeyLimit,           SaveRefused,             end(NotSaved, s(1001), f, f, t, &["new P1"])), // N
        (PastTheKeyLimit,           Unsave,                  end(NotSaved, s(1000), f, f, f, &["delete P1"])),
        (PastTheKeyLimit,           UnsaveRefused,           end(NotSaved, s(1001), f, f, t, &["delete P1"])), // N
        // Nothing says the post was already one of the 1,001, so the pending save counts once more.
        (PastTheKeyLimit,           SaveCancelledAndLost,    end(Saved,    s(1002), t, t, t, &["new P1"])), // #
        (PastTheKeyLimit,           SaveCancelledButApplied, end(Saved,    s(1002), t, t, t, &["new P1"])), // #
        (PastTheKeyLimit,           UnsaveCancelledAndLost,  end(NotSaved, s(1001), f, t, t, &["delete P1"])),
        (PastTheKeyLimit,           Sync,                    end(NotSaved, s(1001), f, f, t, &["keys"])), // N
        (PastTheKeyLimit,           SyncFails,               end(NotSaved, s(1001), f, f, t, &["keys"])), // N
        (PastTheKeyLimit,           ListLoads,               end(Saved,    s(1001), t, f, t, &["keys", "list"])), // A L
        (PastTheKeyLimit,           ListFails,               end(NotSaved, s(1001), f, f, t, &["keys", "list"])), // A N
        (PastTheKeyLimit,           Relaunch,                end(NotSaved, s(1001), f, f, t, &[])), // N

        (PastTheKeyLimitAndListed,  Save,                    end(Saved,    s(1001), t, f, t, &[])), // K L
        (PastTheKeyLimitAndListed,  SaveRefused,             end(Saved,    s(1001), t, f, t, &[])), // K L
        (PastTheKeyLimitAndListed,  Unsave,                  end(NotSaved, s(1000), f, f, f, &["delete P1"])),
        (PastTheKeyLimitAndListed,  UnsaveRefused,           end(Saved,    s(1001), t, f, t, &["delete P1"])), // L
        (PastTheKeyLimitAndListed,  SaveCancelledAndLost,    end(Saved,    s(1001), t, f, t, &[])), // K L
        (PastTheKeyLimitAndListed,  SaveCancelledButApplied, end(Saved,    s(1001), t, f, t, &[])), // K L
        (PastTheKeyLimitAndListed,  UnsaveCancelledAndLost,  end(NotSaved, s(1000), f, t, t, &["delete P1"])), // #
        // The row is older than every key returned, so a sync can't say and leaves it.
        (PastTheKeyLimitAndListed,  Sync,                    end(Saved,    s(1001), t, f, t, &["keys"])), // L
        (PastTheKeyLimitAndListed,  SyncFails,               end(Saved,    s(1001), t, f, t, &["keys"])), // L
        (PastTheKeyLimitAndListed,  ListLoads,               end(Saved,    s(1001), t, f, t, &["keys", "list"])), // A L
        (PastTheKeyLimitAndListed,  ListFails,               end(Saved,    s(1001), t, f, t, &["keys", "list"])), // A L
        (PastTheKeyLimitAndListed,  Relaunch,                end(Saved,    s(1001), t, f, t, &[])), // L
    ]
}

#[test]
fn every_start_and_action() {
    let rows = rows();

    // Every pair is in the table, once.
    let mut seen = [[0; ACTIONS.len()]; STARTS.len()];
    for (start_, action, _) in &rows {
        let (s, a) = position(*start_, *action);
        assert_eq!(
            (STARTS[s], ACTIONS[a]),
            (*start_, *action),
            "out of order in the lists"
        );
        seen[s][a] += 1;
    }
    for (s, start_) in STARTS.iter().enumerate() {
        for (a, action) in ACTIONS.iter().enumerate() {
            assert_eq!(
                seen[s][a], 1,
                "{start_:?} + {action:?} is in the table {} times",
                seen[s][a]
            );
        }
    }

    let mut failures = Vec::new();
    for (start_, action, want) in rows {
        let mut world = start(start_);
        let before = world.sent_count();
        act(&mut world, action);
        let got = End {
            status: world.status(identity(1)),
            count: world.count(),
            in_list: world.shown().contains(&"P1".to_string()),
            pending: world.pending(identity(1)),
            on_server: world.server.has("P1"),
            sent: world.sent_since(before),
        };
        if got != want {
            failures.push(format!(
                "{start_:?} + {action:?}:\n    want {want:?}\n    got  {got:?}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} rows failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

// Order: one request at a time, oldest tap first.

#[test]
fn writes_to_different_posts_also_go_out_one_at_a_time_in_tap_order() {
    let mut world = World::new();
    world.sync();
    world.save(identity(1));
    world.save(identity(2));
    world.unsave(identity(1));
    assert_eq!(
        world.sent_since(1),
        ["new P1"],
        "the others wait their turn"
    );

    world.answer_writes();

    assert_eq!(world.sent_since(1), ["new P1", "new P2", "delete P1"]);
    assert!(!world.server.has("P1"));
    assert!(world.server.has("P2"));
}

/// #1729 sent the second unsave too. The taps end as one row that wants the post unsaved, and the
/// delete in flight settles it.
#[test]
fn taps_on_a_post_with_a_write_in_flight_end_as_one_change_whatever_is_cancelled() {
    let mut world = World::new();
    world.server.saved.insert("P1".to_string());
    world.sync();
    world.unsave(identity(1));
    let save = world.save(identity(1));
    world.unsave(identity(1));
    world.cancel(save);
    assert_eq!(world.status(identity(1)), NotSaved);
    assert_eq!(world.sent_since(1), ["delete P1"]);

    world.answer_writes();

    assert_eq!(world.sent_since(1), ["delete P1"]);
    assert!(!world.server.has("P1"));
    assert_eq!(world.status(identity(1)), NotSaved);
}

/// The answer to the save through the URL says it's post 9, so the unsave behind it goes to post 9
/// alone. #1729 also sent `delete U9`.
#[test]
fn a_tap_through_another_key_waits_for_a_write_through_the_url() {
    let mut world = World::new();
    world.sync();
    world.save_url(U9);
    world.unsave_key("U9");
    world.answer_next_write();
    assert_eq!(world.sent_since(1), ["new U9", "delete P9"]);

    world.save(identity(9));
    assert_eq!(
        world.sent_since(1),
        ["new U9", "delete P9"],
        "the save waits"
    );

    world.answer_writes();
    assert_eq!(world.sent_since(1), ["new U9", "delete P9", "new P9"]);
    assert!(world.server.has("P9"));
    assert_eq!(world.status(identity(9)), Saved);
}

/// Before the device knows a URL is post 9's, a save through the URL still waits for an unsave of
/// the post. Its answer merges the two, and the newer tap, the save, wins.
#[test]
fn a_tap_through_a_url_waits_for_a_write_to_the_post_it_turns_out_to_be() {
    let mut world = World::new();
    world.sync();
    world.save(identity(9));
    world.answer_writes();
    world.unsave(identity(9));
    world.save_url(U9);
    assert_eq!(
        world.sent_since(1),
        ["new P9", "delete P9"],
        "the save waits"
    );

    world.answer_writes();
    assert_eq!(world.sent_since(1), ["new P9", "delete P9", "new U9"]);
    assert!(world.server.has("P9"));
    assert_eq!(world.url_status(U9), Saved);
    assert_eq!(world.status(identity(9)), Saved);
}

/// #1729 sent all four taps' requests. The first answer settles the row the three taps share, and
/// the last tap, save, already matches it.
#[test]
fn a_save_tapped_after_an_unsave_still_decides_when_the_answer_says_another_key() {
    let mut world = World::new();
    world.sync();
    world.save(jetpack_external());
    world.unsave(jetpack_external());
    world.save(jetpack_external());

    world.answer_next_write();
    assert_eq!(world.status(jetpack_external()), Saved);

    world.answer_writes();
    assert_eq!(world.sent_since(1), ["new F4"]);
    assert!(world.server.has("P3"));
    assert_eq!(world.status(jetpack_external()), Saved);
    assert_eq!(world.status(jetpack()), Saved);
}

// Jetpack posts

#[test]
fn saving_a_jetpack_post_sends_its_post_key() {
    let mut world = World::new();
    world.sync();
    world.save(jetpack());
    world.answer_writes();

    assert_eq!(world.sent_since(1), ["new P3"]);
    assert_eq!(world.server.saved, ["P3".to_string()].into());
    assert_eq!(world.status(jetpack()), Saved);
}

/// #1729 also sent `delete P3`. The keys returned every save, and the post is saved under F4 alone.
#[test]
fn a_jetpack_post_saved_under_its_feed_item_reads_saved_and_unsaves_that_key() {
    let mut world = World::new();
    world.server.saved.insert("F4".to_string());
    world.sync();
    assert_eq!(world.status(jetpack()), Saved);
    assert_eq!(world.status(jetpack_external()), Saved);

    world.unsave(jetpack());
    world.answer_writes();

    assert_eq!(world.sent_since(1), ["delete F4"]);
    assert!(world.server.saved.is_empty());
    assert_eq!(world.status(jetpack()), NotSaved);
    assert_eq!(world.count(), Some(0));
}

#[test]
fn before_a_sync_unsaving_a_jetpack_post_sends_both_keys() {
    let mut world = World::new();
    world.server.saved.insert("F4".to_string());

    world.unsave(jetpack());
    world.answer_writes();

    assert_eq!(world.sent_since(0), ["delete P3", "delete F4"]);
    assert!(world.server.saved.is_empty());
    assert_eq!(world.status(jetpack()), NotSaved);
}

/// Older saves under both of its keys, and the device knows feed item 4 is post 3.
#[test]
fn unsaving_a_jetpack_post_saved_under_both_keys_counts_both() {
    let mut world = World::new();
    world.server.saved.insert("F4".to_string());
    world.save(jetpack_external());
    world.answer_writes();
    world.sync();
    assert_eq!(world.count(), Some(2));

    world.unsave(jetpack());
    world.answer_writes();

    assert!(world.server.saved.is_empty());
    assert_eq!(world.count(), Some(0));
}

#[test]
fn saving_the_external_view_of_a_jetpack_post_saves_the_post() {
    let mut world = World::new();
    world.sync();
    world.save(jetpack_external());
    world.answer_writes();

    assert_eq!(world.sent_since(1), ["new F4"]);
    assert_eq!(world.server.saved, ["P3".to_string()].into());
    assert_eq!(world.status(jetpack_external()), Saved);
    assert_eq!(world.status(jetpack()), Saved);
    assert_eq!(world.shown(), ["P3"]);
}

/// The keys don't say which feed item a post save came from, so the external view of a post saved
/// elsewhere reads as not saved. #1729 still sent `delete F4` when it was unsaved from there; now
/// there's nothing to send, because the keys are complete and the view reads not saved.
#[test]
fn the_external_view_of_a_jetpack_post_saved_elsewhere_reads_not_saved() {
    let mut world = World::new();
    world.server.saved.insert("P3".to_string());
    world.sync();
    assert_eq!(
        world.status(jetpack_external()),
        NotSaved,
        "the API doesn't say"
    );

    world.unsave(jetpack_external());
    world.answer_writes();

    assert_eq!(world.sent_since(1), Vec::<String>::new());
    assert_eq!(world.status(jetpack()), Saved);
}

/// The saved list shows the post with both keys, but it's saved under its main key alone, so an
/// unsave from a screen that only knows the post key unsaves it everywhere. #1729 also checked a
/// list read lagging behind the unsave; reads were measured not to lag, and the lane orders them.
#[test]
fn a_listed_jetpack_post_unsaved_by_its_post_key_reads_unsaved_with_both_keys() {
    let mut world = World::new();
    world.server.saved.insert("P3".to_string());
    world.list_refresh();

    world.unsave(identity(3));
    world.answer_writes();

    assert!(world.server.saved.is_empty());
    assert_eq!(world.status(identity(3)), NotSaved);
    assert_eq!(world.status(jetpack()), NotSaved);
    assert_eq!(world.count(), Some(0));
    assert!(world.list_shown().is_empty());
}

/// #1729 also sent `delete F4`. The list told the device F4 is post 3's, and the post is saved once.
#[test]
fn unsaving_a_listed_jetpack_post_takes_one_off_the_count() {
    let mut world = World::new();
    world.server.saved.insert("P1".to_string());
    world.server.saved.insert("P3".to_string());
    world.list_refresh();
    assert_eq!(world.count(), Some(2));

    let before = world.sent_count();
    world.unsave(jetpack());
    world.answer_writes();

    assert_eq!(world.sent_since(before), ["delete P3"]);
    assert_eq!(world.server.total(), 1);
    assert_eq!(world.count(), Some(1));
}

/// Delete P3 is answered, then delete F4 is applied but the call is cancelled before its answer.
/// The sync says what the server holds.
#[test]
fn the_count_after_a_jetpack_unsave_cancelled_mid_way_and_a_sync() {
    let mut world = World::new();
    world.server.saved.insert("P1".to_string());
    world.server.saved.insert("P3".to_string());
    world.server.saved.insert("F4".to_string());
    world.sync();

    let call = world.unsave(jetpack());
    world.answer_next_write();
    let _ = world.apply("delete F4");
    world.cancel(call);
    assert_eq!(world.server.total(), 1);
    assert_eq!(world.count(), Some(1), "before the sync");

    world.sync();
    assert_eq!(world.count(), Some(1), "the server holds 1");
    assert_eq!(world.status(jetpack()), NotSaved);
}

/// A Jetpack post saved under F4 (older) and P3. Unsave: delete P3 times out, and delete F4 is
/// answered. #1729 held the timed-out delete for 60s; now a sync says the server still has P3, so
/// it's sent again.
#[test]
fn a_jetpack_unsave_partly_timed_out_is_sent_again_after_a_sync() {
    let mut world = World::new();
    world.server.saved.insert("F4".to_string());
    world.save(jetpack_external());
    world.answer_writes();
    assert_eq!(
        world.server.saved,
        ["F4".to_string(), "P3".to_string()].into()
    );
    world.sync();

    let before = world.sent_count();
    world.unsave(jetpack());
    let late = world.time_out_next_write();
    assert_eq!(late, "delete P3");
    world.answer_next_write();
    assert_eq!(world.sent_since(before), ["delete P3", "delete F4"]);
    assert_eq!(world.status(jetpack()), NotSaved);

    // The server hasn't applied the late delete, and the sync says P3 is saved.
    world.sync();
    assert_eq!(
        world.sent_since(before),
        ["delete P3", "delete F4", "keys", "delete P3"]
    );
    assert!(world.server.saved.is_empty());
    assert_eq!(world.status(jetpack()), NotSaved);

    // It lands late, and changes nothing.
    let _ = world.apply(&late);
    world.sync();
    assert!(world.server.saved.is_empty());
    assert_eq!(world.status(jetpack()), NotSaved);
}

/// Saved under post 3. The list returns it by its feed item while its WordPress.com copy fails to
/// load.
#[test]
fn a_jetpack_post_the_list_returns_by_its_feed_item_shows_in_the_list() {
    let mut world = World::new();
    world.server.saved.insert("P3".to_string());
    world.sync();

    world.start_list_refresh();
    let keys = world.take_held(|held| held.what == "keys").unwrap();
    world.answer(keys, false);
    let list = world.take_held(|held| held.what == "list").unwrap();
    let _ = list
        .reply
        .send(respond(200, json!({ "posts": [listed_rss_post(4)] })));
    world.pool.run_until_stalled();

    assert_eq!(world.status(jetpack()), Saved);
    assert_eq!(world.shown(), ["F4"], "the list shows a saved post");
}

// A URL of a WordPress.com post: the API stores a save by the URL as the post.

#[test]
fn saving_a_url_of_a_post_saves_the_post() {
    let mut world = World::new();
    world.sync();
    world.save_url(U9);
    world.answer_writes();

    assert_eq!(world.sent_since(1), ["new U9"]);
    assert_eq!(world.server.saved, ["P9".to_string()].into());
    assert_eq!(world.url_status(U9), Saved);
    assert_eq!(world.status(identity(9)), Saved);
    assert_eq!(world.shown(), ["P9"]);
}

#[test]
fn a_sync_says_which_url_a_post_was_saved_by() {
    let mut world = World::new();
    world.server.saved.insert("P9".to_string());
    world.sync();

    assert_eq!(
        world.url_status("https://example.com/p9/?utm_source=x"),
        Saved
    );
}

/// #1729 also sent `delete U9`. The keys say U9 is stored as post 9.
#[test]
fn unsaving_a_url_sends_the_post_it_is_stored_as() {
    let mut world = World::new();
    world.server.saved.insert("P9".to_string());
    world.sync();

    world.unsave_key("U9");
    world.answer_writes();

    assert_eq!(world.sent_since(1), ["delete P9"]);
    assert!(world.server.saved.is_empty());
    assert_eq!(world.url_status(U9), NotSaved);
    assert_eq!(world.status(identity(9)), NotSaved);
}

/// The row keeps the URL once it's unsaved, so saving the URL again saves post 9. #1729 checked
/// that a repeated unsave sent both keys; a repeated unsave now sends nothing.
#[test]
fn an_unsaved_url_still_knows_its_post_next_time() {
    let mut world = World::new();
    world.sync();
    world.save_url(U9);
    world.answer_writes();
    world.unsave_key("U9");
    world.answer_writes();
    let before = world.sent_count();
    world.unsave_key("U9");
    world.answer_writes();
    assert_eq!(world.sent_since(before), Vec::<String>::new());

    world.save_url(U9);
    world.answer_writes();

    assert_eq!(world.sent_since(before), ["new P9"]);
    assert_eq!(world.status(identity(9)), Saved);
}

// RSS items, saved under their feed item.

#[test]
fn an_rss_item_saves_and_unsaves_under_its_feed_item() {
    let mut world = World::new();
    world.sync();
    world.save(rss_identity(10));
    world.answer_writes();
    assert_eq!(world.server.saved, ["F10".to_string()].into());
    assert_eq!(world.status(rss_identity(10)), Saved);

    world.unsave(rss_identity(10));
    world.answer_writes();

    assert_eq!(world.sent_since(1), ["new F10", "delete F10"]);
    assert!(world.server.saved.is_empty());
    assert_eq!(world.status(rss_identity(10)), NotSaved);
}

#[test]
fn saving_the_url_of_an_rss_item_saves_the_item() {
    let mut world = World::new();
    world.sync();
    world.save_url(U10);
    world.answer_writes();

    assert_eq!(world.server.saved, ["F10".to_string()].into());
    assert_eq!(world.status(rss_identity(10)), Saved);
    assert_eq!(world.url_status(U10), Saved);
}

/// The API answers `post_not_found` only for a feed item that doesn't exist, and nothing can be
/// saved under one. Before a sync, so the delete is sent.
#[test]
fn unsaving_a_feed_item_that_does_not_exist_counts_as_unsaved() {
    let mut world = World::new();

    world.unsave(rss_identity(99));
    world.answer_writes();

    assert_eq!(world.sent_since(0), ["delete F99"]);
    assert_eq!(world.status(rss_identity(99)), NotSaved);
}

// Requests in doubt

/// A known limit, as in #1729: the timed-out save lands after the unsave tapped after it, so the
/// server ends up opposite to the last tap. Now the next sync puts it right.
#[test]
fn a_timed_out_write_applied_after_a_later_tap_is_put_right_by_the_next_sync() {
    let mut world = World::new();
    world.sync();
    world.save(identity(1));
    let sent = world.time_out_next_write();
    world.unsave(identity(1));
    world.answer_writes();
    let _ = world.apply(&sent);

    assert!(world.server.has("P1"), "the server applied the save last");
    assert_eq!(world.status(identity(1)), NotSaved, "the user's last tap");

    world.sync();
    assert!(!world.server.has("P1"));
    assert_eq!(world.status(identity(1)), NotSaved);
}

/// An unsave that timed out, then a save. The save waits for a sync, which says the unsave never
/// landed, so there's nothing to send.
#[test]
fn a_save_after_a_timed_out_unsave_waits_for_a_sync() {
    let mut world = World::new();
    world.server.saved.insert("P1".to_string());
    world.sync();
    world.unsave(identity(1));
    world.time_out_next_write();
    world.save(identity(1));
    world.answer_writes();
    assert_eq!(world.sent_since(1), ["delete P1"]);
    assert_eq!(world.status(identity(1)), Saved);

    for _ in 0..2 {
        world.sync();
        assert_eq!(world.status(identity(1)), Saved);
        assert_eq!(world.count(), Some(1));
    }
    assert!(world.server.has("P1"));
}

/// The keys said 1 save. The first save timed out but was applied; the second's answer says 3.
/// #1729 counted 3 straight away; the pending first save counts once more until a sync.
#[test]
fn a_timed_out_write_before_an_answered_one_is_counted_right_after_a_sync() {
    let mut world = World::new();
    world.server.saved.insert("P5".to_string());
    world.sync();
    world.save(identity(1));
    let sent = world.time_out_next_write();
    let _ = world.apply(&sent);
    world.save(identity(2));
    world.answer_writes();
    assert_eq!(world.server.total(), 3);
    assert_eq!(world.count(), Some(4), "the save in doubt counts again");

    world.sync();
    assert_eq!(world.count(), Some(3));
}

/// Save P1 is answered. Save P2 times out, but the server applied it.
#[test]
fn the_count_after_a_sync_that_has_a_timed_out_save() {
    let mut world = World::new();
    world.sync();
    world.save(identity(1));
    world.answer_writes();
    world.save(identity(2));
    let sent = world.time_out_next_write();
    let _ = world.apply(&sent);
    assert_eq!(world.server.total(), 2);
    assert_eq!(world.count(), Some(2), "before the sync");

    world.sync();
    assert_eq!(world.count(), Some(2), "the server holds 2");
}

// Past the 1,000 keys

/// 1,002 saves, with `hidden` among the oldest, which the keys don't return.
fn past_the_keys_with(world: &mut World, hidden: &[&str]) {
    for id in 1001..=2000 {
        world.server.saved.insert(format!("P{id}"));
    }
    for name in hidden {
        world.server.saved.insert(name.to_string());
        world.server.hidden.insert(name.to_string());
    }
}

/// #1729 read `Unknown` before the unsave.
#[test]
fn past_the_key_limit_a_post_unsaved_here_stays_unsaved_after_a_sync() {
    let mut world = World::new();
    past_the_keys_with(&mut world, &["P1", "P2"]);
    world.sync();
    assert_eq!(world.status(identity(1)), NotSaved);

    world.unsave(identity(1));
    world.answer_writes();
    for _ in 0..2 {
        world.sync();
        assert_eq!(world.status(identity(1)), NotSaved);
        assert_eq!(world.count(), Some(1001));
    }
}

#[test]
fn past_the_key_limit_a_post_saved_here_stays_saved_after_a_sync() {
    let mut world = World::new();
    past_the_keys_with(&mut world, &["P1", "P2"]);
    world.sync();
    world.save(identity(1));
    world.answer_writes();
    world.sync();
    assert_eq!(world.status(identity(1)), Saved);
    assert_eq!(world.count(), Some(1002));
}

/// Unsaving one of 1,001 brings the account back within the keys, so they say about the rest.
/// #1729 read post 2 as `Unknown` before.
#[test]
fn back_within_the_key_limit_the_keys_say_again() {
    let mut world = World::new();
    past_the_keys_with(&mut world, &["P1"]);
    world.server.saved.remove("P2000");
    world.sync();
    assert_eq!(world.status(identity(2)), NotSaved);

    world.unsave(identity(1));
    world.answer_writes();
    world.server.hidden.clear();
    world.sync();
    assert_eq!(world.status(identity(1)), NotSaved);
    assert_eq!(world.status(identity(2)), NotSaved);
}

/// 1,002 saves, P1 and P2 past the keys. Save P5 (answered: 1,003). Unsave P1500, which the keys
/// have: the request times out, but the server applied it, so it holds 1,002.
#[test]
fn past_the_key_limit_a_sync_that_has_a_timed_out_unsave_counts_it_once() {
    let mut world = World::new();
    past_the_keys_with(&mut world, &["P1", "P2"]);
    world.sync();
    world.save(identity(5));
    world.answer_writes();
    assert_eq!(world.count(), Some(1003));

    world.unsave(identity(1500));
    let sent = world.time_out_next_write();
    let _ = world.apply(&sent);
    assert_eq!(world.server.total(), 1002);
    assert_eq!(world.count(), Some(1002));

    world.sync();
    assert_eq!(world.status(identity(1500)), NotSaved);
    assert_eq!(world.count(), Some(1002));
}

/// New risk: past the 1,000 keys a sync can't tell a save older than every key returned from one
/// that was unsaved, so it leaves those rows alone. A row no older than the oldest key returned
/// would have been returned, so it isn't saved on the server.
#[test]
fn past_the_key_limit_a_sync_decides_only_rows_no_older_than_the_oldest_key_returned() {
    let mut world = World::new();
    past_the_keys_with(&mut world, &["P1"]);
    world.list_refresh();
    world.save(identity(7));
    world.answer_writes();
    assert_eq!(world.status(identity(1)), Saved, "the list showed it");
    assert_eq!(world.status(identity(7)), Saved);

    // Post 7 is unsaved on the web, and post 1 stays saved, past the end of the keys.
    world.server.saved.remove("P7");
    world.sync();

    assert_eq!(world.status(identity(1)), Saved, "older than every key");
    assert_eq!(
        world.status(identity(7)),
        NotSaved,
        "newer, so it'd be there"
    );
    assert_eq!(world.shown().first().map(String::as_str), Some("P2000"));
    assert!(world.shown().contains(&"P1".to_string()));
    assert!(!world.shown().contains(&"P7".to_string()));
    assert_eq!(world.count(), Some(1001));
}

/// New risk: a save made here that the keys don't return, past the 1,000 keys, while the server
/// does have it. The date rule says it was unsaved elsewhere, so the device drops it. This can't
/// happen with the live API, whose newest save is always among the keys returned.
#[test]
fn past_the_key_limit_a_save_the_keys_wrongly_leave_out_reads_unsaved() {
    let mut world = World::new();
    past_the_keys_with(&mut world, &["P1"]);
    world.sync();
    world.save(identity(7));
    world.answer_writes();

    world.server.hidden.insert("P7".to_string());
    world.sync();

    assert_eq!(world.status(identity(7)), NotSaved);
    assert!(world.server.has("P7"));
}

// The saved list

/// Saved on the web between the list refresh's read of the keys and its page. The page has it, so
/// it shows, and reads saved. #1729 read it as not saved until the keys had it.
#[test]
fn a_post_saved_elsewhere_shows_in_the_list_while_the_keys_lag() {
    let mut world = World::new();
    world.sync();

    world.start_list_refresh();
    let keys = world.take_held(|held| held.what == "keys").unwrap();
    world.answer(keys, false);
    world.server.saved.insert("P1".to_string());
    let list = world.take_held(|held| held.what == "list").unwrap();
    world.answer(list, false);

    assert_eq!(world.status(identity(1)), Saved);
    assert_eq!(world.shown(), ["P1"], "the list shows a saved post");
}

/// A sync never removes a post this device saved from the list. Replaces #1729's
/// `a_keys_refresh_keeps_a_recent_save_the_list_has_not_shown`.
#[test]
fn a_sync_keeps_a_post_saved_here_in_the_list() {
    let mut world = World::new();
    world.sync();
    world.save(identity(1));
    world.answer_writes();
    world.sync();
    assert_eq!(world.shown(), ["P1"]);
}

// Found in review: the sequences that lost a save, in their new form

/// A Jetpack post the device knows is stored as its post. An unsave, a save, then an unsave whose
/// delete times out, then a save. #1729 sent the unsave as two deletes and timed out the second;
/// now the unsave is one delete, of P3. The save waits for a sync, which finds the late delete
/// landed and sends the save again.
#[test]
fn a_jetpack_post_saved_again_after_its_unsave_timed_out_reads_saved() {
    let mut world = World::new();
    world.server.saved.insert("P3".to_string());
    world.sync();
    world.unsave(jetpack());
    world.answer_writes();
    world.save(jetpack());
    world.answer_writes();
    world.unsave(jetpack());
    let late = world.time_out_next_write();
    assert_eq!(late, "delete P3");
    world.save(jetpack());
    world.answer_writes();
    assert_eq!(world.status(jetpack()), Saved);
    assert_eq!(world.shown(), ["P3"]);

    // The late delete lands, and the sync sends the save again.
    let _ = world.apply(&late);
    assert!(!world.server.has("P3"));
    world.sync();

    assert!(world.server.has("P3"));
    assert_eq!(world.status(jetpack()), Saved);
    assert_eq!(world.shown(), ["P3"]);
}

/// A save of U9 times out, then an unsave of U9 waits for a sync, before the device knows U9 is
/// post 9. The save of post 9 is answered, and a sync then says U9 is post 9. Merging the rows,
/// the newer tap, the save of post 9, wins.
#[test]
fn a_sync_that_maps_a_url_does_not_let_an_older_unsave_undo_an_answered_save() {
    let mut world = World::new();
    world.sync();
    world.save_url(U9);
    world.time_out_next_write();
    world.unsave_key("U9");
    world.save(identity(9));
    world.answer_writes();
    assert_eq!(world.status(identity(9)), Saved);

    world.sync();
    assert!(world.server.has("P9"));
    assert_eq!(world.status(identity(9)), Saved);
    assert_eq!(world.url_status(U9), Saved);
    assert_eq!(world.shown(), ["P9"]);
}

/// Post 9 is saved here and answered, and a sync says U9 is post 9. A second save, through U9, is
/// already what the row wants, so nothing is sent and nothing can be refused. #1729 sent it.
#[test]
fn a_second_save_through_the_url_sends_nothing_and_keeps_the_post_in_the_list() {
    let mut world = World::new();
    world.sync();
    world.save(identity(9));
    world.answer_writes();
    world.sync();
    assert_eq!(world.shown(), ["P9"]);

    let before = world.sent_count();
    world.save_url(U9);
    world.answer_writes_refusing("new U9", before);
    assert_eq!(world.sent_since(before), Vec::<String>::new());
    assert!(world.server.has("P9"));
    assert_eq!(world.status(identity(9)), Saved);
    assert_eq!(world.shown(), ["P9"]);
}

// New risks

/// A save in doubt is sent again once a sync says the server lacks it, and the late original
/// landing too leaves one save.
#[test]
fn a_save_in_doubt_is_sent_again_after_a_sync_and_never_duplicated() {
    let mut world = World::new();
    world.sync();
    world.save(identity(1));
    let late = world.time_out_next_write();
    assert_eq!(world.status(identity(1)), Saved);
    assert!(world.pending(identity(1)));

    let before = world.sent_count();
    world.sync();
    assert_eq!(world.sent_since(before), ["keys", "new P1"]);
    let _ = world.apply(&late);
    world.sync();

    assert_eq!(world.server.saved, ["P1".to_string()].into());
    assert_eq!(world.count(), Some(1));
    assert!(!world.pending(identity(1)));
    assert_eq!(world.sent_since(before), ["keys", "new P1", "keys"]);
}

/// The same, when the server applied the timed-out save: the sync says so, and nothing is sent.
#[test]
fn a_save_in_doubt_the_server_applied_is_not_sent_again() {
    let mut world = World::new();
    world.sync();
    world.save(identity(1));
    let late = world.time_out_next_write();
    let _ = world.apply(&late);

    let before = world.sent_count();
    world.sync();

    assert_eq!(world.sent_since(before), ["keys"]);
    assert_eq!(world.count(), Some(1));
    assert!(!world.pending(identity(1)));
}

/// A tap made while a sync is reading shows at once, and is sent once the read is answered.
#[test]
fn a_tap_waits_behind_a_running_sync_but_shows_at_once() {
    let mut world = World::new();
    world.sync();
    let before = world.sent_count();
    world.start_sync();
    assert_eq!(world.sent_since(before), ["keys"]);

    world.save(identity(1));
    assert_eq!(world.status(identity(1)), Saved);
    assert_eq!(world.shown(), ["P1"]);
    assert!(world.pending(identity(1)));
    assert_eq!(world.sent_since(before), ["keys"], "the save waits");

    world.answer_since(before, &[]);

    assert_eq!(world.sent_since(before), ["keys", "new P1"]);
    assert!(world.server.has("P1"));
    assert!(!world.pending(identity(1)));
    assert_eq!(world.shown(), ["P1"]);
}

/// The same with the saved list's refresh, whose page doesn't have the save yet.
#[test]
fn a_save_tapped_during_a_list_refresh_stays_in_the_list() {
    let mut world = World::new();
    world.server.saved.insert("P2".to_string());
    world.list_refresh();
    let before = world.sent_count();
    world.start_list_refresh();
    let keys = world.take_held(|held| held.what == "keys").unwrap();
    world.answer(keys, false);

    world.save(identity(1));
    assert_eq!(world.shown(), ["P1", "P2"]);
    let list = world.take_held(|held| held.what == "list").unwrap();
    world.answer(list, false);
    assert_eq!(world.shown(), ["P1", "P2"], "the page didn't have it");

    world.answer_writes();
    assert_eq!(world.sent_since(before), ["keys", "list", "new P1"]);
    assert_eq!(world.shown(), ["P1", "P2"]);
}

/// An unsave tapped offline waits on the device, through a relaunch, and the first sync once the
/// device is online sends it.
#[test]
fn an_unsave_queued_while_offline_is_sent_once_online() {
    let mut world = World::new();
    world.server.saved.insert("P1".to_string());
    world.list_refresh();
    world.set_offline(true);

    let before = world.sent_count();
    world.unsave(identity(1));
    assert_eq!(world.sent_since(before), ["delete P1"]);
    assert_eq!(world.status(identity(1)), NotSaved);
    assert!(world.shown().is_empty());

    world.relaunch();
    assert_eq!(world.status(identity(1)), NotSaved);
    assert!(world.pending(identity(1)));

    world.set_offline(false);
    world.sync();
    assert_eq!(world.sent_since(before), ["delete P1", "delete P1", "keys"]);
    assert!(!world.server.has("P1"));
    assert!(!world.pending(identity(1)));
    assert_eq!(world.status(identity(1)), NotSaved);
}

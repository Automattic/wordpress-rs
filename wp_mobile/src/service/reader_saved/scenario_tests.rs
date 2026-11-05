//! The service against a fake saved posts API, one known starting state and one known action at a
//! time, each with the end state written out by hand.
//!
//! [`every_start_and_action`] covers every state a post can be in that we know of, crossed with
//! every action. The tests after it cover the shapes of key the API stores differently: Jetpack
//! posts, URLs and RSS items.
//!
//! The fake answers the way the live API was seen to on 2026-11-04 (see `FakeServer::delete`).
//! Requests are held until a test answers them, so a test decides the order things happen in.

use super::*;
use crate::service::reader_saved::state::MAX_READ_LAG;
use ReaderSavedStatus::{NotSaved, Saved, Unknown};
use futures::executor::LocalPool;
use futures::future::RemoteHandle;
use futures::task::LocalSpawnExt;
use std::collections::BTreeSet;
use std::time::{Duration, Instant};
use wp_api::request::endpoint::WpEndpointUrl;

// Keys, by short name: `P1` is post 1, `F4` feed item 4, `U9` the URL of post 9.

const U9: &str = "https://example.com/p9";
const U10: &str = "https://example.com/rss10";

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

fn names(keys: &[ReaderSavedKey]) -> Vec<String> {
    keys.iter().map(name).collect()
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
    /// Saved keys past the 1,000 the keys endpoint returns.
    hidden: BTreeSet<String>,
    /// How many other saves there are, all past the 1,000.
    others: u64,
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
        self.saved.len() as u64 + self.others
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

    fn keys_response(&self) -> Value {
        let entries: Vec<_> = self
            .saved
            .difference(&self.hidden)
            .enumerate()
            .map(|(index, name)| {
                let mut entry = json_of(name);
                entry["id"] = json!(index + 1);
                entry["date_saved"] = json!("2026-11-02T10:15:00+00:00");
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
            let post = match name.as_str() {
                "P3" | "F4" => json!({
                    "ID": 3, "site_ID": 123, "feed_ID": 5, "feed_item_ID": 4, "is_external": false
                }),
                name if name.starts_with('P') => listed_post(name[1..].parse().unwrap()),
                name => listed_rss_post(name[1..].parse().unwrap()),
            };
            if !posts.contains(&post) {
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
        500,
        json!({ "code": "internal", "message": "", "data": { "status": 500 } }),
    )
}

type Reply = oneshot::Sender<Result<WpNetworkResponse, RequestExecutionError>>;

/// A request the executor is holding, described as `new P1`, `delete U9`, `keys` or `list`.
struct Held {
    what: String,
    reply: Reply,
}

/// Holds every request until the test answers it.
#[derive(Default)]
struct HoldingExecutor {
    held: Mutex<Vec<Held>>,
    /// Every request made, in order.
    sent: Mutex<Vec<String>>,
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
        let (reply, response) = oneshot::channel();
        self.sent.lock().unwrap().push(what.clone());
        self.held.lock().unwrap().push(Held { what, reply });
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

/// The service, the fake API, and the clock, all driven by hand.
struct World {
    pool: LocalPool,
    service: Arc<ReaderSavedService>,
    executor: Arc<HoldingExecutor>,
    clock: Arc<Mutex<Instant>>,
    server: FakeServer,
    /// What a read that may predate the latest change returns.
    stale: Option<FakeServer>,
    /// Each call started, until it's cancelled.
    calls: Vec<Option<RemoteHandle<()>>>,
}

impl World {
    fn new() -> Self {
        let clock = Arc::new(Mutex::new(Instant::now()));
        let executor = Arc::new(HoldingExecutor::default());
        let service_clock = clock.clone();
        let service = Arc::new(ReaderSavedService::with_clock(
            WpApiClientDelegate {
                auth_provider: Arc::new(WpAuthenticationProvider::none()),
                request_executor: executor.clone(),
                middleware_pipeline: Arc::new(WpApiMiddlewarePipeline::default()),
                app_notifier: Arc::new(EmptyAppNotifier),
                language_provider: None,
            },
            ReaderSavedSource::Android,
            Arc::new(move || *service_clock.lock().unwrap()),
        ));
        Self {
            pool: LocalPool::new(),
            service,
            executor,
            clock,
            server: FakeServer::default(),
            stale: None,
            calls: Vec::new(),
        }
    }

    fn advance(&mut self, by: Duration) {
        *self.clock.lock().unwrap() += by;
    }

    /// Starts a call, and runs it until it waits on a request. Returns its index, to cancel it.
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

    /// Cancels a call, as Kotlin does by dropping its future. A request it had sent never reaches
    /// the server.
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

    /// Answers the oldest write held, applying it to the server, or refusing it.
    fn answer_next_write(&mut self, refuse: bool) -> bool {
        let Some(held) =
            self.take_held(|held| held.what.starts_with("new") || held.what.starts_with("delete"))
        else {
            return false;
        };
        let (verb, sent) = held.what.split_once(' ').unwrap();
        let response = match (refuse, verb) {
            (true, _) => refused(),
            (false, "new") => self.server.save(sent),
            (false, _) => self.server.delete(sent),
        };
        let _ = held.reply.send(response);
        self.pool.run_until_stalled();
        true
    }

    /// Answers every write, oldest first, including any a write sends once it's answered.
    fn answer_writes(&mut self) {
        while self.answer_next_write(false) {}
    }

    /// Answers every write, refusing those sent after the first `before` requests.
    fn answer_writes_refusing_after(&mut self, before: usize) {
        loop {
            let refuse = {
                let held = self.executor.held.lock().unwrap();
                let sent = self.executor.sent.lock().unwrap();
                let Some(next) = held
                    .iter()
                    .find(|held| held.what.starts_with("new") || held.what.starts_with("delete"))
                else {
                    break;
                };
                // A request is new if it was sent after `before`, counting from the newest.
                let index = sent.iter().rposition(|what| *what == next.what).unwrap();
                index >= before
            };
            self.answer_next_write(refuse);
        }
    }

    /// Refreshes, and answers with what `read` holds.
    fn refresh_from(&mut self, read: FakeServer) {
        self.call(|service| async move { service.refresh().await });
        let held = self.take_held(|held| held.what == "keys").unwrap();
        let _ = held.reply.send(respond(200, read.keys_response()));
        self.pool.run_until_stalled();
    }

    fn refresh(&mut self) {
        self.refresh_from(self.server.clone());
    }

    /// Starts a refresh that will read `older`, then after `MAX_READ_LAG` one that reads what the
    /// server holds, and answers the newer one first.
    fn refreshes_land_out_of_order(&mut self, older: FakeServer) {
        self.call(|service| async move { service.refresh().await });
        self.advance(MAX_READ_LAG + Duration::from_secs(1));
        self.call(|service| async move { service.refresh().await });
        let newer = {
            let mut held = self.executor.held.lock().unwrap();
            let index = held.iter().rposition(|held| held.what == "keys").unwrap();
            held.remove(index)
        };
        let _ = newer.reply.send(respond(200, self.server.keys_response()));
        self.pool.run_until_stalled();
        let older_held = self.take_held(|held| held.what == "keys").unwrap();
        let _ = older_held.reply.send(respond(200, older.keys_response()));
        self.pool.run_until_stalled();
    }

    fn list_from(&mut self, read: FakeServer) {
        self.call(
            |service| async move { service.list_posts(ReaderSavedPostsParams::default()).await },
        );
        let held = self.take_held(|held| held.what == "list").unwrap();
        let _ = held.reply.send(respond(200, read.list_response()));
        self.pool.run_until_stalled();
    }

    fn sent_since(&self, before: usize) -> Vec<String> {
        self.executor.sent.lock().unwrap()[before..].to_vec()
    }

    fn sent_count(&self) -> usize {
        self.executor.sent.lock().unwrap().len()
    }

    fn status(&self, post: ReaderPostSaveIdentity) -> ReaderSavedStatus {
        self.service.status(post)
    }

    fn url_status(&self, url: &str) -> ReaderSavedStatus {
        self.service.state().status(&[ReaderSavedKey::Url {
            url: url.to_string(),
        }])
    }

    fn recent(&self) -> Vec<String> {
        names(&self.service.recent_saves())
    }
}

// Every start, crossed with every action, for a plain WordPress.com post: post 1.

#[derive(Debug, Clone, Copy, PartialEq)]
enum Start {
    /// No refresh yet, and the post isn't saved.
    UnknownUnsaved,
    /// No refresh yet, and the post is saved.
    UnknownSaved,
    /// Refreshed, and not saved.
    KnownUnsaved,
    /// Refreshed, and saved.
    KnownSaved,
    /// Read as saved, then unsaved on another device.
    SavedButUnsavedElsewhere,
    /// Read as not saved, then saved on another device.
    UnsavedButSavedElsewhere,
    /// Saved on this device, and the request hasn't been answered.
    Saving,
    /// Unsaved on this device, and the request hasn't been answered.
    Unsaving,
    /// Saved on this device 10s ago.
    JustSaved,
    /// Unsaved on this device 10s ago.
    JustUnsaved,
    /// Saved, but older than the 1,000 keys a refresh returns: 200 saves in all.
    PastTheKeyLimit,
}

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
    /// Tap save, then cancel the call before it's answered. Its request never reaches the server.
    SaveCancelled,
    /// Refresh after `MAX_READ_LAG`, so the read includes everything.
    FreshRefresh,
    /// Refresh now, and the read predates the latest change.
    StaleRefresh,
    /// A stale refresh starts, then a fresh one after `MAX_READ_LAG`, and the fresh one lands
    /// first.
    RefreshesOutOfOrder,
    /// The saved list loads, showing what the server holds.
    ListLoads,
    /// An hour passes, and nothing else happens.
    TimePasses,
}

/// What the user sees afterwards, what the server holds, and the requests the action sent.
#[derive(Debug, PartialEq)]
struct End {
    status: ReaderSavedStatus,
    count: Option<u64>,
    in_recent_saves: bool,
    on_server: bool,
    sent: Vec<String>,
}

fn start(start: Start) -> World {
    let mut world = World::new();
    let p1 = || "P1".to_string();
    match start {
        Start::UnknownUnsaved => {}
        Start::UnknownSaved => {
            world.server.saved.insert(p1());
        }
        Start::KnownUnsaved => world.refresh(),
        Start::KnownSaved => {
            world.server.saved.insert(p1());
            world.refresh();
        }
        Start::SavedButUnsavedElsewhere => {
            world.server.saved.insert(p1());
            world.refresh();
            world.stale = Some(world.server.clone());
            world.server.saved.remove("P1");
        }
        Start::UnsavedButSavedElsewhere => {
            world.refresh();
            world.stale = Some(world.server.clone());
            world.server.saved.insert(p1());
        }
        Start::Saving => {
            world.refresh();
            world.save(identity(1));
        }
        Start::Unsaving => {
            world.server.saved.insert(p1());
            world.refresh();
            world.unsave(identity(1));
        }
        Start::JustSaved => {
            world.refresh();
            world.stale = Some(world.server.clone());
            world.save(identity(1));
            world.answer_writes();
        }
        Start::JustUnsaved => {
            world.server.saved.insert(p1());
            world.refresh();
            world.stale = Some(world.server.clone());
            world.unsave(identity(1));
            world.answer_writes();
        }
        Start::PastTheKeyLimit => {
            world.server.saved.insert(p1());
            world.server.hidden.insert(p1());
            world.server.others = 199;
            world.refresh();
        }
    }
    if world.stale.is_none() {
        world.stale = Some(world.server.clone());
    }
    world.advance(Duration::from_secs(10));
    world
}

fn act(world: &mut World, action: Action) {
    match action {
        Action::Save => {
            world.save(identity(1));
            world.answer_writes();
        }
        Action::SaveRefused => {
            let before = world.sent_count();
            world.save(identity(1));
            world.answer_writes_refusing_after(before);
        }
        Action::Unsave => {
            world.unsave(identity(1));
            world.answer_writes();
        }
        Action::UnsaveRefused => {
            let before = world.sent_count();
            world.unsave(identity(1));
            world.answer_writes_refusing_after(before);
        }
        Action::SaveCancelled => {
            let call = world.save(identity(1));
            world.cancel(call);
        }
        Action::FreshRefresh => {
            world.advance(MAX_READ_LAG + Duration::from_secs(1));
            world.refresh();
        }
        Action::StaleRefresh => {
            let stale = world.stale.clone().unwrap();
            world.refresh_from(stale);
        }
        Action::RefreshesOutOfOrder => {
            let stale = world.stale.clone().unwrap();
            world.refreshes_land_out_of_order(stale);
        }
        Action::ListLoads => {
            let server = world.server.clone();
            world.list_from(server);
        }
        Action::TimePasses => world.advance(Duration::from_secs(3600)),
    }
}

fn end(
    status: ReaderSavedStatus,
    count: Option<u64>,
    in_recent_saves: bool,
    on_server: bool,
    sent: &[&str],
) -> End {
    End {
        status,
        count,
        in_recent_saves,
        on_server,
        sent: sent.iter().map(|what| what.to_string()).collect(),
    }
}

/// Every row: a start, an action, and the end state.
///
/// Two rows, marked "can't know", expect a count that differs from the server's: the device never
/// learns that a post past the key limit was saved, so a save or unsave of it counts as a change.
#[rustfmt::skip]
fn rows() -> Vec<(Start, Action, End)> {
    use Action::*;
    use Start::*;
    let n = None;
    vec![
        // start                    action                   status    count       recent server sent
        (UnknownUnsaved,            Save,                end(Saved,    n,          true,  true,  &["new P1"])),
        (UnknownUnsaved,            SaveRefused,         end(Unknown,  n,          false, false, &["new P1"])),
        (UnknownUnsaved,            Unsave,              end(NotSaved, n,          false, false, &["delete P1"])),
        (UnknownUnsaved,            UnsaveRefused,       end(Unknown,  n,          false, false, &["delete P1"])),
        (UnknownUnsaved,            SaveCancelled,       end(Unknown,  n,          false, false, &["new P1"])),
        (UnknownUnsaved,            FreshRefresh,        end(NotSaved, Some(0),    false, false, &["keys"])),
        (UnknownUnsaved,            RefreshesOutOfOrder, end(NotSaved, Some(0),    false, false, &["keys", "keys"])),
        (UnknownUnsaved,            StaleRefresh,        end(NotSaved, Some(0),    false, false, &["keys"])),
        (UnknownUnsaved,            ListLoads,           end(Unknown,  n,          false, false, &["list"])),
        (UnknownUnsaved,            TimePasses,          end(Unknown,  n,          false, false, &[])),

        (UnknownSaved,              Save,                end(Saved,    n,          true,  true,  &["new P1"])),
        (UnknownSaved,              SaveRefused,         end(Unknown,  n,          false, true,  &["new P1"])),
        (UnknownSaved,              Unsave,              end(NotSaved, n,          false, false, &["delete P1"])),
        (UnknownSaved,              UnsaveRefused,       end(Unknown,  n,          false, true,  &["delete P1"])),
        (UnknownSaved,              SaveCancelled,       end(Unknown,  n,          false, true,  &["new P1"])),
        (UnknownSaved,              FreshRefresh,        end(Saved,    Some(1),    false, true,  &["keys"])),
        (UnknownSaved,              RefreshesOutOfOrder, end(Saved,    Some(1),    false, true,  &["keys", "keys"])),
        (UnknownSaved,              StaleRefresh,        end(Saved,    Some(1),    false, true,  &["keys"])),
        // The list shows the post before any refresh, so it's saved, but there's no count yet.
        (UnknownSaved,              ListLoads,           end(Saved,    n,          false, true,  &["list"])),
        (UnknownSaved,              TimePasses,          end(Unknown,  n,          false, true,  &[])),

        (KnownUnsaved,              Save,                end(Saved,    Some(1),    true,  true,  &["new P1"])),
        (KnownUnsaved,              SaveRefused,         end(NotSaved, Some(0),    false, false, &["new P1"])),
        (KnownUnsaved,              Unsave,              end(NotSaved, Some(0),    false, false, &["delete P1"])),
        (KnownUnsaved,              UnsaveRefused,       end(NotSaved, Some(0),    false, false, &["delete P1"])),
        (KnownUnsaved,              SaveCancelled,       end(NotSaved, Some(0),    false, false, &["new P1"])),
        (KnownUnsaved,              FreshRefresh,        end(NotSaved, Some(0),    false, false, &["keys"])),
        (KnownUnsaved,              RefreshesOutOfOrder, end(NotSaved, Some(0),    false, false, &["keys", "keys"])),
        (KnownUnsaved,              StaleRefresh,        end(NotSaved, Some(0),    false, false, &["keys"])),
        (KnownUnsaved,              ListLoads,           end(NotSaved, Some(0),    false, false, &["list"])),
        (KnownUnsaved,              TimePasses,          end(NotSaved, Some(0),    false, false, &[])),

        (KnownSaved,                Save,                end(Saved,    Some(1),    true,  true,  &["new P1"])),
        (KnownSaved,                SaveRefused,         end(Saved,    Some(1),    false, true,  &["new P1"])),
        (KnownSaved,                Unsave,              end(NotSaved, Some(0),    false, false, &["delete P1"])),
        (KnownSaved,                UnsaveRefused,       end(Saved,    Some(1),    false, true,  &["delete P1"])),
        (KnownSaved,                SaveCancelled,       end(Saved,    Some(1),    false, true,  &["new P1"])),
        (KnownSaved,                FreshRefresh,        end(Saved,    Some(1),    false, true,  &["keys"])),
        (KnownSaved,                RefreshesOutOfOrder, end(Saved,    Some(1),    false, true,  &["keys", "keys"])),
        (KnownSaved,                StaleRefresh,        end(Saved,    Some(1),    false, true,  &["keys"])),
        (KnownSaved,                ListLoads,           end(Saved,    Some(1),    false, true,  &["list"])),
        (KnownSaved,                TimePasses,          end(Saved,    Some(1),    false, true,  &[])),

        (SavedButUnsavedElsewhere,  Save,                end(Saved,    Some(1),    true,  true,  &["new P1"])),
        (SavedButUnsavedElsewhere,  SaveRefused,         end(Saved,    Some(1),    false, false, &["new P1"])),
        (SavedButUnsavedElsewhere,  Unsave,              end(NotSaved, Some(0),    false, false, &["delete P1"])),
        (SavedButUnsavedElsewhere,  UnsaveRefused,       end(Saved,    Some(1),    false, false, &["delete P1"])),
        (SavedButUnsavedElsewhere,  SaveCancelled,       end(Saved,    Some(1),    false, false, &["new P1"])),
        (SavedButUnsavedElsewhere,  FreshRefresh,        end(NotSaved, Some(0),    false, false, &["keys"])),
        (SavedButUnsavedElsewhere,  RefreshesOutOfOrder, end(NotSaved, Some(0),    false, false, &["keys", "keys"])),
        (SavedButUnsavedElsewhere,  StaleRefresh,        end(Saved,    Some(1),    false, false, &["keys"])),
        // The list leaving a post out doesn't say it was unsaved: it may be on a later page.
        (SavedButUnsavedElsewhere,  ListLoads,           end(Saved,    Some(1),    false, false, &["list"])),
        (SavedButUnsavedElsewhere,  TimePasses,          end(Saved,    Some(1),    false, false, &[])),

        (UnsavedButSavedElsewhere,  Save,                end(Saved,    Some(1),    true,  true,  &["new P1"])),
        (UnsavedButSavedElsewhere,  SaveRefused,         end(NotSaved, Some(0),    false, true,  &["new P1"])),
        (UnsavedButSavedElsewhere,  Unsave,              end(NotSaved, Some(0),    false, false, &["delete P1"])),
        (UnsavedButSavedElsewhere,  UnsaveRefused,       end(NotSaved, Some(0),    false, true,  &["delete P1"])),
        (UnsavedButSavedElsewhere,  SaveCancelled,       end(NotSaved, Some(0),    false, true,  &["new P1"])),
        (UnsavedButSavedElsewhere,  FreshRefresh,        end(Saved,    Some(1),    false, true,  &["keys"])),
        (UnsavedButSavedElsewhere,  RefreshesOutOfOrder, end(Saved,    Some(1),    false, true,  &["keys", "keys"])),
        (UnsavedButSavedElsewhere,  StaleRefresh,        end(NotSaved, Some(0),    false, true,  &["keys"])),
        (UnsavedButSavedElsewhere,  ListLoads,           end(Saved,    Some(1),    false, true,  &["list"])),
        (UnsavedButSavedElsewhere,  TimePasses,          end(NotSaved, Some(0),    false, true,  &[])),

        // The save in flight is answered before the new request is sent.
        (Saving,                    Save,                end(Saved,    Some(1),    true,  true,  &["new P1"])),
        (Saving,                    SaveRefused,         end(Saved,    Some(1),    true,  true,  &["new P1"])),
        (Saving,                    Unsave,              end(NotSaved, Some(0),    false, false, &["delete P1"])),
        (Saving,                    UnsaveRefused,       end(Saved,    Some(1),    true,  true,  &["delete P1"])),
        // Cancelled while waiting its turn, so it's never sent; the save in flight still is.
        (Saving,                    SaveCancelled,       end(Saved,    Some(1),    true,  false, &[])),
        (Saving,                    FreshRefresh,        end(Saved,    Some(1),    true,  false, &["keys"])),
        (Saving,                    RefreshesOutOfOrder, end(Saved,    Some(1),    true,  false, &["keys", "keys"])),
        (Saving,                    StaleRefresh,        end(Saved,    Some(1),    true,  false, &["keys"])),
        (Saving,                    ListLoads,           end(Saved,    Some(1),    true,  false, &["list"])),
        (Saving,                    TimePasses,          end(Saved,    Some(1),    true,  false, &[])),

        (Unsaving,                  Save,                end(Saved,    Some(1),    true,  true,  &["new P1"])),
        (Unsaving,                  SaveRefused,         end(NotSaved, Some(0),    false, false, &["new P1"])),
        (Unsaving,                  Unsave,              end(NotSaved, Some(0),    false, false, &["delete P1"])),
        (Unsaving,                  UnsaveRefused,       end(NotSaved, Some(0),    false, false, &["delete P1"])),
        (Unsaving,                  SaveCancelled,       end(NotSaved, Some(0),    false, true,  &[])),
        (Unsaving,                  FreshRefresh,        end(NotSaved, Some(0),    false, true,  &["keys"])),
        (Unsaving,                  RefreshesOutOfOrder, end(NotSaved, Some(0),    false, true,  &["keys", "keys"])),
        (Unsaving,                  StaleRefresh,        end(NotSaved, Some(0),    false, true,  &["keys"])),
        // The server still has the post, but this device is unsaving it, so the list hides it.
        (Unsaving,                  ListLoads,           end(NotSaved, Some(0),    false, true,  &["list"])),
        (Unsaving,                  TimePasses,          end(NotSaved, Some(0),    false, true,  &[])),

        (JustSaved,                 Save,                end(Saved,    Some(1),    true,  true,  &["new P1"])),
        (JustSaved,                 SaveRefused,         end(Saved,    Some(1),    true,  true,  &["new P1"])),
        (JustSaved,                 Unsave,              end(NotSaved, Some(0),    false, false, &["delete P1"])),
        (JustSaved,                 UnsaveRefused,       end(Saved,    Some(1),    true,  true,  &["delete P1"])),
        (JustSaved,                 SaveCancelled,       end(Saved,    Some(1),    true,  true,  &["new P1"])),
        // A read that started after the window includes the save, so it no longer needs holding.
        (JustSaved,                 FreshRefresh,        end(Saved,    Some(1),    false, true,  &["keys"])),
        (JustSaved,                 RefreshesOutOfOrder, end(Saved,    Some(1),    false, true,  &["keys", "keys"])),
        // A read from before the save can't undo it.
        (JustSaved,                 StaleRefresh,        end(Saved,    Some(1),    true,  true,  &["keys"])),
        (JustSaved,                 ListLoads,           end(Saved,    Some(1),    true,  true,  &["list"])),
        // Nothing has read the save yet, so it holds however long it's been.
        (JustSaved,                 TimePasses,          end(Saved,    Some(1),    true,  true,  &[])),

        (JustUnsaved,               Save,                end(Saved,    Some(1),    true,  true,  &["new P1"])),
        (JustUnsaved,               SaveRefused,         end(NotSaved, Some(0),    false, false, &["new P1"])),
        (JustUnsaved,               Unsave,              end(NotSaved, Some(0),    false, false, &["delete P1"])),
        (JustUnsaved,               UnsaveRefused,       end(NotSaved, Some(0),    false, false, &["delete P1"])),
        (JustUnsaved,               SaveCancelled,       end(NotSaved, Some(0),    false, false, &["new P1"])),
        (JustUnsaved,               FreshRefresh,        end(NotSaved, Some(0),    false, false, &["keys"])),
        (JustUnsaved,               RefreshesOutOfOrder, end(NotSaved, Some(0),    false, false, &["keys", "keys"])),
        (JustUnsaved,               StaleRefresh,        end(NotSaved, Some(0),    false, false, &["keys"])),
        (JustUnsaved,               ListLoads,           end(NotSaved, Some(0),    false, false, &["list"])),
        (JustUnsaved,               TimePasses,          end(NotSaved, Some(0),    false, false, &[])),

        // Can't know: the server still has 200 saves.
        (PastTheKeyLimit,           Save,                end(Saved,    Some(201),  true,  true,  &["new P1"])),
        (PastTheKeyLimit,           SaveRefused,         end(Unknown,  Some(200),  false, true,  &["new P1"])),
        // Can't know: the server has 199 saves.
        (PastTheKeyLimit,           Unsave,              end(NotSaved, Some(200),  false, false, &["delete P1"])),
        (PastTheKeyLimit,           UnsaveRefused,       end(Unknown,  Some(200),  false, true,  &["delete P1"])),
        (PastTheKeyLimit,           SaveCancelled,       end(Unknown,  Some(200),  false, true,  &["new P1"])),
        (PastTheKeyLimit,           FreshRefresh,        end(Unknown,  Some(200),  false, true,  &["keys"])),
        (PastTheKeyLimit,           RefreshesOutOfOrder, end(Unknown,  Some(200),  false, true,  &["keys", "keys"])),
        (PastTheKeyLimit,           StaleRefresh,        end(Unknown,  Some(200),  false, true,  &["keys"])),
        (PastTheKeyLimit,           ListLoads,           end(Saved,    Some(200),  false, true,  &["list"])),
        (PastTheKeyLimit,           TimePasses,          end(Unknown,  Some(200),  false, true,  &[])),
    ]
}

#[test]
fn every_start_and_action() {
    let rows = rows();
    // Every pair is in the table, once.
    for start_ in [
        Start::UnknownUnsaved,
        Start::UnknownSaved,
        Start::KnownUnsaved,
        Start::KnownSaved,
        Start::SavedButUnsavedElsewhere,
        Start::UnsavedButSavedElsewhere,
        Start::Saving,
        Start::Unsaving,
        Start::JustSaved,
        Start::JustUnsaved,
        Start::PastTheKeyLimit,
    ] {
        for action in [
            Action::Save,
            Action::SaveRefused,
            Action::Unsave,
            Action::UnsaveRefused,
            Action::SaveCancelled,
            Action::FreshRefresh,
            Action::StaleRefresh,
            Action::RefreshesOutOfOrder,
            Action::ListLoads,
            Action::TimePasses,
        ] {
            let count = rows
                .iter()
                .filter(|(s, a, _)| *s == start_ && *a == action)
                .count();
            assert_eq!(
                count, 1,
                "{start_:?} + {action:?} is in the table {count} times"
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
            count: world.service.saved_count(),
            in_recent_saves: world.recent().contains(&"P1".to_string()),
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

// A post's writes go out in the order they were tapped.

#[test]
fn taps_reach_the_server_in_order_whatever_order_they_are_answered() {
    let mut world = World::new();
    world.refresh();
    world.save(identity(1));
    world.unsave(identity(1));
    world.save(identity(1));
    assert_eq!(
        world.sent_since(1),
        ["new P1"],
        "the others wait their turn"
    );

    world.answer_writes();

    assert_eq!(world.sent_since(1), ["new P1", "delete P1", "new P1"]);
    assert!(world.server.has("P1"));
    assert_eq!(world.status(identity(1)), Saved);
}

#[test]
fn a_tap_cancelled_while_waiting_its_turn_is_never_sent() {
    let mut world = World::new();
    world.refresh();
    world.save(identity(1));
    let unsave = world.unsave(identity(1));
    world.save(identity(1));
    world.cancel(unsave);

    world.answer_writes();

    assert_eq!(world.sent_since(1), ["new P1", "new P1"]);
    assert!(world.server.has("P1"));
    assert_eq!(world.status(identity(1)), Saved);
}

// Jetpack posts: post 3, also known by its feed item 4. The API stores a save by the feed item as
// the post, but older saves can still be under the feed item.

#[test]
fn saving_a_jetpack_post_sends_its_post_key() {
    let mut world = World::new();
    world.refresh();
    world.save(jetpack());
    world.answer_writes();

    assert_eq!(world.sent_since(1), ["new P3"]);
    assert_eq!(world.server.saved, ["P3".to_string()].into());
    assert_eq!(world.status(jetpack()), Saved);
}

#[test]
fn a_jetpack_post_saved_under_its_feed_item_reads_saved_and_unsaves_both_keys() {
    let mut world = World::new();
    world.server.saved.insert("F4".to_string());
    world.refresh();
    assert_eq!(world.status(jetpack()), Saved);
    assert_eq!(world.status(jetpack_external()), Saved);

    world.unsave(jetpack());
    world.answer_writes();

    assert_eq!(world.sent_since(1), ["delete P3", "delete F4"]);
    assert!(world.server.saved.is_empty());
    assert_eq!(world.status(jetpack()), NotSaved);
    assert_eq!(world.service.saved_count(), Some(0));
}

#[test]
fn before_a_refresh_unsaving_a_jetpack_post_sends_both_keys() {
    let mut world = World::new();
    world.server.saved.insert("F4".to_string());

    world.unsave(jetpack());
    world.answer_writes();

    assert_eq!(world.sent_since(0), ["delete P3", "delete F4"]);
    assert!(world.server.saved.is_empty());
    assert_eq!(world.status(jetpack()), NotSaved);
}

#[test]
fn saving_the_external_view_of_a_jetpack_post_saves_the_post() {
    let mut world = World::new();
    world.refresh();
    world.save(jetpack_external());
    world.answer_writes();

    assert_eq!(world.sent_since(1), ["new F4"]);
    assert_eq!(world.server.saved, ["P3".to_string()].into());
    assert_eq!(world.status(jetpack_external()), Saved);
    assert_eq!(world.status(jetpack()), Saved);
    assert_eq!(world.recent(), ["P3"]);
}

/// The keys don't say which feed item a post save came from, so the external view of a post saved
/// elsewhere reads as not saved until a write's answer says so. Unsaving it still works.
#[test]
fn the_external_view_of_a_jetpack_post_saved_elsewhere_reads_not_saved_but_unsaves() {
    let mut world = World::new();
    world.server.saved.insert("P3".to_string());
    world.refresh();
    assert_eq!(
        world.status(jetpack_external()),
        NotSaved,
        "the API doesn't say"
    );

    world.unsave(jetpack_external());
    world.answer_writes();

    assert_eq!(world.sent_since(1), ["delete F4"]);
    assert!(world.server.saved.is_empty());
    assert_eq!(world.status(jetpack()), NotSaved);
}

// A URL of a WordPress.com post: the API stores a save by the URL as the post.

#[test]
fn saving_a_url_of_a_post_saves_the_post() {
    let mut world = World::new();
    world.refresh();
    world.save_url(U9);
    world.answer_writes();

    assert_eq!(world.sent_since(1), ["new U9"]);
    assert_eq!(world.server.saved, ["P9".to_string()].into());
    assert_eq!(world.url_status(U9), Saved);
    assert_eq!(world.status(identity(9)), Saved);
    assert_eq!(world.recent(), ["P9"]);
}

#[test]
fn a_refresh_says_which_url_a_post_was_saved_by() {
    let mut world = World::new();
    world.server.saved.insert("P9".to_string());
    world.refresh();

    assert_eq!(
        world.url_status("https://example.com/p9/?utm_source=x"),
        Saved
    );
}

#[test]
fn unsaving_a_url_sends_the_url_and_the_post_it_is_stored_as() {
    let mut world = World::new();
    world.server.saved.insert("P9".to_string());
    world.refresh();

    world.unsave_key("U9");
    world.answer_writes();

    assert_eq!(world.sent_since(1), ["delete U9", "delete P9"]);
    assert!(world.server.saved.is_empty());
    assert_eq!(world.url_status(U9), NotSaved);
    assert_eq!(world.status(identity(9)), NotSaved);
}

/// With nothing saved, deleting a URL echoes the URL. That doesn't mean the URL is no longer the
/// post's.
#[test]
fn unsaving_a_url_that_is_not_saved_keeps_what_it_is_stored_as() {
    let mut world = World::new();
    world.refresh();
    world.save_url(U9);
    world.answer_writes();
    world.unsave_key("U9");
    world.answer_writes();
    world.unsave_key("U9");
    world.answer_writes();

    // Saved again elsewhere: the URL still reads as the post.
    world.server.saved.insert("P9".to_string());
    world.advance(MAX_READ_LAG + Duration::from_secs(1));
    world.refresh();
    assert_eq!(world.url_status(U9), Saved);
}

// RSS items, saved under their feed item.

#[test]
fn an_rss_item_saves_and_unsaves_under_its_feed_item() {
    let mut world = World::new();
    world.refresh();
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
    world.refresh();
    world.save_url(U10);
    world.answer_writes();

    assert_eq!(world.server.saved, ["F10".to_string()].into());
    assert_eq!(world.status(rss_identity(10)), Saved);
    assert_eq!(world.url_status(U10), Saved);
}

/// The API answers `post_not_found` only for a feed item that doesn't exist, and nothing can be
/// saved under one.
#[test]
fn unsaving_a_feed_item_that_does_not_exist_counts_as_unsaved() {
    let mut world = World::new();
    world.refresh();

    world.unsave(rss_identity(99));
    world.answer_writes();

    assert_eq!(world.sent_since(1), ["delete F99"]);
    assert_eq!(world.status(rss_identity(99)), NotSaved);
}

// Sequences the rows above can't show.

/// A write to a URL waits for one queued before the device learned which post the URL is stored
/// as, though one now locks the URL and the other the post.
#[test]
fn a_tap_after_a_url_is_known_to_be_a_post_waits_for_one_queued_before() {
    let mut world = World::new();
    world.refresh();
    world.save_url(U9);
    world.unsave_key("U9");
    // The save is answered: the URL is post 9. The unsave goes out, and is held.
    world.answer_next_write(false);
    assert_eq!(world.sent_since(1), ["new U9", "delete U9"]);

    world.save_url(U9);
    assert_eq!(
        world.sent_since(1),
        ["new U9", "delete U9"],
        "the save waits for the unsave"
    );

    world.answer_writes();
    assert_eq!(
        world.sent_since(1),
        ["new U9", "delete U9", "delete P9", "new U9"]
    );
    assert!(world.server.has("P9"));
    assert_eq!(world.url_status(U9), Saved);
}

/// An unsave that learns, once it's sent, that the post is stored under another key also unsaves
/// that key. A save tapped after it still decides.
#[test]
fn a_save_tapped_after_an_unsave_still_decides_when_the_unsave_learns_another_key() {
    let mut world = World::new();
    world.refresh();
    world.save(jetpack_external());
    world.unsave(jetpack_external());
    world.save(jetpack_external());

    // The first save is answered: feed item 4 is post 3. The unsave goes out.
    world.answer_next_write(false);
    assert_eq!(
        world.status(jetpack_external()),
        Saved,
        "the last tap was save"
    );

    world.answer_writes();
    assert_eq!(
        world.sent_since(1),
        ["new F4", "delete F4", "delete P3", "new F4"]
    );
    assert!(world.server.has("P3"));
    assert_eq!(world.status(jetpack_external()), Saved);
}

/// Deleting a URL that isn't saved echoes the URL, which doesn't mean the URL stopped being the
/// post's: the next unsave still sends both.
#[test]
fn an_unsave_of_a_url_that_is_not_saved_still_knows_the_post_next_time() {
    let mut world = World::new();
    world.refresh();
    world.save_url(U9);
    world.answer_writes();
    world.unsave_key("U9");
    world.answer_writes();
    world.unsave_key("U9");
    world.answer_writes();

    world.unsave_key("U9");
    world.answer_writes();

    let sent = world.sent_since(1);
    assert_eq!(sent[sent.len() - 2..], ["delete U9", "delete P9"]);
}

/// Two list reads land in the opposite order to the one they started in, around a refresh that
/// started between them and predates the save. The newer list read still stands.
#[test]
fn a_slow_list_read_does_not_replace_a_newer_one() {
    let mut world = World::new();
    let list = |service: Arc<ReaderSavedService>| async move {
        service.list_posts(ReaderSavedPostsParams::default()).await
    };
    world.call(list);
    world.advance(Duration::from_secs(5));
    world.call(|service| async move { service.refresh().await });
    world.advance(Duration::from_secs(5));
    world.call(list);
    world.server.saved.insert("P1".to_string());

    let answer = |world: &mut World, what: &str, newest: bool, body: Value| {
        let held = {
            let mut held = world.executor.held.lock().unwrap();
            let matching = |held: &Held| held.what == what;
            let index = if newest {
                held.iter().rposition(matching)
            } else {
                held.iter().position(matching)
            };
            held.remove(index.unwrap())
        };
        let _ = held.reply.send(respond(200, body));
        world.pool.run_until_stalled();
    };
    let listed = world.server.list_response();
    answer(&mut world, "list", true, listed.clone());
    answer(&mut world, "list", false, listed);
    answer(
        &mut world,
        "keys",
        false,
        FakeServer::default().keys_response(),
    );

    assert_eq!(world.status(identity(1)), Saved);
}

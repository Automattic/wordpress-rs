//! The service against a fake saved posts API, through random taps, response orders, failures,
//! cancellations and stale reads.
//!
//! The fake knows nothing about how WordPress.com works inside. It's a set of saved keys with a
//! history, and it follows only the three facts `state.rs` relies on: an answered write was
//! applied, a read includes every write applied [`MAX_READ_LAG`] before it started, and requests
//! are applied in the order the service sends them. Which older state a read returns, and how each
//! request ends, are picked at random.
//!
//! There's no second implementation to compare against. The expected answer is always what the
//! user last tapped, or what the fake holds:
//!
//! 1. **Taps show.** The post tapped last shows its tap while the call is in flight, and after it
//!    succeeds until a read that includes it has been applied.
//! 2. **The server ends up as tapped.** Once a post's calls have all finished and the last one
//!    succeeded, the fake holds what it asked for.
//! 3. **It settles.** Once everything has finished, a fresh read makes the service show exactly
//!    what the fake holds.
//! 4. **Reads show.** A post this device hasn't touched, which other devices may change, shows what
//!    the newest read said: the keys read that started last, or a list read that started after it.

use super::*;
use crate::service::reader_saved::state::MAX_READ_LAG;
use futures::executor::LocalPool;
use futures::future::RemoteHandle;
use futures::task::LocalSpawnExt;
use rand::{RngExt, SeedableRng, rngs::StdRng};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::time::{Duration, Instant};
use wp_api::request::endpoint::WpEndpointUrl;

/// How the fake stores a key that differs from the one sent: an RSS item of a WordPress.com post is
/// stored as the post, and so is the post's URL.
fn stored_as(key: &ReaderSavedKey) -> ReaderSavedKey {
    match key.normalized() {
        ReaderSavedKey::FeedItem { feed_item_id } if feed_item_id.0 == 7 => post_key(8),
        ReaderSavedKey::Url { url } if url == "https://example.com/p9" => post_key(9),
        key => key,
    }
}

const P9_URL: &str = "https://example.com/p9";

#[derive(Clone, Copy, Debug)]
enum Object {
    /// A WordPress.com post.
    Post1,
    /// A Jetpack post, which may be saved under its blog and post IDs or its feed item.
    Jetpack,
    /// The same Jetpack post, shown as external while its WordPress.com copy fails to load.
    JetpackExternal,
    /// An RSS item the server stores as post 8, and post 8 itself.
    Rss7,
    Post8,
    /// A page saved by URL that the server stores as post 9, and post 9 itself.
    Url9,
    Post9,
    /// An RSS post.
    Rss10,
}

const OBJECTS: [Object; 8] = [
    Object::Post1,
    Object::Jetpack,
    Object::JetpackExternal,
    Object::Rss7,
    Object::Post8,
    Object::Url9,
    Object::Post9,
    Object::Rss10,
];

impl Object {
    fn identity(self) -> ReaderPostSaveIdentity {
        match self {
            Object::Post1 => identity(1),
            Object::Jetpack => jetpack_identity(3, 4),
            Object::JetpackExternal => rss_identity(4),
            Object::Rss7 => rss_identity(7),
            Object::Post8 => identity(8),
            Object::Url9 => ReaderPostSaveIdentity {
                url: Some(P9_URL.to_string()),
                ..unsaveable_identity()
            },
            Object::Post9 => identity(9),
            Object::Rss10 => rss_identity(10),
        }
    }

    /// The keys a status check looks at. A page saved by URL has no identity with IDs.
    fn keys(self) -> Vec<ReaderSavedKey> {
        match self {
            Object::Url9 => vec![ReaderSavedKey::Url {
                url: P9_URL.to_string(),
            }],
            object => reader_saved_keys_for_post(object.identity()),
        }
    }

    /// Posts the server holds as one.
    fn group(self) -> u8 {
        match self {
            Object::Post1 => 0,
            Object::Jetpack | Object::JetpackExternal => 1,
            Object::Rss7 | Object::Post8 => 2,
            Object::Url9 | Object::Post9 => 3,
            Object::Rss10 => 4,
        }
    }

    fn status(self, service: &ReaderSavedService) -> ReaderSavedStatus {
        match self {
            Object::Url9 => service.state().status(&self.keys()),
            object => service.status(object.identity()),
        }
    }
}

#[derive(Default)]
struct Fake {
    /// Every state the fake has been in, and when it started.
    history: Vec<(Instant, BTreeSet<ReaderSavedKeyOrd>)>,
    /// Whether deleting a key that isn't saved is an error, as the API may answer.
    delete_missing_is_error: bool,
}

/// `ReaderSavedKey` has no ordering, so the fake keeps its JSON.
type ReaderSavedKeyOrd = String;

fn ord(key: &ReaderSavedKey) -> ReaderSavedKeyOrd {
    serde_json::to_string(&key.normalized()).unwrap()
}

fn unord(key: &str) -> ReaderSavedKey {
    serde_json::from_str(key).unwrap()
}

impl Fake {
    fn now(&self) -> &BTreeSet<ReaderSavedKeyOrd> {
        &self.history.last().unwrap().1
    }

    fn apply(&mut self, at: Instant, key: &ReaderSavedKey, saved: bool) -> bool {
        let mut next = self.now().clone();
        let key = ord(&stored_as(key));
        let changed = if saved {
            next.insert(key)
        } else {
            next.remove(&key)
        };
        self.history.push((at, next));
        changed
    }

    /// A state a read that started at `started` may return: one that includes every write applied
    /// `MAX_READ_LAG` before it.
    fn read(&self, started: Instant, rng: &mut StdRng) -> BTreeSet<ReaderSavedKeyOrd> {
        let oldest = self
            .history
            .iter()
            .rposition(|(at, _)| *at + MAX_READ_LAG <= started)
            .unwrap_or(0);
        let newest = self.history.len() - 1;
        self.history[rng.random_range(oldest..=newest)].1.clone()
    }

    /// The keys an object may be saved under, as the fake stores them.
    fn has(&self, object: Object) -> bool {
        object
            .keys()
            .iter()
            .any(|key| self.now().contains(&ord(&stored_as(key))))
    }
}

struct Request {
    url: String,
    body: Option<Value>,
    started: Instant,
    reply: Option<oneshot::Sender<Result<WpNetworkResponse, RequestExecutionError>>>,
}

#[derive(Debug)]
struct FakeExecutor {
    requests: Mutex<Vec<RequestBox>>,
    clock: Arc<Mutex<Instant>>,
}

struct RequestBox(Request);

impl std::fmt::Debug for RequestBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0.url)
    }
}

#[async_trait]
impl RequestExecutor for FakeExecutor {
    async fn execute(
        &self,
        request: Arc<WpNetworkRequest>,
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        let (reply, response) = oneshot::channel();
        self.requests.lock().unwrap().push(RequestBox(Request {
            url: request.url().0,
            body: request
                .body_as_string()
                .map(|body| serde_json::from_str(&body).unwrap()),
            started: *self.clock.lock().unwrap(),
            reply: Some(reply),
        }));
        let mut response = response.await.expect("the fake dropped a request");
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

fn ok(body: Value) -> Result<WpNetworkResponse, RequestExecutionError> {
    respond(200, body)
}

fn respond(status_code: u32, body: Value) -> Result<WpNetworkResponse, RequestExecutionError> {
    Ok(WpNetworkResponse {
        body: body.to_string().into_bytes(),
        status_code,
        response_header_map: Default::default(),
        request_url: WpEndpointUrl(String::new()),
        request_method: RequestMethod::GET,
        request_header_map: Default::default(),
    })
}

fn offline() -> Result<WpNetworkResponse, RequestExecutionError> {
    Err(RequestExecutionError::RequestExecutionFailed {
        status_code: None,
        redirects: None,
        reason: RequestExecutionErrorReason::HttpTimeoutError,
        request_url: String::new(),
        request_method: RequestMethod::POST,
    })
}

fn listed(key: &ReaderSavedKey) -> Option<Value> {
    match key {
        ReaderSavedKey::Post { post_id, .. } if post_id.0 == 3 => Some(
            json!({ "ID": 3, "site_ID": 123, "feed_ID": 5, "feed_item_ID": 4, "is_external": false }),
        ),
        ReaderSavedKey::Post { post_id, .. } => Some(listed_post(post_id.0 as u64)),
        ReaderSavedKey::FeedItem { feed_item_id } if feed_item_id.0 == 4 => Some(
            json!({ "ID": 3, "site_ID": 123, "feed_ID": 5, "feed_item_ID": 4, "is_external": false }),
        ),
        ReaderSavedKey::FeedItem { feed_item_id } => Some(listed_rss_post(feed_item_id.0)),
        ReaderSavedKey::Url { .. } => None,
    }
}

/// The keys of the post the saved list shows for a saved key.
fn listed_keys(key: &ReaderSavedKey) -> Vec<ReaderSavedKey> {
    match listed(key) {
        Some(post) => reader_saved_keys_for_post(serde_json::from_value(post).unwrap()),
        None => vec![],
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Outcome {
    Ok,
    Failed,
    Cancelled,
}

struct Tap {
    object: Object,
    saved: bool,
    call: usize,
    /// When it was made, in failed write requests so far.
    failures_before: usize,
}

struct Call {
    handle: Option<RemoteHandle<()>>,
    outcome: Rc<RefCell<Option<(Outcome, Instant)>>>,
    /// For a read, when it started.
    read_started: Option<Instant>,
}

struct Sim {
    pool: LocalPool,
    service: Arc<ReaderSavedService>,
    executor: Arc<FakeExecutor>,
    clock: Arc<Mutex<Instant>>,
    fake: Fake,
    rng: StdRng,
    calls: Vec<Call>,
    taps: Vec<Tap>,
    /// The keys of write requests that failed, in order, as the fake stores them.
    failed_writes: Vec<ReaderSavedKeyOrd>,
    /// Reads answered, with when they started and whether they were of the list.
    reads: Vec<(Instant, bool, BTreeSet<ReaderSavedKeyOrd>)>,
    /// The read `serve` just answered.
    last_read: Option<(Instant, bool, BTreeSet<ReaderSavedKeyOrd>)>,
    log: Vec<String>,
}

impl Sim {
    fn new(seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let start = Instant::now();
        let clock = Arc::new(Mutex::new(start));
        let executor = Arc::new(FakeExecutor {
            requests: Mutex::default(),
            clock: clock.clone(),
        });
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

        // Saves made elsewhere before the session: some of the Jetpack post's are under its feed
        // item, which this device's identity for it doesn't send first.
        let mut initial = BTreeSet::new();
        for key in [feed_key(4), post_key(1), post_key(8), feed_key(10)] {
            if rng.random_bool(0.5) {
                initial.insert(ord(&key));
            }
        }
        let fake = Fake {
            history: vec![(start, initial)],
            delete_missing_is_error: rng.random_bool(0.5),
        };
        Self {
            pool: LocalPool::new(),
            service,
            executor,
            clock,
            fake,
            rng,
            calls: Vec::new(),
            taps: Vec::new(),
            failed_writes: Vec::new(),
            reads: Vec::new(),
            last_read: None,
            log: Vec::new(),
        }
    }

    fn now(&self) -> Instant {
        *self.clock.lock().unwrap()
    }

    fn t(&self) -> f64 {
        (self.now() - self.fake.history[0].0).as_secs_f64()
    }

    fn say(&mut self, line: String) {
        let t = self.t();
        self.log.push(format!("t={t:>8.3}s  {line}"));
    }

    fn spawn<F>(&mut self, future: F, read: bool) -> usize
    where
        F: std::future::Future<Output = Outcome> + 'static,
    {
        let outcome = Rc::new(RefCell::new(None));
        let set = outcome.clone();
        let clock = self.clock.clone();
        let task = async move {
            let result = future.await;
            *set.borrow_mut() = Some((result, *clock.lock().unwrap()));
        };
        let handle = self.pool.spawner().spawn_local_with_handle(task).unwrap();
        self.calls.push(Call {
            handle: Some(handle),
            outcome,
            read_started: read.then(|| self.now()),
        });
        self.pool.run_until_stalled();
        self.calls.len() - 1
    }

    fn tap(&mut self, object: Object, saved: bool) {
        let service = self.service.clone();
        let verb = if saved { "save" } else { "unsave" };
        let call = self.calls.len();
        self.say(format!("call {call}: {verb} {object:?}"));
        let outcome = |ok: bool| if ok { Outcome::Ok } else { Outcome::Failed };
        let call = match (object, saved) {
            (Object::Url9, true) => self.spawn(
                async move { outcome(service.save_url(P9_URL.to_string(), None).await.is_ok()) },
                false,
            ),
            (Object::Url9, false) => self.spawn(
                async move {
                    let key = ReaderSavedKey::Url {
                        url: P9_URL.to_string(),
                    };
                    outcome(service.unsave_key(key).await.is_ok())
                },
                false,
            ),
            (object, true) => self.spawn(
                async move { outcome(service.save(object.identity()).await.is_ok()) },
                false,
            ),
            (object, false) => self.spawn(
                async move { outcome(service.unsave(object.identity()).await.is_ok()) },
                false,
            ),
        };
        self.taps.push(Tap {
            object,
            saved,
            call,
            failures_before: self.failed_writes.len(),
        });
    }

    fn pending_calls(&self) -> Vec<usize> {
        (0..self.calls.len())
            .filter(|call| {
                self.calls[*call].outcome.borrow().is_none() && self.calls[*call].handle.is_some()
            })
            .collect()
    }

    /// The service can't send an unsave after a save it doesn't yet know is stored under another
    /// key: until the save is answered, nothing says they're one post. So the generator doesn't
    /// tap one object while another of its post's objects with no key in common has a call open.
    fn would_race_an_unknown_alias(&self, object: Object) -> bool {
        let keys: Vec<_> = object
            .keys()
            .iter()
            .map(ReaderSavedKey::normalized)
            .collect();
        self.taps.iter().any(|tap| {
            tap.object.group() == object.group()
                && self.calls[tap.call].outcome.borrow().is_none()
                && self.calls[tap.call].handle.is_some()
                && !tap
                    .object
                    .keys()
                    .iter()
                    .any(|key| keys.contains(&key.normalized()))
        })
    }

    fn step(&mut self) {
        match self.rng.random_range(0..100) {
            0..20 => {
                let secs = match self.rng.random_range(0..4) {
                    0 => self.rng.random_range(0..5),
                    1 => self.rng.random_range(5..60),
                    2 => MAX_READ_LAG.as_secs() + self.rng.random_range(0..3) - 1,
                    _ => self.rng.random_range(100..400),
                };
                *self.clock.lock().unwrap() += Duration::from_secs(secs);
            }
            20..24 => {
                // Another device changes a post this one hasn't touched.
                let untouched: Vec<_> = OBJECTS
                    .into_iter()
                    .filter(|object| {
                        !self
                            .taps
                            .iter()
                            .any(|tap| tap.object.group() == object.group())
                    })
                    .collect();
                if !untouched.is_empty() {
                    let object = untouched[self.rng.random_range(0..untouched.len())];
                    let keys = object.keys();
                    let key = &keys[self.rng.random_range(0..keys.len())];
                    let saved = !self.fake.now().contains(&ord(&stored_as(key)));
                    self.say(format!(
                        "elsewhere: {} {key:?}",
                        if saved { "save" } else { "unsave" }
                    ));
                    let now = self.now();
                    self.fake.apply(now, key, saved);
                }
            }
            24..45 => {
                let object = OBJECTS[self.rng.random_range(0..OBJECTS.len())];
                if !self.would_race_an_unknown_alias(object) {
                    let saved = self.rng.random_bool(0.5);
                    self.tap(object, saved);
                }
            }
            45..75 => self.answer_a_request(),
            75..80 => {
                let pending = self.pending_calls();
                if !pending.is_empty() {
                    let call = pending[self.rng.random_range(0..pending.len())];
                    self.say(format!("call {call} cancelled"));
                    self.calls[call].handle = None;
                    *self.calls[call].outcome.borrow_mut() = Some((Outcome::Cancelled, self.now()));
                    self.pool.run_until_stalled();
                    self.settle_dropped_requests();
                }
            }
            80..90 => {
                let service = self.service.clone();
                let call = self.calls.len();
                self.say(format!("call {call}: refresh"));
                self.spawn(
                    async move {
                        if service.refresh().await.is_ok() {
                            Outcome::Ok
                        } else {
                            Outcome::Failed
                        }
                    },
                    true,
                );
            }
            _ => {
                let service = self.service.clone();
                let call = self.calls.len();
                self.say(format!("call {call}: list_posts"));
                self.spawn(
                    async move {
                        match service.list_posts(ReaderSavedPostsParams::default()).await {
                            Ok(_) => Outcome::Ok,
                            Err(_) => Outcome::Failed,
                        }
                    },
                    true,
                );
            }
        }
        self.pool.run_until_stalled();
    }

    /// A request whose call was cancelled may or may not have reached the server.
    fn settle_dropped_requests(&mut self) {
        let dropped: Vec<Request> = {
            let mut requests = self.executor.requests.lock().unwrap();
            let (dropped, kept): (Vec<_>, Vec<_>) = requests
                .drain(..)
                .partition(|request| request.0.reply.as_ref().unwrap().is_canceled());
            *requests = kept;
            dropped.into_iter().map(|request| request.0).collect()
        };
        for request in dropped {
            if self.rng.random_bool(0.5) {
                self.say(format!(
                    "  dropped request reached the server: {}",
                    request.url
                ));
                // Applied, but nobody hears the answer.
                let _ = self.serve(&request);
            }
        }
    }

    fn answer_a_request(&mut self) {
        let request = {
            let mut requests = self.executor.requests.lock().unwrap();
            if requests.is_empty() {
                return;
            }
            let index = self.rng.random_range(0..requests.len());
            requests.remove(index).0
        };
        let roll = self.rng.random_range(0..10);
        if roll < 3
            && let Some(body) = &request.body
            && let Ok(key) = serde_json::from_value::<ReaderSavedKey>(body.clone())
        {
            self.failed_writes.push(ord(&stored_as(&key)));
        }
        let response = match roll {
            0 => {
                self.say(format!("  refused: {} {:?}", request.url, request.body));
                respond(
                    500,
                    json!({ "code": "internal", "message": "", "data": { "status": 500 } }),
                )
            }
            1 => {
                self.say(format!("  offline: {} {:?}", request.url, request.body));
                offline()
            }
            2 => {
                self.say(format!(
                    "  applied, response lost: {} {:?}",
                    request.url, request.body
                ));
                // Applied, but nobody hears the answer.
                let _ = self.serve(&request);
                offline()
            }
            _ => {
                let response = self.serve(&request);
                self.reads.extend(self.last_read.take());
                response
            }
        };
        self.last_read = None;
        let mut request = request;
        let _ = request.reply.take().unwrap().send(response);
    }

    fn total(&self) -> usize {
        self.fake.now().len()
    }

    /// Applies a request to the fake, and returns its answer.
    fn serve(&mut self, request: &Request) -> Result<WpNetworkResponse, RequestExecutionError> {
        self.last_read = None;
        let now = self.now();
        let url = request.url.as_str();
        if url.starts_with(KEYS_URL) {
            let saved = self.fake.read(request.started, &mut self.rng);
            let keys: Vec<_> = saved
                .iter()
                .enumerate()
                .map(|(index, key)| {
                    let mut entry = serde_json::from_str::<Value>(key).unwrap();
                    entry["id"] = json!(index + 1);
                    entry["date_saved"] = json!("2026-11-02T10:15:00+00:00");
                    if unord(key) == post_key(9) {
                        entry["url"] = json!(P9_URL);
                    }
                    entry
                })
                .collect();
            self.say(format!("  keys read: {saved:?}"));
            self.last_read = Some((request.started, false, saved.clone()));
            return ok(json!({ "keys": keys, "total": saved.len() }));
        }
        if url.starts_with(LIST_URL) {
            let saved = self.fake.read(request.started, &mut self.rng);
            self.say(format!("  list read: {saved:?}"));
            self.last_read = Some((request.started, true, saved.clone()));
            let mut posts: Vec<Value> = Vec::new();
            for key in &saved {
                if let Some(post) = listed(&unord(key))
                    && !posts.contains(&post)
                {
                    posts.push(post);
                }
            }
            return ok(json!({ "posts": posts }));
        }
        let body = request.body.clone().unwrap();
        if url.starts_with(BULK_URL) {
            let mut results = Vec::new();
            for key in body["keys"].as_array().unwrap() {
                let key: ReaderSavedKey = serde_json::from_value(key.clone()).unwrap();
                self.fake.apply(now, &key, true);
                results.push(json!({ "key": stored_as(&key), "saved": true }));
            }
            self.say(format!("  bulk saved: {:?}", body["keys"]));
            return ok(json!({ "results": results, "total": self.total() }));
        }
        let key: ReaderSavedKey = serde_json::from_value(body.clone()).unwrap();
        let stored = stored_as(&key);
        if url.starts_with(NEW_URL) {
            self.fake.apply(now, &key, true);
            self.say(format!("  saved {key:?} as {stored:?}"));
            ok(write_response(
                true,
                serde_json::to_value(&stored).unwrap(),
                self.total() as u64,
            ))
        } else {
            assert!(url.starts_with(DELETE_URL), "{url}");
            let removed = self.fake.apply(now, &key, false);
            self.say(format!("  unsaved {key:?} (was saved: {removed})"));
            if !removed && self.fake.delete_missing_is_error {
                // An error, if the service last read the key as saved.
                self.failed_writes.push(ord(&stored));
                return respond(404, error_response());
            }
            ok(json!({
                "saved": false,
                "key": serde_json::to_value(&stored).unwrap(),
                "total": self.total(),
            }))
        }
    }

    fn check(&self) -> Result<(), String> {
        let mut last_in_group: BTreeMap<u8, &Tap> = BTreeMap::new();
        for tap in &self.taps {
            last_in_group.insert(tap.object.group(), tap);
        }
        for tap in last_in_group.values() {
            let outcome = *self.calls[tap.call].outcome.borrow();
            let want = if tap.saved {
                ReaderSavedStatus::Saved
            } else {
                ReaderSavedStatus::NotSaved
            };

            // 1. Taps show, unless one of the tap's requests has failed: then the call will
            //    fail, and the post may still be saved under the key that did.
            let stored_keys: Vec<_> = tap
                .object
                .keys()
                .iter()
                .map(|key| ord(&stored_as(key)))
                .collect();
            let partly_failed = self.failed_writes[tap.failures_before..]
                .iter()
                .any(|key| stored_keys.contains(key));
            let shows = match outcome {
                None => !partly_failed,
                Some((Outcome::Ok, finished)) => !self.calls.iter().any(|call| {
                    call.read_started
                        .is_some_and(|started| started >= finished + MAX_READ_LAG)
                        && matches!(*call.outcome.borrow(), Some((Outcome::Ok, _)))
                }),
                Some(_) => false,
            };
            let got = tap.object.status(&self.service);
            if shows && got != want {
                return Err(format!(
                    "tap-shows: {:?} shows {got:?} after {}",
                    tap.object,
                    if tap.saved { "save" } else { "unsave" }
                ));
            }

            // 2. The server ends up as tapped.
            let group_done = self.taps.iter().all(|other| {
                other.object.group() != tap.object.group()
                    || self.calls[other.call].outcome.borrow().is_some()
            });
            if group_done
                && matches!(outcome, Some((Outcome::Ok, _)))
                && self.fake.has(tap.object) != tap.saved
            {
                return Err(format!(
                    "server-as-tapped: the fake has {:?} saved: {}, but the last tap was {}",
                    tap.object,
                    self.fake.has(tap.object),
                    if tap.saved { "save" } else { "unsave" }
                ));
            }
        }
        // 4. Reads show.
        let refreshes: Vec<_> = self.reads.iter().filter(|(_, list, _)| !list).collect();
        let newest = refreshes.iter().map(|(started, _, _)| *started).max();
        let tied: Vec<_> = refreshes
            .iter()
            .filter(|(started, _, _)| Some(*started) == newest)
            .map(|(_, _, keys)| keys)
            .collect();
        if tied.windows(2).all(|pair| pair[0] == pair[1]) {
            for object in OBJECTS {
                let touched = self
                    .taps
                    .iter()
                    .any(|tap| tap.object.group() == object.group());
                if touched || !self.knows_alias(object) {
                    continue;
                }
                let post = self.service.state().post_keys(&object.keys());
                let listed = self.reads.iter().any(|(started, list, keys)| {
                    *list
                        && newest.is_none_or(|newest| *started > newest)
                        && keys.iter().any(|key| {
                            listed_keys(&unord(key))
                                .iter()
                                .any(|key| post.contains(key))
                        })
                });
                let in_keys = tied.first().map(|keys| {
                    object
                        .keys()
                        .iter()
                        .any(|key| keys.contains(&ord(&stored_as(key))))
                });
                let want = match (listed, in_keys) {
                    (true, _) | (_, Some(true)) => ReaderSavedStatus::Saved,
                    (false, Some(false)) => ReaderSavedStatus::NotSaved,
                    (false, None) => ReaderSavedStatus::Unknown,
                };
                let got = object.status(&self.service);
                if got != want {
                    return Err(format!(
                        "reads-show: {object:?} shows {got:?}, the newest reads say {want:?}"
                    ));
                }
            }
        }
        Ok(())
    }

    /// The keys don't say a feed item is stored as a post, so until a write shows it, the device
    /// can't know the feed item's post is saved.
    fn knows_alias(&self, object: Object) -> bool {
        object.keys().iter().all(|key| {
            self.service.state().post_keys(std::slice::from_ref(key)) == [stored_as(key)]
        })
    }

    /// Finishes everything, then reads the fake once its newest state is guaranteed.
    fn settle(&mut self) -> Result<(), String> {
        loop {
            self.pool.run_until_stalled();
            let request = self.executor.requests.lock().unwrap().pop();
            let Some(mut request) = request.map(|request| request.0) else {
                break;
            };
            let response = self.serve(&request);
            self.reads.extend(self.last_read.take());
            let _ = request.reply.take().unwrap().send(response);
        }
        self.check()?;
        *self.clock.lock().unwrap() += MAX_READ_LAG;
        self.say("settle: refresh".to_string());
        let service = self.service.clone();
        self.spawn(
            async move {
                service
                    .refresh()
                    .await
                    .map_or(Outcome::Failed, |()| Outcome::Ok)
            },
            true,
        );
        let mut request = self.executor.requests.lock().unwrap().pop().unwrap().0;
        let response = self.serve(&request);
        self.reads.extend(self.last_read.take());
        let _ = request.reply.take().unwrap().send(response);
        self.pool.run_until_stalled();

        for object in OBJECTS {
            if !self.knows_alias(object) {
                continue;
            }
            let want = if self.fake.has(object) {
                ReaderSavedStatus::Saved
            } else {
                ReaderSavedStatus::NotSaved
            };
            let got = object.status(&self.service);
            if got != want {
                return Err(format!(
                    "settles: {object:?} shows {got:?}, the fake has {want:?}"
                ));
            }
        }
        let (got, want) = (self.service.saved_count(), Some(self.total() as u64));
        if got != want {
            return Err(format!(
                "settles: saved_count {got:?}, the fake has {want:?}"
            ));
        }
        if !self.service.recent_saves().is_empty() {
            return Err(format!(
                "settles: recent_saves {:?}",
                self.service.recent_saves()
            ));
        }
        Ok(())
    }
}

fn run(seed: u64, steps: usize) -> Result<(), (String, Vec<String>)> {
    let mut sim = Sim::new(seed);
    for _ in 0..steps {
        sim.step();
        if let Err(error) = sim.check() {
            return Err((error, sim.log));
        }
    }
    sim.settle().map_err(|error| (error, sim.log.clone()))
}

fn check_seeds(seeds: std::ops::Range<u64>, steps: usize) {
    // The shortest failure of each kind.
    let mut failures: BTreeMap<String, (usize, u64, String, Vec<String>)> = BTreeMap::new();
    for seed in seeds {
        if let Err((error, log)) = run(seed, steps) {
            let kind = error.split(':').next().unwrap().to_string();
            let entry = failures
                .entry(kind)
                .or_insert((0, seed, error.clone(), log.clone()));
            entry.0 += 1;
            if log.len() < entry.3.len() {
                (entry.1, entry.2, entry.3) = (seed, error, log);
            }
        }
    }
    if !failures.is_empty() {
        let mut report = String::new();
        for (kind, (count, seed, error, log)) in &failures {
            report += &format!(
                "\n== {kind}: {count} seeds. Shortest, seed {seed}:\n{}\n=> {error}\n",
                log.join("\n")
            );
        }
        panic!("the service and the fake server disagree:{report}");
    }
}

#[test]
fn the_service_matches_the_fake_server() {
    check_seeds(0..200, 150);
}

/// Override the size with `FAKE_SEEDS` and `FAKE_STEPS`.
#[test]
#[ignore = "long-running; run with --ignored"]
fn the_service_matches_the_fake_server_deeply() {
    let env = |name: &str, default: u64| {
        std::env::var(name)
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(default)
    };
    check_seeds(0..env("FAKE_SEEDS", 5_000), env("FAKE_STEPS", 300) as usize);
}

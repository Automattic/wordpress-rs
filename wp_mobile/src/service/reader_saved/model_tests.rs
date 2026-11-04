//! Differential test: a deliberately simple model of what the user should see, written from the
//! spec rather than from `state.rs`, driven alongside `ReaderSavedState` through random
//! interleavings. Any disagreement is shrunk to a minimal call sequence.

use super::*;
use rand::{RngExt, SeedableRng, rngs::StdRng};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};
use wp_api::posts::PostId;
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::reader_saved::ReaderFeedItemId;

/// The spec's window, not the code's constant.
const TTL: Duration = Duration::from_secs(120);
const MS: Duration = Duration::from_millis(1);

/// A key as the model sees it. URL spellings are one `U` by construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum K {
    P(i64),
    F(u64),
    U(u8),
}

const UNIVERSE: [K; 8] = [
    K::P(1),
    K::P(2),
    K::P(3),
    K::F(2),
    K::F(20),
    K::F(5),
    K::U(0),
    K::U(3),
];
const BULK: [K; 6] = [K::P(1), K::P(2), K::P(3), K::F(2), K::F(20), K::F(5)];

/// The server stores RSS item 20 as post 2, and the URL `p3` as post 3.
fn canon(k: K) -> K {
    match k {
        K::F(20) => K::P(2),
        K::U(3) => K::P(3),
        k => k,
    }
}

fn is_alias(k: K) -> bool {
    canon(k) != k
}

const URLS: &[(&str, u8)] = &[
    ("https://example.com/page", 0),
    ("https://example.com/page/", 0),
    ("HTTPS://Example.COM/page", 0),
    ("https://example.com:443/page", 0),
    ("https://example.com/page#frag", 0),
    ("https://example.com/page?utm_source=x", 0),
    ("https://EXAMPLE.com:443/page/?utm_medium=y&fbclid=z#top", 0),
    ("https://example.com/p3", 3),
    ("https://example.com/p3/", 3),
    ("HTTPS://example.com/p3/?gclid=1", 3),
];

fn spellings(id: u8) -> Vec<&'static str> {
    URLS.iter()
        .filter(|(_, u)| *u == id)
        .map(|(s, _)| *s)
        .collect()
}

fn rk(k: K, spelling: usize) -> ReaderSavedKey {
    match k {
        K::P(n) => ReaderSavedKey::Post {
            blog_id: WpComSiteId(123),
            post_id: PostId(n),
        },
        K::F(n) => ReaderSavedKey::FeedItem {
            feed_item_id: ReaderFeedItemId(n),
        },
        K::U(id) => {
            let all = spellings(id);
            ReaderSavedKey::Url {
                url: all[spelling % all.len()].to_string(),
            }
        }
    }
}

fn mk(key: &ReaderSavedKey) -> K {
    match key {
        ReaderSavedKey::Post { blog_id, post_id } => {
            assert_eq!(blog_id.0, 123);
            K::P(post_id.0)
        }
        ReaderSavedKey::FeedItem { feed_item_id } => K::F(feed_item_id.0),
        ReaderSavedKey::Url { url } => K::U(
            URLS.iter()
                .find(|(s, _)| s == url)
                .unwrap_or_else(|| panic!("unknown URL {url}"))
                .1,
        ),
    }
}

fn spelling_of(key: &ReaderSavedKey) -> usize {
    match key {
        ReaderSavedKey::Url { url } => {
            let K::U(id) = mk(key) else { unreachable!() };
            spellings(id).iter().position(|s| s == url).unwrap()
        }
        _ => 0,
    }
}

struct Item {
    name: String,
    keys: Vec<(K, usize)>,
}

impl Item {
    fn code_keys(&self) -> Vec<ReaderSavedKey> {
        self.keys.iter().map(|(k, s)| rk(*k, *s)).collect()
    }
    fn model_keys(&self) -> Vec<K> {
        self.keys.iter().map(|(k, _)| *k).collect()
    }
    fn has_alias(&self) -> bool {
        self.keys.iter().any(|(k, _)| is_alias(*k))
    }
}

fn items() -> Vec<Item> {
    let mut items: Vec<Item> = [
        ("P1", vec![(K::P(1), 0)]),
        ("Jetpack[P1,F2]", vec![(K::P(1), 0), (K::F(2), 0)]),
        ("JetpackExternal[F2]", vec![(K::F(2), 0)]),
        ("P2", vec![(K::P(2), 0)]),
        ("RSS[F20->P2]", vec![(K::F(20), 0)]),
        ("RSS[F5]", vec![(K::F(5), 0)]),
        ("P3", vec![(K::P(3), 0)]),
    ]
    .into_iter()
    .map(|(name, keys)| Item {
        name: name.to_string(),
        keys,
    })
    .collect();
    for (id, label) in [(0, "U0"), (3, "U3->P3")] {
        for (i, s) in spellings(id).iter().enumerate() {
            items.push(Item {
                name: format!("{label} {s}"),
                keys: vec![(K::U(id), i)],
            });
        }
    }
    items
}

// The model

#[derive(Clone, Copy, Debug, PartialEq)]
enum Outcome {
    Pending,
    /// Succeeded at this time.
    Ok(Duration),
    /// Failed or cancelled: as if it was never made.
    Failed,
}

#[derive(Debug)]
struct MWrite {
    key: K,
    saved: bool,
    outcome: Outcome,
}

/// What the user should see, straight from the spec:
/// - A post's writes are kept in the order the user made them. The newest one that hasn't failed
///   decides while it's in flight, or for TTL after it succeeds.
/// - After that the newest observation decides: the latest refresh accepted, a listed post noted
///   after it, or the write itself unless a refresh *started* at or after the end of its window.
///   (One that started earlier may have read the server from before the write, however late it
///   lands.)
/// - Keys are one post when they're spellings of one URL (by construction of `K`), or once a
///   successful write has shown that the server stores one as the other.
#[derive(Default)]
struct Model {
    writes: Vec<MWrite>,
    learned: Vec<(K, K)>,
    max_ticket: Option<usize>,
    /// The latest refresh accepted: when it landed, its sequence number, its keys, and when it
    /// started.
    refresh: Option<(Duration, u64, BTreeSet<K>, Duration)>,
    notes: Vec<(K, Duration, u64)>,
    seq: u64,
    extra: u64,
}

impl Model {
    fn class(&self, k: K) -> BTreeSet<K> {
        let mut class = BTreeSet::from([k]);
        loop {
            let mut grew = false;
            for (a, b) in &self.learned {
                if class.contains(a) != class.contains(b) {
                    class.insert(*a);
                    class.insert(*b);
                    grew = true;
                }
            }
            if !grew {
                return class;
            }
        }
    }

    fn rep(&self, k: K) -> K {
        *self.class(k).first().unwrap()
    }

    fn classes(&self, keys: &[K]) -> Vec<BTreeSet<K>> {
        let mut classes = Vec::new();
        for k in keys {
            let class = self.class(*k);
            if !classes.contains(&class) {
                classes.push(class);
            }
        }
        classes
    }

    fn newest(&self, class: &BTreeSet<K>) -> Option<(usize, &MWrite)> {
        self.writes
            .iter()
            .enumerate()
            .rev()
            .find(|(_, w)| class.contains(&w.key) && w.outcome != Outcome::Failed)
    }

    fn live(w: &MWrite, t: Duration) -> bool {
        match w.outcome {
            Outcome::Pending => true,
            Outcome::Ok(at) => t < at + TTL,
            Outcome::Failed => false,
        }
    }

    fn display(&self, class: &BTreeSet<K>, t: Duration) -> Option<bool> {
        let newest = self.newest(class).map(|(_, w)| w);
        if let Some(w) = newest.filter(|w| Self::live(w, t)) {
            return Some(w.saved);
        }
        let (applied, seq, data, started) = self.refresh.as_ref()?;
        let mut when = (*applied, *seq);
        // When the newest observation read the server: a refresh's start, or a listing's time.
        let mut observed = *started;
        let mut value = class.iter().any(|k| data.contains(k));
        for (k, at, seq) in &self.notes {
            if class.contains(k) && (*at, *seq) > when {
                when = (*at, *seq);
                observed = *at;
                value = true;
            }
        }
        if let Some(MWrite {
            outcome: Outcome::Ok(at),
            saved,
            ..
        }) = newest
            && *at + TTL > observed
        {
            value = *saved;
        }
        Some(value)
    }

    fn status(&self, keys: &[K], t: Duration) -> ReaderSavedStatus {
        if keys.is_empty() {
            return ReaderSavedStatus::NotSaved;
        }
        let values: Vec<_> = self
            .classes(keys)
            .iter()
            .map(|c| self.display(c, t))
            .collect();
        if values.contains(&Some(true)) {
            ReaderSavedStatus::Saved
        } else if values.contains(&None) {
            ReaderSavedStatus::Unknown
        } else {
            ReaderSavedStatus::NotSaved
        }
    }

    fn count(&self, t: Duration) -> Option<u64> {
        self.refresh.as_ref()?;
        let saved = self
            .classes(&UNIVERSE)
            .iter()
            .filter(|c| self.display(c, t) == Some(true))
            .count();
        Some(self.extra + saved as u64)
    }

    fn recent(&self, t: Duration) -> Vec<K> {
        let mut recent: Vec<_> = self
            .classes(&UNIVERSE)
            .iter()
            .filter_map(|c| {
                let (index, w) = self.newest(c)?;
                (w.saved && Self::live(w, t)).then(|| (index, *c.first().unwrap()))
            })
            .collect();
        recent.sort_by_key(|(index, _)| Reverse(*index));
        recent.into_iter().map(|(_, k)| k).collect()
    }

    fn should_show(&self, keys: &[K], t: Duration) -> bool {
        let recently_unsaved = self.classes(keys).iter().any(|c| {
            self.newest(c)
                .is_some_and(|(_, w)| !w.saved && Self::live(w, t))
        });
        !recently_unsaved || self.status(keys, t) == ReaderSavedStatus::Saved
    }

    fn note(&mut self, keys: &[K], t: Duration) -> bool {
        if self.refresh.is_none()
            || self.extra > 0
            || self.status(keys, t) == ReaderSavedStatus::Saved
        {
            return false;
        }
        self.seq += 1;
        self.notes.push((keys[0], t, self.seq));
        true
    }

    fn learn(&mut self, requested: K, stored: K) {
        if requested != stored && !self.learned.contains(&(requested, stored)) {
            self.learned.push((requested, stored));
        }
    }
}

// Driving the code and the model together

#[derive(Clone, Copy, Debug)]
struct Cfg {
    aliases: bool,
    realistic: bool,
    extra: u64,
    delete_resolves: bool,
    check_unsave_key: bool,
    initial_refresh: bool,
}

#[derive(Clone, Debug)]
enum Op {
    Advance(Duration),
    Save(usize),
    Unsave(usize),
    UnsaveKey(K, usize),
    Import(Vec<K>),
    /// Finishes the call's first unfinished write: its requests go one at a time.
    Finish {
        call: usize,
        ok: bool,
        on_server: bool,
        spelling: usize,
    },
    FinishImport {
        call: usize,
        oks: Vec<bool>,
    },
    /// The call's future is dropped, or its request fails: every unfinished write fails.
    Cancel {
        call: usize,
        on_server: bool,
    },
    BeginRefresh {
        lag: Duration,
        spelling: usize,
    },
    ApplyRefresh(usize),
    List {
        lag: Duration,
        pair: bool,
    },
}

struct CallWrite {
    model: usize,
    key: ReaderSavedKey,
    id: WriteId,
    done: bool,
}

struct Call {
    import: bool,
    writes: Vec<CallWrite>,
}

struct Refresh {
    ticket: RefreshTicket,
    started: Duration,
    keys: Vec<ReaderSavedKey>,
    data: BTreeSet<K>,
    applied: bool,
}

/// The server's writes, so a read can lag behind them.
#[derive(Default)]
struct Server {
    history: Vec<(Duration, K, bool)>,
}

impl Server {
    fn apply(&mut self, t: Duration, key: K, saved: bool) {
        self.history.push((t, key, saved));
    }

    fn read(&self, at: Duration) -> BTreeSet<K> {
        let mut saved = BTreeSet::new();
        for (t, key, value) in &self.history {
            if *t <= at {
                if *value {
                    saved.insert(*key);
                } else {
                    saved.remove(key);
                }
            }
        }
        saved
    }
}

#[derive(Default, Debug, Clone)]
struct Coverage {
    late_older_completion_ignored: u64,
    newer_failed_while_older_pending: u64,
    stale_refresh_ignored: u64,
    refresh_contradicting_live_write: u64,
    refresh_after_window_replacing_write: u64,
    checks_at_window_boundary: u64,
    notes_added: u64,
    listed_posts_hidden: u64,
    two_key_unsaves: u64,
    cancelled_midway: u64,
    unknown_seen: u64,
    aliases_learned: u64,
}

struct Sim {
    cov: Coverage,
    cfg: Cfg,
    items: Vec<Item>,
    state: ReaderSavedState,
    model: Model,
    server: Server,
    start: Instant,
    t: Duration,
    calls: Vec<Call>,
    refreshes: Vec<Refresh>,
    log: Option<Vec<String>>,
}

impl Sim {
    fn new(cfg: Cfg) -> Self {
        Self {
            cov: Coverage::default(),
            cfg,
            items: items(),
            state: ReaderSavedState::default(),
            model: Model {
                extra: cfg.extra,
                ..Model::default()
            },
            server: Server::default(),
            start: Instant::now(),
            t: Duration::ZERO,
            calls: Vec::new(),
            refreshes: Vec::new(),
            log: None,
        }
    }

    fn now(&self) -> Instant {
        self.start + self.t
    }

    fn say(&mut self, line: String) {
        let t = self.t;
        if let Some(log) = &mut self.log {
            log.push(format!("t={:>8.3}s  {line}", t.as_secs_f64()));
        }
    }

    fn begin(&mut self, key: ReaderSavedKey, saved: bool) -> CallWrite {
        let id = self.state.begin_write(&key, saved);
        self.model.writes.push(MWrite {
            key: mk(&key),
            saved,
            outcome: Outcome::Pending,
        });
        CallWrite {
            model: self.model.writes.len() - 1,
            key,
            id,
            done: false,
        }
    }

    /// The key the server stores a write under and sends back.
    fn stored(&self, requested: K, saved: bool) -> K {
        if saved || self.cfg.delete_resolves {
            canon(requested)
        } else {
            requested
        }
    }

    fn check_sent_key(&self, sent: &ReaderSavedKey) -> Result<(), String> {
        let k = mk(sent);
        if self.cfg.check_unsave_key && self.model.class(k).len() > 1 && canon(k) != k {
            return Err(format!(
                "unsave-key: sent {k:?}, known to be stored as {:?}",
                canon(k)
            ));
        }
        Ok(())
    }

    fn exec(&mut self, op: &Op) -> Result<(), String> {
        let now = self.now();
        match op {
            Op::Advance(d) => self.t += *d,
            Op::Save(item) => {
                let (k, s) = self.items[*item].keys[0];
                let key = rk(k, s);
                self.say(format!(
                    "call {}: save {key:?} [{}]",
                    self.calls.len(),
                    self.items[*item].name
                ));
                let w = self.begin(key, true);
                self.calls.push(Call {
                    import: false,
                    writes: vec![w],
                });
            }
            Op::Unsave(item) => {
                let item = &self.items[*item];
                let (name, code_keys, model_keys) =
                    (item.name.clone(), item.code_keys(), item.model_keys());
                let sent = self.state.keys_to_unsave(&code_keys, now);
                let mut want: BTreeSet<K> = self
                    .model
                    .classes(&model_keys)
                    .iter()
                    .filter(|c| self.model.display(c, self.t) == Some(true))
                    .map(|c| *c.first().unwrap())
                    .collect();
                if want.is_empty() {
                    want.insert(self.model.rep(model_keys[0]));
                }
                let got: BTreeSet<K> = sent.iter().map(|k| self.model.rep(mk(k))).collect();
                self.say(format!(
                    "call {}: unsave [{name}] -> keys_to_unsave = {sent:?}",
                    self.calls.len()
                ));
                if got != want {
                    return Err(format!(
                        "unsave-classes: [{name}] sent {sent:?} (posts {got:?}), want posts {want:?}"
                    ));
                }
                for key in &sent {
                    self.check_sent_key(key)?;
                }
                if sent.len() > 1 {
                    self.cov.two_key_unsaves += 1;
                }
                let writes = sent.into_iter().map(|key| self.begin(key, false)).collect();
                self.calls.push(Call {
                    import: false,
                    writes,
                });
            }
            Op::UnsaveKey(k, s) => {
                // As the service does: send the key the server stores the post under, if known.
                let key = self.state.key_to_send(&rk(*k, *s));
                self.say(format!("call {}: unsave_key {key:?}", self.calls.len()));
                self.check_sent_key(&key)?;
                let w = self.begin(key, false);
                self.calls.push(Call {
                    import: false,
                    writes: vec![w],
                });
            }
            Op::Import(keys) => {
                self.say(format!("call {}: import {keys:?}", self.calls.len()));
                let writes = keys.iter().map(|k| self.begin(rk(*k, 0), true)).collect();
                self.calls.push(Call {
                    import: true,
                    writes,
                });
            }
            Op::Finish {
                call,
                ok,
                on_server,
                spelling,
            } => {
                let Some(c) = self.calls.get(*call) else {
                    return Ok(());
                };
                if c.import {
                    return Ok(());
                }
                let Some(pos) = c.writes.iter().position(|w| !w.done) else {
                    return Ok(());
                };
                let (index, key, id) = (
                    c.writes[pos].model,
                    c.writes[pos].key.clone(),
                    c.writes[pos].id,
                );
                let (requested, saved) =
                    (self.model.writes[index].key, self.model.writes[index].saved);
                let verb = if saved { "save" } else { "unsave" };
                let class = self.model.class(requested);
                let newer_done = self.model.writes[index + 1..]
                    .iter()
                    .any(|w| class.contains(&w.key) && matches!(w.outcome, Outcome::Ok(_)));
                let older_pending = self.model.writes[..index]
                    .iter()
                    .any(|w| class.contains(&w.key) && w.outcome == Outcome::Pending);
                if *ok && newer_done {
                    self.cov.late_older_completion_ignored += 1;
                }
                if !*ok && older_pending {
                    self.cov.newer_failed_while_older_pending += 1;
                }
                if *ok {
                    let stored = self.stored(requested, saved);
                    let stored_key = rk(stored, *spelling);
                    self.say(format!(
                        "call {call}: {verb} {key:?} succeeds, stored as {stored_key:?}"
                    ));
                    self.server.apply(self.t, stored, saved);
                    self.state.complete_write(&key, id, &stored_key, now);
                    self.model.writes[index].outcome = Outcome::Ok(self.t);
                    if requested != stored {
                        self.cov.aliases_learned += 1;
                    }
                    self.model.learn(requested, stored);
                } else {
                    self.say(format!(
                        "call {call}: {verb} {key:?} fails (reached server: {on_server})"
                    ));
                    if *on_server {
                        self.server
                            .apply(self.t, self.stored(requested, saved), saved);
                    }
                    self.state.fail_write(&key, id);
                    self.model.writes[index].outcome = Outcome::Failed;
                }
                self.calls[*call].writes[pos].done = true;
            }
            Op::FinishImport { call, oks } => {
                let Some(c) = self.calls.get(*call) else {
                    return Ok(());
                };
                if !c.import || c.writes.iter().all(|w| w.done) {
                    return Ok(());
                }
                let pending: Vec<_> = c
                    .writes
                    .iter()
                    .map(|w| (w.model, w.key.clone(), w.id))
                    .collect();
                self.say(format!(
                    "call {call}: import finishes, accepted per key {oks:?}"
                ));
                for (i, (index, key, id)) in pending.into_iter().enumerate() {
                    let requested = self.model.writes[index].key;
                    if oks[i % oks.len()] {
                        let stored = canon(requested);
                        self.server.apply(self.t, stored, true);
                        self.state.complete_write(&key, id, &rk(stored, 0), now);
                        self.model.writes[index].outcome = Outcome::Ok(self.t);
                        self.model.learn(requested, stored);
                    } else {
                        self.state.fail_write(&key, id);
                        self.model.writes[index].outcome = Outcome::Failed;
                    }
                }
                for w in &mut self.calls[*call].writes {
                    w.done = true;
                }
            }
            Op::Cancel { call, on_server } => {
                let Some(c) = self.calls.get(*call) else {
                    return Ok(());
                };
                let import = c.import;
                let pending: Vec<_> = c
                    .writes
                    .iter()
                    .enumerate()
                    .filter(|(_, w)| !w.done)
                    .map(|(pos, w)| (pos, w.model, w.key.clone(), w.id))
                    .collect();
                if pending.is_empty() {
                    return Ok(());
                }
                if pending.len() < self.calls[*call].writes.len() {
                    self.cov.cancelled_midway += 1;
                }
                self.say(format!(
                    "call {call}: cancelled (in-flight request reached server: {on_server})"
                ));
                for (n, (pos, index, key, id)) in pending.into_iter().enumerate() {
                    let (requested, saved) =
                        (self.model.writes[index].key, self.model.writes[index].saved);
                    // Only a request already sent can have reached the server.
                    if *on_server && (import || n == 0) {
                        self.server
                            .apply(self.t, self.stored(requested, saved), saved);
                    }
                    self.state.fail_write(&key, id);
                    self.model.writes[index].outcome = Outcome::Failed;
                    self.calls[*call].writes[pos].done = true;
                }
            }
            Op::BeginRefresh { lag, spelling } => {
                let ticket = self.state.begin_refresh(now);
                let data = self.server.read(self.t.saturating_sub(*lag));
                let keys: Vec<_> = data.iter().map(|k| rk(*k, *spelling)).collect();
                self.say(format!(
                    "refresh {} begins, will return {data:?}",
                    self.refreshes.len()
                ));
                self.refreshes.push(Refresh {
                    ticket,
                    started: self.t,
                    keys,
                    data,
                    applied: false,
                });
            }
            Op::ApplyRefresh(n) => {
                let Some(r) = self.refreshes.get_mut(*n) else {
                    return Ok(());
                };
                if r.applied {
                    return Ok(());
                }
                r.applied = true;
                let (ticket, started, keys, data) =
                    (r.ticket, r.started, r.keys.clone(), r.data.clone());
                self.say(format!("refresh {n} lands with {data:?}"));
                let total = keys.len() as u64 + self.cfg.extra;
                self.state.apply_refresh(ticket, keys, total);
                if self.model.max_ticket.is_some_and(|max| *n < max) {
                    self.cov.stale_refresh_ignored += 1;
                } else {
                    for k in UNIVERSE {
                        let class = self.model.class(k);
                        if let Some((_, w)) = self.model.newest(&class) {
                            let in_data = class.iter().any(|k| data.contains(k));
                            if Model::live(w, self.t) && w.saved != in_data {
                                self.cov.refresh_contradicting_live_write += 1;
                            }
                            if let Outcome::Ok(at) = w.outcome
                                && started >= at + TTL
                                && w.saved != in_data
                            {
                                self.cov.refresh_after_window_replacing_write += 1;
                            }
                        }
                    }
                }
                if self.model.max_ticket.is_none_or(|max| *n >= max) {
                    self.model.max_ticket = Some(*n);
                    self.model.seq += 1;
                    self.model.refresh = Some((self.t, self.model.seq, data, started));
                }
            }
            Op::List { lag, pair } => {
                let data = self.server.read(self.t.saturating_sub(*lag));
                self.say(format!(
                    "list_posts returns {data:?} (Jetpack post listed with both keys: {pair})"
                ));
                for k in data {
                    let keys: Vec<K> = match k {
                        K::P(1) | K::F(2) if *pair => vec![K::P(1), K::F(2)],
                        k => vec![k],
                    };
                    let code_keys: Vec<_> = keys.iter().map(|k| rk(*k, 0)).collect();
                    let show = self.state.should_show_listed_post(&code_keys, now);
                    let want = self.model.should_show(&keys, self.t);
                    if !show {
                        self.cov.listed_posts_hidden += 1;
                    }
                    if show != want {
                        return Err(format!("list-show: {keys:?} code={show} model={want}"));
                    }
                    if show {
                        let noted = self.state.note_listed_post(&code_keys, now);
                        let want = self.model.note(&keys, self.t);
                        if noted {
                            self.cov.notes_added += 1;
                        }
                        if noted != want {
                            return Err(format!("list-note: {keys:?} code={noted} model={want}"));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn check(&mut self) -> Result<(), String> {
        let now = self.now();
        let boundary = self.model.writes.iter().any(|w| match w.outcome {
            Outcome::Ok(at) => [at + TTL - MS, at + TTL, at + TTL + MS].contains(&self.t),
            _ => false,
        });
        if boundary {
            self.cov.checks_at_window_boundary += 1;
        }
        for item in &self.items {
            if !self.cfg.aliases && item.has_alias() {
                continue;
            }
            let got = self.state.status(&item.code_keys(), now);
            let want = self.model.status(&item.model_keys(), self.t);
            if got == ReaderSavedStatus::Unknown {
                self.cov.unknown_seen += 1;
            }
            if got != want {
                return Err(format!(
                    "status: [{}] code={got:?} model={want:?}",
                    item.name
                ));
            }
        }
        let (got, want) = (self.state.saved_count(now), self.model.count(self.t));
        if got != want {
            return Err(format!("count: code={got:?} model={want:?}"));
        }
        let got: Vec<K> = self
            .state
            .recent_saves(now)
            .iter()
            .map(|k| self.model.rep(mk(k)))
            .collect();
        let want = self.model.recent(self.t);
        if got != want {
            return Err(format!("recent: code={got:?} model={want:?}"));
        }
        Ok(())
    }

    fn status_of(&self, item: usize) -> ReaderSavedStatus {
        self.model.status(&self.items[item].model_keys(), self.t)
    }

    fn gen_advance(&self, rng: &mut StdRng) -> Duration {
        match rng.random_range(0..10) {
            0..4 => Duration::from_millis(rng.random_range(0..=10_000)),
            4..7 => {
                // Up to just before, at, or just after the end of a write's window.
                let ends: Vec<Duration> = self
                    .model
                    .writes
                    .iter()
                    .filter_map(|w| match w.outcome {
                        Outcome::Ok(at) => Some(at + TTL),
                        _ => None,
                    })
                    .filter(|end| *end + MS > self.t)
                    .collect();
                if ends.is_empty() {
                    return Duration::from_secs(rng.random_range(0..=5));
                }
                let end = ends[rng.random_range(0..ends.len())];
                let target = match rng.random_range(0..3) {
                    0 => end - MS,
                    1 => end,
                    _ => end + MS,
                };
                target.saturating_sub(self.t)
            }
            7..9 => Duration::from_secs(rng.random_range(20..=60)),
            _ => Duration::from_secs(rng.random_range(100..=300)),
        }
    }

    fn next_op(&self, rng: &mut StdRng) -> Op {
        let usable: Vec<usize> = (0..self.items.len())
            .filter(|i| self.cfg.aliases || !self.items[*i].has_alias())
            .collect();
        let bulk: Vec<K> = BULK
            .into_iter()
            .filter(|k| self.cfg.aliases || !is_alias(*k))
            .collect();
        let open: Vec<usize> = (0..self.calls.len())
            .filter(|c| self.calls[*c].writes.iter().any(|w| !w.done))
            .collect();
        loop {
            match rng.random_range(0..100) {
                0..25 => return Op::Advance(self.gen_advance(rng)),
                25..37 => {
                    let item = usable[rng.random_range(0..usable.len())];
                    if self.cfg.realistic && self.status_of(item) == ReaderSavedStatus::Saved {
                        continue;
                    }
                    return Op::Save(item);
                }
                37..47 => {
                    let item = usable[rng.random_range(0..usable.len())];
                    if self.cfg.realistic && self.status_of(item) != ReaderSavedStatus::Saved {
                        continue;
                    }
                    return Op::Unsave(item);
                }
                47..51 => {
                    if self.cfg.realistic {
                        let recent = self.state.recent_saves(self.now());
                        if recent.is_empty() {
                            continue;
                        }
                        let key = &recent[rng.random_range(0..recent.len())];
                        return Op::UnsaveKey(mk(key), spelling_of(key));
                    }
                    let item = &self.items[usable[rng.random_range(0..usable.len())]];
                    let (k, s) = item.keys[rng.random_range(0..item.keys.len())];
                    return Op::UnsaveKey(k, s);
                }
                51..55 => {
                    let n = rng.random_range(1..=3);
                    return Op::Import(
                        (0..n)
                            .map(|_| bulk[rng.random_range(0..bulk.len())])
                            .collect(),
                    );
                }
                55..76 => {
                    let calls: Vec<_> = open
                        .iter()
                        .copied()
                        .filter(|c| !self.calls[*c].import)
                        .collect();
                    if calls.is_empty() {
                        continue;
                    }
                    return Op::Finish {
                        call: calls[rng.random_range(0..calls.len())],
                        ok: rng.random_bool(0.75),
                        on_server: rng.random_bool(0.5),
                        spelling: rng.random_range(0..10),
                    };
                }
                76..80 => {
                    if open.is_empty() {
                        continue;
                    }
                    return Op::Cancel {
                        call: open[rng.random_range(0..open.len())],
                        on_server: rng.random_bool(0.5),
                    };
                }
                80..84 => {
                    let calls: Vec<_> = open
                        .iter()
                        .copied()
                        .filter(|c| self.calls[*c].import)
                        .collect();
                    if calls.is_empty() {
                        continue;
                    }
                    return Op::FinishImport {
                        call: calls[rng.random_range(0..calls.len())],
                        oks: (0..3).map(|_| rng.random_bool(0.8)).collect(),
                    };
                }
                84..91 => {
                    return Op::BeginRefresh {
                        lag: Duration::from_secs(rng.random_range(0..=90)),
                        spelling: rng.random_range(0..10),
                    };
                }
                91..96 => {
                    let open: Vec<_> = (0..self.refreshes.len())
                        .filter(|r| !self.refreshes[*r].applied)
                        .collect();
                    if open.is_empty() {
                        continue;
                    }
                    return Op::ApplyRefresh(open[rng.random_range(0..open.len())]);
                }
                _ => {
                    return Op::List {
                        lag: Duration::from_secs(rng.random_range(0..=90)),
                        pair: rng.random_bool(0.5),
                    };
                }
            }
        }
    }
}

fn category(error: &str) -> &str {
    error.split(':').next().unwrap()
}

fn prelude(cfg: Cfg) -> Vec<Op> {
    if cfg.initial_refresh {
        vec![
            Op::BeginRefresh {
                lag: Duration::ZERO,
                spelling: 0,
            },
            Op::ApplyRefresh(0),
        ]
    } else {
        vec![]
    }
}

fn replay(cfg: Cfg, ops: &[Op]) -> Option<(usize, String)> {
    let mut sim = Sim::new(cfg);
    for op in &prelude(cfg) {
        sim.exec(op).unwrap();
    }
    for (i, op) in ops.iter().enumerate() {
        if let Err(error) = sim.exec(op).and_then(|()| sim.check()) {
            return Some((i, error));
        }
    }
    None
}

fn still_fails(cfg: Cfg, ops: &[Op], cat: &str) -> Option<usize> {
    replay(cfg, ops)
        .filter(|(_, e)| category(e) == cat)
        .map(|(i, _)| i)
}

/// Removes ops, then merges and rounds advances, while the same kind of disagreement remains.
fn shrink(cfg: Cfg, ops: &[Op], cat: &str) -> Vec<Op> {
    let mut ops = ops[..=still_fails(cfg, ops, cat).unwrap()].to_vec();
    let mut chunk = ops.len().div_ceil(2);
    loop {
        let mut removed = false;
        let mut start = 0;
        while start < ops.len() {
            let mut candidate = ops.clone();
            candidate.drain(start..(start + chunk).min(ops.len()));
            match still_fails(cfg, &candidate, cat) {
                Some(i) => {
                    candidate.truncate(i + 1);
                    ops = candidate;
                    removed = true;
                }
                None => start += chunk,
            }
        }
        if chunk == 1 && !removed {
            break;
        }
        chunk = (chunk / 2).max(1);
    }
    // Merge neighbouring advances, then round each down to whole seconds where that still fails.
    let mut i = 0;
    while i + 1 < ops.len() {
        if let (Op::Advance(a), Op::Advance(b)) = (&ops[i], &ops[i + 1]) {
            let mut candidate = ops.clone();
            candidate[i] = Op::Advance(*a + *b);
            candidate.remove(i + 1);
            if still_fails(cfg, &candidate, cat).is_some() {
                ops = candidate;
                continue;
            }
        }
        i += 1;
    }
    for i in 0..ops.len() {
        if let Op::Advance(d) = ops[i] {
            for shorter in [Duration::ZERO, Duration::from_secs(d.as_secs())] {
                if shorter < d {
                    let mut candidate = ops.clone();
                    candidate[i] = Op::Advance(shorter);
                    if still_fails(cfg, &candidate, cat).is_some() {
                        ops = candidate;
                        break;
                    }
                }
            }
        }
    }
    ops.retain(|op| !matches!(op, Op::Advance(d) if d.is_zero()));
    ops
}

fn explain(cfg: Cfg, ops: &[Op]) -> String {
    let mut sim = Sim::new(cfg);
    sim.log = Some(Vec::new());
    let mut out = String::new();
    for op in ops {
        let result = sim.exec(op).and_then(|()| sim.check());
        for line in sim.log.as_mut().unwrap().drain(..) {
            out += &format!("    {line}\n");
        }
        if let Err(error) = result {
            out += &format!("    t={:>8.3}s  => {error}\n", sim.t.as_secs_f64());
            break;
        }
    }
    out
}

/// Each distinct shrunk failure, by its explanation: the seed it came from and its operations.
type DistinctShapes = BTreeMap<String, (u64, Vec<Op>)>;

fn configs(seed: u64) -> Vec<(&'static str, Cfg)> {
    let extra = if seed.is_multiple_of(3) { 3 } else { 0 };
    let base = Cfg {
        aliases: false,
        realistic: true,
        extra,
        delete_resolves: true,
        check_unsave_key: false,
        initial_refresh: false,
    };
    let loaded = Cfg {
        aliases: true,
        initial_refresh: true,
        ..base
    };
    vec![
        ("plain-realistic", base),
        (
            "plain-free",
            Cfg {
                realistic: false,
                ..base
            },
        ),
        (
            "alias-realistic",
            Cfg {
                aliases: true,
                ..base
            },
        ),
        (
            "alias-free",
            Cfg {
                aliases: true,
                realistic: false,
                ..base
            },
        ),
        ("alias-loaded-realistic", loaded),
        (
            "alias-loaded-free",
            Cfg {
                realistic: false,
                ..loaded
            },
        ),
        (
            "alias-loaded-realistic-literal-delete",
            Cfg {
                delete_resolves: false,
                ..loaded
            },
        ),
        (
            "alias-loaded-realistic-unsave-key",
            Cfg {
                check_unsave_key: true,
                ..loaded
            },
        ),
    ]
}

/// A short run on every `cargo test`, so a change that breaks the rules fails CI.
#[test]
fn the_state_matches_a_reference_model_quickly() {
    check_against_the_model(40, 150);
}

/// The deep run. Override the size with `MODEL_SEEDS` and `MODEL_STEPS`, and pick one config with
/// `MODEL_ONLY`. Run with `--ignored --nocapture`. 3,000 seeds of 300 steps take about 10 minutes.
#[test]
#[ignore = "long-running; run with --ignored --nocapture"]
fn the_state_matches_a_reference_model() {
    let env = |name: &str, default: u64| {
        std::env::var(name)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    };
    check_against_the_model(env("MODEL_SEEDS", 300), env("MODEL_STEPS", 200) as usize);
}

fn check_against_the_model(seeds: u64, steps: usize) {
    let env = |name: &str, default: u64| {
        std::env::var(name)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    };
    let only = std::env::var("MODEL_ONLY").ok();
    let shrink_limit = env("MODEL_SHRINK", 40);

    let mut runs: BTreeMap<&str, (u64, u64)> = BTreeMap::new();
    let mut coverage: BTreeMap<&str, Coverage> = BTreeMap::new();
    let mut failures: BTreeMap<(String, String), (u64, u64, Vec<Op>)> = BTreeMap::new();
    let mut distinct: BTreeMap<(String, String), DistinctShapes> = BTreeMap::new();
    for seed in 0..seeds {
        for (label, cfg) in configs(seed) {
            if only.as_deref().is_some_and(|only| only != label) {
                continue;
            }
            let mut rng = StdRng::seed_from_u64(seed);
            let mut sim = Sim::new(cfg);
            let mut ops = Vec::new();
            for op in &prelude(cfg) {
                sim.exec(op).unwrap();
            }
            let run = runs.entry(label).or_default();
            run.0 += 1;
            for _ in 0..steps {
                let op = sim.next_op(&mut rng);
                ops.push(op.clone());
                run.1 += 1;
                if let Err(error) = sim.exec(&op).and_then(|()| sim.check()) {
                    let cat = category(&error).to_string();
                    let entry = failures.entry((label.to_string(), cat.clone()));
                    let slot = entry.or_insert((0, seed, Vec::new()));
                    slot.0 += 1;
                    // Shrink the first few of each kind, and keep the shortest.
                    if slot.0 <= shrink_limit {
                        let shrunk = shrink(cfg, &ops, &cat);
                        if slot.2.is_empty() || shrunk.len() < slot.2.len() {
                            slot.1 = seed;
                            slot.2 = shrunk.clone();
                        }
                        let signature: String = shrunk
                            .iter()
                            .map(|op| {
                                format!("{op:?}")
                                    .split(['(', ' '])
                                    .next()
                                    .unwrap()
                                    .to_string()
                            })
                            .collect::<Vec<_>>()
                            .join(",");
                        distinct
                            .entry((label.to_string(), cat.clone()))
                            .or_default()
                            .entry(signature)
                            .or_insert((seed, shrunk));
                    }
                    break;
                }
            }
            let cov = coverage.entry(label).or_default();
            macro_rules! add {
                ($($f:ident),*) => { $(cov.$f += sim.cov.$f;)* };
            }
            add!(
                late_older_completion_ignored,
                newer_failed_while_older_pending,
                stale_refresh_ignored,
                refresh_contradicting_live_write,
                refresh_after_window_replacing_write,
                checks_at_window_boundary,
                notes_added,
                listed_posts_hidden,
                two_key_unsaves,
                cancelled_midway,
                unknown_seen,
                aliases_learned
            );
        }
    }
    println!("\nCoverage:");
    for (label, cov) in &coverage {
        println!("  {label:<40} {cov:?}");
    }

    println!("\nRuns (seeds, steps):");
    for (label, (n, steps)) in &runs {
        println!("  {label:<32} {n:>6} seeds {steps:>9} steps");
    }
    println!("\nDisagreements: {}", failures.len());
    for ((label, cat), (count, seed, ops)) in &failures {
        let cfg = configs(*seed)
            .into_iter()
            .find(|(l, _)| l == label)
            .unwrap()
            .1;
        println!(
            "\n== {label} / {cat}: {count} seeds; shortest from seed {seed}, {} ops ({cfg:?})",
            ops.len()
        );
        print!("{}", explain(cfg, &[prelude(cfg), ops.clone()].concat()));
        let shapes = &distinct[&(label.clone(), cat.clone())];
        let verbose = std::env::var("MODEL_DISTINCT").is_ok();
        println!("   ({} distinct shrunk shapes)", shapes.len());
        if verbose {
            for (shape, (seed, ops)) in shapes {
                println!("   -- shape from seed {seed}: {shape}");
                print!("{}", explain(cfg, &[prelude(cfg), ops.clone()].concat()));
            }
        }
    }
    assert!(failures.is_empty(), "the code and the model disagree");
}

// Minimal repros, judged against the spec.

mod repro {
    use super::*;

    fn post(n: i64) -> ReaderSavedKey {
        rk(K::P(n), 0)
    }
    fn feed(n: u64) -> ReaderSavedKey {
        rk(K::F(n), 0)
    }
    fn loaded(now: Instant) -> ReaderSavedState {
        let mut state = ReaderSavedState::default();
        let ticket = state.begin_refresh(now);
        state.apply_refresh(ticket, Vec::new(), 0);
        state
    }

    /// Save an RSS item the server stores as a WordPress.com post, then unsave it before the save
    /// returns. The save's response lands first.
    #[test]
    fn a_save_stored_under_another_key_does_not_override_a_newer_unsave_in_flight() {
        let now = Instant::now();
        let mut state = loaded(now);
        let rss = [feed(20)];
        let save = state.begin_write(&feed(20), true);
        let to_unsave = state.keys_to_unsave(&rss, now);
        assert_eq!(to_unsave, vec![feed(20)]);
        let unsave = state.begin_write(&feed(20), false);
        assert_eq!(state.status(&rss, now), ReaderSavedStatus::NotSaved);

        state.complete_write(&feed(20), save, &post(2), now);

        // The user's newest write is the unsave, still in flight.
        let seen = (
            state.status(&rss, now),
            state.saved_count(now),
            state.recent_saves(now),
        );
        let _ = unsave;
        assert_eq!(
            seen,
            (ReaderSavedStatus::NotSaved, Some(0), vec![]),
            "(status, saved_count, recent_saves) while the newer unsave is in flight"
        );
    }

    /// As above, but the unsave's response names the key it was sent with.
    #[test]
    fn a_succeeded_unsave_is_not_overridden_by_an_older_save_stored_under_another_key() {
        let now = Instant::now();
        let mut state = loaded(now);
        let rss = [feed(20)];
        let save = state.begin_write(&feed(20), true);
        let unsave = state.begin_write(&feed(20), false);
        state.complete_write(&feed(20), save, &post(2), now);
        state.complete_write(&feed(20), unsave, &feed(20), now);

        let later = now + Duration::from_secs(60);
        assert_eq!(
            (state.status(&rss, later), state.saved_count(later)),
            (ReaderSavedStatus::NotSaved, Some(0)),
            "the newest write, an unsave, succeeded 60s ago"
        );
    }

    /// An older save still in flight, and a newer unsave of the same key that the server reports
    /// under the key it stored the post as.
    #[test]
    fn an_older_save_in_flight_does_not_take_over_when_a_newer_unsave_returns_another_key() {
        let now = Instant::now();
        let mut state = loaded(now);
        let rss = [feed(20)];
        let _save = state.begin_write(&feed(20), true);
        let unsave = state.begin_write(&feed(20), false);
        state.complete_write(&feed(20), unsave, &post(2), now);

        assert_eq!(state.status(&rss, now), ReaderSavedStatus::NotSaved);
        assert!(state.recent_saves(now).is_empty());
    }

    /// The existing test `a_write_that_hangs_past_a_newer_ones_window_is_ignored_when_it_lands`,
    /// with the URL a WordPress.com post's.
    #[test]
    fn a_write_that_hangs_past_a_newer_ones_window_is_ignored_when_the_key_resolves() {
        let now = Instant::now();
        let mut state = loaded(now);
        let page = rk(K::U(3), 0);
        let older = state.begin_write(&page, true);
        let newer = state.begin_write(&page, false);
        state.complete_write(&page, newer, &post(3), now);

        let later = now + Duration::from_secs(200);
        let ticket = state.begin_refresh(now);
        state.apply_refresh(ticket, Vec::new(), 0);
        state.complete_write(&page, older, &post(3), later + Duration::from_secs(10));

        let at = later + Duration::from_secs(11);
        assert_eq!(
            state.status(std::slice::from_ref(&page), at),
            ReaderSavedStatus::NotSaved
        );
        assert_eq!(state.saved_count(at), Some(0));
    }

    /// Re-saving by a key whose stored key is already known and saved, e.g. importing again.
    #[test]
    fn importing_a_post_already_saved_under_its_stored_key_counts_it_once() {
        let now = Instant::now();
        let mut state = loaded(now);
        let first = state.begin_write(&feed(20), true);
        state.complete_write(&feed(20), first, &post(2), now);
        assert_eq!(state.saved_count(now), Some(1));

        state.begin_write(&feed(20), true);

        assert_eq!(state.saved_count(now), Some(1));
    }

    /// Before the first refresh: save a URL that resolves to a post, then unsave it with the key
    /// `recent_saves` returns.
    #[test]
    fn a_url_unsaved_by_its_stored_key_reads_unsaved_before_the_first_refresh() {
        let now = Instant::now();
        let mut state = ReaderSavedState::default();
        let page = rk(K::U(3), 0);
        let save = state.begin_write(&page, true);
        state.complete_write(&page, save, &post(3), now);
        assert_eq!(state.recent_saves(now), vec![post(3)]);
        state.begin_write(&post(3), false);

        assert_eq!(
            state.status(std::slice::from_ref(&page), now),
            ReaderSavedStatus::NotSaved
        );
    }

    /// A refresh that starts while a write is in its window, but whose response is slow.
    #[test]
    fn a_slow_refresh_started_inside_the_window_keeps_the_write() {
        let now = Instant::now();
        let mut state = loaded(now);
        let save = state.begin_write(&post(1), true);
        state.complete_write(&post(1), save, &post(1), now);

        // Starts 85s after the save, reading a replica 90s behind: from before the save. It lands
        // after the save's window has ended, at 121s, but it's judged by when it started.
        let ticket = state.begin_refresh(now + Duration::from_secs(85));
        state.apply_refresh(ticket, Vec::new(), 0);

        let later = now + Duration::from_secs(600);
        assert_eq!(state.status(&[post(1)], later), ReaderSavedStatus::Saved);
    }
}

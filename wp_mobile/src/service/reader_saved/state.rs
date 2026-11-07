//! The saved posts this device knows about, without any networking.
//!
//! The saved posts API's reads can lag behind its writes. Rather than guess what the server holds,
//! this relies on four facts about it, and nothing else:
//!
//! 1. A successful response means the write was applied.
//! 2. A read includes every write applied at least [`MAX_READ_LAG`] before the read started.
//! 3. Writes are applied in the order they're sent. The service sends them one at a time, in the
//!    order the user tapped.
//! 4. A write the device gave up on, if the server applies it at all, applies within
//!    [`MAX_READ_LAG`] of the device giving up.
//!
//! So a key's newest write is what the server ends up holding, and it decides the key's state until
//! a refresh that started late enough to include it has been applied: [`MAX_READ_LAG`] after the
//! write was answered, or twice that after the device gave up on it. Time passing changes nothing on
//! its own: only a tap, a response or a refresh does. A post is saved if any of the keys it may be
//! saved under is.
//!
//! Only the saved keys and this device's writes say whether a post is saved. The saved list
//! doesn't: it doesn't say which key a post is saved under.
//!
//! Known limits:
//!
//! - A write the device gave up on can still be applied after a later tap's write, so the server can
//!   end up opposite to the user's last tap until they tap again.
//! - The saved keys stop at 1,000. Past that, which only accounts that saved more before the server's
//!   cap can be, a post the keys don't have reads [`ReaderSavedStatus::Unknown`] unless this device
//!   has had a write to it answered. A post in the saved list that reads `Unknown` is saved.
//! - A post leaves [`ReaderSavedState::recent_saves`] once the saved list shows it, or once a
//!   refresh finds it unsaved with no write left to it. A later read of the list that lags behind
//!   can leave it in neither, until the next read.
//! - The count is the newest total the server gave, adjusted for this device's writes since. A
//!   refresh that lags can be off until the next one: by saves made elsewhere, and past the 1,000
//!   keys by this device's writes to posts the keys don't have. A save given up on,
//!   of a key the server stores as one this device doesn't know about yet (a feed item stored as
//!   its post), can count twice until a refresh includes it.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use wp_api::wp_com::reader_saved::ReaderSavedKey;

/// How far behind the server's writes a read can be.
pub(super) const MAX_READ_LAG: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct WriteId(u64);

/// A tap's place in line. A tap's writes are applied after every earlier tap's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Tap(u64);

/// A refresh in flight. Ordered by when it started.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct RefreshTicket {
    started: Instant,
    id: u64,
}

/// Whether a post is saved, as far as this device knows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ReaderSavedStatus {
    Saved,
    NotSaved,
    /// The saved keys haven't loaded, or the post is older than the 1,000 keys they stop at, and
    /// this device hasn't changed it.
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Progress {
    /// Queued or in flight.
    Pending,
    /// The server answered.
    Answered { at: Instant },
    /// Given up on after it was sent, so the server may or may not apply it.
    Abandoned { at: Instant },
}

#[derive(Debug)]
struct Write {
    tap: Tap,
    id: WriteId,
    /// The key sent, or the key the server stored once it answered.
    key: ReaderSavedKey,
    saved: bool,
    progress: Progress,
}

impl Write {
    /// Whether a read that started at `started` includes this write, if the server applied it.
    fn included_in(&self, started: Instant) -> bool {
        self.included_from().is_some_and(|from| from <= started)
    }

    /// When reads start to include this write, if the server applied it.
    fn included_from(&self) -> Option<Instant> {
        match self.progress {
            Progress::Pending => None,
            Progress::Answered { at } => Some(at + MAX_READ_LAG),
            Progress::Abandoned { at } => Some(at + 2 * MAX_READ_LAG),
        }
    }

    fn is_answered(&self) -> bool {
        matches!(self.progress, Progress::Answered { .. })
    }

    fn order(&self) -> (Tap, WriteId) {
        (self.tap, self.id)
    }
}

#[derive(Debug)]
struct Snapshot {
    ticket: RefreshTicket,
    /// The server's keys, normalized.
    keys: HashSet<ReaderSavedKey>,
    /// The same keys, as the server stores them.
    stored: HashSet<ReaderSavedKey>,
    /// How many keys the server returned, before any normalized to the same key.
    returned: u64,
    total: u64,
}

impl Snapshot {
    /// Whether there are saves beyond the keys, which stop at 1,000.
    fn past_the_keys(&self) -> bool {
        self.total > self.returned
    }
}

/// The server's total in a write's answer. Writes are answered in the order they're applied, so
/// it includes every write before that one too.
#[derive(Debug, Clone, Copy)]
struct AnsweredTotal {
    total: u64,
    at: Instant,
    /// The newest write the total includes.
    includes: (Tap, WriteId),
}

#[derive(Debug, Default)]
pub(super) struct ReaderSavedState {
    /// Writes that may still decide their key's state, in the order the server applies them.
    writes: Vec<Write>,
    /// The newest refresh applied.
    snapshot: Option<Snapshot>,
    /// Keys saved by a tap here, oldest first, until the saved list shows them.
    recent: Vec<ReaderSavedKey>,
    /// Keys the server stores as another key, as it last said: a URL of a WordPress.com post is
    /// stored as the post.
    stored_as: HashMap<ReaderSavedKey, ReaderSavedKey>,
    /// The server's total in the newest write answer.
    answered_total: Option<AnsweredTotal>,
    /// Keys whose unsave a refresh included and pruned, and when reads start to include it. A read
    /// of the saved list that started before then may still predate the unsave.
    unsaved_before: HashMap<ReaderSavedKey, Instant>,
    next_tap: u64,
    next_write_id: u64,
    next_refresh_id: u64,
    /// Bumped each time a refresh is applied, so the service only stores the keys when they change.
    snapshot_generation: u64,
}

impl ReaderSavedState {
    /// Takes the next place in line for a tap's writes.
    pub(super) fn begin_tap(&mut self) -> Tap {
        let tap = Tap(self.next_tap);
        self.next_tap += 1;
        tap
    }

    /// Records a write about to be sent, which takes effect straight away.
    pub(super) fn begin_write(&mut self, tap: Tap, key: &ReaderSavedKey, saved: bool) -> WriteId {
        let id = WriteId(self.next_write_id);
        self.next_write_id += 1;
        let write = Write {
            tap,
            id,
            key: key.clone(),
            saved,
            progress: Progress::Pending,
        };
        let index = self
            .writes
            .partition_point(|other| other.order() < write.order());
        self.writes.insert(index, write);
        id
    }

    /// Records that the user saved a key with a tap, so it shows in recent saves until the saved
    /// list shows it.
    pub(super) fn note_save_tap(&mut self, key: &ReaderSavedKey) {
        let stored = self.stored_as_of(key);
        let stored_as = &self.stored_as;
        self.recent
            .retain(|other| stored_key_in(stored_as, other) != stored);
        self.recent.push(key.clone());
    }

    /// Records that the server applied a write. `stored_key` is the key it stored, which can differ
    /// from the one sent, and `total` the number of saves it reported.
    pub(super) fn complete_write(
        &mut self,
        id: WriteId,
        stored_key: &ReaderSavedKey,
        total: Option<u64>,
        now: Instant,
    ) {
        let Some(write) = self.pending(id) else {
            return;
        };
        let requested = write.key.normalized();
        let stored = stored_key.normalized();
        let saved = write.saved;
        let order = write.order();
        write.key = stored_key.clone();
        write.progress = Progress::Answered { at: now };
        if let Some(total) = total {
            self.answered_total = Some(AnsweredTotal {
                total,
                at: now,
                includes: order,
            });
        }

        // What a save stores now replaces what the server said before, so a URL whose post was
        // deleted stops being looked up as the post. An unsave echoes the key it was sent when
        // nothing was saved under it, which says nothing about where the post is stored.
        if requested != stored {
            self.stored_as.insert(requested.clone(), stored);
            self.restore_snapshot_keys();
        } else if saved && self.stored_as.remove(&requested).is_some() {
            self.restore_snapshot_keys();
        }
        if saved {
            for key in &mut self.recent {
                if key.normalized() == requested {
                    *key = stored_key.clone();
                }
            }
        }
    }

    /// Records that a write was given up on after it was sent. The server may still apply it, so it
    /// stands until a read can say.
    pub(super) fn abandon_write(&mut self, id: WriteId, now: Instant) {
        if let Some(write) = self.pending(id) {
            write.progress = Progress::Abandoned { at: now };
        }
    }

    /// Undoes a write the server refused, or that was never sent. A save undone is no longer a
    /// recent save, unless another save of the key still stands.
    pub(super) fn fail_write(&mut self, id: WriteId) {
        let Some(write) = self.pending(id) else {
            return;
        };
        let undone_save = write.saved.then(|| write.key.clone());
        let undone_save = undone_save.map(|key| self.stored_as_of(&key));
        self.writes.retain(|write| write.id != id);
        if let Some(key) = undone_save {
            let stored_as = &self.stored_as;
            let still_saved = self
                .writes
                .iter()
                .any(|write| write.saved && stored_key_in(stored_as, &write.key) == key);
            if !still_saved {
                self.recent
                    .retain(|other| stored_key_in(stored_as, other) != key);
            }
        }
    }

    pub(super) fn begin_refresh(&mut self, now: Instant) -> RefreshTicket {
        let ticket = RefreshTicket {
            started: now,
            id: self.next_refresh_id,
        };
        self.next_refresh_id += 1;
        ticket
    }

    /// Replaces the server's keys, unless a refresh that started later has already been applied.
    /// Each key comes with the URL of a post saved by its IDs, if it has one.
    pub(super) fn apply_refresh(
        &mut self,
        ticket: RefreshTicket,
        keys: impl IntoIterator<Item = (ReaderSavedKey, Option<String>)>,
        total: u64,
    ) {
        if self
            .snapshot
            .as_ref()
            .is_some_and(|snapshot| ticket < snapshot.ticket)
        {
            return;
        }

        let mut saved = HashSet::new();
        let mut returned = 0;
        for (key, url) in keys {
            returned += 1;
            let key = key.normalized();
            if let Some(url) = url.filter(|_| !matches!(key, ReaderSavedKey::Url { .. })) {
                let url = ReaderSavedKey::Url { url }.normalized();
                self.stored_as.insert(url, key.clone());
            }
            saved.insert(key);
        }
        self.snapshot_generation += 1;
        self.snapshot = Some(Snapshot {
            ticket,
            keys: saved,
            stored: HashSet::new(),
            returned,
            total,
        });
        self.restore_snapshot_keys();

        // The refresh includes each key's newest write that finished long enough before it started,
        // and with it every older write to the same key. A post's writes go together, so while it
        // has an older write to another of its keys that the refresh may not include, they all
        // stay. Past the 1,000 keys, the refresh can't see a key it doesn't have, so an answered
        // write to one stays: the server applied it.
        let snapshot = self.snapshot.as_ref().expect("just set");
        let stored_as = &self.stored_as;
        let mut included: HashMap<ReaderSavedKey, usize> = HashMap::new();
        for (index, write) in self.writes.iter().enumerate().rev() {
            if write.included_in(ticket.started) {
                included
                    .entry(stored_key_in(stored_as, &write.key))
                    .or_insert(index);
            }
        }
        included.retain(|post, newest| {
            let key = self.writes[*newest].key.normalized();
            !self.writes[..*newest].iter().any(|write| {
                stored_key_in(stored_as, &write.key) == *post
                    && write.key.normalized() != key
                    && !write.included_in(ticket.started)
            })
        });
        // A post unsaved by its newest write stays hidden from list reads that may predate any of
        // the unsaves pruned with it.
        for (key, &newest) in &included {
            if self.writes[newest].saved {
                continue;
            }
            let from = self.writes[..=newest]
                .iter()
                .filter(|write| !write.saved && stored_key_in(stored_as, &write.key) == *key)
                .filter_map(Write::included_from)
                .max();
            if let Some(from) = from {
                let before = self.unsaved_before.entry(key.clone()).or_insert(from);
                *before = (*before).max(from);
            }
        }
        let mut index = 0;
        self.writes.retain(|write| {
            let this = index;
            index += 1;
            let key = stored_key_in(stored_as, &write.key);
            let Some(&newest) = included.get(&key) else {
                return true;
            };
            this > newest
                || (this == newest
                    && write.is_answered()
                    && snapshot.past_the_keys()
                    && !snapshot.stored.contains(&key))
        });

        // A recent save that reads as unsaved, with no write left that could change that, never
        // shows again: a new save tap adds it back. Without this, every post saved here and later
        // unsaved would stay.
        let recent = std::mem::take(&mut self.recent);
        self.recent = recent
            .into_iter()
            .filter(|key| {
                let post = self.stored_as_of(key);
                self.status(std::slice::from_ref(key)) != ReaderSavedStatus::NotSaved
                    || self
                        .writes
                        .iter()
                        .any(|write| self.stored_as_of(&write.key) == post)
            })
            .collect();

        // Where a key is stored only matters while the key, or what it's stored as, is saved or
        // has a write or recent save. Without this, the map keeps every post ever saved under
        // another key. Past the 1,000 keys, a saved post can be missing from them, so all stay.
        let snapshot = self.snapshot.as_ref().expect("just set");
        if !snapshot.past_the_keys() {
            let used: HashSet<ReaderSavedKey> = self
                .writes
                .iter()
                .map(|write| &write.key)
                .chain(&self.recent)
                .map(ReaderSavedKey::normalized)
                .collect();
            self.stored_as.retain(|key, stored| {
                [key.normalized(), stored.normalized()]
                    .iter()
                    .any(|key| snapshot.keys.contains(key) || used.contains(key))
            });
        }

        // A refresh that started after a write's answer gives the count instead: its total agrees
        // with its own keys, which may already have writes sent after that answer.
        if self
            .answered_total
            .is_some_and(|answered| answered.at < ticket.started)
        {
            self.answered_total = None;
        }
    }

    /// Whether a post is saved, given every key it may be saved under.
    pub(super) fn status(&self, keys: &[ReaderSavedKey]) -> ReaderSavedStatus {
        let states: Vec<_> = self
            .post_keys(keys)
            .iter()
            .map(|key| self.is_saved(key))
            .collect();
        if states.contains(&Some(true)) {
            ReaderSavedStatus::Saved
        } else if states.contains(&None) {
            ReaderSavedStatus::Unknown
        } else {
            ReaderSavedStatus::NotSaved
        }
    }

    /// The keys to send to unsave a post: every key it may be saved under, and the keys the server
    /// stores them as. Sending one the post isn't saved under does nothing, while missing one
    /// leaves the post saved.
    pub(super) fn keys_to_unsave(&self, keys: &[ReaderSavedKey]) -> Vec<ReaderSavedKey> {
        let mut sent: Vec<ReaderSavedKey> = Vec::new();
        for key in keys {
            for key in [key.clone(), self.stored_as_of(key)] {
                if !sent
                    .iter()
                    .any(|other| other.normalized() == key.normalized())
                {
                    sent.push(key);
                }
            }
        }
        sent
    }

    /// Whether this device has seen the server hold a save under this exact key: as the answer to
    /// its newest answered write to it, or else in the keys it last read.
    pub(super) fn known_saved_under(&self, key: &ReaderSavedKey) -> bool {
        let key = key.normalized();
        match self
            .writes
            .iter()
            .rev()
            .find(|write| write.is_answered() && write.key.normalized() == key)
        {
            Some(write) => write.saved,
            None => self
                .snapshot
                .as_ref()
                .is_some_and(|snapshot| snapshot.keys.contains(&key)),
        }
    }

    /// Whether to show a post from a read of the saved list that started at `started`: not if this
    /// device unsaved it, unless the read is new enough to include every key's unsave.
    pub(super) fn should_show_listed_post(
        &self,
        keys: &[ReaderSavedKey],
        started: Instant,
    ) -> bool {
        let post = self.post_keys(keys);
        let unsaved_since = self.writes.iter().any(|write| {
            !write.saved
                && !write.included_in(started)
                && post.contains(&self.stored_as_of(&write.key))
        }) || post.iter().any(|key| {
            self.unsaved_before
                .get(key)
                .is_some_and(|from| started < *from)
        });
        !unsaved_since || self.status(keys) == ReaderSavedStatus::Saved
    }

    /// Records that the saved list showed a post, so it no longer needs showing as a recent save.
    /// Returns whether that changed recent saves.
    pub(super) fn note_listed_post(&mut self, keys: &[ReaderSavedKey]) -> bool {
        let post = self.post_keys(keys);
        let stored_as = &self.stored_as;
        let before = self.recent.len();
        self.recent
            .retain(|key| !post.contains(&stored_key_in(stored_as, key)));
        self.recent.len() != before
    }

    /// How many saves there are, or `None` before the first refresh or answered write. Like the
    /// server's total, a post saved under two keys counts twice.
    pub(super) fn saved_count(&self) -> Option<u64> {
        // The newest total the server reported: a write's, unless a refresh started after the answer.
        let (mut count, includes) = match (self.answered_total, &self.snapshot) {
            (Some(answered), _) => (answered.total as i64, Some(answered.includes)),
            (None, Some(snapshot)) => (snapshot.total as i64, None),
            (None, None) => return None,
        };

        // Each key's newest write the total doesn't include, against what the total counted for
        // that key: the key's newest write it does include, or else what the keys had. Past the
        // 1,000 keys, a key they don't have can't be compared.
        let after = |write: &Write| includes.is_none_or(|includes| write.order() > includes);
        let mut seen: HashSet<ReaderSavedKey> = HashSet::new();
        for write in self.writes.iter().rev().filter(|write| after(write)) {
            let key = self.counted_key(write);
            if !seen.insert(key.clone()) {
                continue;
            }
            let counted = self
                .writes
                .iter()
                .rev()
                .filter(|other| !after(other))
                .find(|other| self.counted_key(other) == key)
                .map(|other| other.saved)
                .or_else(|| match &self.snapshot {
                    None => Some(false),
                    Some(snapshot) if snapshot.keys.contains(&key) => Some(true),
                    Some(snapshot) if snapshot.past_the_keys() => None,
                    Some(_) => Some(false),
                });
            if let Some(counted) = counted {
                count += i64::from(write.saved) - i64::from(counted);
            }
        }
        Some(count.max(0) as u64)
    }

    /// The posts saved by a tap here that read as saved and that the saved list hasn't shown yet,
    /// newest first.
    pub(super) fn recent_saves(&self) -> Vec<ReaderSavedKey> {
        let mut seen = HashSet::new();
        self.recent
            .iter()
            .rev()
            .filter(|key| seen.insert(self.stored_as_of(key)))
            .filter(|key| self.status(std::slice::from_ref(key)) == ReaderSavedStatus::Saved)
            .cloned()
            .collect()
    }

    /// The keys a post's saves are stored under, main key first: its own keys, normalized, or the
    /// keys the server stores them as.
    pub(super) fn post_keys(&self, keys: &[ReaderSavedKey]) -> Vec<ReaderSavedKey> {
        let mut all = Vec::new();
        for key in keys {
            let key = self.stored_as_of(key);
            if !all.contains(&key) {
                all.push(key);
            }
        }
        all
    }

    /// The key a write counts under, as the server's total counts it. A save not yet answered counts
    /// under the key the server will store it as, if the device knows it. Any other write counts
    /// under its own key: an answer gives the key stored, and an unsave of a key stored as another
    /// can still remove an older save under the key itself.
    fn counted_key(&self, write: &Write) -> ReaderSavedKey {
        if write.saved && !write.is_answered() {
            self.stored_as_of(&write.key)
        } else {
            write.key.normalized()
        }
    }

    /// The key the server stores a key as, normalized.
    fn stored_as_of(&self, key: &ReaderSavedKey) -> ReaderSavedKey {
        stored_key_in(&self.stored_as, key)
    }

    /// Recomputes the snapshot's keys as stored, after what keys are stored as changed.
    fn restore_snapshot_keys(&mut self) {
        let stored_as = &self.stored_as;
        if let Some(snapshot) = &mut self.snapshot {
            snapshot.stored = snapshot
                .keys
                .iter()
                .map(|key| stored_key_in(stored_as, key))
                .collect();
        }
    }

    fn pending(&mut self, id: WriteId) -> Option<&mut Write> {
        self.writes
            .iter_mut()
            .find(|write| write.id == id && write.progress == Progress::Pending)
    }

    fn newest_write(&self, key: &ReaderSavedKey) -> Option<&Write> {
        self.writes
            .iter()
            .rev()
            .find(|write| self.stored_as_of(&write.key) == *key)
    }

    fn is_saved(&self, key: &ReaderSavedKey) -> Option<bool> {
        match self.newest_write(key) {
            Some(write) => Some(write.saved),
            None => self.server_has(key),
        }
    }

    /// What the server held when it was last read: `None` before the first refresh, and for a key
    /// past the 1,000 keys.
    fn server_has(&self, key: &ReaderSavedKey) -> Option<bool> {
        let snapshot = self.snapshot.as_ref()?;
        if snapshot.stored.contains(key) {
            Some(true)
        } else if snapshot.past_the_keys() {
            None
        } else {
            Some(false)
        }
    }
}

/// Follows what keys are stored as, normalized. A refresh can map a URL to a feed item and a write
/// the feed item to a post, so it follows a chain, and stops at a loop.
fn stored_key_in(
    stored_as: &HashMap<ReaderSavedKey, ReaderSavedKey>,
    key: &ReaderSavedKey,
) -> ReaderSavedKey {
    let mut key = key.normalized();
    for _ in 0..4 {
        match stored_as.get(&key) {
            Some(next) if *next != key => key = next.clone(),
            _ => break,
        }
    }
    key
}

// Keeping the state across launches.

/// How far from now a stored time can be when it's loaded. The rules look back at most twice
/// [`MAX_READ_LAG`], so anything older behaves the same as this, and a wall clock that moved can't
/// put a time far in the future.
const LOAD_HORIZON: Duration = Duration::from_secs(3 * MAX_READ_LAG.as_secs());

/// The version of [`PersistedLocal`]. One stored in another version isn't loaded, which only costs
/// the protection of writes made just before the app quit.
pub(super) const PERSISTED_FORMAT: u32 = 1;

/// The time on both clocks: the state compares monotonic `Instant`s, which don't survive a
/// relaunch, so stored times are wall clock.
#[derive(Debug, Clone, Copy)]
pub(super) struct Now {
    pub(super) instant: Instant,
    pub(super) wall: SystemTime,
}

impl Now {
    fn wall_ms(&self, instant: Instant) -> i64 {
        let wall = unix_ms(self.wall);
        if instant <= self.instant {
            wall - millis(self.instant - instant)
        } else {
            wall + millis(instant - self.instant)
        }
    }

    /// The `Instant` for a stored wall clock time, no further from now than [`LOAD_HORIZON`]. A
    /// time this device's clock can't go back to, early after a reboot, loads as now: that only
    /// keeps a write deciding its key for longer, which is the safe direction.
    fn instant(&self, wall_ms: i64) -> Instant {
        let horizon = millis(LOAD_HORIZON);
        let offset = (wall_ms - unix_ms(self.wall)).clamp(-horizon, horizon);
        let by = Duration::from_millis(offset.unsigned_abs());
        if offset < 0 {
            self.instant.checked_sub(by).unwrap_or(self.instant)
        } else {
            self.instant + by
        }
    }
}

fn millis(duration: Duration) -> i64 {
    i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
}

fn unix_ms(time: SystemTime) -> i64 {
    match time.duration_since(UNIX_EPOCH) {
        Ok(since) => millis(since),
        Err(before) => -millis(before.duration()),
    }
}

/// Everything but the server's keys: this device's writes and what it learned from them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(super) struct PersistedLocal {
    pub(super) format: u32,
    writes: Vec<PersistedWrite>,
    recent: Vec<ReaderSavedKey>,
    stored_as: Vec<(ReaderSavedKey, ReaderSavedKey)>,
    answered_total: Option<PersistedAnsweredTotal>,
    unsaved_before: Vec<(ReaderSavedKey, i64)>,
    next_tap: u64,
    next_write_id: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct PersistedWrite {
    tap: u64,
    id: u64,
    key: ReaderSavedKey,
    saved: bool,
    progress: PersistedProgress,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum PersistedProgress {
    Pending,
    Answered { at: i64 },
    Abandoned { at: i64 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct PersistedAnsweredTotal {
    total: u64,
    at: i64,
    tap: u64,
    id: u64,
}

/// The server's keys from the newest refresh applied, as the cache stores them.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct PersistedSnapshot {
    /// When the refresh started, in milliseconds since the Unix epoch.
    pub(super) started: i64,
    /// Each key, with the URL of a post saved by its IDs when the server sent one.
    pub(super) keys: Vec<(ReaderSavedKey, Option<String>)>,
    pub(super) returned: u64,
    pub(super) total: u64,
}

impl ReaderSavedState {
    pub(super) fn snapshot_generation(&self) -> u64 {
        self.snapshot_generation
    }

    pub(super) fn persisted_local(&self, now: Now) -> PersistedLocal {
        PersistedLocal {
            format: PERSISTED_FORMAT,
            writes: self
                .writes
                .iter()
                .map(|write| PersistedWrite {
                    tap: write.tap.0,
                    id: write.id.0,
                    key: write.key.clone(),
                    saved: write.saved,
                    progress: match write.progress {
                        Progress::Pending => PersistedProgress::Pending,
                        Progress::Answered { at } => PersistedProgress::Answered {
                            at: now.wall_ms(at),
                        },
                        Progress::Abandoned { at } => PersistedProgress::Abandoned {
                            at: now.wall_ms(at),
                        },
                    },
                })
                .collect(),
            recent: self.recent.clone(),
            stored_as: self
                .stored_as
                .iter()
                .map(|(key, stored)| (key.clone(), stored.clone()))
                .collect(),
            answered_total: self.answered_total.map(|answered| PersistedAnsweredTotal {
                total: answered.total,
                at: now.wall_ms(answered.at),
                tap: answered.includes.0.0,
                id: answered.includes.1.0,
            }),
            unsaved_before: self
                .unsaved_before
                .iter()
                .map(|(key, from)| (key.clone(), now.wall_ms(*from)))
                .collect(),
            next_tap: self.next_tap,
            next_write_id: self.next_write_id,
        }
    }

    /// When the newest refresh applied started, in milliseconds since the Unix epoch.
    pub(super) fn snapshot_started_ms(&self, now: Now) -> Option<i64> {
        self.snapshot
            .as_ref()
            .map(|snapshot| now.wall_ms(snapshot.ticket.started))
    }

    /// The state as it was stored. A write that hadn't been answered was cut off by the app quitting
    /// after it may have been sent, so it's given up on now, as a cancelled one is.
    pub(super) fn restore(
        local: Option<PersistedLocal>,
        snapshot: Option<PersistedSnapshot>,
        now: Now,
    ) -> Self {
        let mut state = Self::default();
        if let Some(snapshot) = snapshot {
            // As `apply_refresh` does. What this device's writes learned since replaces it below.
            for (key, url) in &snapshot.keys {
                if let Some(url) = url
                    .clone()
                    .filter(|_| !matches!(key, ReaderSavedKey::Url { .. }))
                {
                    let url = ReaderSavedKey::Url { url }.normalized();
                    state.stored_as.insert(url, key.normalized());
                }
            }
            state.snapshot = Some(Snapshot {
                ticket: RefreshTicket {
                    started: now.instant(snapshot.started).min(now.instant),
                    id: 0,
                },
                keys: snapshot
                    .keys
                    .into_iter()
                    .map(|(key, _)| key.normalized())
                    .collect(),
                stored: HashSet::new(),
                returned: snapshot.returned,
                total: snapshot.total,
            });
            state.next_refresh_id = 1;
            state.snapshot_generation = 1;
        }
        if let Some(local) = local {
            state.writes = local
                .writes
                .into_iter()
                .map(|write| Write {
                    tap: Tap(write.tap),
                    id: WriteId(write.id),
                    key: write.key,
                    saved: write.saved,
                    progress: match write.progress {
                        PersistedProgress::Pending => Progress::Abandoned { at: now.instant },
                        PersistedProgress::Answered { at } => Progress::Answered {
                            at: now.instant(at),
                        },
                        PersistedProgress::Abandoned { at } => Progress::Abandoned {
                            at: now.instant(at),
                        },
                    },
                })
                .collect();
            state.writes.sort_by_key(Write::order);
            state.recent = local.recent;
            state.stored_as.extend(local.stored_as);
            state.answered_total = local.answered_total.map(|answered| AnsweredTotal {
                total: answered.total,
                at: now.instant(answered.at),
                includes: (Tap(answered.tap), WriteId(answered.id)),
            });
            state.unsaved_before = local
                .unsaved_before
                .into_iter()
                .map(|(key, from)| (key, now.instant(from)))
                .collect();
            let last = state.writes.iter().map(Write::order).max();
            state.next_tap = local.next_tap.max(last.map_or(0, |(tap, _)| tap.0 + 1));
            state.next_write_id = local.next_write_id.max(last.map_or(0, |(_, id)| id.0 + 1));
        }
        state.restore_snapshot_keys();
        state
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;

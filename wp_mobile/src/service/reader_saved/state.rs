//! The saved posts this device knows about, without any networking.
//!
//! The saved posts API's reads can lag behind its writes. Rather than guess what the server holds,
//! this relies on three facts about it, and nothing else:
//!
//! 1. A successful response means the write was applied.
//! 2. A read includes every write applied at least [`MAX_READ_LAG`] before the read started.
//! 3. Writes to one post are applied in the order they were sent. The service makes this true by
//!    sending them one at a time, in the order the user made them.
//!
//! So the newest write to a key is what the server holds once it succeeds, and it decides the key's
//! state until a read that started [`MAX_READ_LAG`] after it finished has been applied. Time passing
//! changes nothing on its own: only a tap, a response or a read does. A post is saved if any of the
//! keys it may be saved under is.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use wp_api::wp_com::reader_saved::ReaderSavedKey;

/// How far behind the server's writes a read can be.
pub(super) const MAX_READ_LAG: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct WriteId(u64);

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

#[derive(Debug)]
struct Write {
    id: WriteId,
    /// The key sent, or the key the server stored once it answered.
    key: ReaderSavedKey,
    saved: bool,
    /// When the server answered. `None` while the write is queued or in flight.
    done_at: Option<Instant>,
}

#[derive(Debug)]
struct Snapshot {
    ticket: RefreshTicket,
    /// The server's keys, normalized.
    keys: HashSet<ReaderSavedKey>,
    total: u64,
}

impl Snapshot {
    /// How many saves are beyond the keys, which stop at 1,000.
    fn beyond_keys(&self) -> u64 {
        self.total.saturating_sub(self.keys.len() as u64)
    }
}

/// A post the saved list showed, and when that read started. The list doesn't say which of the
/// post's keys it's saved under.
#[derive(Debug)]
struct Listed {
    post: Vec<ReaderSavedKey>,
    started: Instant,
}

#[derive(Debug, Default)]
pub(super) struct ReaderSavedState {
    /// Writes that may still decide their key's state, in the order they were made.
    writes: Vec<Write>,
    /// The newest refresh applied.
    snapshot: Option<Snapshot>,
    /// Posts the saved list showed since the snapshot's refresh started.
    listed: Vec<Listed>,
    /// Keys the server stores as another key, as it last said: a URL of a WordPress.com post is
    /// stored as the post.
    stored_as: HashMap<ReaderSavedKey, ReaderSavedKey>,
    next_write_id: u64,
    next_refresh_id: u64,
}

impl ReaderSavedState {
    /// The keys a post's saves are stored under: its own keys, normalized, or the keys the server
    /// stores them as.
    pub(super) fn post_keys(&self, keys: &[ReaderSavedKey]) -> Vec<ReaderSavedKey> {
        let mut all = Vec::new();
        for key in keys {
            let key = self.stored_key(key);
            if !all.contains(&key) {
                all.push(key);
            }
        }
        all
    }

    /// The keys a write to these keys must wait its turn on: each key, normalized, and the key the
    /// server stores it as. An earlier write may have been queued before that was known.
    pub(super) fn lock_keys(&self, keys: &[ReaderSavedKey]) -> Vec<ReaderSavedKey> {
        let mut all = Vec::new();
        for key in keys {
            for key in [key.normalized(), self.stored_key(key)] {
                if !all.contains(&key) {
                    all.push(key);
                }
            }
        }
        all
    }

    /// The key the server stores a key as, normalized.
    fn stored_key(&self, key: &ReaderSavedKey) -> ReaderSavedKey {
        let key = key.normalized();
        self.stored_as.get(&key).cloned().unwrap_or(key)
    }

    fn is_for(&self, write: &Write, key: &ReaderSavedKey) -> bool {
        self.stored_key(&write.key) == *key
    }

    fn snapshot_has(&self, snapshot: &Snapshot, key: &ReaderSavedKey) -> bool {
        snapshot.keys.contains(key)
            || (!self.stored_as.is_empty()
                && snapshot
                    .keys
                    .iter()
                    .any(|saved| self.stored_key(saved) == *key))
    }

    /// Records a write about to be sent, which takes effect straight away.
    pub(super) fn begin_write(&mut self, key: &ReaderSavedKey, saved: bool) -> WriteId {
        let id = WriteId(self.next_write_id);
        self.next_write_id += 1;
        self.writes.push(Write {
            id,
            key: key.clone(),
            saved,
            done_at: None,
        });
        id
    }

    /// Like [`Self::begin_write`], for a write that belongs to the same tap as `with`, so it takes
    /// that tap's place among the writes rather than coming after newer taps.
    pub(super) fn begin_write_with(
        &mut self,
        with: WriteId,
        key: &ReaderSavedKey,
        saved: bool,
    ) -> WriteId {
        let id = self.begin_write(key, saved);
        let write = self.writes.pop().expect("just added");
        let index = self
            .writes
            .iter()
            .position(|other| other.id == with)
            .map_or(self.writes.len(), |index| index + 1);
        self.writes.insert(index, write);
        id
    }

    /// Records that the server applied a write. `stored_key` is the key it stored, which can differ
    /// from the one sent.
    pub(super) fn complete_write(
        &mut self,
        id: WriteId,
        stored_key: &ReaderSavedKey,
        now: Instant,
    ) {
        let Some(write) = self.in_flight(id) else {
            return;
        };
        let requested = write.key.normalized();
        let stored = stored_key.normalized();
        write.key = stored_key.clone();
        write.done_at = Some(now);

        // What the server says now replaces what it said before, so a URL whose post was deleted
        // stops being looked up as the post.
        if requested == stored {
            self.stored_as.remove(&requested);
        } else {
            self.stored_as.insert(requested, stored);
        }
    }

    /// Undoes a write the server refused, or that was cancelled.
    pub(super) fn fail_write(&mut self, id: WriteId) {
        if self.in_flight(id).is_some() {
            self.writes.retain(|write| write.id != id);
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
        for (key, url) in keys {
            let key = key.normalized();
            if let Some(url) = url.filter(|_| !matches!(key, ReaderSavedKey::Url { .. })) {
                let url = ReaderSavedKey::Url { url }.normalized();
                self.stored_as.insert(url, key.clone());
            }
            saved.insert(key);
        }
        self.snapshot = Some(Snapshot {
            ticket,
            keys: saved,
            total,
        });

        // This read includes every write that finished `MAX_READ_LAG` before it started, and every
        // post the saved list showed before it started.
        self.writes.retain(|write| {
            write
                .done_at
                .is_none_or(|done_at| done_at + MAX_READ_LAG > ticket.started)
        });
        self.listed.retain(|listed| listed.started > ticket.started);
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

    /// The keys to send to unsave a post: every key it may be saved under. Sending one the post
    /// isn't saved under does nothing, while missing one leaves the post saved.
    pub(super) fn keys_to_unsave(&self, keys: &[ReaderSavedKey]) -> Vec<ReaderSavedKey> {
        let mut sent: Vec<ReaderSavedKey> = Vec::new();
        for key in keys {
            let stored = self.stored_as.get(&key.normalized()).cloned();
            for key in std::iter::once(key.clone()).chain(stored) {
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

    /// Whether this device has seen the server hold a save under this exact key: as the result of
    /// its newest finished write to it, or else in the keys it last read.
    pub(super) fn known_saved_under(&self, key: &ReaderSavedKey) -> bool {
        let key = key.normalized();
        match self
            .writes
            .iter()
            .rev()
            .find(|write| write.done_at.is_some() && write.key.normalized() == key)
        {
            Some(write) => write.saved,
            None => self
                .snapshot
                .as_ref()
                .is_some_and(|snapshot| snapshot.keys.contains(&key)),
        }
    }

    /// Whether to show a post from the saved list: not if this device unsaved it since.
    pub(super) fn should_show_listed_post(&self, keys: &[ReaderSavedKey]) -> bool {
        let post = self.post_keys(keys);
        let unsaved_here = post
            .iter()
            .any(|key| self.deciding_write(key).is_some_and(|write| !write.saved));
        !unsaved_here || self.status(keys) == ReaderSavedStatus::Saved
    }

    /// Records that a read of the saved list that started at `started` showed a post. Returns
    /// whether that changed what this device shows.
    pub(super) fn note_listed_post(&mut self, keys: &[ReaderSavedKey], started: Instant) -> bool {
        let post = self.post_keys(keys);
        if post.is_empty()
            || self
                .snapshot
                .as_ref()
                .is_some_and(|snapshot| snapshot.ticket.started >= started)
            // A slower read that started earlier can't replace a newer one.
            || self
                .listed
                .iter()
                .any(|listed| intersects(&listed.post, &post) && listed.started >= started)
        {
            return false;
        }
        let before = (self.status(&post), self.saved_count());
        self.listed
            .retain(|listed| !intersects(&listed.post, &post));
        self.listed.push(Listed {
            post: post.clone(),
            started,
        });
        before != (self.status(&post), self.saved_count())
    }

    /// How many saves there are, or `None` until the first refresh. Like the server's total, a post
    /// saved under two keys counts twice.
    pub(super) fn saved_count(&self) -> Option<u64> {
        let snapshot = self.snapshot.as_ref()?;
        let mut count = snapshot.total as i64;

        let mut seen: Vec<ReaderSavedKey> = Vec::new();
        for write in self.writes.iter().rev() {
            let key = self.stored_key(&write.key);
            if seen.contains(&key) {
                continue;
            }
            if self.decides(write) {
                count += i64::from(write.saved) - i64::from(self.snapshot_has(snapshot, &key));
            }
            seen.push(key);
        }

        // A listed post the keys don't have yet. Past the 1,000 keys it may already be counted.
        if snapshot.beyond_keys() == 0 {
            for listed in &self.listed {
                let changed_here = listed
                    .post
                    .iter()
                    .any(|key| self.deciding_write(key).is_some());
                if !changed_here
                    && !listed
                        .post
                        .iter()
                        .any(|key| self.snapshot_has(snapshot, key))
                {
                    count += 1;
                }
            }
        }
        Some(count.max(0) as u64)
    }

    /// The keys this device saved and the server may not have read yet, newest first.
    pub(super) fn recent_saves(&self) -> Vec<ReaderSavedKey> {
        let mut seen: Vec<ReaderSavedKey> = Vec::new();
        let mut saves = Vec::new();
        for write in self.writes.iter().rev() {
            let key = self.stored_key(&write.key);
            if seen.contains(&key) {
                continue;
            }
            seen.push(key);
            if write.saved && self.decides(write) {
                saves.push(write.key.clone());
            }
        }
        saves
    }

    fn in_flight(&mut self, id: WriteId) -> Option<&mut Write> {
        self.writes
            .iter_mut()
            .find(|write| write.id == id && write.done_at.is_none())
    }

    fn is_saved(&self, key: &ReaderSavedKey) -> Option<bool> {
        match self.deciding_write(key) {
            Some(write) => Some(write.saved),
            None => self.server_has(key),
        }
    }

    /// The key's newest write, if the server may not have been read since it.
    fn deciding_write(&self, key: &ReaderSavedKey) -> Option<&Write> {
        self.writes
            .iter()
            .rev()
            .find(|write| self.is_for(write, key))
            .filter(|write| self.decides(write))
    }

    fn decides(&self, write: &Write) -> bool {
        let Some(done_at) = write.done_at else {
            return true;
        };
        let key = self.stored_key(&write.key);
        let last_read = self
            .snapshot
            .as_ref()
            .map(|snapshot| snapshot.ticket.started)
            .into_iter()
            .chain(
                self.listed
                    .iter()
                    .filter(|listed| listed.post.contains(&key))
                    .map(|listed| listed.started),
            )
            .max();
        last_read.is_none_or(|read| read < done_at + MAX_READ_LAG)
    }

    /// What the server held when it was last read: `None` before the first refresh, and for a key
    /// beyond the 1,000 keys that the saved list hasn't shown.
    fn server_has(&self, key: &ReaderSavedKey) -> Option<bool> {
        if self.listed.iter().any(|listed| listed.post.contains(key)) {
            return Some(true);
        }
        let snapshot = self.snapshot.as_ref()?;
        if self.snapshot_has(snapshot, key) {
            Some(true)
        } else if snapshot.beyond_keys() > 0 {
            None
        } else {
            Some(false)
        }
    }
}

fn intersects(a: &[ReaderSavedKey], b: &[ReaderSavedKey]) -> bool {
    a.iter().any(|key| b.contains(key))
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;

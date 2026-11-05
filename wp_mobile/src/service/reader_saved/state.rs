//! The saved posts this device knows about, without any networking.
//!
//! The saved posts API's reads can lag behind its writes. Rather than guess what the server holds,
//! this relies on three facts about it, and nothing else:
//!
//! 1. A successful response means the write was applied.
//! 2. A read includes every write applied at least [`MAX_READ_LAG`] before the read started.
//! 3. Writes are applied in the order they're sent. The service sends them one at a time, in the
//!    order the user tapped.
//!
//! So a key's newest write is what the server ends up holding, and it decides the key's state until
//! a read that started [`MAX_READ_LAG`] after the write finished has been applied. Time passing
//! changes nothing on its own: only a tap, a response or a read does. A post is saved if any of the
//! keys it may be saved under is.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
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
    /// neither this device nor the saved list has said.
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Progress {
    /// Queued or in flight.
    Pending,
    /// The server answered.
    Answered { at: Instant },
    /// Cancelled after it was sent, so the server may or may not have applied it.
    Abandoned { at: Instant },
}

#[derive(Debug)]
struct Write {
    tap: Tap,
    id: WriteId,
    /// The key sent, or the key the server stored once it answered.
    key: ReaderSavedKey,
    saved: bool,
    /// Whether it belongs in recent saves: not an import, and not shown by the saved list yet.
    recent: bool,
    progress: Progress,
}

impl Write {
    fn finished_at(&self) -> Option<Instant> {
        match self.progress {
            Progress::Pending => None,
            Progress::Answered { at } | Progress::Abandoned { at } => Some(at),
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
    /// The post's keys, as the server stores them.
    post: Vec<ReaderSavedKey>,
    started: Instant,
}

#[derive(Debug, Default)]
pub(super) struct ReaderSavedState {
    /// Writes that may still decide their key's state, in the order the server applies them.
    writes: Vec<Write>,
    /// The newest refresh applied.
    snapshot: Option<Snapshot>,
    /// Posts the saved list showed that the snapshot may not have.
    listed: Vec<Listed>,
    /// Keys the server stores as another key, as it last said: a URL of a WordPress.com post is
    /// stored as the post.
    stored_as: HashMap<ReaderSavedKey, ReaderSavedKey>,
    /// The server's total in the newest write response, and when it arrived.
    answered_total: Option<(u64, Instant)>,
    next_tap: u64,
    next_write_id: u64,
    next_refresh_id: u64,
}

impl ReaderSavedState {
    /// Takes the next place in line for a tap's writes.
    pub(super) fn begin_tap(&mut self) -> Tap {
        let tap = Tap(self.next_tap);
        self.next_tap += 1;
        tap
    }

    /// Records a write about to be sent, which takes effect straight away. `recent` is whether a
    /// save belongs in recent saves: an import's don't.
    pub(super) fn begin_write(
        &mut self,
        tap: Tap,
        key: &ReaderSavedKey,
        saved: bool,
        recent: bool,
    ) -> WriteId {
        let id = WriteId(self.next_write_id);
        self.next_write_id += 1;
        let write = Write {
            tap,
            id,
            key: key.clone(),
            saved,
            recent,
            progress: Progress::Pending,
        };
        let index = self
            .writes
            .partition_point(|other| other.order() < write.order());
        self.writes.insert(index, write);
        id
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
        write.key = stored_key.clone();
        write.progress = Progress::Answered { at: now };
        if let Some(total) = total {
            self.answered_total = Some((total, now));
        }

        // What a save stores now replaces what the server said before, so a URL whose post was
        // deleted stops being looked up as the post. An unsave echoes the key it was sent when
        // nothing was saved under it, which says nothing about where the post is stored.
        if requested != stored {
            self.stored_as.insert(requested, stored);
            self.restore_snapshot_keys();
        } else if saved && self.stored_as.remove(&requested).is_some() {
            self.restore_snapshot_keys();
        }
    }

    /// Records that a write was cancelled after it was sent. The server may have applied it, so it
    /// stands until a read can say.
    pub(super) fn abandon_write(&mut self, id: WriteId, now: Instant) {
        if let Some(write) = self.pending(id) {
            write.progress = Progress::Abandoned { at: now };
        }
    }

    /// Undoes a write the server refused, or that was cancelled before it was sent.
    pub(super) fn fail_write(&mut self, id: WriteId) {
        if self.pending(id).is_some() {
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
            stored: HashSet::new(),
            total,
        });
        self.restore_snapshot_keys();

        // This read includes every write that finished `MAX_READ_LAG` before it started.
        let covers = |at: Instant| at + MAX_READ_LAG <= ticket.started;
        self.writes
            .retain(|write| write.finished_at().is_none_or(|at| !covers(at)));
        if self.answered_total.is_some_and(|(_, at)| covers(at)) {
            self.answered_total = None;
        }

        // And every post the list showed `MAX_READ_LAG` before it started, unless the post is past
        // the keys, where a refresh can't see it.
        let snapshot = self.snapshot.as_ref().expect("just set");
        let past_the_keys = snapshot.beyond_keys() > 0;
        self.listed.retain(|listed| {
            !covers(listed.started)
                || (past_the_keys && !listed.post.iter().any(|key| snapshot.stored.contains(key)))
        });
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
    /// device unsaved it, unless the read is new enough to include the unsave.
    pub(super) fn should_show_listed_post(
        &self,
        keys: &[ReaderSavedKey],
        started: Instant,
    ) -> bool {
        let unsaved_since = self.post_keys(keys).iter().any(|key| {
            self.newest_write(key).is_some_and(|write| {
                !write.saved
                    && write
                        .finished_at()
                        .is_none_or(|at| started < at + MAX_READ_LAG)
            })
        });
        !unsaved_since || self.status(keys) == ReaderSavedStatus::Saved
    }

    /// Records that a read of the saved list that started at `started` showed a post. Returns
    /// whether that changed anything this device shows.
    pub(super) fn note_listed_post(&mut self, keys: &[ReaderSavedKey], started: Instant) -> bool {
        let post = self.post_keys(keys);
        if post.is_empty() {
            return false;
        }
        let status_before = self.status(&post);
        let mut changed = false;

        // The list shows the post, so it doesn't need showing above it as a recent save.
        let stored_as = &self.stored_as;
        for write in &mut self.writes {
            if write.recent && write.saved && post.contains(&stored_key_in(stored_as, &write.key)) {
                write.recent = false;
                changed = true;
            }
        }

        // A refresh that started `MAX_READ_LAG` after this read includes everything it showed.
        let newer_read = self
            .snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.ticket.started >= started + MAX_READ_LAG)
            || self
                .listed
                .iter()
                .any(|listed| intersects(&listed.post, &post) && listed.started >= started);
        if !newer_read {
            // This read includes the post's writes that finished `MAX_READ_LAG` before it started.
            let stored_as = &self.stored_as;
            let writes = self.writes.len();
            self.writes.retain(|write| {
                !post.contains(&stored_key_in(stored_as, &write.key))
                    || write
                        .finished_at()
                        .is_none_or(|at| started < at + MAX_READ_LAG)
            });
            // A post listed for the first time can change the count, if the keys don't have it.
            let newly_listed = !self
                .listed
                .iter()
                .any(|listed| intersects(&listed.post, &post));
            let in_keys = self
                .snapshot
                .as_ref()
                .is_some_and(|snapshot| post.iter().any(|key| snapshot.stored.contains(key)));
            changed |= self.writes.len() != writes || (newly_listed && !in_keys);
            self.listed
                .retain(|listed| !intersects(&listed.post, &post));
            self.listed.push(Listed {
                post: post.clone(),
                started,
            });
        }
        changed || status_before != self.status(&post)
    }

    /// How many saves there are, or `None` before the first refresh or answered write. Like the
    /// server's total, a post saved under two keys counts twice.
    pub(super) fn saved_count(&self) -> Option<u64> {
        // The newest total the server reported: a write's, unless a refresh started late enough to
        // include that write.
        let from_write = self.answered_total.filter(|(_, at)| {
            self.snapshot
                .as_ref()
                .is_none_or(|snapshot| snapshot.ticket.started < *at + MAX_READ_LAG)
        });
        let (mut count, includes_answered) = match (from_write, &self.snapshot) {
            (Some((total, _)), _) => (total as i64, true),
            (None, Some(snapshot)) => (snapshot.total as i64, false),
            (None, None) => return None,
        };

        // Each key's newest write, where the total doesn't include it yet.
        let mut seen: HashSet<ReaderSavedKey> = HashSet::new();
        for write in self.writes.iter().rev() {
            let key = write.key.normalized();
            if !seen.insert(key.clone()) || (includes_answered && write.is_answered()) {
                continue;
            }
            let before = if includes_answered {
                // The total includes the key's newest answered write, or whatever was last read.
                self.writes
                    .iter()
                    .rev()
                    .find(|other| other.is_answered() && other.key.normalized() == key)
                    .map_or_else(|| self.was_saved(&key), |other| other.saved)
            } else {
                self.in_keys_total(&key)
            };
            count += i64::from(write.saved) - i64::from(before);
        }

        // A listed post the keys don't have yet. Past the 1,000 keys it may already be counted.
        if !includes_answered
            && let Some(snapshot) = &self.snapshot
            && snapshot.beyond_keys() == 0
        {
            let written: HashSet<_> = self
                .writes
                .iter()
                .map(|write| self.stored_as_of(&write.key))
                .collect();
            for listed in &self.listed {
                let changed_here = listed.post.iter().any(|key| written.contains(key));
                if !changed_here && !listed.post.iter().any(|key| snapshot.stored.contains(key)) {
                    count += 1;
                }
            }
        }
        Some(count.max(0) as u64)
    }

    /// The posts this device saved that the saved list hasn't shown yet, newest first. Imports
    /// aren't included.
    pub(super) fn recent_saves(&self) -> Vec<ReaderSavedKey> {
        let mut seen: HashSet<ReaderSavedKey> = HashSet::new();
        let mut saves = Vec::new();
        for write in self.writes.iter().rev() {
            if seen.insert(self.stored_as_of(&write.key)) && write.saved && write.recent {
                saves.push(write.key.clone());
            }
        }
        saves
    }

    /// The keys a post's saves are stored under: its own keys, normalized, or the keys the server
    /// stores them as.
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
    /// past the 1,000 keys that the saved list hasn't shown.
    fn server_has(&self, key: &ReaderSavedKey) -> Option<bool> {
        if self.listed.iter().any(|listed| listed.post.contains(key)) {
            return Some(true);
        }
        let snapshot = self.snapshot.as_ref()?;
        if snapshot.stored.contains(key) {
            Some(true)
        } else if snapshot.beyond_keys() > 0 {
            None
        } else {
            Some(false)
        }
    }

    /// Whether the keys' total counts a save under exactly this key: one in the keys, or one the
    /// list showed past them.
    fn in_keys_total(&self, key: &ReaderSavedKey) -> bool {
        let Some(snapshot) = &self.snapshot else {
            return false;
        };
        let stored = self.stored_as_of(key);
        snapshot.keys.contains(key)
            || (snapshot.beyond_keys() > 0
                && self
                    .listed
                    .iter()
                    .any(|listed| listed.post.contains(&stored)))
    }

    /// Whether the last reads had a save under exactly this key, for counting.
    fn was_saved(&self, key: &ReaderSavedKey) -> bool {
        let in_keys = self
            .snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.keys.contains(key));
        let stored = self.stored_as_of(key);
        in_keys
            || self
                .listed
                .iter()
                .any(|listed| listed.post.contains(&stored))
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

fn intersects(a: &[ReaderSavedKey], b: &[ReaderSavedKey]) -> bool {
    a.iter().any(|key| b.contains(key))
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;

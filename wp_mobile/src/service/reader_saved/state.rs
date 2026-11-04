//! The saved posts this device knows about, without any networking.
//!
//! Reads of the saved posts API can lag behind its writes, so a refetch straight after a write can
//! return the state from before it. Each write this device makes is kept as a [`RecentWrite`] on
//! top of the server's list until the server has had time to catch up.
//!
//! Only a key's newest write counts, and only until it expires; after that the server's list
//! decides. Writes can finish in any order, so an older one that finishes after a newer one has
//! already finished is dropped rather than applied.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use wp_api::wp_com::reader_saved::ReaderSavedKey;

/// How long a finished write wins over what the server says.
pub(super) const RECENT_WRITE_TTL: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct WriteId(u64);

/// A refresh in flight. Ordered by when it started.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct RefreshTicket {
    id: u64,
    /// The server's answer can be no newer than this, so a write still in its window at this time
    /// may be missing from it, however long the request then takes.
    started: Instant,
}

/// Whether a post is saved, as far as this device knows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ReaderSavedStatus {
    Saved,
    NotSaved,
    /// The saved keys haven't loaded yet, and this device hasn't changed the post.
    Unknown,
}

#[derive(Debug)]
struct RecentWrite {
    id: WriteId,
    /// The key as sent, or as the server stored it once the write finishes. The map holds it
    /// normalized; this is the spelling to hand back to apps.
    key: ReaderSavedKey,
    saved: bool,
    /// `None` while the request is in flight, so a slow request can't expire before it finishes.
    expires_at: Option<Instant>,
}

impl RecentWrite {
    fn is_live(&self, now: Instant) -> bool {
        self.expires_at.is_none_or(|expires_at| now < expires_at)
    }
}

#[derive(Debug, Default)]
pub(super) struct ReaderSavedState {
    /// The server's keys, normalized, or `None` until the first refresh.
    saved: Option<HashSet<ReaderSavedKey>>,
    /// How many saves the server has beyond `saved`, because the keys stop at 1,000.
    beyond_keys: u64,
    /// Each post's writes, oldest first. Keyed by the post's key, normalized, or by the key the
    /// server stores it under once that's known, so one post's writes are always in one list.
    recent_writes: HashMap<ReaderSavedKey, Vec<RecentWrite>>,
    /// Keys the server stored a save under instead of the one sent, so the post that was saved
    /// still reads as saved under the key it was saved by.
    stored_as: HashMap<ReaderSavedKey, ReaderSavedKey>,
    /// Each post's newest write that has finished, kept after the write itself is dropped, so an
    /// older write that finishes later still knows it was overridden.
    newest_finished: HashMap<ReaderSavedKey, WriteId>,
    /// When the server was last read: the start of the newest refresh applied, and when each key
    /// was last seen in the saved list. A write whose window ended before then is older than what
    /// was read.
    refreshed_at: Option<Instant>,
    listed_at: HashMap<ReaderSavedKey, Instant>,
    next_write_id: u64,
    next_refresh_ticket: u64,
    /// The newest refresh applied, so a slow response can't replace a newer one.
    applied_refresh: Option<RefreshTicket>,
}

impl ReaderSavedState {
    /// Records a write about to be sent, which takes effect straight away.
    pub(super) fn begin_write(&mut self, key: &ReaderSavedKey, saved: bool) -> WriteId {
        let id = WriteId(self.next_write_id);
        self.next_write_id += 1;
        let slot = self.slot(key);
        self.recent_writes
            .entry(slot)
            .or_default()
            .push(RecentWrite {
                id,
                key: key.clone(),
                saved,
                expires_at: None,
            });
        id
    }

    /// Records that the server accepted a write. `stored_key` is the key it stored, which can
    /// differ from the one sent.
    pub(super) fn complete_write(
        &mut self,
        requested_key: &ReaderSavedKey,
        id: WriteId,
        stored_key: &ReaderSavedKey,
        now: Instant,
    ) {
        let slot = self.slot(requested_key);
        let Some(mut write) = self.take_in_flight_write(&slot, id) else {
            return;
        };
        let stored = self.learn_stored_key(requested_key, &slot, stored_key);

        // A newer write to the post has already finished, so the server has its result, not this
        // one's. Applying this one would undo it.
        if self
            .newest_finished
            .get(&stored)
            .is_some_and(|newest| *newest > write.id)
        {
            return;
        }
        self.newest_finished.insert(stored.clone(), write.id);

        write.key = stored_key.clone();
        write.expires_at = Some(now + RECENT_WRITE_TTL);
        if let Some(saved) = &mut self.saved {
            if write.saved {
                saved.insert(stored.clone());
            } else {
                saved.remove(&stored);
            }
        }

        // Keep the key's writes in the order they were made.
        let writes = self.recent_writes.entry(stored).or_default();
        let index = writes.partition_point(|other| other.id < write.id);
        writes.insert(index, write);
    }

    /// Undoes a write the server refused.
    pub(super) fn fail_write(&mut self, key: &ReaderSavedKey, id: WriteId) {
        let slot = self.slot(key);
        self.take_in_flight_write(&slot, id);
    }

    /// The key to send for a key: the one the server stores the post under, when that's known.
    pub(super) fn key_to_send(&self, key: &ReaderSavedKey) -> ReaderSavedKey {
        let normalized = key.normalized();
        match self.stored_as.get(&normalized) {
            Some(stored) => stored.clone(),
            None => key.clone(),
        }
    }

    pub(super) fn begin_refresh(&mut self, now: Instant) -> RefreshTicket {
        let ticket = RefreshTicket {
            id: self.next_refresh_ticket,
            started: now,
        };
        self.next_refresh_ticket += 1;
        ticket
    }

    /// Replaces the server's keys. Recent writes stay on top of them.
    pub(super) fn apply_refresh(
        &mut self,
        ticket: RefreshTicket,
        keys: impl IntoIterator<Item = ReaderSavedKey>,
        total: u64,
    ) {
        if self.applied_refresh.is_some_and(|applied| ticket < applied) {
            return;
        }
        self.applied_refresh = Some(ticket);

        let mut saved: HashSet<_> = keys.into_iter().map(|key| self.slot(&key)).collect();
        self.beyond_keys = total.saturating_sub(saved.len() as u64);

        // This list may be from before the device's latest writes. Keeping them in it, rather than
        // only on top of it, stops it coming back when they expire if there's no other refresh
        // before then. A write whose window had ended when the request started is older than the
        // list, so the list wins.
        for (key, writes) in &self.recent_writes {
            let finished = writes.iter().rfind(|write| write.expires_at.is_some());
            if let Some(write) = finished.filter(|write| write.is_live(ticket.started)) {
                if write.saved {
                    saved.insert(key.clone());
                } else {
                    saved.remove(key);
                }
            }
        }
        self.saved = Some(saved);
        self.refreshed_at = Some(ticket.started);
        self.prune(ticket.started);
    }

    /// Whether a post is saved, given every key it may be saved under.
    pub(super) fn status(&self, keys: &[ReaderSavedKey], now: Instant) -> ReaderSavedStatus {
        let keys = self.with_stored_keys(keys);
        if keys.is_empty() {
            return ReaderSavedStatus::NotSaved;
        }

        let states: Vec<_> = keys.iter().map(|key| self.is_saved(key, now)).collect();
        if states.contains(&Some(true)) {
            ReaderSavedStatus::Saved
        } else if states.contains(&None) {
            ReaderSavedStatus::Unknown
        } else {
            ReaderSavedStatus::NotSaved
        }
    }

    /// The keys to send to unsave a post: the ones it's saved under, or its main key if none are
    /// known to be.
    pub(super) fn keys_to_unsave(
        &self,
        keys: &[ReaderSavedKey],
        now: Instant,
    ) -> Vec<ReaderSavedKey> {
        let matched: Vec<_> = self
            .with_stored_keys(keys)
            .into_iter()
            .filter(|key| self.is_saved(key, now) == Some(true))
            .collect();

        if matched.is_empty() {
            keys.iter().take(1).cloned().collect()
        } else {
            matched
        }
    }

    /// Whether to show a post from the saved list, which can still have a post that was just
    /// unsaved.
    pub(super) fn should_show_listed_post(&self, keys: &[ReaderSavedKey], now: Instant) -> bool {
        let recently_unsaved = self.with_stored_keys(keys).iter().any(|key| {
            self.current_write(key, now)
                .is_some_and(|write| !write.saved)
        });
        !recently_unsaved || self.status(keys, now) == ReaderSavedStatus::Saved
    }

    /// Records that the saved list has a post, which can reach it before the keys do. Returns
    /// whether that changed anything.
    ///
    /// Nothing is added for a post already saved under any of its keys: a Jetpack post saved under
    /// its feed item can be listed with its blog and post IDs, and adding those would count it
    /// twice. Nor for an account past the 1,000 key limit, whose listed posts may already be
    /// counted beyond the keys.
    pub(super) fn note_listed_post(&mut self, keys: &[ReaderSavedKey], now: Instant) -> bool {
        if self.beyond_keys > 0 || self.status(keys, now) == ReaderSavedStatus::Saved {
            return false;
        }
        let slot = keys.first().map(|key| self.slot(key));
        if let Some(slot) = &slot {
            self.listed_at.insert(slot.clone(), now);
        }
        match (&mut self.saved, slot) {
            (Some(saved), Some(key)) => saved.insert(key),
            _ => false,
        }
    }

    /// How many posts are saved, or `None` until the first refresh.
    pub(super) fn saved_count(&self, now: Instant) -> Option<u64> {
        let saved = self.saved.as_ref()?;

        let mut count = saved.len() as u64 + self.beyond_keys;
        for key in self.recent_writes.keys() {
            let Some(write) = self.current_write(key, now) else {
                continue;
            };
            match (write.saved, saved.contains(key)) {
                (true, false) => count += 1,
                (false, true) => count = count.saturating_sub(1),
                _ => {}
            }
        }
        Some(count)
    }

    /// The keys this device saved recently, newest first, which the saved list may not have yet.
    pub(super) fn recent_saves(&self, now: Instant) -> Vec<ReaderSavedKey> {
        let mut saves: Vec<_> = self
            .recent_writes
            .keys()
            .filter_map(|key| {
                let write = self.current_write(key, now)?;
                write.saved.then_some((write.id, write.key.clone()))
            })
            .collect();
        saves.sort_by_key(|(id, _)| std::cmp::Reverse(*id));
        saves.into_iter().map(|(_, key)| key).collect()
    }

    /// The write that decides a key's state: its newest, unless that has expired.
    fn current_write(&self, key: &ReaderSavedKey, now: Instant) -> Option<&RecentWrite> {
        self.recent_writes
            .get(key)?
            .last()
            .filter(|write| write.is_live(now))
    }

    fn is_saved(&self, key: &ReaderSavedKey, now: Instant) -> Option<bool> {
        match self.current_write(key, now) {
            Some(write) => Some(write.saved),
            None => self.saved.as_ref().map(|saved| saved.contains(key)),
        }
    }

    /// Where a key's writes live: the key the server stores the post under if that's known,
    /// otherwise the key itself, normalized.
    fn slot(&self, key: &ReaderSavedKey) -> ReaderSavedKey {
        let key = key.normalized();
        self.stored_as.get(&key).cloned().unwrap_or(key)
    }

    /// Records the key the server stored a write under, and returns where the post's writes now
    /// live. When it's new, the writes already under the sent key move there, so newest-wins sees
    /// all of them.
    fn learn_stored_key(
        &mut self,
        requested_key: &ReaderSavedKey,
        slot: &ReaderSavedKey,
        stored_key: &ReaderSavedKey,
    ) -> ReaderSavedKey {
        let stored = stored_key.normalized();
        if stored == *slot || stored == requested_key.normalized() {
            return slot.clone();
        }

        self.stored_as
            .insert(requested_key.normalized(), stored.clone());
        for target in self.stored_as.values_mut() {
            if target == slot {
                *target = stored.clone();
            }
        }
        if let Some(moved) = self.newest_finished.remove(slot) {
            let newest = self.newest_finished.entry(stored.clone()).or_insert(moved);
            *newest = (*newest).max(moved);
        }
        if let Some(moved) = self.recent_writes.remove(slot) {
            let writes = self.recent_writes.entry(stored.clone()).or_default();
            for write in moved {
                let index = writes.partition_point(|other| other.id < write.id);
                writes.insert(index, write);
            }

            // A write that finished under the sent key was applied to it, not to the post's stored
            // key, so the stored key may still show the post's state from before it. Unless the
            // server has been read since its window ended, which is newer than the write.
            let last_read = self
                .refreshed_at
                .into_iter()
                .chain(self.listed_at.get(&stored).copied())
                .max();
            let newest = writes
                .iter()
                .rfind(|write| write.expires_at.is_some())
                .filter(|write| {
                    last_read.is_none_or(|read| write.expires_at.is_some_and(|end| end > read))
                });
            if let (Some(write), Some(saved)) = (newest, &mut self.saved) {
                if write.saved {
                    saved.insert(stored.clone());
                } else {
                    saved.remove(&stored);
                }
            }
        }
        stored
    }

    /// The keys to check for a post: each of its keys, or the key the server stores it under once
    /// that's known, since that's where the post's writes and saves are.
    fn with_stored_keys(&self, keys: &[ReaderSavedKey]) -> Vec<ReaderSavedKey> {
        let mut all: Vec<ReaderSavedKey> = Vec::new();
        for key in keys {
            let key = self.slot(key);
            if !all.contains(&key) {
                all.push(key);
            }
        }
        all
    }

    /// Only a write in flight can finish, so finishing one twice does nothing.
    fn take_in_flight_write(&mut self, key: &ReaderSavedKey, id: WriteId) -> Option<RecentWrite> {
        let writes = self.recent_writes.get_mut(key)?;
        let index = writes
            .iter()
            .position(|write| write.id == id && write.expires_at.is_none())?;
        let write = writes.remove(index);
        if writes.is_empty() {
            self.recent_writes.remove(key);
        }
        Some(write)
    }

    /// Drops writes whose window had ended by `started`, the start of a refresh being applied:
    /// that refresh, and any accepted after it, is new enough to include them. A write whose
    /// window ended later is kept, because another refresh that started before then could still
    /// land and needs it on top. Keys with a write still in flight keep everything: an older write
    /// that finishes later has to see the newer ones to know it was overridden.
    ///
    /// Once nothing is in flight only the newest write matters, so the others go too. Dropping
    /// writes one at a time could leave an older one, with a later window, standing in for it.
    fn prune(&mut self, started: Instant) {
        self.recent_writes.retain(|_, writes| {
            if writes.iter().all(|write| write.expires_at.is_some()) {
                let newest = writes.pop().filter(|write| write.is_live(started));
                writes.clear();
                writes.extend(newest);
            }
            !writes.is_empty()
        });
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "model_tests.rs"]
mod model_tests;

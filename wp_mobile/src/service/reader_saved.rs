//! Saved Reader posts, kept in step with what this device has just changed.
//!
//! The saved posts API's reads can lag behind its writes, so an app that refetches after a save can
//! briefly see the post unsaved again. [`ReaderSavedService`] applies each change straight away and
//! keeps it on top of what the server returns until the server has had time to catch up. See
//! [`state`] for the rules.

use futures::FutureExt;
use futures::channel::oneshot;
use futures::future::Shared;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;
use wp_api::prelude::{WpApiClientDelegate, WpApiError, WpErrorCode};
use wp_api::wp_com::client::WpComApiClient;
use wp_api::wp_com::reader_saved::{
    ReaderPostSaveIdentity, ReaderSavePostParams, ReaderSavedBulkKey, ReaderSavedBulkParams,
    ReaderSavedBulkResult, ReaderSavedKey, ReaderSavedPageDetails, ReaderSavedPostsParams,
    ReaderSavedPostsResponse, ReaderSavedSource, ReaderUnsavePostParams, reader_saved_key_for_post,
    reader_saved_keys_for_post,
};

mod state;

pub use state::ReaderSavedStatus;
use state::{ReaderSavedState, WriteId};

/// The most keys `POST /wpcom/v2/read/saved/bulk` takes in a call.
const MAX_BULK_KEYS: usize = 100;

/// The `error` of an import result the server didn't return.
pub const MISSING_RESULT: &str = "missing_result";

#[allow(clippy::large_enum_variant)] // Boxing doesn't cross the FFI boundary.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum ReaderSavedError {
    #[error("{0}")]
    Api(WpApiError),

    /// The post doesn't have the IDs the API needs to save it.
    #[error("This post can't be saved.")]
    PostCannotBeSaved,
}

impl From<WpApiError> for ReaderSavedError {
    fn from(err: WpApiError) -> Self {
        Self::Api(err)
    }
}

/// Told when what [`ReaderSavedService`] knows about saved posts changes, on whichever thread made
/// the change.
#[uniffi::export(with_foreign)]
pub trait ReaderSavedObserver: Send + Sync {
    fn saved_posts_changed(&self);
}

/// The current account's saved Reader posts.
///
/// Saves belong to the account rather than a site, so this is separate from `WpService`. State is
/// kept in memory: call [`Self::refresh`] after launch, until which posts this device hasn't changed
/// are [`ReaderSavedStatus::Unknown`].
///
/// Create one per account and share it. Each instance only knows its own writes, and only sends one
/// write to a post at a time, so a second instance could undo a change made through the first.
#[derive(uniffi::Object)]
pub struct ReaderSavedService {
    client: WpComApiClient,
    source: ReaderSavedSource,
    inner: Mutex<Inner>,
    observer: Mutex<Option<Arc<dyn ReaderSavedObserver>>>,
    clock: Clock,
}

type Clock = Arc<dyn Fn() -> Instant + Send + Sync>;

#[derive(Default)]
struct Inner {
    state: ReaderSavedState,
    /// For each key, the writes that took a turn on it and may not have finished.
    queue: Queue,
}

#[uniffi::export]
impl ReaderSavedService {
    #[uniffi::constructor]
    pub fn new(delegate: WpApiClientDelegate, source: ReaderSavedSource) -> Self {
        Self::with_clock(delegate, source, Arc::new(Instant::now))
    }

    pub fn set_observer(&self, observer: Option<Arc<dyn ReaderSavedObserver>>) {
        *self.observer.lock().expect("observer lock poisoned") = observer;
    }

    /// Whether a post is saved. Doesn't touch the network.
    pub fn status(&self, post: ReaderPostSaveIdentity) -> ReaderSavedStatus {
        self.state().status(&reader_saved_keys_for_post(post))
    }

    /// How many posts are saved, or `None` until the first refresh.
    pub fn saved_count(&self) -> Option<u64> {
        self.state().saved_count()
    }

    /// The keys this device saved recently, newest first. The server's list can lag behind them, and
    /// a write doesn't return the post, so show these above the list.
    pub fn recent_saves(&self) -> Vec<ReaderSavedKey> {
        self.state().recent_saves()
    }

    /// Reloads the saved keys. Changes this device made recently stay in place.
    pub async fn refresh(&self) -> Result<(), ReaderSavedError> {
        let ticket = self.state().begin_refresh(self.now());
        let response = self.client.reader_saved().list_keys().await?.data;

        let keys = response
            .keys
            .into_iter()
            .map(|entry| (entry.key, entry.url));
        self.state().apply_refresh(ticket, keys, response.total);
        self.notify();
        Ok(())
    }

    /// A page of the saved list, without the posts this device recently unsaved.
    ///
    /// Only a missing `next_page_handle` means the end of the list: a page can be short, or even
    /// empty, with more to come. `found` comes straight from the server, so prefer
    /// [`Self::saved_count`] for a count.
    pub async fn list_posts(
        &self,
        params: ReaderSavedPostsParams,
    ) -> Result<ReaderSavedPostsResponse, ReaderSavedError> {
        let started = self.now();
        let mut response = self.client.reader_saved().list_posts(&params).await?.data;

        let mut changed = false;
        {
            let mut state = self.state();
            response.posts.retain(|post| {
                let keys = reader_saved_keys_for_post(post.identity.clone());
                let should_show = state.should_show_listed_post(&keys);
                if should_show {
                    changed |= state.note_listed_post(&keys, started);
                }
                should_show
            });
        }
        if changed {
            self.notify();
        }
        Ok(response)
    }

    /// Saves a post, and returns the key the server stored.
    pub async fn save(
        &self,
        post: ReaderPostSaveIdentity,
    ) -> Result<ReaderSavedKey, ReaderSavedError> {
        let key =
            reader_saved_key_for_post(post.clone()).ok_or(ReaderSavedError::PostCannotBeSaved)?;
        self.save_key(&reader_saved_keys_for_post(post), key, None)
            .await
    }

    /// Saves a page by URL, and returns the key the server stored: the URL of a WordPress.com post
    /// resolves to the post.
    pub async fn save_url(
        &self,
        url: String,
        page_details: Option<ReaderSavedPageDetails>,
    ) -> Result<ReaderSavedKey, ReaderSavedError> {
        let key = ReaderSavedKey::Url { url };
        let post = [key.clone()];
        self.save_key(&post, key, page_details).await
    }

    /// Unsaves a post from every key it may be saved under. The post reads as unsaved straight
    /// away.
    ///
    /// Every key is tried, and the first error is returned. Keys that succeeded stay unsaved.
    pub async fn unsave(&self, post: ReaderPostSaveIdentity) -> Result<(), ReaderSavedError> {
        let keys = reader_saved_keys_for_post(post);
        if keys.is_empty() {
            return Err(ReaderSavedError::PostCannotBeSaved);
        }
        self.unsave_keys(&keys).await
    }

    /// Unsaves a key the server returned, such as one from [`Self::recent_saves`]. If the server
    /// stores the post under another key, that's unsaved too.
    pub async fn unsave_key(&self, key: ReaderSavedKey) -> Result<(), ReaderSavedError> {
        self.unsave_keys(&[key]).await
    }

    /// Imports a device's local saves, given newest first, and returns a result per key in the same
    /// order. A key the server rejected is a result with an `error`, not a failure of the import.
    ///
    /// Sends batches of 100, oldest first, so the list stays in order. Near the 1,000 save cap that
    /// means the newest keys are the ones refused.
    ///
    /// Stops at the first batch that fails and returns its error. Saving a post twice does nothing,
    /// so it's safe to import everything again.
    pub async fn import(
        &self,
        keys: Vec<ReaderSavedBulkKey>,
    ) -> Result<Vec<ReaderSavedBulkResult>, ReaderSavedError> {
        let mut batches = Vec::new();
        for batch in keys.chunks(MAX_BULK_KEYS).rev() {
            batches.push(self.import_batch(batch).await?);
        }
        Ok(batches.into_iter().rev().flatten().collect())
    }
}

impl ReaderSavedService {
    fn with_clock(delegate: WpApiClientDelegate, source: ReaderSavedSource, clock: Clock) -> Self {
        Self {
            client: WpComApiClient::new(delegate),
            source,
            inner: Mutex::default(),
            observer: Mutex::default(),
            clock,
        }
    }

    fn now(&self) -> Instant {
        (self.clock)()
    }

    async fn save_key(
        &self,
        post: &[ReaderSavedKey],
        key: ReaderSavedKey,
        page_details: Option<ReaderSavedPageDetails>,
    ) -> Result<ReaderSavedKey, ReaderSavedError> {
        let (mut writes, turn) = PendingWrites::begin(self, post, vec![key.clone()], true);
        turn.wait().await;
        let params = ReaderSavePostParams {
            key: key.clone(),
            source: self.source,
            page_details,
        };
        let result = self.client.reader_saved().save(&params).await;
        let write = writes.forget_all()[0].1;
        let stored_key = result.map(|response| response.data.key);
        let returned = stored_key.as_ref().ok().cloned();
        self.finish_write(write, stored_key)?;
        Ok(returned.unwrap_or(key))
    }

    async fn unsave_keys(&self, post: &[ReaderSavedKey]) -> Result<(), ReaderSavedError> {
        // Picks the keys and starts their writes under one lock, so the post reads as unsaved
        // straight away.
        let (mut writes, turn) =
            PendingWrites::begin_with(self, post, false, |state| state.keys_to_unsave(post));
        turn.wait().await;

        // A write that went before this one may have shown the post is stored under another key.
        writes.add(|state| state.keys_to_unsave(post));

        let mut first_error = None;
        for (key, write) in writes.writes.clone() {
            let known_saved = self.state().known_saved_under(&key);
            let params = ReaderUnsavePostParams {
                key: key.clone(),
                source: self.source,
            };
            let result = self
                .client
                .reader_saved()
                .unsave(&params)
                .await
                .map(|response| response.data.key);
            writes.forget(write);
            // The post isn't saved under a key the server can't find, unless this device has seen
            // it saved there: a feed item stops resolving once it's pruned.
            let result = match result {
                Err(error) if !known_saved && is_post_not_found(&error) => Ok(key),
                result => result,
            };
            if let Err(error) = self.finish_write(write, result) {
                first_error.get_or_insert(error);
            }
        }
        first_error.map_or(Ok(()), Err)
    }

    async fn import_batch(
        &self,
        keys: &[ReaderSavedBulkKey],
    ) -> Result<Vec<ReaderSavedBulkResult>, ReaderSavedError> {
        // The keys are newest first. Starting their writes oldest first makes the newest the most
        // recent save.
        let saved_keys: Vec<_> = keys.iter().rev().map(saved_key).collect();
        let (mut pending, turn) = PendingWrites::begin_each(self, saved_keys, true);
        turn.wait().await;

        let params = ReaderSavedBulkParams {
            keys: keys.to_vec(),
            source: self.source,
        };
        // A failed request drops `pending`, which undoes every key in the batch.
        let response = self.client.reader_saved().bulk_save(&params).await?.data;
        let mut writes = pending.forget_all();
        writes.reverse();

        // One result per key, in order, so the batches can be joined by position. A result the
        // server left out is reported as one, rather than shifting every result after it.
        let results: Vec<_> = keys
            .iter()
            .enumerate()
            .map(|(index, key)| {
                response
                    .results
                    .get(index)
                    .cloned()
                    .unwrap_or_else(|| ReaderSavedBulkResult {
                        key: Some(key.clone()),
                        saved: false,
                        error: Some(MISSING_RESULT.to_string()),
                    })
            })
            .collect();

        {
            let mut state = self.state();
            let now = self.now();
            for ((_, write), result) in writes.iter().zip(&results) {
                match result {
                    ReaderSavedBulkResult {
                        key: Some(stored_key),
                        saved: true,
                        ..
                    } => state.complete_write(*write, &saved_key(stored_key), now),
                    _ => state.fail_write(*write),
                }
            }
        }
        self.notify();
        Ok(results)
    }

    fn finish_write(
        &self,
        write: WriteId,
        stored_key: Result<ReaderSavedKey, WpApiError>,
    ) -> Result<(), ReaderSavedError> {
        let result = match stored_key {
            Ok(stored_key) => {
                let now = self.now();
                self.state().complete_write(write, &stored_key, now);
                Ok(())
            }
            Err(error) => {
                self.state().fail_write(write);
                Err(error.into())
            }
        };
        self.notify();
        result
    }

    fn inner(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().expect("state lock poisoned")
    }

    fn state(&self) -> StateGuard<'_> {
        StateGuard(self.inner())
    }

    /// Never called with the state locked, because the observer is app code that reads it back.
    fn notify(&self) {
        let observer = self
            .observer
            .lock()
            .expect("observer lock poisoned")
            .clone();
        if let Some(observer) = observer {
            observer.saved_posts_changed();
        }
    }
}

struct StateGuard<'a>(MutexGuard<'a, Inner>);

impl std::ops::Deref for StateGuard<'_> {
    type Target = ReaderSavedState;
    fn deref(&self) -> &ReaderSavedState {
        &self.0.state
    }
}

impl std::ops::DerefMut for StateGuard<'_> {
    fn deref_mut(&mut self) -> &mut ReaderSavedState {
        &mut self.0.state
    }
}

fn is_post_not_found(error: &WpApiError) -> bool {
    matches!(
        error,
        WpApiError::WpError {
            error_code: WpErrorCode::CustomError(code),
            ..
        } if code == "post_not_found"
    )
}

/// A write's place in line: it waits for every earlier write that shares a key with it and hasn't
/// finished, so the server applies a post's writes in the order the user made them. Waiting for all
/// of them, rather than the last, matters when one in between is cancelled before it's sent.
struct Turn {
    /// Dropped when the write finishes or is cancelled.
    _done: oneshot::Sender<()>,
    before: Vec<Shared<oneshot::Receiver<()>>>,
}

type Queue = HashMap<ReaderSavedKey, Vec<Shared<oneshot::Receiver<()>>>>;

impl Turn {
    fn take(queue: &mut Queue, keys: &[ReaderSavedKey]) -> Self {
        let (done, finished) = oneshot::channel();
        let finished = finished.shared();
        let keys: HashSet<_> = keys.iter().map(ReaderSavedKey::normalized).collect();
        let mut before: Vec<Shared<oneshot::Receiver<()>>> = Vec::new();
        for key in keys {
            let waiting = queue.entry(key).or_default();
            waiting.retain(|earlier| earlier.clone().now_or_never().is_none());
            for earlier in waiting.iter() {
                if !before.iter().any(|other| other.ptr_eq(earlier)) {
                    before.push(earlier.clone());
                }
            }
            waiting.push(finished.clone());
        }
        queue.retain(|_, waiting| !waiting.is_empty());
        Self {
            _done: done,
            before,
        }
    }

    async fn wait(&self) {
        for earlier in &self.before {
            // Cancelled is how an earlier write says it's done.
            let _ = earlier.clone().await;
        }
    }
}

/// Writes started but not yet finished. Dropping it undoes the ones still pending, so a call that's
/// cancelled can't leave a write in flight forever. Kotlin cancels a call by dropping its future.
/// Swift, as of UniFFI 0.32, doesn't: a cancelled Swift task still waits for the call to finish.
struct PendingWrites<'a> {
    service: &'a ReaderSavedService,
    saved: bool,
    post: Vec<ReaderSavedKey>,
    writes: Vec<(ReaderSavedKey, WriteId)>,
}

impl<'a> PendingWrites<'a> {
    /// Starts a write for each key, all at once, so the change shows straight away, and takes the
    /// writes' turn.
    fn begin(
        service: &'a ReaderSavedService,
        post: &[ReaderSavedKey],
        keys: Vec<ReaderSavedKey>,
        saved: bool,
    ) -> (Self, Turn) {
        Self::begin_with(service, post, saved, |_| keys)
    }

    /// Like [`Self::begin`], choosing the keys from the state under the same lock.
    fn begin_with(
        service: &'a ReaderSavedService,
        post: &[ReaderSavedKey],
        saved: bool,
        choose: impl FnOnce(&ReaderSavedState) -> Vec<ReaderSavedKey>,
    ) -> (Self, Turn) {
        Self::start(service, Some(post), saved, choose)
    }

    /// Starts a write for each key, each its own post, as an import does.
    fn begin_each(
        service: &'a ReaderSavedService,
        keys: Vec<ReaderSavedKey>,
        saved: bool,
    ) -> (Self, Turn) {
        Self::start(service, None, saved, |_| keys)
    }

    fn start(
        service: &'a ReaderSavedService,
        post: Option<&[ReaderSavedKey]>,
        saved: bool,
        choose: impl FnOnce(&ReaderSavedState) -> Vec<ReaderSavedKey>,
    ) -> (Self, Turn) {
        let mut pending = Self {
            service,
            saved,
            post: post.unwrap_or_default().to_vec(),
            writes: Vec::new(),
        };
        let turn = {
            let mut inner = service.inner();
            let keys = choose(&inner.state);
            let mut lock_keys = inner.state.lock_keys(&pending.post);
            for key in keys {
                let write = inner.state.begin_write(&key, saved);
                lock_keys.extend(inner.state.lock_keys(std::slice::from_ref(&key)));
                pending.writes.push((key, write));
            }
            Turn::take(&mut inner.queue, &lock_keys)
        };
        service.notify();
        (pending, turn)
    }

    /// Starts a write for each key chosen that doesn't have one yet.
    fn add(&mut self, choose: impl FnOnce(&ReaderSavedState) -> Vec<ReaderSavedKey>) {
        let mut state = self.service.state();
        for key in choose(&state) {
            if let Some(&(_, with)) = self.writes.last()
                && !self
                    .writes
                    .iter()
                    .any(|(other, _)| other.normalized() == key.normalized())
            {
                let write = state.begin_write_with(with, &key, self.saved);
                self.writes.push((key, write));
            }
        }
    }

    /// Stops tracking a write, because the caller is about to finish it.
    fn forget(&mut self, write: WriteId) {
        self.writes.retain(|(_, other)| *other != write);
    }

    fn forget_all(&mut self) -> Vec<(ReaderSavedKey, WriteId)> {
        std::mem::take(&mut self.writes)
    }
}

impl Drop for PendingWrites<'_> {
    fn drop(&mut self) {
        if self.writes.is_empty() {
            return;
        }
        {
            let mut state = self.service.state();
            for (_, write) in &self.writes {
                state.fail_write(*write);
            }
        }
        self.service.notify();
    }
}

fn saved_key(key: &ReaderSavedBulkKey) -> ReaderSavedKey {
    match key {
        ReaderSavedBulkKey::Post { blog_id, post_id } => ReaderSavedKey::Post {
            blog_id: *blog_id,
            post_id: *post_id,
        },
        ReaderSavedBulkKey::FeedItem { feed_item_id } => ReaderSavedKey::FeedItem {
            feed_item_id: *feed_item_id,
        },
    }
}

#[cfg(test)]
mod tests;

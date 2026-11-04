//! Saved Reader posts, kept in step with what this device has just changed.
//!
//! The saved posts API's reads can lag behind its writes, so an app that refetches after a save can
//! briefly see the post unsaved again. [`ReaderSavedService`] applies each change straight away and
//! keeps it on top of what the server returns until the server has had time to catch up. See
//! [`state`] for the rules.

use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;
use wp_api::prelude::{WpApiClientDelegate, WpApiError};
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
/// Create one per account and share it. Each instance only knows its own writes, so a second one
/// would let a stale refresh undo a change made through the first.
#[derive(uniffi::Object)]
pub struct ReaderSavedService {
    client: WpComApiClient,
    source: ReaderSavedSource,
    state: Mutex<ReaderSavedState>,
    observer: Mutex<Option<Arc<dyn ReaderSavedObserver>>>,
}

#[uniffi::export]
impl ReaderSavedService {
    #[uniffi::constructor]
    pub fn new(delegate: WpApiClientDelegate, source: ReaderSavedSource) -> Self {
        Self {
            client: WpComApiClient::new(delegate),
            source,
            state: Mutex::default(),
            observer: Mutex::default(),
        }
    }

    pub fn set_observer(&self, observer: Option<Arc<dyn ReaderSavedObserver>>) {
        *self.observer.lock().expect("observer lock poisoned") = observer;
    }

    /// Whether a post is saved. Doesn't touch the network.
    pub fn status(&self, post: ReaderPostSaveIdentity) -> ReaderSavedStatus {
        self.state()
            .status(&reader_saved_keys_for_post(post), Instant::now())
    }

    /// How many posts are saved, or `None` until the first refresh.
    pub fn saved_count(&self) -> Option<u64> {
        self.state().saved_count(Instant::now())
    }

    /// The keys this device saved recently, newest first. The server's list can lag behind them, and
    /// a write doesn't return the post, so show these above the list.
    pub fn recent_saves(&self) -> Vec<ReaderSavedKey> {
        self.state().recent_saves(Instant::now())
    }

    /// Reloads the saved keys. Changes this device made recently stay in place.
    pub async fn refresh(&self) -> Result<(), ReaderSavedError> {
        let ticket = self.state().begin_refresh(Instant::now());
        let response = self.client.reader_saved().list_keys().await?.data;

        let keys = response.keys.into_iter().map(|entry| entry.key);
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
        let mut response = self.client.reader_saved().list_posts(&params).await?.data;

        let mut changed = false;
        {
            let mut state = self.state();
            let now = Instant::now();
            response.posts.retain(|post| {
                let keys = reader_saved_keys_for_post(post.identity.clone());
                let should_show = state.should_show_listed_post(&keys, now);
                if should_show {
                    changed |= state.note_listed_post(&keys, now);
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
        let key = reader_saved_key_for_post(post).ok_or(ReaderSavedError::PostCannotBeSaved)?;
        self.save_key(key, None).await
    }

    /// Saves a page by URL, and returns the key the server stored: the URL of a WordPress.com post
    /// resolves to the post.
    pub async fn save_url(
        &self,
        url: String,
        page_details: Option<ReaderSavedPageDetails>,
    ) -> Result<ReaderSavedKey, ReaderSavedError> {
        self.save_key(ReaderSavedKey::Url { url }, page_details)
            .await
    }

    /// Unsaves a post from every key it's saved under. The post reads as unsaved straight away.
    ///
    /// All the keys are tried, and the first error is returned. Keys that succeeded stay unsaved.
    pub async fn unsave(&self, post: ReaderPostSaveIdentity) -> Result<(), ReaderSavedError> {
        let keys = reader_saved_keys_for_post(post);
        if keys.is_empty() {
            return Err(ReaderSavedError::PostCannotBeSaved);
        }

        // Picks the keys and starts their writes under one lock, so a refresh can't land between.
        let mut writes = PendingWrites::begin_with(self, false, |state| {
            state.keys_to_unsave(&keys, Instant::now())
        });
        let mut first_error = None;
        for (key, write) in writes.writes.clone() {
            let params = ReaderUnsavePostParams {
                key: key.clone(),
                source: self.source,
            };
            let result = self.client.reader_saved().unsave(&params).await;
            writes.forget(write);
            if let Err(error) =
                self.finish_write(&key, write, result.map(|response| response.data.key))
            {
                first_error.get_or_insert(error);
            }
        }
        first_error.map_or(Ok(()), Err)
    }

    /// Unsaves a key the server returned, such as one from [`Self::recent_saves`]. If the server
    /// stores the post under another key, that's the one sent.
    pub async fn unsave_key(&self, key: ReaderSavedKey) -> Result<(), ReaderSavedError> {
        let mut writes =
            PendingWrites::begin_with(self, false, |state| vec![state.key_to_send(&key)]);
        let key = writes.writes[0].0.clone();
        let params = ReaderUnsavePostParams {
            key: key.clone(),
            source: self.source,
        };
        let result = self.client.reader_saved().unsave(&params).await;
        let write = writes.forget_all()[0].1;
        self.finish_write(&key, write, result.map(|response| response.data.key))
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
    async fn save_key(
        &self,
        key: ReaderSavedKey,
        page_details: Option<ReaderSavedPageDetails>,
    ) -> Result<ReaderSavedKey, ReaderSavedError> {
        let mut writes = PendingWrites::begin(self, vec![key.clone()], true);
        let params = ReaderSavePostParams {
            key: key.clone(),
            source: self.source,
            page_details,
        };
        let result = self.client.reader_saved().save(&params).await;
        let write = writes.forget_all()[0].1;
        let stored_key = result.map(|response| response.data.key);
        let returned = stored_key.as_ref().ok().cloned();
        self.finish_write(&key, write, stored_key)?;
        Ok(returned.unwrap_or(key))
    }

    async fn import_batch(
        &self,
        keys: &[ReaderSavedBulkKey],
    ) -> Result<Vec<ReaderSavedBulkResult>, ReaderSavedError> {
        let mut pending = PendingWrites::begin(self, keys.iter().map(saved_key).collect(), true);

        let params = ReaderSavedBulkParams {
            keys: keys.to_vec(),
            source: self.source,
        };
        // A failed request drops `pending`, which undoes every key in the batch.
        let response = self.client.reader_saved().bulk_save(&params).await?.data;
        let writes = pending.forget_all();

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
            let now = Instant::now();
            for ((key, write), result) in writes.iter().zip(&results) {
                match result {
                    ReaderSavedBulkResult {
                        key: Some(stored_key),
                        saved: true,
                        ..
                    } => state.complete_write(key, *write, &saved_key(stored_key), now),
                    _ => state.fail_write(key, *write),
                }
            }
        }
        self.notify();
        Ok(results)
    }

    fn finish_write(
        &self,
        key: &ReaderSavedKey,
        write: WriteId,
        stored_key: Result<ReaderSavedKey, WpApiError>,
    ) -> Result<(), ReaderSavedError> {
        let result = match stored_key {
            Ok(stored_key) => {
                self.state()
                    .complete_write(key, write, &stored_key, Instant::now());
                Ok(())
            }
            Err(error) => {
                self.state().fail_write(key, write);
                Err(error.into())
            }
        };
        self.notify();
        result
    }

    fn state(&self) -> MutexGuard<'_, ReaderSavedState> {
        self.state.lock().expect("state lock poisoned")
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

/// Writes started but not yet finished. Dropping it undoes the ones still pending, so a call that's
/// cancelled can't leave a write in flight forever. Kotlin cancels a call by dropping its future.
/// Swift, as of UniFFI 0.32, doesn't: a cancelled Swift task still waits for the call to finish.
struct PendingWrites<'a> {
    service: &'a ReaderSavedService,
    writes: Vec<(ReaderSavedKey, WriteId)>,
}

impl<'a> PendingWrites<'a> {
    /// Starts a write for each key, all at once, so the change shows straight away.
    fn begin(service: &'a ReaderSavedService, keys: Vec<ReaderSavedKey>, saved: bool) -> Self {
        Self::begin_with(service, saved, |_| keys)
    }

    /// Like [`Self::begin`], choosing the keys from the state under the same lock.
    fn begin_with(
        service: &'a ReaderSavedService,
        saved: bool,
        choose: impl FnOnce(&ReaderSavedState) -> Vec<ReaderSavedKey>,
    ) -> Self {
        let writes = {
            let mut state = service.state();
            let keys = choose(&state);
            keys.into_iter()
                .map(|key| {
                    let write = state.begin_write(&key, saved);
                    (key, write)
                })
                .collect()
        };
        service.notify();
        Self { service, writes }
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
            for (key, write) in &self.writes {
                state.fail_write(key, *write);
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

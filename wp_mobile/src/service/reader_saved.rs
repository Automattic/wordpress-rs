//! Saved Reader posts, kept in step with what this device has just changed.
//!
//! The saved posts API's reads can lag behind its writes, so an app that refetches after a save can
//! briefly see the post unsaved again. [`ReaderSavedService`] applies each change straight away and
//! keeps it on top of what the server returns until the server has had time to catch up. See
//! [`state`] for the rules.

use futures::FutureExt;
use futures::channel::oneshot;
use futures::future::Shared;
use std::collections::HashSet;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Instant, SystemTime};
use wp_api::prelude::{RequestExecutionErrorReason, WpApiClientDelegate, WpApiError, WpErrorCode};
use wp_api::wp_com::client::WpComApiClient;
use wp_api::wp_com::me::WpComUserId;
use wp_api::wp_com::reader_saved::{
    ReaderPostSaveIdentity, ReaderSavePostParams, ReaderSavedBulkKey, ReaderSavedBulkParams,
    ReaderSavedBulkResult, ReaderSavedKey, ReaderSavedPageDetails, ReaderSavedPostsParams,
    ReaderSavedPostsResponse, ReaderSavedSource, ReaderUnsavePostParams, reader_saved_key_for_post,
    reader_saved_keys_for_post,
};
use wp_mobile_cache::repository::reader::ReaderSavedRepository;
use wp_mobile_cache::repository::wordpress_com_accounts::WordPressComAccountRepository;
use wp_mobile_cache::{RowId, SqliteDbError, WpApiCache};

mod state;
mod storage;

pub use state::ReaderSavedStatus;
use state::{Now, PERSISTED_FORMAT, ReaderSavedState, Tap, WriteId};
pub use storage::{
    ReaderLocalSave, ReaderSavedImportReport, ReaderSavedListInfo, ReaderSavedListItem,
    ReaderSavedPageResult, ReaderSavedPostsCollection,
};

/// The most keys `POST /wpcom/v2/read/saved/bulk` takes in a call.
const MAX_BULK_KEYS: usize = 100;

#[allow(clippy::large_enum_variant)] // Boxing doesn't cross the FFI boundary.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum ReaderSavedError {
    #[error("{0}")]
    Api(WpApiError),

    /// The post doesn't have the IDs the API needs to save it, or the key has an ID of `0` or an
    /// empty URL.
    #[error("This post can't be saved.")]
    PostCannotBeSaved,

    /// An import batch got back a different number of results than the keys it sent, so no result
    /// can be matched to its key. The server may have saved any of them, so they read as saved until
    /// a refresh can include them.
    #[error("The server returned {received} results for {sent} posts.")]
    UnexpectedImportResults { sent: u64, received: u64 },

    #[error("{0}")]
    Database(SqliteDbError),

    /// The saved list was refreshed while its next page loaded, so the page was discarded.
    #[error("The saved list was refreshed while the next page loaded.")]
    StaleLoadMore,
}

impl From<WpApiError> for ReaderSavedError {
    fn from(err: WpApiError) -> Self {
        Self::Api(err)
    }
}

impl From<SqliteDbError> for ReaderSavedError {
    fn from(err: SqliteDbError) -> Self {
        Self::Database(err)
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
/// Saves belong to the account rather than a site, so this is separate from `WpService`. What it
/// knows is kept in the cache, under the account, so after a relaunch a post reads as it did before,
/// and a change made just before the app quit still wins over a server that hasn't caught up. Call
/// [`Self::refresh`] after launch to catch up with changes made elsewhere.
///
/// [`WpApiCache::remove_wordpress_com_account`] deletes it all. Drop the service first: one still
/// holding the removed account stops storing anything.
///
/// Create one per account and share it. It sends one write at a time, in the order the user tapped,
/// so the server ends up as the user last asked; a second instance could undo a change made through
/// the first.
///
/// On Android, call the writes from a scope that outlives the screen, or with
/// `withContext(NonCancellable)`. A write cancelled after its request is sent may still reach the
/// server after the next one, which would leave the server out of order.
#[derive(uniffi::Object)]
pub struct ReaderSavedService {
    client: WpComApiClient,
    source: ReaderSavedSource,
    inner: Mutex<Inner>,
    observer: Mutex<Option<Arc<dyn ReaderSavedObserver>>>,
    clock: Clock,
    wall_clock: WallClock,
    cache: Arc<WpApiCache>,
    /// The account's row in the cache.
    account: RowId,
    /// Held while the state is stored, so the newest state is the one left in the cache. Holds the
    /// generation of the server's keys last stored.
    stored_snapshot: Mutex<Option<u64>>,
}

type Clock = Arc<dyn Fn() -> Instant + Send + Sync>;
type WallClock = Arc<dyn Fn() -> SystemTime + Send + Sync>;

#[derive(Default)]
struct Inner {
    state: ReaderSavedState,
    /// The taps whose writes may not have finished.
    queue: Queue,
    /// Saves the server refused on import, normalized. They stay saved on this device.
    local_only: HashSet<ReaderSavedKey>,
}

#[uniffi::export]
impl ReaderSavedService {
    /// Loads what the cache has for the account. `cache` must have had its migrations run.
    #[uniffi::constructor]
    pub fn new(
        delegate: WpApiClientDelegate,
        source: ReaderSavedSource,
        cache: Arc<WpApiCache>,
        user_id: WpComUserId,
    ) -> Result<Self, ReaderSavedError> {
        Self::with_clocks(
            delegate,
            source,
            cache,
            user_id,
            Arc::new(Instant::now),
            Arc::new(SystemTime::now),
        )
    }

    pub fn set_observer(&self, observer: Option<Arc<dyn ReaderSavedObserver>>) {
        *self.observer.lock().expect("observer lock poisoned") = observer;
    }

    /// Whether a post is saved. Doesn't touch the network.
    ///
    /// Past 1,000 saves, a post the saved keys don't include reads `Unknown`. One the saved list
    /// shows is saved.
    ///
    /// A save the server refused on import is saved: it stays saved on this device.
    pub fn status(&self, post: ReaderPostSaveIdentity) -> ReaderSavedStatus {
        let keys = reader_saved_keys_for_post(post);
        let inner = self.inner();
        let status = inner.state.status(&keys);
        if status != ReaderSavedStatus::Saved
            && keys
                .iter()
                .any(|key| inner.local_only.contains(&key.normalized()))
        {
            return ReaderSavedStatus::Saved;
        }
        status
    }

    /// How many saves there are, or `None` until the first refresh or answered write. A post saved
    /// under two keys counts twice, as the server counts it.
    pub fn saved_count(&self) -> Option<u64> {
        self.state().saved_count()
    }

    /// The posts this device saved that the saved list hasn't shown yet, newest first. A write
    /// doesn't return the post, so show these above the list. Imports aren't included.
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
        self.changed();
        Ok(())
    }

    /// A page of the saved list, without the posts this device recently unsaved. The posts are
    /// stored, so [`Self::saved_posts`] shows them.
    ///
    /// Only a missing `next_page_handle` means the end of the list: a page can be short, or even
    /// empty, with more to come. `found` comes straight from the server, so prefer
    /// [`Self::saved_count`] for a count.
    pub async fn list_posts(
        &self,
        params: ReaderSavedPostsParams,
    ) -> Result<ReaderSavedPostsResponse, ReaderSavedError> {
        let response = self.fetch_posts(&params).await?;
        self.store_listed_posts(&response, storage::Page::Any)?;
        Ok(response)
    }

    /// Saves a post, and returns the key the server stored.
    pub async fn save(
        &self,
        post: ReaderPostSaveIdentity,
    ) -> Result<ReaderSavedKey, ReaderSavedError> {
        let key = reader_saved_key_for_post(post).ok_or(ReaderSavedError::PostCannotBeSaved)?;
        self.save_key(key, None, true).await
    }

    /// Saves a page by URL, and returns the key the server stored: the URL of a WordPress.com post
    /// resolves to the post.
    pub async fn save_url(
        &self,
        url: String,
        page_details: Option<ReaderSavedPageDetails>,
    ) -> Result<ReaderSavedKey, ReaderSavedError> {
        self.save_key(ReaderSavedKey::Url { url }, page_details, true)
            .await
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
    /// Stops at the first batch that fails and returns its error, including one that gets back a
    /// different number of results than keys. Saving a post twice does nothing, so it's safe to
    /// import everything again.
    ///
    /// The import takes one place in line, like a tap: a save or unsave tapped while it runs is
    /// sent once every batch is done, so a later batch can't undo it.
    ///
    /// A key with an ID of `0` isn't sent. Its result is an `invalid_input` error, as the server
    /// would give it.
    pub async fn import(
        &self,
        keys: Vec<ReaderSavedBulkKey>,
    ) -> Result<Vec<ReaderSavedBulkResult>, ReaderSavedError> {
        let sendable: Vec<_> = keys
            .iter()
            .filter(|key| saved_key(key).is_complete())
            .cloned()
            .collect();
        let (mut pending, turn) = PendingWrites::begin(self, Vec::new(), true, false);
        turn.wait().await;
        let mut batches = Vec::new();
        for batch in sendable.chunks(MAX_BULK_KEYS).rev() {
            batches.push(self.import_batch(&mut pending, batch).await?);
        }
        // Each batch has one result per key it sent, so the results line up with the sent keys.
        let mut sent = batches.into_iter().rev().flatten();
        Ok(keys
            .iter()
            .filter_map(|key| {
                if saved_key(key).is_complete() {
                    sent.next()
                } else {
                    Some(ReaderSavedBulkResult {
                        key: None,
                        saved: false,
                        error: Some("invalid_input".to_string()),
                    })
                }
            })
            .collect())
    }
}

impl ReaderSavedService {
    fn with_clocks(
        delegate: WpApiClientDelegate,
        source: ReaderSavedSource,
        cache: Arc<WpApiCache>,
        user_id: WpComUserId,
        clock: Clock,
        wall_clock: WallClock,
    ) -> Result<Self, ReaderSavedError> {
        let (account, stored, local_only) = cache.execute(|connection| {
            let account = WordPressComAccountRepository::get_or_create(connection, user_id)?;
            let stored = ReaderSavedRepository::select_state(connection, account)?;
            let local_only = ReaderSavedRepository::select_local_only(connection, account)?;
            Ok::<_, SqliteDbError>((account, stored, local_only))
        })?;
        let now = Now {
            instant: clock(),
            wall: wall_clock(),
        };
        let state = ReaderSavedState::restore(
            parse_stored(stored.local.as_deref(), |local: &state::PersistedLocal| {
                local.format
            }),
            parse_stored(
                stored.snapshot.as_deref(),
                |snapshot: &state::PersistedSnapshot| snapshot.format,
            ),
            now,
        );
        let stored_snapshot = Some(state.snapshot_generation());
        Ok(Self {
            client: WpComApiClient::new(delegate),
            source,
            inner: Mutex::new(Inner {
                state,
                queue: Queue::default(),
                local_only: local_only
                    .into_iter()
                    .map(|save| save.key.normalized())
                    .collect(),
            }),
            observer: Mutex::default(),
            clock,
            wall_clock,
            cache,
            account,
            stored_snapshot: Mutex::new(stored_snapshot),
        })
    }

    /// A service on a new in-memory cache, for tests that drive the clock.
    #[cfg(test)]
    fn with_clock(delegate: WpApiClientDelegate, source: ReaderSavedSource, clock: Clock) -> Self {
        Self::with_clocks(
            delegate,
            source,
            storage::migrated_in_memory_cache(),
            WpComUserId(1),
            clock,
            Arc::new(SystemTime::now),
        )
        .expect("an in-memory cache works")
    }

    fn now(&self) -> Instant {
        (self.clock)()
    }

    fn now_on_both_clocks(&self) -> Now {
        Now {
            instant: self.now(),
            wall: (self.wall_clock)(),
        }
    }

    /// Fetches a page of the saved list, without the posts this device recently unsaved.
    async fn fetch_posts(
        &self,
        params: &ReaderSavedPostsParams,
    ) -> Result<ReaderSavedPostsResponse, ReaderSavedError> {
        let started = self.now();
        let mut response = self.client.reader_saved().list_posts(params).await?.data;
        let mut changed = false;
        {
            let mut state = self.state();
            response.posts.retain(|post| {
                let keys = reader_saved_keys_for_post(post.identity.clone());
                let should_show = state.should_show_listed_post(&keys, started);
                if should_show {
                    changed |= state.note_listed_post(&keys);
                }
                should_show
            });
        }
        if changed {
            self.changed();
        }
        Ok(response)
    }

    /// Saves a key. `recent` is whether it goes in recent saves: an import's don't, because the
    /// device already had them.
    async fn save_key(
        &self,
        key: ReaderSavedKey,
        page_details: Option<ReaderSavedPageDetails>,
        recent: bool,
    ) -> Result<ReaderSavedKey, ReaderSavedError> {
        // The server would refuse it, and a key that can't be stored mustn't become a write.
        if !key.is_complete() {
            return Err(ReaderSavedError::PostCannotBeSaved);
        }
        let (mut writes, turn) = PendingWrites::begin(self, vec![key.clone()], true, recent);
        turn.wait().await;
        let params = ReaderSavePostParams {
            key: key.clone(),
            source: self.source,
            page_details,
        };
        writes.mark_all_sent();
        let result = self.client.reader_saved().save(&params).await;
        let write = writes.forget_all()[0].1;
        let answer = result.map(|response| (response.data.key, Some(response.data.total)));
        let returned = answer.as_ref().ok().map(|(key, _)| key.clone());
        self.finish_write(write, answer)?;
        let returned = returned.unwrap_or(key.clone());
        // The server has it now, so it's no longer only on this device.
        self.remove_local_only(&[key, returned.clone()]);
        Ok(returned)
    }

    async fn unsave_keys(&self, post: &[ReaderSavedKey]) -> Result<(), ReaderSavedError> {
        if post.iter().any(|key| !key.is_complete()) {
            return Err(ReaderSavedError::PostCannotBeSaved);
        }
        // A save only this device has is gone as soon as it's unsaved here.
        self.remove_local_only(post);

        // Picks the keys and starts their writes under one lock, so the post reads as unsaved
        // straight away.
        let (mut writes, turn) =
            PendingWrites::begin_with(self, false, true, |state| state.keys_to_unsave(post));
        turn.wait().await;

        // A write that went before this one may have shown the post is stored under another key.
        if writes.add(|state| state.keys_to_unsave(post)) {
            self.changed();
        }

        let mut first_error = None;
        for (key, write) in writes.unsent() {
            let known_saved = self.state().known_saved_under(&key);
            let params = ReaderUnsavePostParams {
                key: key.clone(),
                source: self.source,
            };
            writes.mark_sent(write);
            let result = self
                .client
                .reader_saved()
                .unsave(&params)
                .await
                .map(|response| (response.data.key, Some(response.data.total)));
            writes.forget(write);
            // The post isn't saved under a key the server can't find, unless this device has seen
            // it saved there: a feed item stops resolving once it's pruned.
            let result = match result {
                Err(error) if !known_saved && is_post_not_found(&error) => Ok((key, None)),
                result => result,
            };
            if let Err(error) = self.finish_write(write, result) {
                first_error.get_or_insert(error);
            }
        }
        first_error.map_or(Ok(()), Err)
    }

    /// Sends one batch of an import, whose turn has come. Its writes start now, so a batch not sent
    /// yet changes nothing.
    async fn import_batch(
        &self,
        pending: &mut PendingWrites<'_>,
        keys: &[ReaderSavedBulkKey],
    ) -> Result<Vec<ReaderSavedBulkResult>, ReaderSavedError> {
        // The keys are newest first, so their writes start oldest first.
        let saved_keys: Vec<_> = keys.iter().rev().map(saved_key).collect();
        let mut writes = pending.start(saved_keys);
        self.changed();

        let params = ReaderSavedBulkParams {
            keys: keys.to_vec(),
            source: self.source,
        };
        for write in &writes {
            pending.mark_sent(*write);
        }
        let result = self.client.reader_saved().bulk_save(&params).await;
        for write in &writes {
            pending.forget(*write);
        }
        let response = match result {
            Ok(response) => response.data,
            Err(error) => {
                {
                    let mut state = self.state();
                    let now = self.now();
                    for write in writes {
                        state.finish_with_error(write, &error, now);
                    }
                }
                self.changed();
                return Err(error.into());
            }
        };

        // Results are matched to keys by position, which only works with one result per key. With
        // any other number, one left out shifts the rest onto the wrong keys, and a post would be
        // recorded as stored under another post's key. The server may have saved any of them.
        if response.results.len() != keys.len() {
            {
                let mut state = self.state();
                let now = self.now();
                for write in writes {
                    state.abandon_write(write, now);
                }
            }
            self.changed();
            return Err(ReaderSavedError::UnexpectedImportResults {
                sent: keys.len() as u64,
                received: response.results.len() as u64,
            });
        }
        writes.reverse();
        let results = response.results;

        // Answered oldest first, the order the server applied them, so the total is the newest's.
        {
            let mut state = self.state();
            let now = self.now();
            for (write, result) in writes.iter().zip(&results).rev() {
                match result {
                    ReaderSavedBulkResult {
                        key: Some(stored_key),
                        saved: true,
                        ..
                    } => state.complete_write(
                        *write,
                        &saved_key(stored_key),
                        Some(response.total),
                        now,
                    ),
                    _ => state.fail_write(*write),
                }
            }
        }
        self.changed();
        Ok(results)
    }

    /// Records a write's answer: the key the server stored and the total it reported, or an error.
    fn finish_write(
        &self,
        write: WriteId,
        answer: Result<(ReaderSavedKey, Option<u64>), WpApiError>,
    ) -> Result<(), ReaderSavedError> {
        let result = match answer {
            Ok((stored_key, total)) => {
                let now = self.now();
                self.state().complete_write(write, &stored_key, total, now);
                Ok(())
            }
            Err(error) => {
                let now = self.now();
                self.state().finish_with_error(write, &error, now);
                Err(error.into())
            }
        };
        self.changed();
        result
    }

    fn inner(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().expect("state lock poisoned")
    }

    fn state(&self) -> StateGuard<'_> {
        StateGuard(self.inner())
    }

    /// Stores the state and tells the observer. Call it after every change, never with the state
    /// locked.
    fn changed(&self) {
        self.store_state();
        self.notify();
    }

    /// Stores the state in the cache, so the next launch starts from it.
    ///
    /// A failure is logged rather than returned. It doesn't change what this launch knows, and it
    /// can follow a change the server has already made, which an error would misreport as failed.
    /// What it costs is the next launch starting from an older state: `Unknown` until a refresh,
    /// or a write that loses to a server that hasn't caught up.
    fn store_state(&self) {
        let mut stored_snapshot = self
            .stored_snapshot
            .lock()
            .expect("stored snapshot lock poisoned");
        let now = self.now_on_both_clocks();
        let (local, snapshot, generation) = {
            let state = self.state();
            let generation = state.snapshot_generation();
            let snapshot = (*stored_snapshot != Some(generation))
                .then(|| state.persisted_snapshot(now))
                .flatten();
            (state.persisted_local(now), snapshot, generation)
        };
        let result = (|| -> Result<(), ReaderSavedError> {
            let local = serde_json::to_string(&local).map_err(stored_state_error)?;
            let snapshot = snapshot
                .map(|snapshot| serde_json::to_string(&snapshot))
                .transpose()
                .map_err(stored_state_error)?;
            self.cache.execute(|connection| {
                let transaction =
                    wp_mobile_cache::repository::TransactionManager::transaction(connection)?;
                ReaderSavedRepository::write_local(&transaction, self.account, &local)?;
                if let Some(snapshot) = &snapshot {
                    ReaderSavedRepository::write_snapshot(&transaction, self.account, snapshot)?;
                }
                transaction.commit().map_err(SqliteDbError::from)
            })?;
            Ok(())
        })();
        match result {
            Ok(()) => *stored_snapshot = Some(generation),
            Err(error) => log::error!("Couldn't store the saved posts state: {error}"),
        }
    }

    /// Removes saves only this device has, under any of these keys.
    fn remove_local_only(&self, keys: &[ReaderSavedKey]) {
        let keys: Vec<_> = keys
            .iter()
            .map(ReaderSavedKey::normalized)
            .filter(|key| self.inner().local_only.contains(key))
            .collect();
        if keys.is_empty() {
            return;
        }
        {
            let mut inner = self.inner();
            for key in &keys {
                inner.local_only.remove(key);
            }
        }
        let result = self.cache.execute(|connection| {
            keys.iter().try_for_each(|key| {
                ReaderSavedRepository::delete_local_only(&*connection, self.account, key)
                    .map(|_| ())
            })
        });
        // Logged, not returned, for the same reason as in `store_state`.
        if let Err(error) = result {
            log::error!("Couldn't remove a local-only save: {error}");
        }
        self.notify();
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

impl ReaderSavedState {
    /// Records a write that failed: undone if the server refused it or never got it, but left to
    /// stand until a read if it may have been applied.
    fn finish_with_error(&mut self, write: WriteId, error: &WpApiError, now: Instant) {
        if may_have_been_applied(error) {
            self.abandon_write(write, now);
        } else {
            self.fail_write(write);
        }
    }
}

/// Whether a failed write may still have been applied: the request may have reached the server,
/// and the server didn't say it refused it. That's a timeout, a connection lost or cancelled
/// mid-request, an answer that couldn't be read, or a server error (5xx) from WordPress.com or a
/// proxy in front of it.
///
/// Errors from before the request was sent are undone: no network, a host that doesn't resolve, a
/// refused connection, TLS. So is any 4xx answer. iOS reports a connection lost mid-request as
/// offline too, so that's undone, and the next refresh corrects it if the server applied it.
fn may_have_been_applied(error: &WpApiError) -> bool {
    use RequestExecutionErrorReason as Reason;
    match error {
        WpApiError::RequestExecutionFailed { reason, .. } => match reason {
            Reason::HttpTimeoutError
            | Reason::HttpError { .. }
            | Reason::GenericError { .. }
            | Reason::CancellationError => true,
            Reason::CloudflareError { status_code, .. } => *status_code >= 500,
            Reason::InvalidSslError { .. }
            | Reason::NonExistentSiteError { .. }
            | Reason::HttpAuthenticationRequiredError { .. }
            | Reason::HttpAuthenticationRejectedError { .. }
            | Reason::HttpForbiddenError { .. }
            | Reason::MisconfiguredHttpAuthenticationError { .. }
            | Reason::MisconfiguredRateLimitError
            | Reason::DeviceIsOfflineError { .. }
            | Reason::ConnectionError { .. } => false,
        },
        WpApiError::ResponseParsingError { .. } => true,
        WpApiError::InvalidHttpStatusCode { status_code, .. }
        | WpApiError::UnknownError { status_code, .. }
        | WpApiError::WpError { status_code, .. } => *status_code >= 500,
        WpApiError::MediaFileNotFound { .. }
        | WpApiError::MediaFileUnreadable { .. }
        | WpApiError::SiteUrlParsingError { .. } => false,
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

/// A tap's place in line. It waits for every earlier tap's writes that haven't finished, so the
/// server applies writes in the order the user made them. Waiting for all of them, rather than the
/// last, matters when one in between is cancelled before it's sent.
struct Turn {
    /// Dropped when the tap's writes finish or are cancelled.
    _done: oneshot::Sender<()>,
    before: Vec<Shared<oneshot::Receiver<()>>>,
}

/// The taps whose writes may not have finished, oldest first.
type Queue = Vec<Shared<oneshot::Receiver<()>>>;

impl Turn {
    fn take(queue: &mut Queue) -> Self {
        queue.retain(|earlier| earlier.clone().now_or_never().is_none());
        let (done, finished) = oneshot::channel();
        let before = queue.clone();
        queue.push(finished.shared());
        Self {
            _done: done,
            before,
        }
    }

    async fn wait(&self) {
        for earlier in &self.before {
            // Cancelled is how an earlier tap says it's done.
            let _ = earlier.clone().await;
        }
    }
}

/// A tap's writes that haven't finished. Dropping it, as Kotlin does when it cancels a call, undoes
/// the ones never sent. One already sent may have reached the server, so it stands until a read can
/// say. Swift, as of UniFFI 0.32, doesn't cancel: a cancelled Swift task still waits for the call.
struct PendingWrites<'a> {
    service: &'a ReaderSavedService,
    tap: Tap,
    saved: bool,
    /// Each write, and whether its request has been sent.
    writes: Vec<(ReaderSavedKey, WriteId, bool)>,
}

impl<'a> PendingWrites<'a> {
    /// Starts a write for each key, all at once, so the change shows straight away, and takes the
    /// tap's turn. `recent` is whether saves go in recent saves: an import's don't, because the
    /// device already had them.
    fn begin(
        service: &'a ReaderSavedService,
        keys: Vec<ReaderSavedKey>,
        saved: bool,
        recent: bool,
    ) -> (Self, Turn) {
        Self::begin_with(service, saved, recent, |_| keys)
    }

    /// Like [`Self::begin`], choosing the keys from the state under the same lock.
    fn begin_with(
        service: &'a ReaderSavedService,
        saved: bool,
        recent: bool,
        choose: impl FnOnce(&ReaderSavedState) -> Vec<ReaderSavedKey>,
    ) -> (Self, Turn) {
        let (pending, turn) = {
            let mut inner = service.inner();
            let tap = inner.state.begin_tap();
            let mut pending = Self {
                service,
                tap,
                saved,
                writes: Vec::new(),
            };
            for key in choose(&inner.state) {
                let write = inner.state.begin_write(tap, &key, saved);
                if saved && recent {
                    inner.state.note_save_tap(&key);
                }
                pending.writes.push((key, write, false));
            }
            let turn = Turn::take(&mut inner.queue);
            (pending, turn)
        };
        service.changed();
        (pending, turn)
    }

    /// Starts a write for each key, in the same tap.
    fn start(&mut self, keys: Vec<ReaderSavedKey>) -> Vec<WriteId> {
        let mut state = self.service.state();
        keys.into_iter()
            .map(|key| {
                let write = state.begin_write(self.tap, &key, self.saved);
                self.writes.push((key, write, false));
                write
            })
            .collect()
    }

    /// Starts a write, in the same tap, for each key chosen that doesn't have one yet. Returns
    /// whether it started any.
    fn add(&mut self, choose: impl FnOnce(&ReaderSavedState) -> Vec<ReaderSavedKey>) -> bool {
        let mut state = self.service.state();
        let before = self.writes.len();
        for key in choose(&state) {
            if !self
                .writes
                .iter()
                .any(|(other, _, _)| other.normalized() == key.normalized())
            {
                let write = state.begin_write(self.tap, &key, self.saved);
                self.writes.push((key, write, false));
            }
        }
        self.writes.len() != before
    }

    fn unsent(&self) -> Vec<(ReaderSavedKey, WriteId)> {
        self.writes
            .iter()
            .filter(|(_, _, sent)| !sent)
            .map(|(key, write, _)| (key.clone(), *write))
            .collect()
    }

    /// Records that a write's request is about to go out.
    fn mark_sent(&mut self, write: WriteId) {
        for (_, other, sent) in &mut self.writes {
            if *other == write {
                *sent = true;
            }
        }
    }

    fn mark_all_sent(&mut self) {
        for (_, _, sent) in &mut self.writes {
            *sent = true;
        }
    }

    /// Stops tracking a write, because the caller is about to finish it.
    fn forget(&mut self, write: WriteId) {
        self.writes.retain(|(_, other, _)| *other != write);
    }

    fn forget_all(&mut self) -> Vec<(ReaderSavedKey, WriteId)> {
        std::mem::take(&mut self.writes)
            .into_iter()
            .map(|(key, write, _)| (key, write))
            .collect()
    }
}

impl Drop for PendingWrites<'_> {
    fn drop(&mut self) {
        if self.writes.is_empty() {
            return;
        }
        let now = self.service.now();
        {
            let mut state = self.service.state();
            for (_, write, sent) in &self.writes {
                if *sent {
                    state.abandon_write(*write, now);
                } else {
                    state.fail_write(*write);
                }
            }
        }
        self.service.changed();
    }
}

fn stored_state_error(error: serde_json::Error) -> ReaderSavedError {
    ReaderSavedError::Database(SqliteDbError::SqliteError(format!(
        "Couldn't encode the saved posts state: {error}"
    )))
}

/// Reads a stored state. One that can't be read, or that an older or newer version of the library
/// stored, is left out, which only costs what a fresh install has.
fn parse_stored<T: serde::de::DeserializeOwned>(
    stored: Option<&str>,
    format: impl Fn(&T) -> u32,
) -> Option<T> {
    let stored = stored?;
    match serde_json::from_str::<T>(stored) {
        Ok(value) if format(&value) == PERSISTED_FORMAT => Some(value),
        Ok(_) => {
            log::warn!("Ignoring a saved posts state stored in another format");
            None
        }
        Err(error) => {
            log::warn!("Ignoring a saved posts state that can't be read: {error}");
            None
        }
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

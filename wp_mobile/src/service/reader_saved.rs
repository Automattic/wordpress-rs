//! Saved Reader posts, kept on this device and sent to the server in the background.
//!
//! The cache holds one row per save on the server, as far as this device knows, with what the user
//! wants for it and what the server last said. A tap changes the row straight away, in the cache,
//! and the service then sends every row the server doesn't agree with yet. The rows are the saved
//! list, so a post saved here shows in it until it's unsaved, whatever a refresh or a page says.
//!
//! Everything that uses the network goes through one lane, in order: sending rows, reading the
//! saved keys, and loading pages of the saved list. So a read on this device always starts after
//! every answer it could have missed.
//!
//! A request that gets no answer, such as one that timed out or was cancelled, leaves its row in
//! doubt: the server may have applied it. It isn't sent again until a refresh of the saved keys
//! says what the server has. Then it's sent again if it's still needed, which is safe because
//! saving and unsaving are idempotent.
//!
//! Known limits:
//!
//! - A request in doubt that the server applies after a newer opposite tap was answered undoes that
//!   tap on the server.
//! - The saved keys stop at 1,000. Past that, which only accounts that saved more before the
//!   server's cap can be, a post the keys don't have reads as not saved until the list shows it.
//! - The count is the newest total the server gave, moved by what's pending, so it can be off until
//!   the next refresh.
//! - A page saved by URL sends its details only while the app is running: a retry after a relaunch
//!   sends the URL alone.

use futures::lock::Mutex as AsyncMutex;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use wp_api::date::WpGmtDateTime;
use wp_api::prelude::{RequestExecutionErrorReason, WpApiClientDelegate, WpApiError, WpErrorCode};
use wp_api::wp_com::client::WpComApiClient;
use wp_api::wp_com::me::WpComUserId;
use wp_api::wp_com::reader_saved::{
    ReaderPostSaveIdentity, ReaderSavePostParams, ReaderSavedBulkKey, ReaderSavedBulkParams,
    ReaderSavedKey, ReaderSavedPageDetails, ReaderSavedSource, ReaderUnsavePostParams,
    reader_saved_key_for_post, reader_saved_keys_for_post,
};
use wp_mobile_cache::db_types::reader::{
    DbReaderSavedKey, DbReaderSavedRow, reader_saved_key_text,
};
use wp_mobile_cache::repository::reader::{ReaderPostRepository, ReaderSavedRepository};
use wp_mobile_cache::repository::wordpress_com_accounts::WordPressComAccountRepository;
use wp_mobile_cache::{RowId, SqliteDbError, WpApiCache};

mod storage;

pub use storage::{
    ReaderLocalSave, ReaderSavedImportReport, ReaderSavedListInfo, ReaderSavedListItem,
    ReaderSavedPageResult, ReaderSavedPostsCollection,
};

/// The most keys `POST /wpcom/v2/read/saved/bulk` takes in a call.
const MAX_BULK_KEYS: usize = 100;

/// How many times one sync sends a row, so a server that keeps changing its answer can't keep it
/// busy.
const MAX_SENDS_PER_ROW: u32 = 3;

#[allow(clippy::large_enum_variant)] // Boxing doesn't cross the FFI boundary.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum ReaderSavedError {
    #[error("{0}")]
    Api(WpApiError),

    /// The post doesn't have the IDs the API needs to save it.
    #[error("This post can't be saved.")]
    PostCannotBeSaved,

    /// An import batch got back a different number of results than the keys it sent, so no result
    /// can be matched to its key. The server may have saved any of them, so they wait for a refresh.
    #[error("The server returned {received} results for {sent} posts.")]
    UnexpectedImportResults { sent: u64, received: u64 },

    #[error("{0}")]
    Database(SqliteDbError),
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

/// Whether a post is saved, as far as this device knows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ReaderSavedStatus {
    Saved,
    NotSaved,
    /// The saved keys haven't loaded yet, and this device hasn't changed the post.
    Unknown,
}

/// What became of a save or an unsave. Both show straight away either way.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum ReaderSavedOutcome {
    /// The server has it. `key` is the key the server stores the post under, or did.
    Synced { key: ReaderSavedKey },
    /// It's on this device, and goes to the server with a later sync: the device is offline, or the
    /// server's answer didn't arrive. `reason` says why.
    Pending { reason: String },
}

/// Told when what [`ReaderSavedService`] knows about saved posts changes, on whichever thread made
/// the change.
#[uniffi::export(with_foreign)]
pub trait ReaderSavedObserver: Send + Sync {
    fn saved_posts_changed(&self);
}

/// The current account's saved Reader posts.
///
/// Saves belong to the account rather than a site, so this is separate from `WpService`. It keeps
/// everything in the cache, under the account: a save or unsave shows straight away and survives a
/// relaunch, and one the server hasn't got yet is sent by a later call. Call [`Self::sync`] at
/// launch, when the app comes to the foreground, and when the device comes back online.
///
/// [`WpApiCache::remove_wordpress_com_account`] deletes it all.
///
/// Create one per account and share it: it sends one request at a time, and a second instance
/// would send alongside it.
#[derive(uniffi::Object)]
pub struct ReaderSavedService {
    client: WpComApiClient,
    source: ReaderSavedSource,
    cache: Arc<WpApiCache>,
    /// The account's row in the cache.
    account: RowId,
    observer: Mutex<Option<Arc<dyn ReaderSavedObserver>>>,
    /// Held by everything that uses the network, so it happens one thing at a time.
    lane: AsyncMutex<()>,
    /// The details of pages saved by URL this launch, by key, for when their save is sent.
    page_details: Mutex<HashMap<String, ReaderSavedPageDetails>>,
    /// Rows the server refused this launch, and why, until the tap waiting on them takes it. The
    /// call that sends a row isn't always the tap's: any call holding the lane sends every row.
    refusals: Mutex<Vec<(Vec<RowId>, WpApiError)>>,
    clock: Clock,
}

type Clock = Arc<dyn Fn() -> SystemTime + Send + Sync>;

#[uniffi::export]
impl ReaderSavedService {
    /// Uses what the cache has for the account. `cache` must have had its migrations run.
    #[uniffi::constructor]
    pub fn new(
        delegate: WpApiClientDelegate,
        source: ReaderSavedSource,
        cache: Arc<WpApiCache>,
        user_id: WpComUserId,
    ) -> Result<Self, ReaderSavedError> {
        Self::with_clock(delegate, source, cache, user_id, Arc::new(SystemTime::now))
    }

    pub fn set_observer(&self, observer: Option<Arc<dyn ReaderSavedObserver>>) {
        *self.observer.lock().expect("observer lock poisoned") = observer;
    }

    /// Whether a post is saved. Doesn't touch the network.
    pub fn status(
        &self,
        post: ReaderPostSaveIdentity,
    ) -> Result<ReaderSavedStatus, ReaderSavedError> {
        self.status_of(&reader_saved_keys_for_post(post))
    }

    /// How many saves there are, or `None` until the first refresh or answer. A post saved under
    /// two keys counts twice, as the server counts it.
    pub fn saved_count(&self) -> Result<Option<u64>, ReaderSavedError> {
        Ok(self
            .cache
            .execute(|connection| ReaderSavedRepository::saved_count(connection, self.account))?)
    }

    /// Saves a post. It reads as saved, and shows in the saved list, straight away.
    pub async fn save(
        &self,
        post: ReaderPostSaveIdentity,
    ) -> Result<ReaderSavedOutcome, ReaderSavedError> {
        let main = reader_saved_key_for_post(post.clone())
            .ok_or(ReaderSavedError::PostCannotBeSaved)?
            .normalized();
        let keys = normalized(&reader_saved_keys_for_post(post));
        self.save_keys(main, keys).await
    }

    /// Saves a page by URL. The URL of a WordPress.com post is saved as the post.
    pub async fn save_url(
        &self,
        url: String,
        page_details: Option<ReaderSavedPageDetails>,
    ) -> Result<ReaderSavedOutcome, ReaderSavedError> {
        let key = ReaderSavedKey::Url { url }.normalized();
        if let Some(details) = page_details {
            self.page_details
                .lock()
                .expect("page details lock poisoned")
                .insert(reader_saved_key_text(&key), details);
        }
        self.save_keys(key.clone(), vec![key]).await
    }

    /// Unsaves a post from every key it may be saved under. It reads as unsaved straight away.
    pub async fn unsave(
        &self,
        post: ReaderPostSaveIdentity,
    ) -> Result<ReaderSavedOutcome, ReaderSavedError> {
        let keys = normalized(&reader_saved_keys_for_post(post));
        if keys.is_empty() {
            return Err(ReaderSavedError::PostCannotBeSaved);
        }
        self.unsave_keys(keys).await
    }

    /// Unsaves a key, such as a saved list item's. If the server stores it as another key, that's
    /// unsaved too.
    pub async fn unsave_key(
        &self,
        key: ReaderSavedKey,
    ) -> Result<ReaderSavedOutcome, ReaderSavedError> {
        self.unsave_keys(vec![key.normalized()]).await
    }

    /// Sends what the server doesn't have yet, then reloads the saved keys. Call it at launch, when
    /// the app comes to the foreground, and when the device comes back online.
    pub async fn sync(&self) -> Result<(), ReaderSavedError> {
        let _lane = self.lane.lock().await;
        self.sync_in_lane().await
    }
}

impl ReaderSavedService {
    fn with_clock(
        delegate: WpApiClientDelegate,
        source: ReaderSavedSource,
        cache: Arc<WpApiCache>,
        user_id: WpComUserId,
        clock: Clock,
    ) -> Result<Self, ReaderSavedError> {
        let account = cache.execute(|connection| {
            WordPressComAccountRepository::get_or_create(connection, user_id)
        })?;
        Ok(Self {
            client: WpComApiClient::new(delegate),
            source,
            cache,
            account,
            observer: Mutex::default(),
            lane: AsyncMutex::new(()),
            page_details: Mutex::default(),
            refusals: Mutex::default(),
            clock,
        })
    }

    fn now(&self) -> WpGmtDateTime {
        WpGmtDateTime(chrono::DateTime::<chrono::Utc>::from((self.clock)()))
    }

    /// Whether a post is saved, given every key it may be saved under.
    fn status_of(&self, keys: &[ReaderSavedKey]) -> Result<ReaderSavedStatus, ReaderSavedError> {
        if keys.is_empty() {
            return Ok(ReaderSavedStatus::NotSaved);
        }
        let keys = normalized(keys);
        let (rows, state) = self.cache.execute(|connection| {
            Ok::<_, SqliteDbError>((
                ReaderSavedRepository::select_rows_for_keys(connection, self.account, &keys)?,
                ReaderSavedRepository::select_state(connection, self.account)?,
            ))
        })?;
        Ok(if rows.iter().any(|row| row.saved) {
            ReaderSavedStatus::Saved
        } else if state.keys_refreshed || !rows.is_empty() {
            ReaderSavedStatus::NotSaved
        } else {
            ReaderSavedStatus::Unknown
        })
    }

    async fn save_keys(
        &self,
        main: ReaderSavedKey,
        keys: Vec<ReaderSavedKey>,
    ) -> Result<ReaderSavedOutcome, ReaderSavedError> {
        let now = self.now();
        let row = self.cache.execute(|connection| {
            let transaction = connection.transaction()?;
            let post = ReaderPostRepository::select_by_key(&transaction, self.account, &main)?
                .map(|(row, _)| row);
            let row = ReaderSavedRepository::tap_save(
                &transaction,
                self.account,
                &main,
                &keys,
                post,
                &now,
            )?;
            transaction.commit().map_err(SqliteDbError::from)?;
            Ok::<_, SqliteDbError>(row)
        })?;
        self.forget_refusals(&[row]);
        self.notify();

        let _lane = self.lane.lock().await;
        let sent = self.send_pending().await?;
        if let Some(error) = self.take_refusal(&[row]) {
            return Err(ReaderSavedError::Api(error));
        }
        let rows = self.rows_for(std::slice::from_ref(&main))?;
        Ok(match rows.iter().find(|row| row.saved) {
            Some(row) if row.server == Some(true) || row.local_only => ReaderSavedOutcome::Synced {
                key: row.key.clone(),
            },
            Some(row) => sent.pending_reason(row),
            // An unsave tapped since decides.
            None => ReaderSavedOutcome::Pending {
                reason: "The save was changed before it was sent.".to_string(),
            },
        })
    }

    async fn unsave_keys(
        &self,
        keys: Vec<ReaderSavedKey>,
    ) -> Result<ReaderSavedOutcome, ReaderSavedError> {
        let waiting = self.cache.execute(|connection| {
            let transaction = connection.transaction()?;
            // Past the keys a refresh returned, or before any, a key without a row may still be
            // saved, so it's sent too.
            let state = ReaderSavedRepository::select_state(&transaction, self.account)?;
            let rows = ReaderSavedRepository::tap_unsave(
                &transaction,
                self.account,
                &keys,
                !state.keys_complete,
            )?;
            transaction.commit().map_err(SqliteDbError::from)?;
            Ok::<_, SqliteDbError>(rows)
        })?;
        self.forget_refusals(&waiting);
        self.notify();

        let _lane = self.lane.lock().await;
        let sent = self.send_pending().await?;
        if let Some(error) = self.take_refusal(&waiting) {
            return Err(ReaderSavedError::Api(error));
        }
        let rows = self.rows_for(&keys)?;
        Ok(
            match rows.iter().find(|row| row.is_pending() || row.saved) {
                Some(row) if !row.saved => sent.pending_reason(row),
                // A save tapped since decides.
                Some(row) => ReaderSavedOutcome::Synced {
                    key: row.key.clone(),
                },
                None => ReaderSavedOutcome::Synced {
                    key: rows.first().map_or(keys[0].clone(), |row| row.key.clone()),
                },
            },
        )
    }

    fn rows_for(&self, keys: &[ReaderSavedKey]) -> Result<Vec<DbReaderSavedRow>, ReaderSavedError> {
        Ok(self.cache.execute(|connection| {
            ReaderSavedRepository::select_rows_for_keys(connection, self.account, keys)
        })?)
    }

    /// Sends pending rows, then reads the saved keys, then sends what that shows is still needed.
    /// Call it holding the lane.
    async fn sync_in_lane(&self) -> Result<(), ReaderSavedError> {
        let sent = self.send_pending().await?;
        sent.log();
        let response = self.client.reader_saved().list_keys().await?.data;
        let returned = response.keys.len() as u64;
        let keys: Vec<_> = response
            .keys
            .into_iter()
            .map(|entry| DbReaderSavedKey {
                key: entry.key,
                date_saved: entry.date_saved,
                url: entry.url,
            })
            .collect();
        self.cache.execute(|connection| {
            let transaction = connection.transaction()?;
            ReaderSavedRepository::apply_keys(
                &transaction,
                self.account,
                &keys,
                returned,
                response.total,
            )?;
            transaction.commit().map_err(SqliteDbError::from)
        })?;
        self.notify();
        self.send_pending().await?.log();
        Ok(())
    }

    /// Sends every pending row, oldest tap first, until none is left or the device is offline. Call
    /// it holding the lane.
    async fn send_pending(&self) -> Result<Sent, ReaderSavedError> {
        let mut sent = Sent::default();
        let mut sends: HashMap<RowId, u32> = HashMap::new();
        loop {
            let pending = self.cache.execute(|connection| {
                ReaderSavedRepository::select_pending(connection, self.account)
            })?;
            let Some(row) = pending
                .iter()
                .find(|row| sends.get(&row.row_id).copied().unwrap_or(0) < MAX_SENDS_PER_ROW)
                .cloned()
            else {
                return Ok(sent);
            };
            let batch: Vec<_> = if row.imported && row.saved && bulk_key(&row.key).is_some() {
                pending
                    .into_iter()
                    .filter(|other| other.imported && other.saved && bulk_key(&other.key).is_some())
                    .take(MAX_BULK_KEYS)
                    .collect()
            } else {
                vec![row]
            };
            for row in &batch {
                *sends.entry(row.row_id).or_default() += 1;
            }
            let go_on = if batch.len() > 1 {
                self.send_bulk(&batch, &mut sent).await?
            } else {
                self.send_row(&batch[0], &mut sent).await?
            };
            self.notify();
            if !go_on {
                return Ok(sent);
            }
        }
    }

    /// Sends one row. Returns whether to go on to the next.
    async fn send_row(
        &self,
        row: &DbReaderSavedRow,
        sent: &mut Sent,
    ) -> Result<bool, ReaderSavedError> {
        self.set_in_doubt(&[row.row_id], true)?;
        let result = if row.saved {
            let page_details = self
                .page_details
                .lock()
                .expect("page details lock poisoned")
                .get(&reader_saved_key_text(&row.key))
                .cloned();
            let params = ReaderSavePostParams {
                key: row.key.clone(),
                source: self.source,
                page_details,
            };
            self.client
                .reader_saved()
                .save(&params)
                .await
                .map(|response| (response.data.key, response.data.total))
        } else {
            let params = ReaderUnsavePostParams {
                key: row.key.clone(),
                source: self.source,
            };
            self.client
                .reader_saved()
                .unsave(&params)
                .await
                .map(|response| (response.data.key, response.data.total))
        };
        let answer = match result {
            Ok((stored, total)) => Ok((stored, Some(total))),
            // Nothing can be saved under a feed item the server can't find, unless this device has
            // seen it saved there: a feed item stops resolving once it's pruned.
            Err(error) if !row.saved && row.server != Some(true) && is_post_not_found(&error) => {
                Ok((row.key.clone(), None))
            }
            Err(error) => Err(error),
        };
        match answer {
            Ok((stored, total)) => {
                self.cache.execute(|connection| {
                    let transaction = connection.transaction()?;
                    ReaderSavedRepository::apply_answer(
                        &transaction,
                        self.account,
                        row.row_id,
                        row.saved,
                        &stored,
                        total,
                    )?;
                    transaction.commit().map_err(SqliteDbError::from)
                })?;
                if row.saved {
                    self.page_details
                        .lock()
                        .expect("page details lock poisoned")
                        .remove(&reader_saved_key_text(&row.key));
                }
                Ok(true)
            }
            Err(error) => self.failed(&[row.row_id], error, sent),
        }
    }

    /// Sends imported saves in one bulk request. Returns whether to go on.
    async fn send_bulk(
        &self,
        rows: &[DbReaderSavedRow],
        sent: &mut Sent,
    ) -> Result<bool, ReaderSavedError> {
        let row_ids: Vec<_> = rows.iter().map(|row| row.row_id).collect();
        self.set_in_doubt(&row_ids, true)?;
        // The rows are oldest first, and the API takes the newest first.
        let params = ReaderSavedBulkParams {
            keys: rows
                .iter()
                .rev()
                .filter_map(|row| bulk_key(&row.key))
                .collect(),
            source: self.source,
        };
        let response = match self.client.reader_saved().bulk_save(&params).await {
            Ok(response) => response.data,
            Err(error) => return self.failed(&row_ids, error, sent),
        };
        // Results are matched to keys by position, which only works with one per key. The server
        // may have saved any of them, so they stay in doubt.
        if response.results.len() != rows.len() {
            sent.unexpected = Some(ReaderSavedError::UnexpectedImportResults {
                sent: rows.len() as u64,
                received: response.results.len() as u64,
            });
            return Ok(false);
        }
        self.cache.execute(|connection| {
            let transaction = connection.transaction()?;
            ReaderSavedRepository::set_total(&transaction, self.account, response.total)?;
            for (row, result) in rows.iter().rev().zip(&response.results) {
                match (&result.key, result.saved) {
                    (Some(stored), true) => {
                        ReaderSavedRepository::apply_answer(
                            &transaction,
                            self.account,
                            row.row_id,
                            true,
                            &saved_key(stored),
                            None,
                        )?;
                    }
                    _ => ReaderSavedRepository::apply_refusal(&transaction, row.row_id)?,
                }
            }
            transaction.commit().map_err(SqliteDbError::from)
        })?;
        Ok(true)
    }

    /// Records a request that failed. Returns whether to go on to the next row.
    fn failed(
        &self,
        rows: &[RowId],
        error: WpApiError,
        sent: &mut Sent,
    ) -> Result<bool, ReaderSavedError> {
        if is_refused(&error) {
            self.cache.execute(|connection| {
                let transaction = connection.transaction()?;
                for row in rows {
                    ReaderSavedRepository::apply_refusal(&transaction, *row)?;
                }
                transaction.commit().map_err(SqliteDbError::from)
            })?;
            log::warn!("The server refused a saved post change: {error}");
            self.refusals
                .lock()
                .expect("refusals lock poisoned")
                .push((rows.to_vec(), error));
            Ok(true)
        } else if may_have_been_applied(&error) {
            // It stays in doubt until a refresh says. The next row may still get through.
            sent.in_doubt = Some(error);
            Ok(true)
        } else {
            // It never reached the server, and nothing else will now.
            self.set_in_doubt(rows, false)?;
            sent.unsent = Some(error);
            Ok(false)
        }
    }

    /// Drops what's known about earlier refusals of rows a new tap changed.
    fn forget_refusals(&self, rows: &[RowId]) {
        self.refusals
            .lock()
            .expect("refusals lock poisoned")
            .retain(|(refused, _)| !refused.iter().any(|row| rows.contains(row)));
    }

    /// Why the server refused one of these rows, if it did since they were tapped.
    fn take_refusal(&self, rows: &[RowId]) -> Option<WpApiError> {
        let mut refusals = self.refusals.lock().expect("refusals lock poisoned");
        let index = refusals
            .iter()
            .position(|(refused, _)| refused.iter().any(|row| rows.contains(row)))?;
        Some(refusals.remove(index).1)
    }

    fn set_in_doubt(&self, rows: &[RowId], in_doubt: bool) -> Result<(), ReaderSavedError> {
        self.cache.execute(|connection| {
            let transaction = connection.transaction()?;
            for row in rows {
                ReaderSavedRepository::set_in_doubt(&transaction, *row, in_doubt)?;
            }
            transaction.commit().map_err(SqliteDbError::from)
        })?;
        Ok(())
    }

    /// Never called with a lock the observer's calls take, because the observer is app code that
    /// reads the state back.
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

/// What a round of sending did.
#[derive(Debug, Default)]
struct Sent {
    /// A request that didn't reach the server, which stopped the round.
    unsent: Option<WpApiError>,
    /// A request whose answer didn't arrive.
    in_doubt: Option<WpApiError>,
    unexpected: Option<ReaderSavedError>,
}

impl Sent {
    fn pending_reason(&self, row: &DbReaderSavedRow) -> ReaderSavedOutcome {
        let reason = if row.in_doubt {
            match &self.in_doubt {
                Some(error) => format!("No answer arrived, so it waits for a refresh: {error}"),
                None => "No answer arrived, so it waits for a refresh.".to_string(),
            }
        } else {
            match &self.unsent {
                Some(error) => format!("It wasn't sent: {error}"),
                None => "It wasn't sent yet.".to_string(),
            }
        };
        ReaderSavedOutcome::Pending { reason }
    }

    /// Logs what went wrong for rows that aren't a tap's, which wait for a later sync.
    fn log(&self) {
        if let Some(error) = &self.unsent {
            log::warn!("Couldn't send a saved post change: {error}");
        }
        if let Some(error) = &self.unexpected {
            log::warn!("{error}");
        }
    }
}

fn normalized(keys: &[ReaderSavedKey]) -> Vec<ReaderSavedKey> {
    keys.iter().map(ReaderSavedKey::normalized).collect()
}

/// Whether a failed request may still have been applied: the request may have reached the server,
/// and the server didn't say it refused it. That's a timeout, a connection lost or cancelled
/// mid-request, an answer that couldn't be read, or a server error (5xx).
///
/// Errors from before the request was sent didn't reach it: no network, a host that doesn't
/// resolve, a refused connection, TLS. Nor did an answer that says it wasn't applied.
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

/// Whether the server refused a request, rather than it failing on the way. An answer that may pass,
/// such as a rate limit or an expired token, isn't a refusal: the row is sent again later. The one
/// forbidden answer that won't pass is the account's save cap.
fn is_refused(error: &WpApiError) -> bool {
    /// Unauthorized, forbidden, request timeout, too many requests.
    const TEMPORARY: [u32; 4] = [401, 403, 408, 429];
    match error {
        WpApiError::WpError {
            error_code: WpErrorCode::CustomError(code),
            ..
        } if code == "saved_limit_reached" => true,
        WpApiError::WpError { status_code, .. }
        | WpApiError::InvalidHttpStatusCode { status_code, .. }
        | WpApiError::UnknownError { status_code, .. } => {
            (400..500).contains(status_code) && !TEMPORARY.contains(status_code)
        }
        _ => false,
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

fn bulk_key(key: &ReaderSavedKey) -> Option<ReaderSavedBulkKey> {
    match key {
        ReaderSavedKey::Post { blog_id, post_id } => Some(ReaderSavedBulkKey::Post {
            blog_id: *blog_id,
            post_id: *post_id,
        }),
        ReaderSavedKey::FeedItem { feed_item_id } => Some(ReaderSavedBulkKey::FeedItem {
            feed_item_id: *feed_item_id,
        }),
        ReaderSavedKey::Url { .. } => None,
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

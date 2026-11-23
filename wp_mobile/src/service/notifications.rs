//! A WordPress.com account's notifications, kept in the cache and synced by comparing hashes.
//!
//! A sync lists the newest notes' IDs and hashes, downloads only the notes that are new or
//! changed, and removes the notes the server no longer lists. Older notes load a page at a time,
//! back to the account's oldest note.
//!
//! What the API does that the sync is built around:
//!
//! - A note's hash only compares with a hash from a request made the same way. Fetching a note
//!   whole gives it a new hash, which a list of IDs and hashes then returns until the note
//!   changes or another client fetches it whole. So a note is kept with the hash it came with
//!   whole, not the hash the check had. A note another client fetched in between is downloaded
//!   again, though it didn't change.
//! - A request in a different `locale` from the account's previous one changes every hash, so
//!   every request sends the same form of one language.
//! - A list of IDs has notes the API never sends whole. Each is remembered with the hash the list
//!   gave it, and asked for again only when that hash changes.
//! - A short or empty page of whole notes isn't the end, and a page that left a note out can be
//!   refilled with an older one. The end is a list of IDs shorter than asked for.
//! - `before` includes its second, so the next page starts at the oldest note's second, and no
//!   note sharing it is skipped.

use futures::lock::Mutex as AsyncMutex;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use wp_api::date::WpGmtDateTime;
use wp_api::prelude::{WpApiClientDelegate, WpApiError};
use wp_api::wp_com::client::WpComApiClient;
use wp_api::wp_com::language::WPComLanguage;
use wp_api::wp_com::me::WpComUserId;
use wp_api::wp_com::notifications::{
    NotificationId, NotificationType, NotificationsParams, NotificationsQuery, WpComNotification,
};
use wp_mobile_cache::repository::notifications::{
    DbKnownHash, DbNotificationFilter, DbNotificationWindow, NotificationRepository, UNDATED_BEFORE,
};
use wp_mobile_cache::repository::wordpress_com_accounts::WordPressComAccountRepository;
use wp_mobile_cache::{DbTable, RowId, SqliteDbError, UpdateHook, WpApiCache};

/// How many of the newest notes a sync checks, and how many notes an older page has.
const PAGE_SIZE: u32 = 100;

/// The most notes the API lists in one request.
const MAX_PAGE_SIZE: u32 = 200;

/// How many notes are downloaded whole in one request.
const DOWNLOAD_AT_A_TIME: usize = 100;

/// The tables the cached notes and their state are read from.
const NOTIFICATION_TABLES: [DbTable; 3] = [
    DbTable::Notifications,
    DbTable::NotificationListedOnly,
    DbTable::NotificationSyncState,
];

#[allow(clippy::large_enum_variant)] // Boxing doesn't cross the FFI boundary.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum NotificationServiceError {
    #[error("{0}")]
    Api(WpApiError),

    #[error("{0}")]
    Database(SqliteDbError),

    /// Loading older notes can't move on without skipping some: more notes share one second, or
    /// more notes in a row can't be sent whole, than the largest page holds. Nothing changed.
    #[error("Older notifications can't be loaded past this point without skipping some.")]
    CannotPageFurther,
}

impl From<WpApiError> for NotificationServiceError {
    fn from(err: WpApiError) -> Self {
        Self::Api(err)
    }
}

impl From<SqliteDbError> for NotificationServiceError {
    fn from(err: SqliteDbError) -> Self {
        Self::Database(err)
    }
}

/// What a sync or a page of older notes did.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct NotificationSyncResult {
    /// How many notes the list of IDs and hashes had.
    pub listed_count: u64,
    /// How many notes were downloaded whole, because they were new or had changed.
    pub downloaded_count: u64,
    /// How many notes were asked for whole and didn't come, or came and couldn't be read. They
    /// aren't asked for again until their hash changes.
    pub not_sent_count: u64,
    /// How many cached notes were removed because the server no longer lists them.
    pub removed_count: u64,
    /// Whether the oldest note has been reached, so [`NotificationService::load_older`] has
    /// nothing more to load.
    pub reached_end: bool,
}

/// Which cached notes [`NotificationService::load_notes`] returns.
#[derive(Debug, Clone, Default, PartialEq, Eq, uniffi::Record)]
pub struct NotificationsFilter {
    /// Only notes the user hasn't read.
    #[uniffi(default = false)]
    pub unread_only: bool,
    /// Only notes of these types. Empty for every type.
    #[uniffi(default = [])]
    pub types: Vec<NotificationType>,
}

/// What the cache has of the account's notifications.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct NotificationsInfo {
    /// How many notes are cached whole.
    pub note_count: u64,
    /// How many notes the API lists and never sends whole, as far as the syncs so far have seen.
    pub listed_only_count: u64,
    /// How many of those came and couldn't be read.
    pub unreadable_count: u64,
    /// Whether the oldest note has been reached.
    pub reached_end: bool,
    /// When the newest notes were last synced. `None` before the first sync.
    #[uniffi(default = None)]
    pub last_synced_at: Option<WpGmtDateTime>,
    /// When the user last looked at their notifications, as the API last said.
    #[uniffi(default = None)]
    pub last_seen_time: Option<WpGmtDateTime>,
}

pub(crate) type Clock = Arc<dyn Fn() -> SystemTime + Send + Sync>;

/// A WordPress.com account's notifications: read from the cache, and kept up to date from the
/// server. Every request it sends is a GET: nothing here marks a note read or seen.
///
/// Create one per account and share it. Read the cached notes with [`Self::load_notes`], which
/// doesn't touch the network, and read them again when [`Self::is_relevant_update`] says a change
/// to the cache is one they show. [`Self::sync`] brings the newest notes up to date, and
/// [`Self::load_older`] loads the next older page, until [`NotificationSyncResult::reached_end`].
///
/// Which notes the API sends can depend on the request's `User-Agent`, which the delegate's
/// request executor sets, not this service.
///
/// [`WpApiCache::remove_wordpress_com_account`] deletes everything it keeps for the account.
#[derive(uniffi::Object)]
pub struct NotificationService {
    client: WpComApiClient,
    cache: Arc<WpApiCache>,
    user_id: WpComUserId,
    /// The account's row in the cache.
    account: RowId,
    language: Mutex<Option<WPComLanguage>>,
    /// Held while a sync or a page of older notes runs, so they take turns.
    lane: AsyncMutex<()>,
    clock: Clock,
    /// How many of the newest notes a sync checks, and how many notes an older page has.
    page_size: u32,
}

#[uniffi::export]
impl NotificationService {
    /// Uses what the cache has for the account. `cache` must have had its migrations run.
    #[uniffi::constructor]
    pub fn new(
        delegate: WpApiClientDelegate,
        cache: Arc<WpApiCache>,
        user_id: WpComUserId,
    ) -> Result<Self, NotificationServiceError> {
        Self::with_clock(delegate, cache, user_id, Arc::new(SystemTime::now))
    }

    /// The account the notifications belong to.
    pub fn user_id(&self) -> WpComUserId {
        self.user_id
    }

    /// Sets the language the notes are written in: pass the account's or the app's. `None`, the
    /// default, is English.
    ///
    /// Every request sends it as its WordPress.com locale, such as `en` or `pt-br`, including the
    /// lists of hashes, because a request in a different `locale` from the account's previous
    /// one changes every note's hash. So the sync after a change downloads every note again, in
    /// the new language. The delegate's `language_provider` isn't used.
    pub fn set_language(&self, language: Option<WPComLanguage>) {
        *self.language.lock().expect("language lock poisoned") = language;
    }

    /// The language set with [`Self::set_language`].
    pub fn language(&self) -> Option<WPComLanguage> {
        *self.language.lock().expect("language lock poisoned")
    }

    /// Brings the newest notes up to date: lists the newest 100 notes' IDs and hashes, downloads
    /// the ones that are new or changed, and removes cached notes the list should have had and
    /// didn't. A call made while another sync or page is running waits for it.
    ///
    /// If more notes arrived since the last sync than the list holds, the notes between them and
    /// the cached ones are loaded by [`Self::load_older`] again, so none is missed.
    pub async fn sync(&self) -> Result<NotificationSyncResult, NotificationServiceError> {
        let _lane = self.lane.lock().await;
        let (had_notes, mut state) = self.cache.execute(|connection| {
            let notes = NotificationRepository::count(&*connection, self.account)?
                + NotificationRepository::count_listed_only(&*connection, self.account)?.0;
            Ok::<_, SqliteDbError>((
                notes > 0,
                NotificationRepository::sync_state(&*connection, self.account)?,
            ))
        })?;

        let window = self.sync_window(None, self.page_size).await?;

        state.last_synced_at = Some(self.now());
        state.last_seen_time = window.last_seen_time.or(state.last_seen_time);
        let gap = had_notes && !window.reached_cached_notes;
        if gap || state.older_number.is_none() || window.is_short() {
            match next_position(None, self.page_size, self.page_size, &window) {
                Position::Page { before, number } => {
                    state.older_before = before;
                    state.older_number = Some(number);
                    state.reached_end = false;
                }
                Position::End => state.reached_end = true,
                // The newest notes already fill a page at one second: the next page asks for
                // the most it can, and stops there if that's still not enough.
                Position::Stuck => {
                    state.older_before = None;
                    state.older_number = Some(MAX_PAGE_SIZE);
                    state.reached_end = false;
                }
            }
        }
        self.cache.execute(|connection| {
            NotificationRepository::set_sync_state(&*connection, self.account, &state)
        })?;
        Ok(window.result(state.reached_end))
    }

    /// Loads the next older page of notes, the same way [`Self::sync`] brings the newest up to
    /// date. Once the oldest note has been reached, it sends nothing and says so.
    ///
    /// Fails with [`NotificationServiceError::CannotPageFurther`] if it can't move on without
    /// skipping notes.
    pub async fn load_older(&self) -> Result<NotificationSyncResult, NotificationServiceError> {
        let _lane = self.lane.lock().await;
        let mut state = self
            .cache
            .execute(|connection| NotificationRepository::sync_state(&*connection, self.account))?;
        if state.reached_end {
            return Ok(NotificationSyncResult {
                listed_count: 0,
                downloaded_count: 0,
                not_sent_count: 0,
                removed_count: 0,
                reached_end: true,
            });
        }
        let (before, number) = (
            state.older_before,
            state.older_number.unwrap_or(self.page_size),
        );
        let window = self.sync_window(before, number).await?;

        state.last_seen_time = window.last_seen_time.or(state.last_seen_time);
        match next_position(before, number, self.page_size, &window) {
            Position::Page { before, number } => {
                state.older_before = before;
                state.older_number = Some(number);
            }
            Position::End => state.reached_end = true,
            Position::Stuck => return Err(NotificationServiceError::CannotPageFurther),
        }
        self.cache.execute(|connection| {
            NotificationRepository::set_sync_state(&*connection, self.account, &state)
        })?;
        Ok(window.result(state.reached_end))
    }

    /// The cached notes that pass `filter`, newest first, each read from the JSON it came as. A
    /// note with no date is last. Doesn't touch the network.
    pub fn load_notes(
        &self,
        filter: NotificationsFilter,
    ) -> Result<Vec<WpComNotification>, NotificationServiceError> {
        let rows = self.cache.execute(|connection| {
            NotificationRepository::select(
                &*connection,
                self.account,
                &DbNotificationFilter {
                    unread_only: filter.unread_only,
                    types: filter.types,
                },
            )
        })?;
        Ok(rows
            .into_iter()
            .filter_map(
                |row| match serde_json::from_str::<WpComNotification>(&row.json) {
                    Ok(note) => Some(note),
                    // Every stored note was read before it was stored, so only a change in how
                    // notes are read can make one unreadable.
                    Err(error) => {
                        log::error!("A cached notification couldn't be read: {error}");
                        None
                    }
                },
            )
            .collect())
    }

    /// What the cache has of the account's notifications. Doesn't touch the network.
    pub fn info(&self) -> Result<NotificationsInfo, NotificationServiceError> {
        Ok(self.cache.execute(|connection| {
            let state = NotificationRepository::sync_state(&*connection, self.account)?;
            let (listed_only_count, unreadable_count) =
                NotificationRepository::count_listed_only(&*connection, self.account)?;
            Ok::<_, SqliteDbError>(NotificationsInfo {
                note_count: NotificationRepository::count(&*connection, self.account)?,
                listed_only_count,
                unreadable_count,
                reached_end: state.reached_end,
                last_synced_at: state.last_synced_at.and_then(date),
                last_seen_time: state.last_seen_time.and_then(date),
            })
        })?)
    }

    /// Whether a change to the cache is one that [`Self::load_notes`] or [`Self::info`] shows.
    pub fn is_relevant_update(&self, hook: &UpdateHook) -> bool {
        NOTIFICATION_TABLES.contains(&hook.table)
    }
}

impl NotificationService {
    pub(crate) fn with_clock(
        delegate: WpApiClientDelegate,
        cache: Arc<WpApiCache>,
        user_id: WpComUserId,
        clock: Clock,
    ) -> Result<Self, NotificationServiceError> {
        let account = cache.execute(|connection| {
            WordPressComAccountRepository::get_or_create(connection, user_id)
        })?;
        Ok(Self {
            // The service sends its own language: see `set_language`.
            client: WpComApiClient::new(WpApiClientDelegate {
                language_provider: None,
                ..delegate
            }),
            cache,
            user_id,
            account,
            language: Mutex::default(),
            lane: AsyncMutex::new(()),
            clock,
            page_size: PAGE_SIZE,
        })
    }

    /// Checks and pages `page_size` notes at a time instead of 100.
    #[cfg(test)]
    pub(crate) fn with_page_size(mut self, page_size: u32) -> Self {
        self.page_size = page_size;
        self
    }

    fn now(&self) -> i64 {
        chrono::DateTime::<chrono::Utc>::from((self.clock)()).timestamp()
    }

    /// The language every request is sent in.
    fn locale(&self) -> WPComLanguage {
        self.language().unwrap_or(WPComLanguage::English)
    }

    /// Checks the `number` newest notes at or before `before`, downloads the new and changed
    /// ones, and removes the cached notes the list covers and didn't have.
    async fn sync_window(
        &self,
        before: Option<i64>,
        number: u32,
    ) -> Result<Window, NotificationServiceError> {
        let locale = self.locale();
        let check = self
            .client
            .notifications()
            .list_hashes(&NotificationsQuery {
                number: Some(number),
                before: before.and_then(date),
                locale: Some(locale),
                ..Default::default()
            })
            .await?
            .data;
        // An entry that couldn't be read still takes a place in the list.
        let listed_count = check.notes.len() + check.unreadable_notes.len();
        let listed_ids: Vec<NotificationId> = check.notes.iter().map(|note| note.id).collect();
        let listed_hashes: HashMap<NotificationId, Option<u64>> = check
            .notes
            .iter()
            .map(|note| (note.id, note.note_hash))
            .collect();

        let known = self.cache.execute(|connection| {
            NotificationRepository::known_hashes(&*connection, self.account, &listed_ids)
        })?;
        let mut seen = HashSet::new();
        let wanted: Vec<NotificationId> = listed_ids
            .iter()
            .filter(|id| seen.insert(**id))
            .filter(|id| {
                let listed = listed_hashes[id].map(|hash| hash.to_string());
                let kept = match known.get(id) {
                    Some(DbKnownHash::Whole(hash) | DbKnownHash::ListedOnly(hash)) => hash,
                    None => return true,
                };
                listed.is_none() || listed != *kept
            })
            .copied()
            .collect();

        let mut downloaded_count = 0;
        let mut not_sent_count = 0;
        for chunk in wanted.chunks(DOWNLOAD_AT_A_TIME) {
            let page = self
                .client
                .notifications()
                .list_with_json(&NotificationsParams {
                    query: NotificationsQuery {
                        number: Some(chunk.len() as u32),
                        ids: chunk.to_vec(),
                        locale: Some(locale),
                        ..Default::default()
                    },
                    include_body: true,
                    include_context: true,
                })
                .await?
                .data;
            let requested: HashSet<NotificationId> = chunk.iter().copied().collect();
            let (downloaded, not_sent) = self.cache.execute(|connection| {
                let transaction = connection.transaction()?;
                let mut came = HashSet::new();
                for note in &page.notes {
                    if requested.contains(&note.note.id) && came.insert(note.note.id) {
                        NotificationRepository::upsert(&transaction, self.account, note)?;
                    }
                }
                let downloaded = came.len() as u64;
                for unreadable in &page.unreadable_notes {
                    if let Some(id) = unreadable.id
                        && requested.contains(&id)
                        && came.insert(id)
                    {
                        NotificationRepository::record_listed_only(
                            &transaction,
                            self.account,
                            id,
                            listed_hashes[&id],
                            Some(&unreadable.error),
                        )?;
                    }
                }
                for id in chunk {
                    if !came.contains(id) {
                        NotificationRepository::record_listed_only(
                            &transaction,
                            self.account,
                            *id,
                            listed_hashes[id],
                            None,
                        )?;
                    }
                }
                transaction.commit()?;
                Ok::<_, SqliteDbError>((downloaded, chunk.len() as u64 - downloaded))
            })?;
            downloaded_count += downloaded;
            not_sent_count += not_sent;
        }

        let listed: HashSet<NotificationId> = listed_ids.iter().copied().collect();
        let (times, removed_count) = self.cache.execute(|connection| {
            let transaction = connection.transaction()?;
            let times = NotificationRepository::times(&transaction, self.account, &listed_ids)?;
            let times: Vec<Option<i64>> = listed_ids
                .iter()
                .filter_map(|id| times.get(id).copied())
                .collect();
            let window = if listed_count < number as usize {
                Some(match before {
                    None => DbNotificationWindow::Everything,
                    Some(before) => DbNotificationWindow::AtOrBefore { before },
                })
            } else {
                oldest_dated(&times)
                    .map(|after| DbNotificationWindow::DatedBetween { after, before })
            };
            let removed = match window {
                Some(window) => NotificationRepository::delete_unlisted(
                    &transaction,
                    self.account,
                    window,
                    &listed,
                )?,
                None => 0,
            };
            transaction.commit()?;
            Ok::<_, SqliteDbError>((times, removed))
        })?;

        Ok(Window {
            listed_count,
            number,
            times,
            reached_cached_notes: !known.is_empty(),
            last_seen_time: check.last_seen_time.map(|time| time.0.timestamp()),
            downloaded_count,
            not_sent_count,
            removed_count,
        })
    }
}

/// What checking one window of notes found.
struct Window {
    /// How many notes the list of IDs had.
    listed_count: usize,
    /// How many it asked for.
    number: u32,
    /// The times of the listed notes that are cached whole, in seconds since 1970.
    times: Vec<Option<i64>>,
    /// Whether the list had a note the cache already knew.
    reached_cached_notes: bool,
    last_seen_time: Option<i64>,
    downloaded_count: u64,
    not_sent_count: u64,
    removed_count: u64,
}

impl Window {
    /// Whether the list had fewer notes than asked for, so it had every note there is from where
    /// it started.
    fn is_short(&self) -> bool {
        self.listed_count < self.number as usize
    }

    fn result(&self, reached_end: bool) -> NotificationSyncResult {
        NotificationSyncResult {
            listed_count: self.listed_count as u64,
            downloaded_count: self.downloaded_count,
            not_sent_count: self.not_sent_count,
            removed_count: self.removed_count,
            reached_end,
        }
    }
}

/// Where loading older notes goes after a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Position {
    Page {
        before: Option<i64>,
        number: u32,
    },
    End,
    /// A full list at the largest page brought no note older than `before`.
    Stuck,
}

/// The page after a window that started at `before` and asked for `number` notes. A page that
/// moves on goes back to `page_size`.
///
/// - A list shorter than asked for had every note left: the end.
/// - Otherwise the next page starts at the oldest listed note's second. `before` includes its
///   second, so the notes sharing it come back, and none is skipped.
/// - A full list that brought no note older than `before` doesn't move on: the page is asked for
///   again at the most a page can hold, and if that's still full, it's stuck.
///
/// Each step moves `before` back, makes the page bigger once, or ends, so it can't loop.
fn next_position(before: Option<i64>, number: u32, page_size: u32, window: &Window) -> Position {
    if window.is_short() {
        return Position::End;
    }
    if let Some(oldest) = oldest(&window.times)
        && before.is_none_or(|before| oldest < before)
    {
        return Position::Page {
            before: Some(oldest),
            number: page_size,
        };
    }
    if number < MAX_PAGE_SIZE {
        return Position::Page {
            before,
            number: MAX_PAGE_SIZE,
        };
    }
    Position::Stuck
}

/// The oldest dated time.
fn oldest_dated(times: &[Option<i64>]) -> Option<i64> {
    times
        .iter()
        .filter_map(|time| *time)
        .filter(|time| *time >= UNDATED_BEFORE)
        .min()
}

/// The second the next page starts at: the oldest dated note's, or, on a page of notes with no
/// date, theirs. Never below 1, since a `before` of 0 could read as no `before` at all.
fn oldest(times: &[Option<i64>]) -> Option<i64> {
    let oldest = oldest_dated(times).or_else(|| times.iter().filter_map(|time| *time).min())?;
    Some(oldest.max(1))
}

fn date(seconds: i64) -> Option<WpGmtDateTime> {
    chrono::DateTime::from_timestamp(seconds, 0).map(WpGmtDateTime)
}

#[cfg(test)]
mod tests;

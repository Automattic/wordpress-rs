//! The Reader: its streams of posts, read from the cache and kept up to date from WordPress.com.
//!
//! Everything here works for a reader who isn't signed in to WordPress.com, as far as the API
//! allows: a tag's, a site's and a feed's posts need no account, and the Following stream does.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use wp_api::date::WpGmtDateTime;
use wp_api::prelude::{WpApiClientDelegate, WpApiError};
use wp_api::request::RequestExecutor;
use wp_api::wp_com::client::WpComApiClient;
use wp_api::wp_com::reader_saved::{ReaderSavedKey, ReaderSavedSource};
use wp_mobile_cache::db_types::reader::ReaderPostRecord;
use wp_mobile_cache::repository::reader::{ReaderPostRepository, ReaderStreamRepository};
use wp_mobile_cache::repository::wordpress_com_accounts::{
    ReaderOwner, WordPressComAccountRepository,
};
use wp_mobile_cache::{RowId, SqliteDbError, WpApiCache};

mod actions;
mod collection;
mod comments;
mod links;
mod page_views;
mod posts;
mod sites;
mod stream;
mod tags;

pub use actions::{ReaderActionOutcome, ReaderFollowTarget};
pub use collection::{
    ReaderStreamCard, ReaderStreamCollection, ReaderStreamInfo, ReaderStreamItem,
    ReaderStreamPageResult,
};
pub use comments::{ReaderCommentsCollection, ReaderCommentsInfo, ReaderCommentsPageResult};
pub use links::{ReaderLink, ReaderLinkNotInReader, reader_link_not_in_reader_message};
pub use page_views::{
    ReaderOpenedPageView, ReaderOpenedSeen, ReaderPageViewChoice, ReaderPageViewOutcome,
    ReaderPostOpenedOutcome,
};
pub use stream::reader_stream_for_menu_item;
pub use stream::{
    ReaderStream, ReaderStreamCardKind, ReaderStreamCardSite, ReaderStreamCardTag,
    ReaderStreamOrdering, reader_stream_needs_account, reader_stream_ordering,
};
pub use tags::ReaderCachedMenu;

use super::reader_saved::ReaderSavedService;
use actions::ActionQueue;
use collection::StreamLanes;

/// How long a stream nobody refreshes is kept.
const STREAM_LIFETIME: Duration = Duration::from_secs(30 * 24 * 60 * 60);

/// How long a post that's in no stream and isn't saved is kept after it was fetched, so one opened
/// on its own, such as from a link, is still there the next few times it's opened.
const UNUSED_POST_LIFETIME: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// The saved list's stream, which the saved posts service keeps, and which is kept whatever its age.
const SAVED_STREAM: &str = "saved";

#[allow(clippy::large_enum_variant)] // Boxing doesn't cross the FFI boundary.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum ReaderError {
    #[error("{0}")]
    Api(WpApiError),

    /// The reader isn't signed in, and this needs a WordPress.com account, such as the Following
    /// stream. Nothing was sent.
    #[error("Sign in to WordPress.com to use this.")]
    NeedsAccount,

    /// The post isn't in the stream, so there is no gap before it to fill.
    #[error("That post isn't in this stream.")]
    PostNotInStream,

    /// What was passed doesn't have the IDs or the address the API needs for this.
    #[error("This is missing the IDs or the address it needs.")]
    MissingIds,

    /// The server answered, and didn't do what was asked, such as following an address that has
    /// no feed. `reason` is what it said.
    #[error("The server didn't accept it: {reason}")]
    Refused { reason: String },

    #[error("{0}")]
    Database(SqliteDbError),
}

/// What a Reader error says, in words, for an app to show. Swift and Kotlin get an error's case
/// and its fields, and not the sentence that goes with it.
#[uniffi::export]
pub fn reader_error_message(error: &ReaderError) -> String {
    error.to_string()
}

/// Something of the Reader's that the cache keeps and that isn't a collection of its own, for
/// [`reader_data_is_relevant_update`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ReaderData {
    /// The menu, with the followed and the recommended tags.
    Menu,
    /// The sites and feeds the account follows, with their delivery settings.
    Subscriptions,
    /// The blocked sites, and the users blocked on this device.
    Blocks,
    /// What the reader turned on or off that is waiting to be sent, saves included.
    PendingActions,
    /// A cached post.
    Post,
    /// What the cache has of a site or a feed besides its posts.
    Site,
}

/// Whether a change to the cache is one that `data` shows, so that a screen reloads only when
/// what it shows changed. Streams, comments and the saved list answer this themselves.
#[uniffi::export]
pub fn reader_data_is_relevant_update(
    data: ReaderData,
    hook: &wp_mobile_cache::UpdateHook,
) -> bool {
    use wp_mobile_cache::DbTable;
    let tables: &[DbTable] = match data {
        ReaderData::Menu => &[DbTable::ReaderMenuItems],
        ReaderData::Subscriptions => &[DbTable::ReaderSubscriptions],
        ReaderData::Blocks => &[DbTable::ReaderBlockedSites, DbTable::ReaderBlockedUsers],
        ReaderData::PendingActions => &[DbTable::ReaderActions, DbTable::ReaderSavedPosts],
        ReaderData::Post => &[DbTable::ReaderPosts],
        ReaderData::Site => &[DbTable::ReaderSites],
    };
    tables.contains(&hook.table)
}

impl From<WpApiError> for ReaderError {
    fn from(err: WpApiError) -> Self {
        Self::Api(err)
    }
}

impl From<SqliteDbError> for ReaderError {
    fn from(err: SqliteDbError) -> Self {
        Self::Database(err)
    }
}

pub(crate) type Clock = Arc<dyn Fn() -> SystemTime + Send + Sync>;

/// What every part of a reader's Reader shares.
pub(crate) struct ReaderContext {
    pub(crate) client: WpComApiClient,
    pub(crate) cache: Arc<WpApiCache>,
    pub(crate) owner: ReaderOwner,
    /// The owner's row in the cache.
    pub(crate) owner_row: RowId,
    /// The app, which is recorded with what the reader does.
    pub(crate) source: ReaderSavedSource,
    /// What the reader turned on or off that's waiting to be sent.
    pub(crate) actions: ActionQueue,
    /// Sends requests that aren't to the API, such as the one that counts a view.
    pub(crate) request_executor: Arc<dyn RequestExecutor>,
    clock: Clock,
}

impl ReaderContext {
    pub(crate) fn now(&self) -> WpGmtDateTime {
        WpGmtDateTime(chrono::DateTime::<chrono::Utc>::from((self.clock)()))
    }

    fn time_ago(&self, duration: Duration) -> WpGmtDateTime {
        WpGmtDateTime(chrono::DateTime::<chrono::Utc>::from(
            (self.clock)() - duration,
        ))
    }

    /// Deletes what nothing shows any more: streams nobody has refreshed in a while, and posts
    /// that are in no stream, aren't saved and weren't fetched lately.
    pub(crate) fn clean_up(&self) -> Result<(), SqliteDbError> {
        let stale = self.time_ago(STREAM_LIFETIME);
        let unused = self.time_ago(UNUSED_POST_LIFETIME);
        self.cache.execute(|connection| {
            let transaction = connection.transaction()?;
            ReaderStreamRepository::delete_stale(
                &transaction,
                self.owner_row,
                &stale,
                SAVED_STREAM,
            )?;
            ReaderPostRepository::delete_unused(&transaction, self.owner_row, &unused)?;
            transaction.commit().map_err(SqliteDbError::from)
        })
    }
}

/// One reader's Reader: an account's, or the reader's who isn't signed in.
///
/// It's the entry point to the Reader, as `WpService` is to a site. Everything it hands out reads
/// from the cache, so it shows what it has straight away and offline, and fetches when asked to.
///
/// Create one per reader and share it. For [`ReaderOwner::SignedOut`], pass a delegate that sends
/// no authentication: a stream that needs an account then fails with
/// [`ReaderError::NeedsAccount`] without sending anything.
///
/// [`WpApiCache::remove_reader_data`] deletes everything it keeps.
#[derive(uniffi::Object)]
pub struct ReaderService {
    context: Arc<ReaderContext>,
    saved: Arc<ReaderSavedService>,
    /// What each stream's requests wait on, by the stream's cache key, so that two collections of
    /// one stream take turns.
    lanes: Mutex<HashMap<String, Arc<StreamLanes>>>,
}

#[uniffi::export]
impl ReaderService {
    /// Uses what the cache has for the reader. `cache` must have had its migrations run.
    /// `saved_source` is the app, which is recorded with each save.
    #[uniffi::constructor]
    pub fn new(
        delegate: WpApiClientDelegate,
        cache: Arc<WpApiCache>,
        owner: ReaderOwner,
        saved_source: ReaderSavedSource,
    ) -> Result<Self, ReaderError> {
        Self::with_clock(
            delegate,
            cache,
            owner,
            saved_source,
            Arc::new(SystemTime::now),
        )
    }

    /// Who this Reader belongs to.
    pub fn owner(&self) -> ReaderOwner {
        self.context.owner
    }

    /// A stream's posts, as the cache has them. Getting it doesn't touch the network.
    pub fn stream(&self, stream: ReaderStream) -> Arc<ReaderStreamCollection> {
        let lanes = self
            .lanes
            .lock()
            .expect("stream lanes lock poisoned")
            .entry(stream.key())
            .or_default()
            .clone();
        Arc::new(ReaderStreamCollection::new(
            self.context.clone(),
            stream,
            lanes,
        ))
    }

    /// A post as the cache has it, with its content, or `None` if the cache doesn't have it.
    /// `key` is the post's [`ReaderPostRecord::key`]. Doesn't touch the network.
    pub fn cached_post(
        &self,
        key: ReaderSavedKey,
    ) -> Result<Option<ReaderPostRecord>, ReaderError> {
        let post = self.context.cache.execute(|connection| {
            ReaderPostRepository::select_by_key(connection, self.context.owner_row, &key)
        })?;
        Ok(post.map(|(_, post)| post))
    }

    /// The reader's saved posts. A signed-out reader's are kept on this device only: see
    /// [`ReaderSavedService::for_reader`].
    pub fn saved(&self) -> Arc<ReaderSavedService> {
        self.saved.clone()
    }

    /// Deletes what nothing shows any more: streams nobody has refreshed in 30 days, and posts
    /// that are in no stream, aren't saved and weren't fetched in the last week. A stream's
    /// refresh does this too, so it only needs calling for a reader who never refreshes.
    pub fn clean_up(&self) -> Result<(), ReaderError> {
        Ok(self.context.clean_up()?)
    }
}

impl ReaderService {
    pub(crate) fn with_clock(
        delegate: WpApiClientDelegate,
        cache: Arc<WpApiCache>,
        owner: ReaderOwner,
        saved_source: ReaderSavedSource,
        clock: Clock,
    ) -> Result<Self, ReaderError> {
        let owner_row = cache.execute(|connection| {
            WordPressComAccountRepository::get_or_create_owner(connection, owner)
        })?;
        let request_executor = delegate.request_executor.clone();
        let saved = ReaderSavedService::for_reader_with_clock(
            delegate.clone(),
            saved_source,
            cache.clone(),
            owner,
            clock.clone(),
        )?;
        Ok(Self {
            saved: Arc::new(saved),
            context: Arc::new(ReaderContext {
                client: WpComApiClient::new(delegate),
                cache,
                owner,
                owner_row,
                source: saved_source,
                actions: ActionQueue::default(),
                request_executor,
                clock,
            }),
            lanes: Mutex::default(),
        })
    }
}

#[cfg(test)]
mod tests;

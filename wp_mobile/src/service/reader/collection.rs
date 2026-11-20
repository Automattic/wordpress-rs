//! A Reader stream as the cache has it, and the fetching that keeps it up to date.
//!
//! A page is stored in one transaction, so a reader of the cache never sees half of one. How a
//! page is merged depends on what orders the stream: see [`ReaderStreamOrdering`].

use super::language::ReaderLanguage;
use super::stream::{ReaderStreamCardKind, StreamCursor, StreamEntry, StreamPage};
use super::{ReaderContext, ReaderError, ReaderStream, ReaderStreamOrdering};
use futures::lock::Mutex as AsyncMutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use wp_api::date::WpGmtDateTime;
use wp_api::prelude::WpApiError;
use wp_api::wp_com::language::WPComLanguage;
use wp_api::wp_com::reader_saved::{
    ReaderSavedKey, reader_saved_key_for_post, reader_saved_keys_for_post,
};
use wp_mobile_cache::db_types::reader::{
    DbReaderPageEntry, DbReaderPageItem, DbReaderPageOutcome, DbReaderStream, ReaderPostRecord,
    reader_date_from_text, reader_date_text, reader_saved_key_text,
};
use wp_mobile_cache::list_metadata::ListState;
use wp_mobile_cache::repository::QueryExecutor;
use wp_mobile_cache::repository::reader::{
    ReaderPostRepository, ReaderSavedRepository, ReaderStreamRepository,
    ReaderUnreadablePostRepository,
};
use wp_mobile_cache::repository::wordpress_com_accounts::ReaderOwner;
use wp_mobile_cache::{DbTable, RowId, SqliteDbError, UpdateHook};

/// The most posts a stream ordered by date keeps. A refresh removes the oldest past it, and they
/// can be loaded again.
const MAX_DATED_POSTS: u64 = 500;

/// How many pages in a row with no posts a call follows past, in a stream that pages with a handle.
/// A search by date pages by time window, and six empty windows in a row have been seen live.
const MAX_EMPTY_PAGES_FOLLOWED: usize = 8;

/// How many of a stream's unreadable posts are asked for again at a time.
const UNREADABLE_RETRIES_AT_A_TIME: usize = 5;

/// How many times an unreadable post is asked for on its own before it's left alone.
const MAX_UNREADABLE_ATTEMPTS: u32 = 5;

/// How long an unreadable post waits before it's asked for again. The server keeps a post it built
/// for an hour, so asking sooner gets the same answer.
const UNREADABLE_RETRY_INTERVAL: Duration = Duration::from_secs(60 * 60);

/// The tables a stream is read from. The saved posts are among them: each item says whether it's
/// saved.
const STREAM_TABLES: [DbTable; 8] = [
    DbTable::ReaderPosts,
    DbTable::ReaderStreams,
    DbTable::ReaderStreamItems,
    DbTable::ReaderStreamCards,
    DbTable::ReaderUnreadablePosts,
    DbTable::ReaderBlockedSites,
    DbTable::ReaderBlockedUsers,
    DbTable::ReaderSavedPosts,
];

/// A post in a stream.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ReaderStreamItem {
    /// What the cache has of the post, without its content: a post's content is most of its size,
    /// and a list of posts doesn't show it.
    pub post: ReaderPostRecord,
    /// The date the stream orders the post by. `None` in a stream kept in the server's order.
    #[uniffi(default = None)]
    pub sort_date: Option<WpGmtDateTime>,
    /// Posts may be missing between this one and the one above it: a refresh didn't reach the
    /// posts the stream already had. [`ReaderStreamCollection::fill_gap`] loads them.
    pub gap_before: bool,
    /// Whether the reader saved the post, as far as this device knows: what
    /// [`ReaderSavedService::status`](crate::service::reader_saved::ReaderSavedService::status)
    /// says for it, without asking for each post.
    pub is_saved: bool,
}

/// A card in a stream that isn't a post, such as Discover's recommended sites.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct ReaderStreamCard {
    pub card: ReaderStreamCardKind,
    /// The key of the post the card comes after, among the posts
    /// [`ReaderStreamCollection::load_items`] returns, or `None` if it comes before them all.
    #[uniffi(default = None)]
    pub after_post: Option<ReaderSavedKey>,
}

/// What loading a page of a stream did.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ReaderStreamPageResult {
    /// The posts the page had that could be read. A post the server sent twice counts once.
    pub post_count: u64,
    /// How many of them the stream didn't have.
    pub new_post_count: u64,
    /// The posts the page had that couldn't be read. They're kept by their IDs, to ask for again.
    pub unreadable_post_count: u64,
    /// Whether the stream has posts past its last one.
    pub has_more: bool,
}

/// Whether a stream is loading, and where it's up to.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ReaderStreamInfo {
    pub state: ListState,
    #[uniffi(default = None)]
    pub error_message: Option<String>,
    /// `None` until the first page loads.
    #[uniffi(default = None)]
    pub has_more: Option<bool>,
    #[uniffi(default = None)]
    pub last_refreshed_at: Option<WpGmtDateTime>,
    /// How many posts the stream's pages had that couldn't be read. While it isn't `0`, the stream
    /// is missing posts the server has.
    pub unreadable_post_count: u64,
}

/// What a stream's requests wait on, shared by every collection of the stream.
#[derive(Default)]
pub(crate) struct StreamLanes {
    /// Held while the first page is fetched and stored.
    refresh: AsyncMutex<()>,
    /// Held while a later page is fetched and stored.
    page: AsyncMutex<()>,
    /// How many refreshes have ended, so one that waited for another knows it needn't run.
    refreshes: AtomicU64,
    /// What the last refresh did, or `None` if it failed.
    last_refresh: Mutex<Option<ReaderStreamPageResult>>,
}

/// Which page is being loaded.
#[derive(Debug, Clone)]
enum PageKind {
    /// The first page.
    First,
    /// The posts older than the stream's oldest, in a stream ordered by date.
    Older { before: WpGmtDateTime },
    /// The posts missing before `below`, in a stream ordered by date: those older than the post
    /// above it.
    Gap {
        before: WpGmtDateTime,
        below: RowId,
        below_sort_key: String,
    },
    /// The page `handle` leads to, in a stream kept in the server's order. `version` is the
    /// stream's when the handle was read: a refresh since makes the handle one for a stream that's
    /// been replaced.
    Next { handle: String, version: i64 },
}

/// A Reader stream's posts, as the cache has them.
///
/// Read with [`Self::load_items`], which doesn't touch the network, and reload when
/// [`Self::is_relevant_update`] says a change to the cache is one it shows. [`Self::refresh`]
/// fetches the first page, [`Self::load_more`] the next, and [`Self::fill_gap`] the posts a refresh
/// skipped.
///
/// A stream whose posts depend on the reader's language, such as a tag's or Discover, keeps the
/// language the service had when the stream was got: it fetches in that language, and reads and
/// stores that language's posts. After [`super::ReaderService::set_language`], get the stream again
/// for the new language's posts.
#[derive(uniffi::Object)]
pub struct ReaderStreamCollection {
    context: Arc<ReaderContext>,
    stream: ReaderStream,
    /// The language the stream is in, when its posts depend on one.
    language: Option<WPComLanguage>,
    /// The stream's key in the cache.
    key: String,
    lanes: Arc<StreamLanes>,
}

#[uniffi::export]
impl ReaderStreamCollection {
    pub fn stream(&self) -> ReaderStream {
        self.stream.clone()
    }

    /// The language the stream's posts are in: the reader's language when the stream was got.
    /// `None` when the reader had none, or when the stream's posts don't depend on it, such as a
    /// site's.
    pub fn language(&self) -> Option<WPComLanguage> {
        self.language
    }

    /// The stream's posts from the cache, in the stream's order, without posts from a site or by a
    /// user the reader blocked.
    pub async fn load_items(&self) -> Result<Vec<ReaderStreamItem>, ReaderError> {
        let dated = self.stream.ordering() == ReaderStreamOrdering::Date;
        let (items, saved) = self.context.cache.execute(|connection| {
            let Some(stream) = self.select_stream(&*connection)? else {
                return Ok((Vec::new(), Default::default()));
            };
            Ok::<_, SqliteDbError>((
                ReaderStreamRepository::select_items_without_content(&*connection, stream.row_id)?,
                ReaderSavedRepository::select_saved_key_texts(
                    &*connection,
                    self.context.owner_row,
                )?,
            ))
        })?;
        items
            .into_iter()
            .map(|item| {
                Ok(ReaderStreamItem {
                    sort_date: dated
                        .then(|| reader_date_from_text(&item.sort_key))
                        .transpose()?,
                    gap_before: item.gap_before,
                    // A post can be saved under any of its keys.
                    is_saved: reader_saved_keys_for_post(item.post.identity.clone())
                        .iter()
                        .any(|key| saved.contains(&reader_saved_key_text(key))),
                    post: item.post,
                })
            })
            .collect()
    }

    /// The stream's cards that aren't posts, from the cache, in the stream's order. Only a stream
    /// of cards has any: [`ReaderStream::Discover`] and [`ReaderStream::FirstPosts`].
    pub async fn load_cards(&self) -> Result<Vec<ReaderStreamCard>, ReaderError> {
        let cards = self.context.cache.execute(|connection| {
            let Some(stream) = self.select_stream(&*connection)? else {
                return Ok(Vec::new());
            };
            ReaderStreamRepository::select_cards(&*connection, stream.row_id)
        })?;
        Ok(cards
            .into_iter()
            // A card a newer version stored that this one can't read is left out.
            .filter_map(|card| {
                Some(ReaderStreamCard {
                    card: serde_json::from_str(&card.card).ok()?,
                    after_post: card.after_post,
                })
            })
            .collect())
    }

    /// Whether the stream is loading, and where it's up to.
    pub fn info(&self) -> Result<ReaderStreamInfo, ReaderError> {
        Ok(self.context.cache.execute(|connection| {
            let Some(stream) = self.select_stream(&*connection)? else {
                return Ok::<_, SqliteDbError>(ReaderStreamInfo {
                    state: ListState::Idle,
                    error_message: None,
                    has_more: None,
                    last_refreshed_at: None,
                    unreadable_post_count: 0,
                });
            };
            Ok(ReaderStreamInfo {
                unreadable_post_count: ReaderUnreadablePostRepository::count(
                    &*connection,
                    stream.row_id,
                )?,
                state: stream.state,
                error_message: stream.error_message,
                has_more: stream.has_more,
                last_refreshed_at: stream.last_refreshed_at,
            })
        })?)
    }

    /// Fetches the stream's first page and merges it into what the cache has.
    ///
    /// In a stream ordered by date, the posts the cache had stay. If the page doesn't reach them,
    /// the newest of them is marked with [`ReaderStreamItem::gap_before`]. In a stream kept in the
    /// server's order, the page replaces what the cache had.
    ///
    /// A stream kept in the server's order can send a page with no posts that isn't its last, as a
    /// search by date does for a time window with none. Such a page's handle is followed within
    /// the call, so it returns posts, or the end. It stops after 8 empty pages past the first:
    /// then it returns no posts with `has_more` true, and [`Self::load_more`] carries on from the
    /// last page fetched.
    ///
    /// A call made while another is running waits for it and returns what it did, without
    /// fetching again.
    pub async fn refresh(&self) -> Result<ReaderStreamPageResult, ReaderError> {
        self.check_account()?;
        let ended = self.lanes.refreshes.load(Ordering::SeqCst);
        let _lane = self.lanes.refresh.lock().await;
        if self.lanes.refreshes.load(Ordering::SeqCst) != ended
            && let Some(result) = self.last_refresh()
        {
            return Ok(result);
        }

        let result = async {
            self.context.cache.execute(|connection| {
                let stream = self.get_or_create_stream(&*connection)?;
                ReaderStreamRepository::begin_refresh(&*connection, stream.row_id)
            })?;
            self.load_page(PageKind::First).await
        }
        .await;
        let result = self.record_failure(result);

        *self
            .lanes
            .last_refresh
            .lock()
            .expect("last refresh lock poisoned") = result.as_ref().ok().cloned();
        self.lanes.refreshes.fetch_add(1, Ordering::SeqCst);
        if result.is_ok() {
            // The refresh worked, so a failure in what follows it is only logged.
            if let Err(error) = self.context.clean_up() {
                log::error!("Couldn't clean up the Reader cache: {error}");
            }
            if let Err(error) = self.retry_unreadable_posts().await {
                log::warn!("Couldn't ask again for a Reader stream's unreadable posts: {error}");
            }
        }
        result
    }

    /// Asks again, one by one, for posts the stream's pages had that couldn't be read. A post
    /// that reads now is stored, and takes its place in a stream ordered by date. Returns how
    /// many read. A refresh does this too.
    ///
    /// A post is asked for at most once an hour, since the server keeps a post it built for that
    /// long, and five times in all. One the server no longer has is forgotten.
    pub async fn retry_unreadable_posts(&self) -> Result<u64, ReaderError> {
        let now = self.context.now();
        let interval = chrono::Duration::from_std(UNREADABLE_RETRY_INTERVAL).unwrap_or_default();
        let due = self.context.cache.execute(|connection| {
            let Some(stream) = self.select_stream(&*connection)? else {
                return Ok::<_, SqliteDbError>(Vec::new());
            };
            let posts = ReaderUnreadablePostRepository::select(&*connection, stream.row_id)?;
            Ok(posts
                .into_iter()
                .filter(|post| {
                    let last = post.last_attempt_at.unwrap_or(post.first_seen_at);
                    post.attempts < MAX_UNREADABLE_ATTEMPTS && now.0 - last.0 >= interval
                })
                .take(UNREADABLE_RETRIES_AT_A_TIME)
                .collect())
        })?;

        let mut readable = 0;
        for post in due {
            let fetched = match self.context.fetch_reader_post(&post.identity).await {
                Ok(fetched) => fetched,
                Err(error) => {
                    let message = error.to_string();
                    self.context.cache.execute(|connection| {
                        if is_gone(&error) {
                            ReaderUnreadablePostRepository::delete(&*connection, post.row_id)
                        } else if was_answered(&error) {
                            ReaderUnreadablePostRepository::record_attempt(
                                &*connection,
                                post.row_id,
                                &now,
                                &message,
                            )
                        } else {
                            // The request didn't get an answer, which says nothing about the post.
                            Ok(())
                        }
                    })?;
                    continue;
                }
            };
            let stored = self.context.store_reader_post(&fetched)?;
            // A post fetched on its own doesn't say when it was liked or featured, which is what
            // Liked and Freshly Pressed are ordered by. It goes where its publish date puts it,
            // until a page of the stream gives its place.
            let sort_date = self.stream.sort_date(&fetched).or(fetched.date);
            let placed = self.context.cache.execute(|connection| {
                let transaction = connection.transaction()?;
                let placed = match (self.stream.ordering(), sort_date) {
                    (ReaderStreamOrdering::Date, Some(date)) => {
                        if let Some(row) = ReaderPostRepository::select_row_id_by_key(
                            &transaction,
                            self.context.owner_row,
                            &stored.key,
                        )? {
                            ReaderStreamRepository::upsert_item(
                                &transaction,
                                post.stream_row_id,
                                row,
                                &reader_date_text(&date),
                            )?;
                        }
                        true
                    }
                    // With no date at all there is nowhere to put it, so it stays listed as
                    // unreadable, and counts as a try.
                    (ReaderStreamOrdering::Date, None) => false,
                    // A stream in the server's order gets the post from its next refresh, which
                    // replaces what's cached.
                    (ReaderStreamOrdering::Server, _) => true,
                };
                if placed {
                    ReaderUnreadablePostRepository::delete(&transaction, post.row_id)?;
                } else {
                    ReaderUnreadablePostRepository::record_attempt(
                        &transaction,
                        post.row_id,
                        &now,
                        "The post has no date to place it by.",
                    )?;
                }
                transaction.commit().map_err(SqliteDbError::from)?;
                Ok::<_, SqliteDbError>(placed)
            })?;
            if placed {
                readable += 1;
            }
        }
        Ok(readable)
    }

    /// Fetches the page after the stream's last post, or refreshes if the stream hasn't loaded
    /// yet. Does nothing when the stream has no more.
    ///
    /// Pages with no posts that aren't the last are followed past, as [`Self::refresh`] follows
    /// them, and with the same limit.
    pub async fn load_more(&self) -> Result<ReaderStreamPageResult, ReaderError> {
        self.check_account()?;
        let lane = self.lanes.page.lock().await;
        let (stream, last_sort_key) = self.context.cache.execute(|connection| {
            let stream = self.get_or_create_stream(&*connection)?;
            let last = ReaderStreamRepository::select_last_sort_key(&*connection, stream.row_id)?;
            Ok::<_, SqliteDbError>((stream, last))
        })?;
        let kind = if stream.last_refreshed_at.is_none() {
            None
        } else if stream.has_more == Some(false) {
            return Ok(self.nothing_loaded(&stream));
        } else {
            match (
                self.stream.ordering(),
                last_sort_key,
                &stream.next_page_handle,
            ) {
                (ReaderStreamOrdering::Date, Some(oldest), _) => Some(PageKind::Older {
                    before: reader_date_from_text(&oldest)?,
                }),
                (ReaderStreamOrdering::Server, _, Some(handle)) => Some(PageKind::Next {
                    handle: handle.clone(),
                    version: stream.version,
                }),
                // A stream ordered by date with no posts has no date to load from.
                (ReaderStreamOrdering::Date, None, _) => None,
                (ReaderStreamOrdering::Server, _, None) => {
                    return Ok(self.nothing_loaded(&stream));
                }
            }
        };
        let Some(kind) = kind else {
            // The stream hasn't loaded, so its first page is the one to load.
            drop(lane);
            return self.refresh().await;
        };
        let result = self.load_later_page(kind).await;
        drop(lane);
        result
    }

    /// Fetches posts missing before a post that has [`ReaderStreamItem::gap_before`]: the page of
    /// posts older than the one above it. `below` is the post's [`ReaderPostRecord::key`].
    ///
    /// One call loads one page. The gap closes when a page reaches the post, and until then the
    /// post keeps `gap_before`, with the page's posts above it. Does nothing if the post has no
    /// gap before it.
    pub async fn fill_gap(
        &self,
        below: ReaderSavedKey,
    ) -> Result<ReaderStreamPageResult, ReaderError> {
        self.check_account()?;
        let _lane = self.lanes.page.lock().await;
        let owner = self.context.owner_row;
        let (stream, gap) = self.context.cache.execute(|connection| {
            let stream = self.get_or_create_stream(&*connection)?;
            let Some(post) =
                ReaderPostRepository::select_row_id_by_key(&*connection, owner, &below)?
            else {
                return Ok::<_, SqliteDbError>((stream, None));
            };
            let Some((sort_key, gap_before)) =
                ReaderStreamRepository::select_item_place(&*connection, stream.row_id, post)?
            else {
                return Ok((stream, None));
            };
            let above =
                ReaderStreamRepository::select_sort_key_above(&*connection, stream.row_id, post)?;
            if gap_before && above.is_none() {
                // Nothing is above the post, so nothing can be missing before it.
                ReaderStreamRepository::clear_gap(&*connection, stream.row_id, post)?;
            }
            Ok((stream, Some((post, sort_key, gap_before, above))))
        })?;
        let Some((post, sort_key, gap_before, above)) = gap else {
            return Err(ReaderError::PostNotInStream);
        };
        let (true, Some(above)) = (gap_before, above) else {
            return Ok(self.nothing_loaded(&stream));
        };
        self.load_later_page(PageKind::Gap {
            before: reader_date_from_text(&above)?,
            below: post,
            below_sort_key: sort_key,
        })
        .await
    }

    /// Whether a change to the cache is one the stream shows.
    pub fn is_relevant_update(&self, hook: &UpdateHook) -> bool {
        STREAM_TABLES.contains(&hook.table)
    }
}

impl ReaderStreamCollection {
    /// `language` is the reader's, and `lanes` are those of the stream's key in it.
    pub(crate) fn new(
        context: Arc<ReaderContext>,
        stream: ReaderStream,
        language: Option<WPComLanguage>,
        lanes: Arc<StreamLanes>,
    ) -> Self {
        let language = language.filter(|_| stream.uses_language());
        Self {
            key: stream.key(language.map(ReaderLanguage::new)),
            context,
            stream,
            language,
            lanes,
        }
    }

    /// Fails for a stream that needs an account when the reader has none, before anything is sent.
    fn check_account(&self) -> Result<(), ReaderError> {
        if self.stream.needs_account() && self.context.owner == ReaderOwner::SignedOut {
            return Err(ReaderError::NeedsAccount);
        }
        Ok(())
    }

    fn select_stream(
        &self,
        executor: &impl QueryExecutor,
    ) -> Result<Option<DbReaderStream>, SqliteDbError> {
        ReaderStreamRepository::select(executor, self.context.owner_row, &self.key)
    }

    fn get_or_create_stream(
        &self,
        executor: &impl QueryExecutor,
    ) -> Result<DbReaderStream, SqliteDbError> {
        ReaderStreamRepository::get_or_create(executor, self.context.owner_row, &self.key)
    }

    fn last_refresh(&self) -> Option<ReaderStreamPageResult> {
        self.lanes
            .last_refresh
            .lock()
            .expect("last refresh lock poisoned")
            .clone()
    }

    fn nothing_loaded(&self, stream: &DbReaderStream) -> ReaderStreamPageResult {
        ReaderStreamPageResult {
            post_count: 0,
            new_post_count: 0,
            unreadable_post_count: 0,
            has_more: stream.has_more.unwrap_or(false),
        }
    }

    /// Loads a page past the first, with the stream marked as fetching it.
    async fn load_later_page(&self, kind: PageKind) -> Result<ReaderStreamPageResult, ReaderError> {
        let result = async {
            self.context.cache.execute(|connection| {
                let stream = self.get_or_create_stream(&*connection)?;
                ReaderStreamRepository::set_state(
                    &*connection,
                    stream.row_id,
                    ListState::FetchingNextPage,
                    None,
                )
            })?;
            self.load_page(kind).await
        }
        .await;
        self.record_failure(result)
    }

    /// Fetches a page and stores it. In a stream kept in the server's order, a page with nothing on
    /// it that has a next handle isn't the end, so the handle is followed, up to
    /// [`MAX_EMPTY_PAGES_FOLLOWED`] times. The page stored is the first with anything on it, or the
    /// last one fetched: the empty pages before it add nothing to the stream, and its handle is
    /// where the next page starts.
    async fn load_page(&self, kind: PageKind) -> Result<ReaderStreamPageResult, ReaderError> {
        let cursor = match &kind {
            PageKind::First => None,
            PageKind::Older { before } | PageKind::Gap { before, .. } => {
                Some(StreamCursor::Before(*before))
            }
            PageKind::Next { handle, .. } => Some(StreamCursor::Handle(handle.clone())),
        };
        let language = self.language.map(ReaderLanguage::new);
        let mut page = self
            .stream
            .fetch(&self.context.client, cursor.as_ref(), language)
            .await?;
        if self.stream.ordering() == ReaderStreamOrdering::Server {
            let mut followed = 0;
            while followed < MAX_EMPTY_PAGES_FOLLOWED
                && page.entries.is_empty()
                && page.unreadable.is_empty()
                && let Some(handle) = page.next_page_handle.take()
            {
                page = self
                    .stream
                    .fetch(
                        &self.context.client,
                        Some(&StreamCursor::Handle(handle)),
                        language,
                    )
                    .await?;
                followed += 1;
            }
        }
        Ok(self.store_page(&kind, page)?)
    }

    /// Records a failed page as the stream's error, and returns the result.
    fn record_failure(
        &self,
        result: Result<ReaderStreamPageResult, ReaderError>,
    ) -> Result<ReaderStreamPageResult, ReaderError> {
        if let Err(error) = &result {
            let message = error.to_string();
            let recorded = self.context.cache.execute(|connection| {
                let stream = self.get_or_create_stream(&*connection)?;
                ReaderStreamRepository::set_state(
                    &*connection,
                    stream.row_id,
                    ListState::Error,
                    Some(&message),
                )
            });
            if let Err(record_error) = recorded {
                // The page's error is the one to return, so a failure to record it is only logged.
                log::error!("Couldn't record the Reader stream's error: {record_error}");
            }
        }
        result
    }

    /// Stores a page and merges it into the stream, in one transaction.
    fn store_page(
        &self,
        kind: &PageKind,
        page: StreamPage,
    ) -> Result<ReaderStreamPageResult, SqliteDbError> {
        let owner = self.context.owner_row;
        let order = self.stream.ordering();
        self.context.cache.execute(|connection| {
            let transaction = connection.transaction()?;
            let stream = self.get_or_create_stream(&transaction)?;
            if let PageKind::Next { version, .. } = kind
                && stream.version != *version
            {
                // A refresh replaced the stream after this page was asked for, so the page follows
                // posts the stream no longer has. The refresh recorded the stream's state.
                return Ok(self.nothing_loaded(&stream));
            }
            let had_posts = ReaderStreamRepository::count_items(&transaction, stream.row_id)? > 0;

            // What couldn't be read, by its IDs where they could be: the server's, then any post
            // this stream can't place.
            let mut unreadable = page.unreadable;
            // The page's posts, and its posts and cards in the order the server sent.
            let mut items = Vec::new();
            let mut entries = Vec::new();
            for entry in &page.entries {
                let post = match entry {
                    StreamEntry::Post(post) => post,
                    StreamEntry::Card(card) => {
                        let card = serde_json::to_string(card)
                            .map_err(|error| SqliteDbError::SqliteError(error.to_string()))?;
                        entries.push(DbReaderPageEntry::Card(card));
                        continue;
                    }
                };
                let Some(record) = self.context.record_of(&post.post) else {
                    unreadable.push((None, "The post has no IDs to keep it by.".to_string()));
                    continue;
                };
                let sort_key = match (order, post.sort_date) {
                    (ReaderStreamOrdering::Date, Some(date)) => reader_date_text(&date),
                    (ReaderStreamOrdering::Date, None) => {
                        unreadable.push((
                            Some(post.post.identity.clone()),
                            "The post has no date to order it by.".to_string(),
                        ));
                        continue;
                    }
                    // Its place in the page is its place in the stream.
                    (ReaderStreamOrdering::Server, _) => String::new(),
                };
                let row = ReaderPostRepository::upsert(&transaction, owner, &record)?;
                ReaderUnreadablePostRepository::resolve(&transaction, stream.row_id, &record.key)?;
                // The server can send a post twice in one page. The stream has it once, where it
                // first came, and the page counts it once.
                if items.iter().any(|item: &DbReaderPageItem| item.post == row) {
                    continue;
                }
                let item = DbReaderPageItem {
                    post: row,
                    sort_key,
                    railcar: post.post.railcar.clone(),
                };
                entries.push(DbReaderPageEntry::Post(item.clone()));
                items.push(item);
            }

            // The page may be older than something the reader turned on or off since.
            super::actions::show_pending(&transaction, owner)?;

            // An unreadable post is kept by its IDs, to ask for again. If the stream has it from
            // an earlier page, it stays: the server still has it.
            let mut keep = Vec::new();
            for (identity, error) in &unreadable {
                let Some(identity) = identity else { continue };
                let Some(key) = reader_saved_key_for_post(identity.clone()) else {
                    continue;
                };
                let key = key.normalized();
                ReaderUnreadablePostRepository::record(
                    &transaction,
                    owner,
                    stream.row_id,
                    &key,
                    identity,
                    error,
                    &self.context.now(),
                )?;
                if let Some(row) =
                    ReaderPostRepository::select_row_id_by_key(&transaction, owner, &key)?
                {
                    keep.push(row);
                }
            }

            // A page with posts, none of which could be read, says nothing about its ends.
            let nothing_read = items.is_empty() && !unreadable.is_empty();
            let oldest = items.iter().map(|item| item.sort_key.clone()).min();
            let (outcome, has_more, next_page_handle) = match (order, kind) {
                (ReaderStreamOrdering::Date, PageKind::Next { .. })
                | (ReaderStreamOrdering::Server, PageKind::Older { .. } | PageKind::Gap { .. }) => {
                    return Err(SqliteDbError::SqliteError(format!(
                        "A page of the wrong kind was loaded for Reader stream {}",
                        self.key
                    )));
                }
                (ReaderStreamOrdering::Date, _) => {
                    let before = match kind {
                        PageKind::Older { before } | PageKind::Gap { before, .. } => {
                            Some(reader_date_text(before))
                        }
                        _ => None,
                    };
                    let outcome = if nothing_read {
                        DbReaderPageOutcome::default()
                    } else {
                        ReaderStreamRepository::apply_dated_page(
                            &transaction,
                            stream.row_id,
                            before.as_deref(),
                            &items,
                            &keep,
                        )?
                    };
                    let has_more = match kind {
                        // An empty first page is an empty stream. Otherwise the first page says
                        // nothing new about the stream's end, unless the stream had no posts.
                        PageKind::First if items.is_empty() && !nothing_read => Some(false),
                        PageKind::First if !had_posts || stream.has_more.is_none() => {
                            Some(!items.is_empty())
                        }
                        PageKind::First => None,
                        // An empty page is the end, and so is one that didn't move on.
                        PageKind::Older { .. } => Some(oldest.is_some() && oldest < before),
                        PageKind::Gap {
                            below,
                            below_sort_key,
                            ..
                        } => {
                            // The gap is closed once a page reaches the post below it, or has
                            // nothing to say there is anything between.
                            let reached = oldest
                                .as_ref()
                                .is_none_or(|oldest| oldest <= below_sort_key);
                            if reached && !nothing_read {
                                ReaderStreamRepository::clear_gap(
                                    &transaction,
                                    stream.row_id,
                                    *below,
                                )?;
                            }
                            None
                        }
                        PageKind::Next { .. } => None,
                    };
                    (outcome, has_more, None)
                }
                (ReaderStreamOrdering::Server, PageKind::First) => (
                    ReaderStreamRepository::replace_entries(&transaction, stream.row_id, &entries)?,
                    Some(page.next_page_handle.is_some()),
                    page.next_page_handle,
                ),
                (ReaderStreamOrdering::Server, _) => (
                    ReaderStreamRepository::append_entries(&transaction, stream.row_id, &entries)?,
                    Some(page.next_page_handle.is_some()),
                    page.next_page_handle,
                ),
            };

            let first = matches!(kind, PageKind::First);
            // A stream ordered by date grows with every page. A refresh is when it's cut back: its
            // oldest posts go, and can be loaded again.
            let trimmed = first
                && order == ReaderStreamOrdering::Date
                && ReaderStreamRepository::trim(&transaction, stream.row_id, MAX_DATED_POSTS)? > 0;
            let now = self.context.now();
            ReaderStreamRepository::finish_page(
                &transaction,
                stream.row_id,
                if trimmed { Some(true) } else { has_more },
                next_page_handle.as_deref(),
                first.then_some(&now),
            )?;
            let has_more = self
                .get_or_create_stream(&transaction)?
                .has_more
                .unwrap_or(false);
            transaction.commit()?;
            Ok(ReaderStreamPageResult {
                post_count: items.len() as u64,
                new_post_count: outcome.added,
                unreadable_post_count: unreadable.len() as u64,
                has_more,
            })
        })
    }
}

/// Whether an error says the server no longer has the post.
fn is_gone(error: &ReaderError) -> bool {
    matches!(error, ReaderError::Api(error) if matches!(error.status_code(), Some(404 | 410)))
}

/// Whether an error is the server's answer about the post, as opposed to a request that failed on
/// the way: an answer that couldn't be read, or one in the 400s.
fn was_answered(error: &ReaderError) -> bool {
    match error {
        ReaderError::Api(WpApiError::ResponseParsingError { .. }) => true,
        ReaderError::Api(error) => {
            matches!(error.status_code(), Some(status) if (400..500).contains(&status))
        }
        _ => false,
    }
}

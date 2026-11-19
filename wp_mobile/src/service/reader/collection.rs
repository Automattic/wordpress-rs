//! A Reader stream as the cache has it, and the fetching that keeps it up to date.
//!
//! A page is stored in one transaction, so a reader of the cache never sees half of one. How a
//! page is merged depends on what orders the stream: see [`ReaderStreamOrdering`].

use super::stream::{ReaderStreamCardKind, StreamCursor, StreamEntry, StreamPage};
use super::{ReaderContext, ReaderError, ReaderStream, ReaderStreamOrdering};
use futures::lock::Mutex as AsyncMutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use wp_api::date::WpGmtDateTime;
use wp_api::wp_com::reader_saved::{ReaderSavedKey, reader_saved_key_for_post};
use wp_mobile_cache::db_types::reader::{
    DbReaderPageEntry, DbReaderPageItem, DbReaderPageOutcome, DbReaderStream, ReaderPostRecord,
    reader_date_from_text, reader_date_text,
};
use wp_mobile_cache::list_metadata::ListState;
use wp_mobile_cache::repository::QueryExecutor;
use wp_mobile_cache::repository::reader::{
    ReaderPostRepository, ReaderStreamRepository, ReaderUnreadablePostRepository,
};
use wp_mobile_cache::repository::wordpress_com_accounts::ReaderOwner;
use wp_mobile_cache::{DbTable, RowId, SqliteDbError, UpdateHook};

/// The most posts a stream ordered by date keeps. A refresh removes the oldest past it, and they
/// can be loaded again.
const MAX_DATED_POSTS: u64 = 500;

/// The tables a stream is read from.
const STREAM_TABLES: [DbTable; 7] = [
    DbTable::ReaderPosts,
    DbTable::ReaderStreams,
    DbTable::ReaderStreamItems,
    DbTable::ReaderStreamCards,
    DbTable::ReaderUnreadablePosts,
    DbTable::ReaderBlockedSites,
    DbTable::ReaderBlockedUsers,
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
    /// The posts the page had that could be read.
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
#[derive(uniffi::Object)]
pub struct ReaderStreamCollection {
    context: Arc<ReaderContext>,
    stream: ReaderStream,
    /// The stream's key in the cache.
    key: String,
    lanes: Arc<StreamLanes>,
}

#[uniffi::export]
impl ReaderStreamCollection {
    pub fn stream(&self) -> ReaderStream {
        self.stream.clone()
    }

    /// The stream's posts from the cache, in the stream's order, without posts from a site or by a
    /// user the reader blocked.
    pub async fn load_items(&self) -> Result<Vec<ReaderStreamItem>, ReaderError> {
        let dated = self.stream.ordering() == ReaderStreamOrdering::Date;
        let items = self.context.cache.execute(|connection| {
            let Some(stream) = self.select_stream(&*connection)? else {
                return Ok(Vec::new());
            };
            ReaderStreamRepository::select_items_without_content(&*connection, stream.row_id)
        })?;
        items
            .into_iter()
            .map(|item| {
                Ok(ReaderStreamItem {
                    sort_date: dated
                        .then(|| reader_date_from_text(&item.sort_key))
                        .transpose()?,
                    gap_before: item.gap_before,
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
        if result.is_ok()
            && let Err(error) = self.context.clean_up()
        {
            // The refresh worked, so a failure to tidy up afterwards is only logged.
            log::error!("Couldn't clean up the Reader cache: {error}");
        }
        result
    }

    /// Fetches the page after the stream's last post, or refreshes if the stream hasn't loaded
    /// yet. Does nothing when the stream has no more.
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
    pub(crate) fn new(
        context: Arc<ReaderContext>,
        stream: ReaderStream,
        lanes: Arc<StreamLanes>,
    ) -> Self {
        Self {
            key: stream.key(),
            context,
            stream,
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

    async fn load_page(&self, kind: PageKind) -> Result<ReaderStreamPageResult, ReaderError> {
        let cursor = match &kind {
            PageKind::First => None,
            PageKind::Older { before } | PageKind::Gap { before, .. } => {
                Some(StreamCursor::Before(*before))
            }
            PageKind::Next { handle, .. } => Some(StreamCursor::Handle(handle.clone())),
        };
        let page = self
            .stream
            .fetch(&self.context.client, cursor.as_ref())
            .await?;
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
                let Some(record) = ReaderPostRecord::from_reader_post(&post.post) else {
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

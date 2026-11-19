//! The saved list, the collection apps observe it through, and imports of saves.

use super::offline::ReaderOfflineState;
use super::{ReaderSavedError, ReaderSavedService};
use std::collections::HashSet;
use std::sync::Arc;
use wp_api::date::WpGmtDateTime;
use wp_api::wp_com::reader_posts::{ReaderStreamOrder, ReaderUnreadablePost};
use wp_api::wp_com::reader_saved::{
    ReaderPostSaveIdentity, ReaderSavedKey, ReaderSavedOrderBy, ReaderSavedPostsParams,
    ReaderSavedPostsResponse, reader_saved_key_for_post, reader_saved_keys_for_post,
};
#[cfg(test)]
use wp_mobile_cache::WpApiCache;
use wp_mobile_cache::db_types::reader::ReaderPostRecord;
use wp_mobile_cache::list_metadata::ListState;
use wp_mobile_cache::repository::QueryExecutor;
use wp_mobile_cache::repository::reader::{
    ReaderPostRepository, ReaderSavedRepository, ReaderStreamRepository,
};
use wp_mobile_cache::repository::reader_offline::ReaderOfflineRepository;
use wp_mobile_cache::repository::wordpress_com_accounts::{
    ReaderOwner, WordPressComAccountRepository,
};
use wp_mobile_cache::{DbTable, SqliteDbError, UpdateHook};

/// The saved list's key in `reader_streams`, which keeps where the list is up to. The list in
/// another order has its own, which starts with this and a colon.
const SAVED_STREAM: &str = "saved";

/// The tables the saved list is read from.
const SAVED_LIST_TABLES: [DbTable; 5] = [
    DbTable::ReaderPosts,
    DbTable::ReaderStreams,
    DbTable::ReaderSavedState,
    DbTable::ReaderSavedPosts,
    DbTable::ReaderSavedOffline,
];

/// How many posts a page asks for when it's loaded to get saved posts' content, rather than for
/// a reader scrolling the list. It's the most the API returns.
pub(super) const CATCH_UP_PAGE_SIZE: u32 = 40;

/// The order of a saved list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SavedOrder {
    pub(super) by: ReaderSavedOrderBy,
    pub(super) order: ReaderStreamOrder,
}

impl SavedOrder {
    /// Newest save first: the order the API returns the list in when it isn't asked for another.
    pub(super) const NEWEST_SAVE_FIRST: Self = Self {
        by: ReaderSavedOrderBy::Date,
        order: ReaderStreamOrder::Desc,
    };

    /// The key in `reader_streams` of the list in this order. Each order pages on its own: a
    /// page handle only continues the order that returned it.
    fn stream_key(&self) -> String {
        if *self == Self::NEWEST_SAVE_FIRST {
            SAVED_STREAM.to_string()
        } else {
            format!("{SAVED_STREAM}:{}:{}", self.by, self.order)
        }
    }
}

/// A post in the saved list.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ReaderSavedListItem {
    /// The key the post is saved under. To unsave, prefer [`ReaderSavedService::unsave`] with the
    /// post's identity when there's a post, and use [`ReaderSavedService::unsave_key`] otherwise.
    pub key: ReaderSavedKey,
    #[uniffi(default = None)]
    pub date_saved: Option<WpGmtDateTime>,
    /// What the cache has of the post. `None` for a post saved on this device that the cache
    /// doesn't have yet.
    #[uniffi(default = None)]
    pub post: Option<ReaderPostRecord>,
    /// Saved on this device, and the server hasn't confirmed it yet.
    pub is_pending: bool,
    /// Pending because the account has reached its save cap. It's sent once there's room.
    pub is_waiting_for_room: bool,
    /// Whether the post can be read with no connection. See
    /// [`ReaderSavedService::download_for_offline`].
    pub offline: ReaderOfflineState,
}

/// What a page of the saved list did.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ReaderSavedPageResult {
    /// The posts the page had.
    pub post_count: u64,
    pub has_more: bool,
}

/// Whether the saved list is loading, and where it's up to.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ReaderSavedListInfo {
    pub state: ListState,
    #[uniffi(default = None)]
    pub error_message: Option<String>,
    /// `None` until the first page loads.
    #[uniffi(default = None)]
    pub has_more: Option<bool>,
    #[uniffi(default = None)]
    pub last_refreshed_at: Option<WpGmtDateTime>,
}

/// A save to import with [`ReaderSavedService::import_saves`], such as one the app kept on the
/// device before saves were synced, or read from a file. The fields besides the identity are shown
/// until the server sends the post.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ReaderLocalSave {
    pub identity: ReaderPostSaveIdentity,
    #[uniffi(default = None)]
    pub date_saved: Option<WpGmtDateTime>,
    #[uniffi(default = None)]
    pub title: Option<String>,
    #[uniffi(default = None)]
    pub excerpt: Option<String>,
    /// The image the app shows for the post. It's stored as both the featured image and the display
    /// image.
    #[uniffi(default = None)]
    pub featured_image: Option<String>,
    #[uniffi(default = None)]
    pub site_name: Option<String>,
    #[uniffi(default = None)]
    pub site_url: Option<String>,
}

/// What [`ReaderSavedService::import_saves`] did.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ReaderSavedImportReport {
    /// Saves the server has, including ones it had before.
    pub imported: u64,
    /// Saves the server refused, such as a post it can't find. They were undone.
    pub rejected: Vec<ReaderSavedKey>,
    /// Saves still waiting to be sent, which a later sync sends.
    pub pending: u64,
    /// Saves past the account's save cap. They stay saved, and are sent once there's room.
    pub waiting_for_room: u64,
    /// Posts missing the IDs the API needs, or with an ID of `0` or an empty URL. They can't be
    /// imported or kept.
    pub unsaveable: u64,
}

#[uniffi::export]
impl ReaderSavedService {
    /// The saved list, from the cache, newest save first.
    pub fn saved_posts(self: Arc<Self>) -> Arc<ReaderSavedPostsCollection> {
        Arc::new(ReaderSavedPostsCollection {
            service: self,
            order: SavedOrder::NEWEST_SAVE_FIRST,
        })
    }

    /// The saved list in another order: by when each post was saved or published, newest or
    /// oldest first. A save without that date comes last.
    ///
    /// It's the same saves, read from the cache in that order. Only loading differs: each order
    /// loads its own pages from the server, so the list in one order may be further along than
    /// in another.
    pub fn saved_posts_ordered(
        self: Arc<Self>,
        order_by: ReaderSavedOrderBy,
        order: ReaderStreamOrder,
    ) -> Arc<ReaderSavedPostsCollection> {
        Arc::new(ReaderSavedPostsCollection {
            service: self,
            order: SavedOrder {
                by: order_by,
                order,
            },
        })
    }

    /// Imports saves, given newest first, such as a device's saves from before saves were synced
    /// or a list the app read from a file. Each is a save: a post already saved stays as it is,
    /// and one unsaved is saved again, so importing the same list twice changes nothing. Call it
    /// whenever there's a list to import.
    ///
    /// The saves are in the cache before anything is sent, so they show in the saved list, and a
    /// later sync sends any this one couldn't. A save the server refuses is undone, and listed in
    /// [`ReaderSavedImportReport::rejected`]. One past the account's save cap stays saved instead,
    /// and is sent once there's room: see [`Self::saves_waiting_for_room`].
    pub async fn import_saves(
        &self,
        saves: Vec<ReaderLocalSave>,
    ) -> Result<ReaderSavedImportReport, ReaderSavedError> {
        let mut unsaveable = 0;
        let mut keys = Vec::new();
        let added = self.cache.execute(|connection| {
            let transaction = connection.transaction()?;
            let mut added = false;
            // Oldest first, so the newest are sent last and stay newest.
            for save in saves.into_iter().rev() {
                // A key the server would refuse isn't sent, as it would answer `invalid_input`.
                let Some(key) = reader_saved_key_for_post(save.identity.clone())
                    .filter(ReaderSavedKey::is_complete)
                else {
                    unsaveable += 1;
                    continue;
                };
                let key = key.normalized();
                let Some(mut record) = ReaderPostRecord::from_identity(save.identity.clone())
                else {
                    continue;
                };
                // A blank field is one the app doesn't have. Stored as text, it would keep a later
                // save's details from filling it in.
                record.title = known(&save.title);
                record.excerpt = known(&save.excerpt);
                record.featured_image = known(&save.featured_image);
                record.display_image_url = known(&save.featured_image);
                record.site_name = known(&save.site_name);
                record.site_url = known(&save.site_url);
                // The import's copy may be older than what the saved list already stored.
                let post = ReaderPostRepository::fill_missing(&transaction, self.account, &record)?;
                added |= ReaderSavedRepository::import_save(
                    &transaction,
                    self.account,
                    &key,
                    Some(post),
                    save.date_saved.as_ref(),
                )?;
                if !keys.contains(&key) {
                    keys.push(key);
                }
            }
            transaction.commit().map_err(SqliteDbError::from)?;
            Ok::<_, SqliteDbError>(added)
        })?;
        if added {
            self.notify();
        }

        self.send_imported(keys, unsaveable).await
    }

    /// Takes over the saves made on this device while nobody was signed in, as an import: each is
    /// saved to this account, with what the device kept of its post, and the signed-out reader's
    /// saved list is emptied. Call it after sign-in, on the account's service. A post the account
    /// already saved stays as it is.
    pub async fn adopt_signed_out_saves(
        &self,
    ) -> Result<ReaderSavedImportReport, ReaderSavedError> {
        let mut keys = Vec::new();
        if !self.signed_out {
            self.cache.execute(|connection| {
                let transaction = connection.transaction()?;
                let Some(signed_out) = WordPressComAccountRepository::select_owner(
                    &transaction,
                    ReaderOwner::SignedOut,
                )?
                else {
                    return Ok(());
                };
                // Oldest first, so the newest are sent last and stay newest.
                for item in ReaderSavedRepository::select_list(&transaction, signed_out)?
                    .into_iter()
                    .rev()
                {
                    let post = match &item.post {
                        Some(post) => Some(ReaderPostRepository::fill_missing(
                            &transaction,
                            self.account,
                            post,
                        )?),
                        None => None,
                    };
                    ReaderSavedRepository::import_save(
                        &transaction,
                        self.account,
                        &item.row.key,
                        post,
                        item.row.date_saved.as_ref(),
                    )?;
                    if !keys.contains(&item.row.key) {
                        keys.push(item.row.key);
                    }
                }
                // They're the account's now.
                ReaderSavedRepository::delete_all(&transaction, signed_out)?;
                transaction.commit().map_err(SqliteDbError::from)
            })?;
        }
        if !keys.is_empty() {
            self.notify();
        }
        self.send_imported(keys, 0).await
    }
}

impl ReaderSavedService {
    /// Sends imported saves, and reports what became of them.
    async fn send_imported(
        &self,
        keys: Vec<ReaderSavedKey>,
        unsaveable: u64,
    ) -> Result<ReaderSavedImportReport, ReaderSavedError> {
        let sent = {
            let _lane = self.lane.lock().await;
            self.send_pending().await?
        };
        let rows = self.rows_for(&keys)?;
        // A key without a saved row was undone: the server refused it.
        let rejected: Vec<_> = keys
            .iter()
            .filter(|key| {
                !rows.iter().any(|row| {
                    row.saved
                        && std::iter::once(&row.key)
                            .chain(&row.url_keys)
                            .chain(&row.feed_item_key)
                            .any(|other| other == *key)
                })
            })
            .cloned()
            .collect();
        let saved: Vec<_> = rows.iter().filter(|row| row.saved).collect();
        let waiting_for_room = saved.iter().filter(|row| row.waiting_for_room).count() as u64;
        let pending = saved
            .iter()
            .filter(|row| row.is_pending() && !row.waiting_for_room)
            .count() as u64;
        if let Some(error) = sent.unexpected {
            return Err(error);
        }
        // Saves that couldn't be sent are in `pending`, and a later sync sends them.
        sent.log();
        Ok(ReaderSavedImportReport {
            imported: (keys.len() - rejected.len()) as u64 - pending - waiting_for_room,
            rejected,
            pending,
            waiting_for_room,
            unsaveable,
        })
    }
}

impl ReaderSavedService {
    /// Stores the posts of a page of the saved list, and where the list is up to. Call it holding
    /// the lane.
    fn store_page(
        &self,
        response: &ReaderSavedPostsResponse,
        first: bool,
        order: SavedOrder,
    ) -> Result<(), ReaderSavedError> {
        let now = self.now();
        self.cache.execute(|connection| {
            let transaction = connection.transaction()?;
            let stream = ReaderStreamRepository::get_or_create(
                &transaction,
                self.account,
                &order.stream_key(),
            )?;
            for post in &response.posts {
                let Some(record) = ReaderPostRecord::from_reader_post(&post.post) else {
                    continue;
                };
                let row = ReaderPostRepository::upsert(&transaction, self.account, &record)?;
                // The post's URL is the same save, as the server stores it: it finds the row of a
                // Jetpack post the list returns by its feed item while its WordPress.com copy
                // fails to load, since the keys give that row the URL.
                let url = post
                    .post
                    .identity
                    .url
                    .clone()
                    .map(|url| ReaderSavedKey::Url { url })
                    .filter(ReaderSavedKey::is_complete);
                let keys: Vec<_> = reader_saved_keys_for_post(post.post.identity.clone())
                    .into_iter()
                    .chain(url)
                    .map(|key| key.normalized())
                    .collect();
                ReaderSavedRepository::apply_listed(
                    &transaction,
                    self.account,
                    &record.key,
                    &keys,
                    row,
                    post.date_saved.as_ref(),
                )?;
                // The list sends each post in full, so the device has all the content there is.
                ReaderOfflineRepository::set_fetched_for_post(&transaction, row)?;
            }
            for post in &response.unreadable_posts {
                self.store_unreadable(&transaction, post, &now)?;
            }
            ReaderStreamRepository::set_cursor(
                &transaction,
                stream.row_id,
                response.next_page_handle.as_deref(),
                first,
            )?;
            transaction.commit().map_err(SqliteDbError::from)
        })?;
        self.notify();
        Ok(())
    }

    /// Keeps a post the saved list sent that couldn't be read. It's still saved, so it shows in
    /// the list as a save, with what the cache has of it and why its content couldn't be loaded.
    /// One whose IDs couldn't be read either can't be told from any other save, so it's left to
    /// the saved keys, which still have it.
    fn store_unreadable(
        &self,
        transaction: &impl QueryExecutor,
        post: &ReaderUnreadablePost,
        now: &WpGmtDateTime,
    ) -> Result<(), SqliteDbError> {
        let Some(identity) = &post.identity else {
            log::warn!("A saved post couldn't be read: {}", post.error);
            return Ok(());
        };
        let Some(record) = ReaderPostRecord::from_identity(identity.clone()) else {
            log::warn!("A saved post couldn't be read: {}", post.error);
            return Ok(());
        };
        let row = ReaderPostRepository::fill_missing(transaction, self.account, &record)?;
        let keys: Vec<_> = reader_saved_keys_for_post(identity.clone())
            .into_iter()
            .map(|key| key.normalized())
            .collect();
        ReaderSavedRepository::apply_listed(
            transaction,
            self.account,
            &record.key,
            &keys,
            row,
            None,
        )?;
        let Some((saved, _)) =
            ReaderSavedRepository::select_row_id_for_key(transaction, self.account, &record.key)?
        else {
            return Ok(());
        };
        // A copy stored before, from a stream or an earlier page, is still good to read.
        let has_content = ReaderPostRepository::select_by_row_id(transaction, row)?
            .is_some_and(|post| post.content.is_some());
        // The reason is recorded once: a later page that sends the post again says nothing new.
        let offline = ReaderOfflineRepository::select(transaction, saved)?;
        if !has_content && !offline.fetched && offline.error.is_none() {
            ReaderOfflineRepository::record_failure(
                transaction,
                saved,
                &format!("The post couldn't be read: {}", post.error),
                now,
            )?;
        }
        Ok(())
    }

    fn set_list_state(
        &self,
        state: ListState,
        error: Option<&ReaderSavedError>,
        order: SavedOrder,
    ) -> Result<(), ReaderSavedError> {
        let message = error.map(ToString::to_string);
        self.cache.execute(|connection| {
            let stream = ReaderStreamRepository::get_or_create(
                &*connection,
                self.account,
                &order.stream_key(),
            )?;
            ReaderStreamRepository::set_state(
                &*connection,
                stream.row_id,
                state,
                message.as_deref(),
            )
        })?;
        Ok(())
    }

    /// Syncs, then loads the first page of the list. Call it holding the lane.
    pub(super) async fn refresh_list_in_lane(
        &self,
        order: SavedOrder,
    ) -> Result<ReaderSavedPageResult, ReaderSavedError> {
        if self.signed_out {
            // The list is what this device saved, and all of it is in the cache.
            return Ok(ReaderSavedPageResult {
                post_count: 0,
                has_more: false,
            });
        }
        self.set_list_state(ListState::FetchingFirstPage, None, order)?;
        let result = async {
            self.sync_in_lane().await?;
            let response = self
                .client
                .reader_saved()
                .list_posts(&saved_list_params(None, order, None))
                .await?
                .data;
            self.store_page(&response, true, order)?;
            Ok(page_result(&response))
        }
        .await;
        self.list_result(result, order)
    }

    /// Loads the page after the last one loaded. `number` is how many posts to ask for, or
    /// `None` for the API's own page size. Call it holding the lane.
    pub(super) async fn load_next_page_in_lane(
        &self,
        order: SavedOrder,
        number: Option<u32>,
    ) -> Result<ReaderSavedPageResult, ReaderSavedError> {
        if self.signed_out {
            return Ok(ReaderSavedPageResult {
                post_count: 0,
                has_more: false,
            });
        }
        let stream = self.cache.execute(|connection| {
            ReaderStreamRepository::get_or_create(&*connection, self.account, &order.stream_key())
        })?;
        if stream.last_refreshed_at.is_none() {
            return self.refresh_list_in_lane(order).await;
        }
        let Some(page_handle) = stream.next_page_handle else {
            return Ok(ReaderSavedPageResult {
                post_count: 0,
                has_more: false,
            });
        };
        self.set_list_state(ListState::FetchingNextPage, None, order)?;
        let result = async {
            let response = self
                .client
                .reader_saved()
                .list_posts(&saved_list_params(Some(page_handle), order, number))
                .await?
                .data;
            self.store_page(&response, false, order)?;
            Ok(page_result(&response))
        }
        .await;
        self.list_result(result, order)
    }

    /// Records a failed page as the list's error, and returns the result.
    fn list_result(
        &self,
        result: Result<ReaderSavedPageResult, ReaderSavedError>,
        order: SavedOrder,
    ) -> Result<ReaderSavedPageResult, ReaderSavedError> {
        if let Err(error) = &result
            && let Err(record_error) = self.set_list_state(ListState::Error, Some(error), order)
        {
            // The page's error is the one to return, so a failure to record it is only logged.
            log::error!("Couldn't record the saved list's error: {record_error}");
        }
        result
    }

    /// The saved list from the cache, newest save first.
    #[cfg(test)]
    pub(super) fn saved_list_items(&self) -> Result<Vec<ReaderSavedListItem>, ReaderSavedError> {
        self.saved_list_items_ordered(SavedOrder::NEWEST_SAVE_FIRST)
    }

    /// The saved list from the cache, in `order`: what this device saved and what the list sent.
    /// A post this device is unsaving is left out.
    fn saved_list_items_ordered(
        &self,
        order: SavedOrder,
    ) -> Result<Vec<ReaderSavedListItem>, ReaderSavedError> {
        let items = self.cache.execute(|connection| {
            ReaderSavedRepository::select_list_ordered(
                &*connection,
                self.account,
                order.by,
                order.order,
            )
        })?;
        let mut shown = HashSet::new();
        Ok(items
            .into_iter()
            // A post saved under two keys shows once.
            .filter(|item| item.row.post_row_id.is_none_or(|post| shown.insert(post)))
            .map(|item| ReaderSavedListItem {
                is_pending: item.row.is_pending(),
                is_waiting_for_room: item.row.waiting_for_room,
                offline: ReaderOfflineState::of(item.post.as_ref(), &item.offline),
                key: item.row.key,
                date_saved: item.row.date_saved,
                post: item.post,
            })
            .collect())
    }

    fn saved_list_info(&self, order: SavedOrder) -> Result<ReaderSavedListInfo, ReaderSavedError> {
        let stream = self.cache.execute(|connection| {
            ReaderStreamRepository::select(&*connection, self.account, &order.stream_key())
        })?;
        Ok(match stream {
            Some(stream) => ReaderSavedListInfo {
                state: stream.state,
                error_message: stream.error_message,
                has_more: stream.has_more,
                last_refreshed_at: stream.last_refreshed_at,
            },
            None => ReaderSavedListInfo {
                state: ListState::Idle,
                error_message: None,
                has_more: None,
                last_refreshed_at: None,
            },
        })
    }
}

/// What the saved list asks for: `meta=site,feed` gives every post its site's icon, including an
/// RSS post, which otherwise has none.
///
/// The order is sent only when it isn't the API's own, so the list newest save first is asked for
/// as it always was.
fn saved_list_params(
    page_handle: Option<String>,
    order: SavedOrder,
    number: Option<u32>,
) -> ReaderSavedPostsParams {
    let (order_by, order) = if order == SavedOrder::NEWEST_SAVE_FIRST {
        (None, None)
    } else {
        (Some(order.by), Some(order.order))
    };
    ReaderSavedPostsParams {
        page_handle,
        number,
        meta: Some("site,feed".to_string()),
        order_by,
        order,
        ..Default::default()
    }
}

fn page_result(response: &ReaderSavedPostsResponse) -> ReaderSavedPageResult {
    ReaderSavedPageResult {
        post_count: response.posts.len() as u64,
        has_more: response.next_page_handle.is_some(),
    }
}

/// The account's saved posts, as the cache has them.
///
/// Read with [`Self::load_items`], which doesn't touch the network, and reload it when
/// [`Self::is_relevant_update`] says a change to the cache is one it shows. [`Self::refresh`]
/// syncs and fetches the first page, and [`Self::load_next_page`] the rest.
#[derive(uniffi::Object)]
pub struct ReaderSavedPostsCollection {
    service: Arc<ReaderSavedService>,
    order: SavedOrder,
}

#[uniffi::export]
impl ReaderSavedPostsCollection {
    /// The saved list, from the cache, in the collection's order: newest save first, unless it
    /// came from [`ReaderSavedService::saved_posts_ordered`].
    pub async fn load_items(&self) -> Result<Vec<ReaderSavedListItem>, ReaderSavedError> {
        self.service.saved_list_items_ordered(self.order)
    }

    /// Syncs, then reloads the first page of the list. Posts past the first page stay, so the list
    /// is still whole offline.
    pub async fn refresh(&self) -> Result<ReaderSavedPageResult, ReaderSavedError> {
        let _lane = self.service.lane.lock().await;
        self.service.refresh_list_in_lane(self.order).await
    }

    /// Loads the page after the last one loaded, or refreshes if the list hasn't loaded yet. It
    /// waits for a refresh already running, then loads the page after it.
    pub async fn load_next_page(&self) -> Result<ReaderSavedPageResult, ReaderSavedError> {
        let _lane = self.service.lane.lock().await;
        self.service.load_next_page_in_lane(self.order, None).await
    }

    pub fn list_info(&self) -> Result<ReaderSavedListInfo, ReaderSavedError> {
        self.service.saved_list_info(self.order)
    }

    /// Whether a change to the cache is one the saved list shows.
    pub fn is_relevant_update(&self, hook: &UpdateHook) -> bool {
        SAVED_LIST_TABLES.contains(&hook.table)
    }
}

/// A new in-memory cache with every migration run.
/// The text, unless it's blank.
fn known(text: &Option<String>) -> Option<String> {
    text.clone().filter(|text| !text.trim().is_empty())
}

#[cfg(test)]
pub(super) fn migrated_in_memory_cache() -> Arc<WpApiCache> {
    let cache = WpApiCache::new(None).expect("an in-memory cache opens");
    cache.perform_migrations().expect("migrations run");
    Arc::new(cache)
}

#[cfg(test)]
#[path = "storage_tests.rs"]
mod tests;

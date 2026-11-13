//! The saved list, the collection apps observe it through, and the import of local saves.

use super::{ReaderSavedError, ReaderSavedService};
use std::collections::HashSet;
use std::sync::Arc;
use wp_api::date::WpGmtDateTime;
use wp_api::wp_com::reader_saved::{
    ReaderPostSaveIdentity, ReaderSavedKey, ReaderSavedPostsParams, ReaderSavedPostsResponse,
    reader_saved_key_for_post, reader_saved_keys_for_post,
};
#[cfg(test)]
use wp_mobile_cache::WpApiCache;
use wp_mobile_cache::db_types::reader::ReaderPostRecord;
use wp_mobile_cache::list_metadata::ListState;
use wp_mobile_cache::repository::reader::{
    ReaderPostRepository, ReaderSavedRepository, ReaderStreamRepository,
};
use wp_mobile_cache::{DbTable, SqliteDbError, UpdateHook};

/// The saved list's key in `reader_streams`, which keeps where the list is up to.
const SAVED_STREAM: &str = "saved";

/// The tables the saved list is read from.
const SAVED_LIST_TABLES: [DbTable; 4] = [
    DbTable::ReaderPosts,
    DbTable::ReaderStreams,
    DbTable::ReaderSavedState,
    DbTable::ReaderSavedPosts,
];

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
    /// Refused by the server on import, so only saved on this device.
    pub is_local_only: bool,
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

/// A post saved on this device, to import with [`ReaderSavedService::import_local_saves`]. The
/// fields besides the identity are shown until the server sends the post, and for good if the
/// server refuses the save.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ReaderLocalSave {
    pub identity: ReaderPostSaveIdentity,
    #[uniffi(default = None)]
    pub date_saved: Option<WpGmtDateTime>,
    #[uniffi(default = None)]
    pub title: Option<String>,
    #[uniffi(default = None)]
    pub excerpt: Option<String>,
    #[uniffi(default = None)]
    pub featured_image: Option<String>,
    #[uniffi(default = None)]
    pub site_name: Option<String>,
    #[uniffi(default = None)]
    pub site_url: Option<String>,
}

/// What [`ReaderSavedService::import_local_saves`] did.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ReaderSavedImportReport {
    /// The account's local saves were imported before, so nothing was added.
    pub already_imported: bool,
    /// Saves the server has.
    pub imported: u64,
    /// Saves the server refused, such as past its 1,000 save cap. They stay saved on this device.
    pub rejected: Vec<ReaderSavedKey>,
    /// Saves still waiting to be sent, which a later sync sends.
    pub pending: u64,
    /// Posts missing the IDs the API needs, or with an ID of `0` or an empty URL. They can't be
    /// imported or kept.
    pub unsaveable: u64,
}

#[uniffi::export]
impl ReaderSavedService {
    /// The saved list, from the cache.
    pub fn saved_posts(self: Arc<Self>) -> Arc<ReaderSavedPostsCollection> {
        Arc::new(ReaderSavedPostsCollection { service: self })
    }

    /// Adds the posts this device saved before saves were synced, given newest first, and sends
    /// them. Runs once per account: after that, it returns `already_imported` without adding
    /// anything.
    ///
    /// The saves are in the cache before anything is sent, so they show in the saved list, and a
    /// later sync sends any this one couldn't. A save the server refuses stays saved on this device.
    pub async fn import_local_saves(
        &self,
        saves: Vec<ReaderLocalSave>,
    ) -> Result<ReaderSavedImportReport, ReaderSavedError> {
        let mut unsaveable = 0;
        let mut keys = Vec::new();
        let added = self.cache.execute(|connection| {
            let transaction = connection.transaction()?;
            if ReaderSavedRepository::select_state(&transaction, self.account)?.imported_local_saves
            {
                return Ok(false);
            }
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
                record.title = save.title.clone();
                record.excerpt = save.excerpt.clone();
                record.featured_image = save.featured_image.clone();
                record.site_name = save.site_name.clone();
                record.site_url = save.site_url.clone();
                // The device's copy may be older than what the saved list already stored.
                let post = ReaderPostRepository::fill_missing(&transaction, self.account, &record)?;
                if ReaderSavedRepository::insert_imported(
                    &transaction,
                    self.account,
                    &key,
                    Some(post),
                    save.date_saved.as_ref(),
                )? {
                    keys.push(key);
                }
            }
            ReaderSavedRepository::set_imported_local_saves(&transaction, self.account)?;
            transaction.commit().map_err(SqliteDbError::from)?;
            Ok::<_, SqliteDbError>(true)
        })?;
        if !added {
            return Ok(ReaderSavedImportReport {
                already_imported: true,
                imported: 0,
                rejected: Vec::new(),
                pending: 0,
                unsaveable: 0,
            });
        }
        self.notify();

        let sent = {
            let _lane = self.lane.lock().await;
            self.send_pending().await?
        };
        let rows = self.rows_for(&keys)?;
        let rejected: Vec<_> = rows
            .iter()
            .filter(|row| row.local_only)
            .map(|row| row.key.clone())
            .collect();
        let pending = rows.iter().filter(|row| row.is_pending()).count() as u64;
        if let Some(error) = sent.unexpected {
            return Err(error);
        }
        // Saves that couldn't be sent are in `pending`, and a later sync sends them.
        sent.log();
        Ok(ReaderSavedImportReport {
            already_imported: false,
            imported: rows.len() as u64 - rejected.len() as u64 - pending,
            rejected,
            pending,
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
    ) -> Result<(), ReaderSavedError> {
        self.cache.execute(|connection| {
            let transaction = connection.transaction()?;
            let stream =
                ReaderStreamRepository::get_or_create(&transaction, self.account, SAVED_STREAM)?;
            for post in &response.posts {
                let Some(record) = ReaderPostRecord::from_saved_post(post) else {
                    continue;
                };
                let row = ReaderPostRepository::upsert(&transaction, self.account, &record)?;
                // The post's URL is the same save, as the server stores it: it finds the row of a
                // Jetpack post the list returns by its feed item while its WordPress.com copy
                // fails to load, since the keys give that row the URL.
                let url = post
                    .identity
                    .url
                    .clone()
                    .map(|url| ReaderSavedKey::Url { url })
                    .filter(ReaderSavedKey::is_complete);
                let keys: Vec<_> = reader_saved_keys_for_post(post.identity.clone())
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

    fn set_list_state(
        &self,
        state: ListState,
        error: Option<&ReaderSavedError>,
    ) -> Result<(), ReaderSavedError> {
        let message = error.map(ToString::to_string);
        self.cache.execute(|connection| {
            let stream =
                ReaderStreamRepository::get_or_create(&*connection, self.account, SAVED_STREAM)?;
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
    async fn refresh_list_in_lane(&self) -> Result<ReaderSavedPageResult, ReaderSavedError> {
        self.set_list_state(ListState::FetchingFirstPage, None)?;
        let result = async {
            self.sync_in_lane().await?;
            let response = self
                .client
                .reader_saved()
                .list_posts(&ReaderSavedPostsParams::default())
                .await?
                .data;
            self.store_page(&response, true)?;
            Ok(page_result(&response))
        }
        .await;
        self.list_result(result)
    }

    async fn load_next_page_in_lane(&self) -> Result<ReaderSavedPageResult, ReaderSavedError> {
        let stream = self.cache.execute(|connection| {
            ReaderStreamRepository::get_or_create(&*connection, self.account, SAVED_STREAM)
        })?;
        if stream.last_refreshed_at.is_none() {
            return self.refresh_list_in_lane().await;
        }
        let Some(page_handle) = stream.next_page_handle else {
            return Ok(ReaderSavedPageResult {
                post_count: 0,
                has_more: false,
            });
        };
        self.set_list_state(ListState::FetchingNextPage, None)?;
        let result = async {
            let params = ReaderSavedPostsParams {
                page_handle: Some(page_handle),
                ..Default::default()
            };
            let response = self.client.reader_saved().list_posts(&params).await?.data;
            self.store_page(&response, false)?;
            Ok(page_result(&response))
        }
        .await;
        self.list_result(result)
    }

    /// Records a failed page as the list's error, and returns the result.
    fn list_result(
        &self,
        result: Result<ReaderSavedPageResult, ReaderSavedError>,
    ) -> Result<ReaderSavedPageResult, ReaderSavedError> {
        if let Err(error) = &result
            && let Err(record_error) = self.set_list_state(ListState::Error, Some(error))
        {
            // The page's error is the one to return, so a failure to record it is only logged.
            log::error!("Couldn't record the saved list's error: {record_error}");
        }
        result
    }

    /// The saved list from the cache, newest first: what this device saved, what the list sent,
    /// and what only this device has. A post this device is unsaving is left out.
    pub(super) fn saved_list_items(&self) -> Result<Vec<ReaderSavedListItem>, ReaderSavedError> {
        let items = self
            .cache
            .execute(|connection| ReaderSavedRepository::select_list(&*connection, self.account))?;
        let mut shown = HashSet::new();
        Ok(items
            .into_iter()
            // A post saved under two keys shows once.
            .filter(|item| item.row.post_row_id.is_none_or(|post| shown.insert(post)))
            .map(|item| ReaderSavedListItem {
                is_pending: item.row.is_pending(),
                is_local_only: item.row.local_only,
                key: item.row.key,
                date_saved: item.row.date_saved,
                post: item.post,
            })
            .collect())
    }

    fn saved_list_info(&self) -> Result<ReaderSavedListInfo, ReaderSavedError> {
        let stream = self.cache.execute(|connection| {
            ReaderStreamRepository::select(&*connection, self.account, SAVED_STREAM)
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
}

#[uniffi::export]
impl ReaderSavedPostsCollection {
    /// The saved list, from the cache, newest first.
    pub async fn load_items(&self) -> Result<Vec<ReaderSavedListItem>, ReaderSavedError> {
        self.service.saved_list_items()
    }

    /// Syncs, then reloads the first page of the list. Posts past the first page stay, so the list
    /// is still whole offline.
    pub async fn refresh(&self) -> Result<ReaderSavedPageResult, ReaderSavedError> {
        let _lane = self.service.lane.lock().await;
        self.service.refresh_list_in_lane().await
    }

    /// Loads the page after the last one loaded, or refreshes if the list hasn't loaded yet. It
    /// waits for a refresh already running, then loads the page after it.
    pub async fn load_next_page(&self) -> Result<ReaderSavedPageResult, ReaderSavedError> {
        let _lane = self.service.lane.lock().await;
        self.service.load_next_page_in_lane().await
    }

    pub fn list_info(&self) -> Result<ReaderSavedListInfo, ReaderSavedError> {
        self.service.saved_list_info()
    }

    /// Whether a change to the cache is one the saved list shows.
    pub fn is_relevant_update(&self, hook: &UpdateHook) -> bool {
        SAVED_LIST_TABLES.contains(&hook.table)
    }
}

/// A new in-memory cache with every migration run.
#[cfg(test)]
pub(super) fn migrated_in_memory_cache() -> Arc<WpApiCache> {
    let cache = WpApiCache::new(None).expect("an in-memory cache opens");
    cache.perform_migrations().expect("migrations run");
    Arc::new(cache)
}

#[cfg(test)]
#[path = "storage_tests.rs"]
mod tests;

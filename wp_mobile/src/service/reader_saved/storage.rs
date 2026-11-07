//! The saved list as the cache keeps it, and the collection apps observe it through.

use super::{ReaderSavedError, ReaderSavedService, ReaderSavedStatus, may_have_been_applied};
use std::collections::HashSet;
use std::sync::Arc;
use wp_api::date::WpGmtDateTime;
use wp_api::prelude::WpApiError;
use wp_api::wp_com::reader_saved::{
    ReaderPostSaveIdentity, ReaderSavedBulkKey, ReaderSavedKey, ReaderSavedPostsParams,
    ReaderSavedPostsResponse, reader_saved_key_for_post, reader_saved_keys_for_post,
};
#[cfg(test)]
use wp_mobile_cache::WpApiCache;
use wp_mobile_cache::db_types::reader::{
    ReaderPostRecord, reader_date_from_text, reader_date_text,
};
use wp_mobile_cache::list_metadata::ListState;
use wp_mobile_cache::repository::TransactionManager;
use wp_mobile_cache::repository::reader::{
    ReaderPostRepository, ReaderSavedRepository, ReaderStreamRepository,
};
use wp_mobile_cache::{DbTable, SqliteDbError, UpdateHook};

/// The saved list's key in `reader_streams`.
const SAVED_STREAM: &str = "saved";

/// The tables the saved list is read from.
const SAVED_LIST_TABLES: [DbTable; 5] = [
    DbTable::ReaderPosts,
    DbTable::ReaderStreams,
    DbTable::ReaderStreamItems,
    DbTable::ReaderSavedState,
    DbTable::ReaderSavedLocalOnly,
];

/// A post in the saved list.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ReaderSavedListItem {
    /// The key the post is saved, or stored, under. To unsave, prefer
    /// [`ReaderSavedService::unsave`] with the post's identity when there's a post, because it
    /// covers every key the post may be saved under, and use [`ReaderSavedService::unsave_key`]
    /// otherwise.
    pub key: ReaderSavedKey,
    #[uniffi(default = None)]
    pub date_saved: Option<WpGmtDateTime>,
    /// What the cache has of the post: at least its identity, and whatever else the server sent.
    /// `None` for a post saved on this device that the saved list hasn't sent yet.
    #[uniffi(default = None)]
    pub post: Option<ReaderPostRecord>,
    /// Saved on this device, and not in the saved list yet.
    pub is_recent: bool,
    /// Refused by the server on import, so only saved on this device.
    pub is_local_only: bool,
}

/// What a page of the saved list did.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ReaderSavedPageResult {
    /// The posts the page had, after leaving out the ones this device recently unsaved.
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
/// fields besides the identity are shown if the server refuses the save, which keeps it on this
/// device.
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
    /// The account's local saves were imported before, so nothing was sent.
    pub already_imported: bool,
    /// Keys the server saved.
    pub imported: u64,
    /// Keys the server refused, such as past its 1,000 save cap. They stay saved on this device.
    pub rejected: Vec<ReaderSavedKey>,
    /// Posts missing the IDs the API needs. They can't be imported or kept.
    pub unsaveable: u64,
}

/// Which page of the saved list a fetch was, for what it stores.
pub(super) enum Page {
    /// Any page, from [`ReaderSavedService::list_posts`]: its posts are stored, but not where the
    /// list is up to.
    Any,
    /// The first page of a refresh, which started at this version of the stream.
    First { version: i64 },
    /// The next page, which started at this version of the stream.
    Next { version: i64 },
}

#[uniffi::export]
impl ReaderSavedService {
    /// The saved list, from the cache.
    pub fn saved_posts(self: Arc<Self>) -> Arc<ReaderSavedPostsCollection> {
        Arc::new(ReaderSavedPostsCollection { service: self })
    }

    /// Imports the posts this device saved before saves were synced, given newest first. Runs once
    /// per account: once it's finished, it returns `already_imported` without sending anything.
    ///
    /// Keys go through [`Self::import`], and pages saved by URL one at a time. A key the server
    /// refuses stays saved on this device, with the fields given, and shows in the saved list.
    ///
    /// Stops at the first failure that isn't the server refusing a key, such as the device going
    /// offline, and returns its error, so it runs again next time. Saving a post twice does nothing,
    /// so that's safe.
    pub async fn import_local_saves(
        &self,
        saves: Vec<ReaderLocalSave>,
    ) -> Result<ReaderSavedImportReport, ReaderSavedError> {
        let imported_before = self
            .cache
            .execute(|connection| ReaderSavedRepository::select_state(connection, self.account))?;
        if imported_before.imported_local_saves {
            return Ok(ReaderSavedImportReport {
                already_imported: true,
                imported: 0,
                rejected: Vec::new(),
                unsaveable: 0,
            });
        }

        let mut unsaveable = 0;
        let mut seen = HashSet::new();
        let mut posts = Vec::new();
        let mut bulk_keys = Vec::new();
        let mut urls = Vec::new();
        for save in saves {
            let Some(key) = reader_saved_key_for_post(save.identity.clone()) else {
                unsaveable += 1;
                continue;
            };
            if !seen.insert(key.normalized()) {
                continue;
            }
            match &key {
                ReaderSavedKey::Post { blog_id, post_id } => {
                    bulk_keys.push(ReaderSavedBulkKey::Post {
                        blog_id: *blog_id,
                        post_id: *post_id,
                    })
                }
                ReaderSavedKey::FeedItem { feed_item_id } => {
                    bulk_keys.push(ReaderSavedBulkKey::FeedItem {
                        feed_item_id: *feed_item_id,
                    })
                }
                ReaderSavedKey::Url { url } => urls.push(url.clone()),
            }
            posts.push((key, save));
        }

        let mut rejected = Vec::new();
        let results = self.import(bulk_keys.clone()).await?;
        for (key, result) in bulk_keys.iter().zip(&results) {
            if !result.saved {
                rejected.push(super::saved_key(key));
            }
        }
        for url in urls {
            let key = ReaderSavedKey::Url { url: url.clone() };
            match self.save_key(key.clone(), None, false).await {
                Ok(_) => {}
                Err(ReaderSavedError::Api(error)) if is_refused(&error) => rejected.push(key),
                Err(error) => return Err(error),
            }
        }

        let rejected_keys: HashSet<_> = rejected.iter().map(ReaderSavedKey::normalized).collect();
        self.cache.execute(|connection| {
            let transaction = TransactionManager::transaction(connection)?;
            for (key, save) in &posts {
                let Some(mut record) = ReaderPostRecord::from_identity(save.identity.clone())
                else {
                    continue;
                };
                record.title = save.title.clone();
                record.excerpt = save.excerpt.clone();
                record.featured_image = save.featured_image.clone();
                record.site_name = save.site_name.clone();
                record.site_url = save.site_url.clone();
                let row = ReaderPostRepository::upsert(&transaction, self.account, &record)?;
                if rejected_keys.contains(&key.normalized()) {
                    ReaderSavedRepository::upsert_local_only(
                        &transaction,
                        self.account,
                        key,
                        Some(row),
                        save.date_saved.as_ref().map(reader_date_text).as_deref(),
                    )?;
                }
            }
            ReaderSavedRepository::set_imported_local_saves(&transaction, self.account)?;
            transaction.commit().map_err(SqliteDbError::from)
        })?;
        self.inner().local_only.extend(rejected_keys);
        self.notify();

        Ok(ReaderSavedImportReport {
            already_imported: false,
            imported: (posts.len() - rejected.len()) as u64,
            rejected,
            unsaveable,
        })
    }

    /// The saves the server refused on import, which stay saved on this device.
    pub fn local_only_keys(&self) -> Vec<ReaderSavedKey> {
        let mut keys: Vec<_> = self.inner().local_only.iter().cloned().collect();
        keys.sort_by_key(wp_mobile_cache::db_types::reader::reader_saved_key_text);
        keys
    }
}

impl ReaderSavedService {
    /// Stores the posts of a page of the saved list, and for a page of the collection, where the
    /// list is up to. Returns `false`, storing nothing, for a page a refresh has made stale.
    pub(super) fn store_listed_posts(
        &self,
        response: &ReaderSavedPostsResponse,
        page: Page,
    ) -> Result<bool, ReaderSavedError> {
        self.cache.execute(|connection| {
            let transaction = TransactionManager::transaction(connection)?;
            let stream =
                ReaderStreamRepository::get_or_create(&transaction, self.account, SAVED_STREAM)?;
            if let Page::First { version } | Page::Next { version } = page
                && stream.version != version
            {
                return Ok(false);
            }
            for post in &response.posts {
                let Some(record) = ReaderPostRecord::from_saved_post(post) else {
                    continue;
                };
                let row = ReaderPostRepository::upsert(&transaction, self.account, &record)?;
                let sort_key = post
                    .date_saved
                    .as_ref()
                    .map(reader_date_text)
                    .unwrap_or_default();
                ReaderStreamRepository::upsert_item(&transaction, stream.row_id, row, &sort_key)?;
            }
            if let Page::First { .. } | Page::Next { .. } = page {
                ReaderStreamRepository::set_cursor(
                    &transaction,
                    stream.row_id,
                    response.next_page_handle.as_deref(),
                    matches!(page, Page::First { .. }),
                )?;
            }
            transaction.commit().map_err(SqliteDbError::from)?;
            Ok(true)
        })
    }

    fn saved_stream_state(
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

    /// Records a failed page as the list's error, and returns the error.
    fn page_failed(&self, error: ReaderSavedError) -> ReaderSavedError {
        // The page's error is the one to return, so a failure to record it is only logged.
        if let Err(record_error) = self.saved_stream_state(ListState::Error, Some(&error)) {
            log::error!("Couldn't record the saved list's error: {record_error}");
        }
        error
    }

    /// Removes the posts this device knows aren't saved any more from the stored list. Posts it
    /// doesn't know about stay, so the list stays whole past the pages loaded since launch.
    fn prune_unsaved(&self) -> Result<(), ReaderSavedError> {
        let items = self.cache.execute(|connection| {
            let Some(stream) =
                ReaderStreamRepository::select(&*connection, self.account, SAVED_STREAM)?
            else {
                return Ok(Vec::new());
            };
            Ok::<_, SqliteDbError>(
                ReaderStreamRepository::select_items(&*connection, stream.row_id)?
                    .into_iter()
                    .map(|item| (stream.row_id, item))
                    .collect::<Vec<_>>(),
            )
        })?;
        let unsaved: Vec<_> = items
            .into_iter()
            .filter(|(_, item)| {
                self.status(item.post.identity.clone()) == ReaderSavedStatus::NotSaved
            })
            .collect();
        if unsaved.is_empty() {
            return Ok(());
        }
        self.cache.execute(|connection| {
            let transaction = TransactionManager::transaction(connection)?;
            for (stream, item) in &unsaved {
                ReaderStreamRepository::remove_item(&transaction, *stream, item.post_row_id)?;
            }
            transaction.commit().map_err(SqliteDbError::from)
        })?;
        Ok(())
    }

    async fn refresh_saved_list(&self) -> Result<ReaderSavedPageResult, ReaderSavedError> {
        let version = self.cache.execute(|connection| {
            let stream =
                ReaderStreamRepository::get_or_create(&*connection, self.account, SAVED_STREAM)?;
            ReaderStreamRepository::begin_refresh(&*connection, stream.row_id)
        })?;
        let result = async {
            self.refresh().await?;
            let response = self.fetch_posts(&ReaderSavedPostsParams::default()).await?;
            if !self.store_listed_posts(&response, Page::First { version })? {
                // A newer refresh started, and stores its own page.
                return Ok(page_result(&response));
            }
            self.prune_unsaved()?;
            Ok(page_result(&response))
        }
        .await;
        result.map_err(|error| self.page_failed(error))
    }

    async fn load_next_saved_page(&self) -> Result<ReaderSavedPageResult, ReaderSavedError> {
        let stream = self.cache.execute(|connection| {
            ReaderStreamRepository::get_or_create(&*connection, self.account, SAVED_STREAM)
        })?;
        if stream.last_refreshed_at.is_none() {
            return self.refresh_saved_list().await;
        }
        let Some(page_handle) = stream.next_page_handle else {
            return Ok(ReaderSavedPageResult {
                post_count: 0,
                has_more: false,
            });
        };
        self.saved_stream_state(ListState::FetchingNextPage, None)?;
        let result = async {
            let params = ReaderSavedPostsParams {
                page_handle: Some(page_handle),
                ..Default::default()
            };
            let response = self.fetch_posts(&params).await?;
            if !self.store_listed_posts(
                &response,
                Page::Next {
                    version: stream.version,
                },
            )? {
                return Err(ReaderSavedError::StaleLoadMore);
            }
            Ok(page_result(&response))
        }
        .await;
        result.map_err(|error| match error {
            // The refresh that made the page stale owns the list's state.
            ReaderSavedError::StaleLoadMore => error,
            error => self.page_failed(error),
        })
    }

    /// The saved list from the cache: this device's recent saves first, then the saved list newest
    /// first, then the saves only this device has. A post this device knows isn't saved is left
    /// out, even while the cache still has it.
    fn saved_list_items(&self) -> Result<Vec<ReaderSavedListItem>, ReaderSavedError> {
        let (stream_items, local_only) = self.cache.execute(|connection| {
            let stream_items =
                match ReaderStreamRepository::select(&*connection, self.account, SAVED_STREAM)? {
                    Some(stream) => {
                        ReaderStreamRepository::select_items(&*connection, stream.row_id)?
                    }
                    None => Vec::new(),
                };
            let local_only = ReaderSavedRepository::select_local_only(&*connection, self.account)?;
            Ok::<_, SqliteDbError>((stream_items, local_only))
        })?;
        let recent = self.recent_saves();
        let recent_posts = self.cache.execute(|connection| {
            recent
                .iter()
                .map(|key| {
                    ReaderPostRepository::select_by_key(&*connection, self.account, key)
                        .map(|found| found.map(|(_, post)| post))
                })
                .collect::<Result<Vec<_>, _>>()
        })?;

        let mut items = Vec::new();
        let mut shown = HashSet::new();
        let state = self.state();
        for (key, post) in recent.into_iter().zip(recent_posts) {
            shown.extend(state.post_keys(std::slice::from_ref(&key)));
            items.push(ReaderSavedListItem {
                key,
                date_saved: None,
                post,
                is_recent: true,
                is_local_only: false,
            });
        }
        for item in stream_items {
            let keys = reader_saved_keys_for_post(item.post.identity.clone());
            let post_keys = state.post_keys(&keys);
            if post_keys.iter().any(|key| shown.contains(key))
                || state.status(&keys) == ReaderSavedStatus::NotSaved
            {
                continue;
            }
            shown.extend(post_keys);
            items.push(ReaderSavedListItem {
                key: item.post.key.clone(),
                date_saved: (!item.sort_key.is_empty())
                    .then(|| reader_date_from_text(&item.sort_key))
                    .transpose()?,
                post: Some(item.post),
                is_recent: false,
                is_local_only: false,
            });
        }
        for save in local_only {
            let post_keys = state.post_keys(std::slice::from_ref(&save.key));
            if post_keys.iter().any(|key| shown.contains(key)) {
                continue;
            }
            shown.extend(post_keys);
            items.push(ReaderSavedListItem {
                key: save.key,
                date_saved: save.date_saved,
                post: save.post,
                is_recent: false,
                is_local_only: true,
            });
        }
        Ok(items)
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

/// Whether the server refused a save, rather than the request failing on the way.
fn is_refused(error: &WpApiError) -> bool {
    let status_code = match error {
        WpApiError::WpError { status_code, .. }
        | WpApiError::InvalidHttpStatusCode { status_code, .. }
        | WpApiError::UnknownError { status_code, .. } => *status_code,
        _ => return false,
    };
    (400..500).contains(&status_code) && !may_have_been_applied(error)
}

/// The account's saved posts, as the cache has them.
///
/// Read with [`Self::load_items`], which doesn't touch the network, and reload it when
/// [`Self::is_relevant_update`] says a change to the cache is one it shows. [`Self::refresh`]
/// fetches the first page, and [`Self::load_next_page`] the rest.
#[derive(uniffi::Object)]
pub struct ReaderSavedPostsCollection {
    service: Arc<ReaderSavedService>,
}

#[uniffi::export]
impl ReaderSavedPostsCollection {
    /// The saved list, from the cache. See [`ReaderSavedService::saved_posts`] for the order.
    pub async fn load_items(&self) -> Result<Vec<ReaderSavedListItem>, ReaderSavedError> {
        self.service.saved_list_items()
    }

    /// Reloads the saved keys and the first page of the list. Posts past the first page stay, so
    /// the list is still whole offline. Posts this device knows aren't saved are removed.
    pub async fn refresh(&self) -> Result<ReaderSavedPageResult, ReaderSavedError> {
        self.service.refresh_saved_list().await
    }

    /// Loads the page after the last one loaded, or refreshes if the list hasn't loaded yet.
    ///
    /// Fails with [`ReaderSavedError::StaleLoadMore`] if a refresh started while it loaded.
    pub async fn load_next_page(&self) -> Result<ReaderSavedPageResult, ReaderSavedError> {
        self.service.load_next_saved_page().await
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

//! Saved posts for reading with no connection: content for every save, and their images kept as
//! files.
//!
//! A save can reach this device without its content: one another device made, one from an
//! import, one made by tapping a link. [`ReaderSavedService::download_for_offline`] fetches what's
//! missing. The saved list sends whole posts, so it loads the list until every save has its
//! content, then fetches on its own any save the list didn't send.
//!
//! Images are stored only once the app says where, with
//! [`ReaderSavedService::set_offline_image_store`], and only by a download the app asks to include
//! them, so the app decides whether to wait for Wi-Fi. They stay inside the size the app gave:
//! when a newer save's images need room, the oldest saves' images go, and their text stays.
//!
//! An image is fetched into memory and then written, so one over 20 MB is left out. Video and
//! audio aren't stored.

use super::storage::{CATCH_UP_PAGE_SIZE, SavedOrder};
use super::{ReaderSavedError, ReaderSavedService};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use wp_api::date::WpGmtDateTime;
use wp_api::posts::PostId;
use wp_api::prelude::{RequestExecutionError, RequestExecutionErrorReason, WpApiError};
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::reader_offline_images::{reader_image_file_extension, reader_image_request};
use wp_api::wp_com::reader_post::ReaderPostParams;
use wp_api::wp_com::reader_post_display::{
    reader_post_content_image_urls, reader_post_content_with_stored_images,
};
use wp_api::wp_com::reader_posts::ReaderPost;
use wp_api::wp_com::reader_saved::{ReaderFeedId, ReaderFeedItemId, ReaderSavedKey};
use wp_mobile_cache::db_types::reader::{
    DbReaderOfflineImages, DbReaderSavedOffline, ReaderPostRecord,
};
use wp_mobile_cache::repository::reader::{ReaderPostRepository, ReaderSavedRepository};
use wp_mobile_cache::repository::reader_offline::{
    DbReaderPostForImages, DbReaderSaveWithoutContent, ReaderOfflineRepository,
};
use wp_mobile_cache::{RowId, SqliteDbError};

/// The most pages of the saved list one download loads to get saves' content: enough for the
/// most saves an account can have.
const MAX_CATCH_UP_PAGES: u32 = 30;

/// The most saves one download fetches on their own, after the saved list.
const MAX_FETCHES_PER_DOWNLOAD: usize = 50;

/// How many times a save's content is tried, counting only tries the server answered.
const MAX_ATTEMPTS: u32 = 5;

/// How long after a failed try a save's content is tried again.
const RETRY_AFTER: Duration = Duration::from_secs(60 * 60);

/// The biggest image that's stored.
const MAX_IMAGE_BYTES: usize = 20 * 1024 * 1024;

/// What every stored file's name starts with. Only such files are ever deleted from the
/// directory, so nothing else an app keeps there is touched.
const FILE_PREFIX: &str = "rdr-";

/// What a single post is asked for with: its site's or feed's details, as the saved list has them.
const POST_META: &str = "site,feed";

const NOT_IN_THE_LIST: &str =
    "The saved list didn't send this post, and it can't be fetched on its own.";

/// Whether a saved post can be read with no connection.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum ReaderOfflineState {
    /// Its content isn't on the device yet.
    NotDownloaded,
    /// Its content or its images are being fetched.
    Downloading,
    /// Its content is on the device, so it can be read. `has_images` says whether its images are
    /// too: they aren't until a download stores them, and they go first when room runs out.
    Ready { has_images: bool },
    /// Its content couldn't be fetched. It's tried again by a later download, and straight away by
    /// [`ReaderSavedService::retry_offline`].
    Failed { reason: String },
}

impl ReaderOfflineState {
    pub(super) fn of(post: Option<&ReaderPostRecord>, offline: &DbReaderSavedOffline) -> Self {
        if offline.downloading {
            return Self::Downloading;
        }
        if offline.fetched || post.is_some_and(|post| post.content.is_some()) {
            return Self::Ready {
                has_images: offline.images == DbReaderOfflineImages::Stored,
            };
        }
        match &offline.error {
            Some(reason) => Self::Failed {
                reason: reason.clone(),
            },
            None => Self::NotDownloaded,
        }
    }
}

/// What the device has of the saved posts after [`ReaderSavedService::download_for_offline`].
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ReaderOfflineReport {
    /// Saves whose content is on the device.
    pub ready: u64,
    /// Of those, the saves whose images are too.
    pub with_images: u64,
    /// Saves whose content couldn't be fetched. Each says why in the saved list.
    pub failed: u64,
    /// Saves whose content hasn't been fetched yet.
    pub not_downloaded: u64,
    /// How much room the stored images take, in bytes.
    pub image_bytes: u64,
    /// Why the download stopped before it had tried everything, such as no connection. `None`
    /// when it did all it could.
    #[uniffi(default = None)]
    pub stopped_because: Option<String>,
}

/// A saved post for reading with no connection.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ReaderOfflinePost {
    /// The post, with each stored image in its content pointing at its copy.
    pub post: ReaderPostRecord,
    /// The post's stored images.
    pub images: Vec<ReaderOfflineImage>,
}

/// An image of a saved post, stored on the device.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ReaderOfflineImage {
    /// The image's address, as the post has it.
    pub url: String,
    /// The file's name in the directory given to
    /// [`ReaderSavedService::set_offline_image_store`].
    pub file_name: String,
    pub bytes: u64,
}

/// Where saved posts' images are kept, and how much room they may take.
#[derive(Debug, Clone)]
pub(super) struct ImageStore {
    directory: PathBuf,
    max_bytes: u64,
}

/// What became of a try to fetch one save's content.
enum Fetch {
    /// It's stored, or the server answered that it can't be had.
    Answered,
    /// No answer came, so nothing more is tried now.
    Stopped(String),
}

/// What became of a try to fetch one image.
enum Image {
    Fetched {
        bytes: Vec<u8>,
        extension: &'static str,
    },
    /// It can't be had: the server said so, or it isn't an image.
    Unavailable,
    /// It may be had another time.
    Later,
    /// No answer came, so nothing more is tried now.
    Stopped(String),
}

/// What became of a try to store a post's images.
enum Images {
    Stored,
    /// Some may be had another time.
    Later,
    /// They don't fit, and no older save has images to remove for them.
    NoRoom,
    Stopped(String),
}

#[uniffi::export]
impl ReaderSavedService {
    /// Says where to keep saved posts' images, and how much room they may take, in bytes. Until
    /// it's called, only posts' text is kept.
    ///
    /// `directory` is created if it isn't there. Give one that is only for these files and isn't
    /// cleared by the system, and give the same one each launch: the files are found by name.
    /// Readers on one cache can share it. Nothing is fetched until
    /// [`Self::download_for_offline`] is asked to include images.
    pub fn set_offline_image_store(
        &self,
        directory: String,
        max_bytes: u64,
    ) -> Result<(), ReaderSavedError> {
        let directory = PathBuf::from(directory);
        std::fs::create_dir_all(&directory)
            .map_err(|error| files_error("create the directory", &directory, error))?;
        *self.image_store.lock().expect("image store lock poisoned") = Some(ImageStore {
            directory,
            max_bytes,
        });
        Ok(())
    }

    /// Deletes every stored image of this reader's saved posts, and stops storing them until
    /// [`Self::set_offline_image_store`] is called again. The posts' text stays.
    pub fn remove_offline_images(&self) -> Result<(), ReaderSavedError> {
        let names = self.cache.execute(|connection| {
            let transaction = connection.transaction()?;
            let names = ReaderOfflineRepository::delete_all_files(&transaction, self.account)?;
            transaction.commit().map_err(SqliteDbError::from)?;
            Ok::<_, SqliteDbError>(names)
        })?;
        let store = self
            .image_store
            .lock()
            .expect("image store lock poisoned")
            .take();
        if let Some(store) = store {
            delete_files(&store.directory, &names)?;
        }
        self.notify();
        Ok(())
    }

    /// How much room the stored images take, in bytes. It counts every reader's that share the
    /// cache.
    pub fn offline_image_bytes(&self) -> Result<u64, ReaderSavedError> {
        Ok(self
            .cache
            .execute(|connection| ReaderOfflineRepository::stored_bytes(&*connection))?)
    }

    /// Gets every saved post ready to read with no connection. Call it where [`Self::sync`] is
    /// called, when there's a connection worth using: it syncs first.
    ///
    /// It fetches the content of each save that has none, by loading the saved list as far as it
    /// needs to and then fetching what's left one post at a time. A save the server can't give
    /// the content of reads as [`ReaderOfflineState::Failed`], and is tried again an hour later,
    /// five times at most.
    ///
    /// With `include_images`, and once [`Self::set_offline_image_store`] has said where, it then
    /// stores the images of each saved post, newest save first. When they'd take more room than
    /// allowed, the oldest saves' images are removed to fit the newer ones.
    ///
    /// A failure to reach the server isn't an error here: the report says why the download
    /// stopped, and what the device has so far.
    pub async fn download_for_offline(
        &self,
        include_images: bool,
    ) -> Result<ReaderOfflineReport, ReaderSavedError> {
        let _lane = self.lane.lock().await;
        self.remove_files_of_unsaved_posts()?;
        self.remove_stray_files()?;
        let mut stopped = self.fetch_missing_content().await?;
        // Content was just fetched or checked, so the device is online unless that stopped.
        if include_images && stopped.is_none() {
            stopped = self.store_images(!self.signed_out).await?;
        }
        self.offline_report(stopped)
    }

    /// Tries again, now, to fetch the content of a save that
    /// [`ReaderOfflineState::Failed`], whatever its earlier tries. Returns what it is afterwards.
    /// Fails with [`ReaderSavedError::OfflineUnreachable`] if the server can't be reached.
    pub async fn retry_offline(
        &self,
        key: ReaderSavedKey,
    ) -> Result<ReaderOfflineState, ReaderSavedError> {
        let key = key.normalized();
        let _lane = self.lane.lock().await;
        let found = self.cache.execute(|connection| {
            let Some((row, _)) =
                ReaderSavedRepository::select_row_id_for_key(&*connection, self.account, &key)?
            else {
                return Ok(None);
            };
            ReaderOfflineRepository::forget_failures(&*connection, row)?;
            let save = ReaderOfflineRepository::select_without_content(&*connection, self.account)?
                .into_iter()
                .find(|save| save.row_id == row);
            Ok::<_, SqliteDbError>(Some((row, save)))
        })?;
        let Some((row, save)) = found else {
            return Ok(ReaderOfflineState::NotDownloaded);
        };
        if let Some(save) = save
            && let Fetch::Stopped(reason) = self.fetch_save(&save).await?
        {
            return Err(ReaderSavedError::OfflineUnreachable { reason });
        }
        // Read from the save's own row: one that failed again isn't in the saved list.
        Ok(self.cache.execute(|connection| {
            let post =
                ReaderSavedRepository::select_row_id_for_key(&*connection, self.account, &key)?
                    .and_then(|(_, post)| post);
            let post = match post {
                Some(post) => ReaderPostRepository::select_by_row_id(&*connection, post)?,
                None => None,
            };
            let offline = ReaderOfflineRepository::select(&*connection, row)?;
            Ok::<_, SqliteDbError>(ReaderOfflineState::of(post.as_ref(), &offline))
        })?)
    }

    /// A saved post for reading with no connection, or `None` if the key isn't saved or the
    /// cache has no post for it.
    ///
    /// Each stored image in its content points at `image_url_prefix` followed by the file's
    /// name, and so do [`ReaderPostRecord::featured_image`] and
    /// [`ReaderPostRecord::display_image_url`] when they're stored. Pass the directory's `file:`
    /// address with a `/` at the end, or the prefix of a scheme the app's web view serves the
    /// files through. An image that isn't stored keeps its own address.
    pub fn offline_post(
        &self,
        key: ReaderSavedKey,
        image_url_prefix: String,
    ) -> Result<Option<ReaderOfflinePost>, ReaderSavedError> {
        let key = key.normalized();
        Ok(self.cache.execute(|connection| {
            let rows = ReaderSavedRepository::select_rows_for_keys(
                &*connection,
                self.account,
                std::slice::from_ref(&key),
            )?;
            let Some(post_row) = rows
                .iter()
                .find(|row| row.saved)
                .and_then(|row| row.post_row_id)
            else {
                return Ok(None);
            };
            let Some(mut post) = ReaderPostRepository::select_by_row_id(&*connection, post_row)?
            else {
                return Ok(None);
            };
            let images: Vec<_> = ReaderOfflineRepository::select_files(&*connection, post_row)?
                .into_iter()
                .filter_map(|file| {
                    Some(ReaderOfflineImage {
                        url: file.url,
                        file_name: file.file_name?,
                        bytes: file.bytes,
                    })
                })
                .collect();
            let stored: HashMap<String, String> = images
                .iter()
                .map(|image| {
                    (
                        image.url.clone(),
                        format!("{image_url_prefix}{}", image.file_name),
                    )
                })
                .collect();
            let copy = |url: &Option<String>| {
                url.as_ref()
                    .and_then(|url| stored.get(url.trim()).cloned())
                    .or_else(|| url.clone())
            };
            post.featured_image = copy(&post.featured_image);
            post.display_image_url = copy(&post.display_image_url);
            post.content = post
                .content
                .map(|content| reader_post_content_with_stored_images(&content, stored));
            Ok::<_, SqliteDbError>(Some(ReaderOfflinePost { post, images }))
        })?)
    }
}

impl ReaderSavedService {
    fn image_store(&self) -> Option<ImageStore> {
        self.image_store
            .lock()
            .expect("image store lock poisoned")
            .clone()
    }

    /// Deletes the stored images of posts that aren't saved any more. Called after an unsave,
    /// and before a download.
    pub(super) fn remove_files_of_unsaved_posts(&self) -> Result<(), ReaderSavedError> {
        let names = self.cache.execute(|connection| {
            let transaction = connection.transaction()?;
            let names =
                ReaderOfflineRepository::delete_files_of_unsaved_posts(&transaction, self.account)?;
            transaction.commit().map_err(SqliteDbError::from)?;
            Ok::<_, SqliteDbError>(names)
        })?;
        // Without a store, the files wait for one: a download removes files no post has.
        if let Some(store) = self.image_store() {
            delete_files(&store.directory, &names)?;
        }
        Ok(())
    }

    /// Deletes the files in the directory that are this library's and that no post has: one
    /// written just before the app stopped, or one whose post was deleted with its account.
    fn remove_stray_files(&self) -> Result<(), ReaderSavedError> {
        let Some(store) = self.image_store() else {
            return Ok(());
        };
        let used = self
            .cache
            .execute(|connection| ReaderOfflineRepository::select_file_names(&*connection))?;
        let entries = std::fs::read_dir(&store.directory)
            .map_err(|error| files_error("read the directory", &store.directory, error))?;
        let mut strays = Vec::new();
        for entry in entries {
            let entry = entry
                .map_err(|error| files_error("read the directory", &store.directory, error))?;
            if let Some(name) = entry.file_name().to_str()
                && is_stored_file_name(name)
                && !used.contains(name)
            {
                strays.push(name.to_string());
            }
        }
        delete_files(&store.directory, &strays)
    }

    fn count_without_content(&self) -> Result<u64, ReaderSavedError> {
        Ok(self.cache.execute(|connection| {
            ReaderOfflineRepository::count_without_content(&*connection, self.account)
        })?)
    }

    /// Fetches the content of every save that has none. Returns why it stopped, if it did before
    /// it had tried them all. Call it holding the lane.
    async fn fetch_missing_content(&self) -> Result<Option<String>, ReaderSavedError> {
        if !self.signed_out {
            // The saved list sends whole posts, many to a request, so it comes first.
            let order = SavedOrder::NEWEST_SAVE_FIRST;
            let mut page = match self.refresh_list_in_lane(order).await {
                Ok(page) => page,
                Err(error) => return stopped_or_failed(error),
            };
            let mut pages = 1;
            while page.has_more && pages < MAX_CATCH_UP_PAGES && self.count_without_content()? > 0 {
                page = match self
                    .load_next_page_in_lane(order, Some(CATCH_UP_PAGE_SIZE))
                    .await
                {
                    Ok(page) => page,
                    Err(error) => return stopped_or_failed(error),
                };
                pages += 1;
            }
        }

        let now = self.now();
        let saves = self.cache.execute(|connection| {
            ReaderOfflineRepository::select_without_content(&*connection, self.account)
        })?;
        let mut fetched = 0;
        for save in saves.iter().filter(|save| is_due(&save.offline, &now)) {
            if fetched == MAX_FETCHES_PER_DOWNLOAD {
                break;
            }
            match self.fetch_save(save).await? {
                Fetch::Answered => fetched += 1,
                Fetch::Stopped(reason) => return Ok(Some(reason)),
            }
        }
        Ok(None)
    }

    /// Fetches the content of one save on its own, and records what came of it.
    async fn fetch_save(
        &self,
        save: &DbReaderSaveWithoutContent,
    ) -> Result<Fetch, ReaderSavedError> {
        enum Source {
            Site(WpComSiteId, PostId),
            Feed(ReaderFeedId, ReaderFeedItemId),
        }
        // A feed item's key doesn't say its feed, and a page saved by its address has no post to
        // fetch: only the saved list sends those.
        let source = match (&save.key, save.feed_id) {
            (ReaderSavedKey::Post { blog_id, post_id }, _) => {
                Some(Source::Site(*blog_id, *post_id))
            }
            (ReaderSavedKey::FeedItem { feed_item_id }, Some(feed_id)) => {
                Some(Source::Feed(feed_id, *feed_item_id))
            }
            _ => None,
        };
        let Some(source) = source else {
            self.record_failure(save.row_id, NOT_IN_THE_LIST)?;
            return Ok(Fetch::Answered);
        };

        self.cache.execute(|connection| {
            ReaderOfflineRepository::set_downloading(&*connection, save.row_id, true)
        })?;
        self.notify();
        let params = ReaderPostParams {
            meta: Some(POST_META.to_string()),
            fields: None,
        };
        let posts = self.client.reader_post();
        let result = match source {
            Source::Site(site, post) => posts
                .site_post(&site, &post, &params)
                .await
                .map(|response| response.data),
            Source::Feed(feed, item) => posts
                .feed_post(&feed, &item, &params)
                .await
                .map(|response| response.data),
        };
        match result {
            Ok(post) => {
                self.store_fetched(save.row_id, &post)?;
                Ok(Fetch::Answered)
            }
            Err(error @ WpApiError::RequestExecutionFailed { .. }) => {
                self.cache.execute(|connection| {
                    ReaderOfflineRepository::set_downloading(&*connection, save.row_id, false)
                })?;
                self.notify();
                Ok(Fetch::Stopped(error.to_string()))
            }
            Err(error) => {
                self.record_failure(save.row_id, &error.to_string())?;
                Ok(Fetch::Answered)
            }
        }
    }

    fn record_failure(&self, saved: RowId, reason: &str) -> Result<(), ReaderSavedError> {
        let now = self.now();
        self.cache.execute(|connection| {
            ReaderOfflineRepository::record_failure(&*connection, saved, reason, &now)
        })?;
        self.notify();
        Ok(())
    }

    /// Stores a post fetched for a save, as that save's post.
    fn store_fetched(&self, saved: RowId, post: &ReaderPost) -> Result<(), ReaderSavedError> {
        let Some(record) = ReaderPostRecord::from_reader_post(post) else {
            return self.record_failure(saved, "The post came without the IDs it's stored under.");
        };
        self.cache.execute(|connection| {
            let transaction = connection.transaction()?;
            let row = ReaderPostRepository::upsert(&transaction, self.account, &record)?;
            ReaderSavedRepository::link_post(&transaction, saved, row)?;
            ReaderOfflineRepository::set_fetched(&transaction, saved)?;
            transaction.commit().map_err(SqliteDbError::from)
        })?;
        self.notify();
        Ok(())
    }

    /// Stores the images of every saved post that has its content, newest save first. Returns
    /// why it stopped, if it did before it had tried them all. `online` is whether a request was
    /// just answered: then an image whose host can't be reached is the host's doing, and not the
    /// device's. Call it holding the lane.
    async fn store_images(&self, online: bool) -> Result<Option<String>, ReaderSavedError> {
        let Some(store) = self.image_store() else {
            return Ok(None);
        };
        std::fs::create_dir_all(&store.directory)
            .map_err(|error| files_error("create the directory", &store.directory, error))?;
        let posts = self.cache.execute(|connection| {
            ReaderOfflineRepository::select_posts_for_images(&*connection, self.account)
        })?;
        // The posts whose images went to make room during this download.
        let mut removed = HashSet::new();
        // Whether a post's images didn't fit. Newer saves have the room then, so no older post's
        // are fetched: the posts with images are always the newest ones.
        let mut full = false;
        for (index, post) in posts.iter().enumerate() {
            if self.image_store().is_none() {
                // The app stopped storing images while this ran.
                break;
            }
            if post.images == DbReaderOfflineImages::Stored {
                continue;
            }
            let known: HashSet<String> = self
                .cache
                .execute(|connection| {
                    ReaderOfflineRepository::select_files(&*connection, post.post)
                })?
                .into_iter()
                .map(|file| file.url)
                .collect();
            let missing: Vec<String> = image_urls(post)
                .into_iter()
                .filter(|url| !known.contains(url))
                .collect();
            if missing.is_empty() {
                self.set_images(post.post, DbReaderOfflineImages::Stored)?;
                continue;
            }
            let older = &posts[index + 1..];
            let has_room = |removed: &HashSet<RowId>| -> Result<bool, ReaderSavedError> {
                Ok(self.offline_image_bytes()? < store.max_bytes
                    || older.iter().any(|older| can_make_room(older, removed)))
            };
            if full || !has_room(&removed)? {
                full = true;
                if post.images != DbReaderOfflineImages::NoRoom {
                    self.set_images(post.post, DbReaderOfflineImages::NoRoom)?;
                }
                // An older post with no images to store is still complete, so the rest are read.
                continue;
            }

            self.set_downloading_images(post.post, true)?;
            let outcome = self
                .store_post_images(&store, post, &missing, older, &mut removed, online)
                .await;
            self.set_downloading_images(post.post, false)?;
            match outcome? {
                Images::Stored => self.set_images(post.post, DbReaderOfflineImages::Stored)?,
                Images::Later => {}
                Images::NoRoom => {
                    // Part of a post's images is no use, and takes room.
                    self.remove_images(&store, post.post)?;
                    full = true;
                }
                Images::Stopped(reason) => return Ok(Some(reason)),
            }
        }
        Ok(None)
    }

    /// Stores the images of one post that aren't stored yet. `older` are the saved posts older
    /// than it, newest first: the ones whose images go to make room.
    async fn store_post_images(
        &self,
        store: &ImageStore,
        post: &DbReaderPostForImages,
        missing: &[String],
        older: &[DbReaderPostForImages],
        removed: &mut HashSet<RowId>,
        online: bool,
    ) -> Result<Images, ReaderSavedError> {
        let mut later = false;
        for url in missing {
            // Another saved post may have the same image.
            let shared = self.cache.execute(|connection| {
                ReaderOfflineRepository::select_stored_file(&*connection, self.account, url)
            })?;
            if let Some((name, bytes)) = shared {
                self.record_file(post.post, url, Some(&name), bytes)?;
                continue;
            }
            match self.fetch_image(url, post.site_is_private, online).await {
                Image::Fetched { bytes, extension } => {
                    let size = bytes.len() as u64;
                    if size > store.max_bytes {
                        // It can never fit, so the post is whole without it.
                        self.record_file(post.post, url, None, 0)?;
                        continue;
                    }
                    while self.offline_image_bytes()? + size > store.max_bytes {
                        let Some(oldest) = older
                            .iter()
                            .rev()
                            .find(|older| can_make_room(older, removed))
                        else {
                            return Ok(Images::NoRoom);
                        };
                        self.remove_images(store, oldest.post)?;
                        removed.insert(oldest.post);
                    }
                    let name = format!("{FILE_PREFIX}{:032x}.{extension}", rand::random::<u128>());
                    let path = store.directory.join(&name);
                    std::fs::write(&path, &bytes)
                        .map_err(|error| files_error("write", &path, error))?;
                    self.record_file(post.post, url, Some(&name), size)?;
                }
                Image::Unavailable => self.record_file(post.post, url, None, 0)?,
                Image::Later => later = true,
                Image::Stopped(reason) => return Ok(Images::Stopped(reason)),
            }
        }
        Ok(if later { Images::Later } else { Images::Stored })
    }

    /// Fetches one image. An image of a private site is asked for as the account, which only
    /// WordPress.com is told.
    async fn fetch_image(&self, url: &str, site_is_private: bool, online: bool) -> Image {
        let authentication = site_is_private.then_some(&*self.auth_provider);
        let Some(request) = reader_image_request(url, authentication) else {
            return Image::Unavailable;
        };
        match self.request_executor.execute(Arc::new(request)).await {
            Ok(response) => match response.status_code {
                200..=299 => match reader_image_file_extension(&response) {
                    Some(extension)
                        if !response.body.is_empty() && response.body.len() <= MAX_IMAGE_BYTES =>
                    {
                        Image::Fetched {
                            bytes: response.body,
                            extension,
                        }
                    }
                    _ => Image::Unavailable,
                },
                408 | 429 | 500..=599 => Image::Later,
                _ => Image::Unavailable,
            },
            Err(RequestExecutionError::RequestExecutionFailed {
                status_code,
                reason,
                ..
            }) => {
                use RequestExecutionErrorReason as Reason;
                match (&reason, status_code) {
                    (Reason::DeviceIsOfflineError { .. } | Reason::CancellationError, _) => {
                        Image::Stopped(reason.to_string())
                    }
                    (_, Some(408 | 429 | 500..=599)) => Image::Later,
                    // The host answered, and not with the image.
                    (_, Some(_)) => Image::Unavailable,
                    (Reason::HttpTimeoutError, None) if online => Image::Later,
                    // A host that's gone, or a certificate that's wrong: the image can't be had,
                    // as long as the device itself is known to be online.
                    (_, None) if online => Image::Unavailable,
                    (_, None) => Image::Stopped(reason.to_string()),
                }
            }
            Err(error) => Image::Stopped(error.to_string()),
        }
    }

    fn record_file(
        &self,
        post: RowId,
        url: &str,
        file_name: Option<&str>,
        bytes: u64,
    ) -> Result<(), ReaderSavedError> {
        Ok(self.cache.execute(|connection| {
            ReaderOfflineRepository::record_file(&*connection, post, url, file_name, bytes)
        })?)
    }

    fn set_images(
        &self,
        post: RowId,
        images: DbReaderOfflineImages,
    ) -> Result<(), ReaderSavedError> {
        self.cache.execute(|connection| {
            let transaction = connection.transaction()?;
            ReaderOfflineRepository::set_images(&transaction, post, images)?;
            transaction.commit().map_err(SqliteDbError::from)
        })?;
        self.notify();
        Ok(())
    }

    fn set_downloading_images(
        &self,
        post: RowId,
        downloading: bool,
    ) -> Result<(), ReaderSavedError> {
        self.cache.execute(|connection| {
            let transaction = connection.transaction()?;
            ReaderOfflineRepository::set_downloading_for_post(&transaction, post, downloading)?;
            transaction.commit().map_err(SqliteDbError::from)
        })?;
        self.notify();
        Ok(())
    }

    /// Removes a post's stored images because there's no room for them.
    fn remove_images(&self, store: &ImageStore, post: RowId) -> Result<(), ReaderSavedError> {
        let names = self.cache.execute(|connection| {
            let transaction = connection.transaction()?;
            let names = ReaderOfflineRepository::delete_files_of_post(
                &transaction,
                post,
                DbReaderOfflineImages::NoRoom,
            )?;
            transaction.commit().map_err(SqliteDbError::from)?;
            Ok::<_, SqliteDbError>(names)
        })?;
        delete_files(&store.directory, &names)?;
        self.notify();
        Ok(())
    }

    fn offline_report(
        &self,
        stopped_because: Option<String>,
    ) -> Result<ReaderOfflineReport, ReaderSavedError> {
        Ok(self.cache.execute(|connection| {
            let (ready, with_images) =
                ReaderOfflineRepository::count_ready(&*connection, self.account)?;
            let without =
                ReaderOfflineRepository::select_without_content(&*connection, self.account)?;
            let failed = without
                .iter()
                .filter(|save| save.offline.error.is_some())
                .count() as u64;
            Ok::<_, SqliteDbError>(ReaderOfflineReport {
                ready,
                with_images,
                failed,
                not_downloaded: without.len() as u64 - failed,
                image_bytes: ReaderOfflineRepository::stored_bytes(&*connection)?,
                stopped_because,
            })
        })?)
    }
}

/// Whether it's time to try a save's content again.
fn is_due(offline: &DbReaderSavedOffline, now: &WpGmtDateTime) -> bool {
    offline.attempts < MAX_ATTEMPTS
        && offline.attempted_at.as_ref().is_none_or(|attempted| {
            now.0
                .signed_duration_since(attempted.0)
                .to_std()
                .is_ok_and(|since| since >= RETRY_AFTER)
        })
}

/// Whether an older saved post has stored images that can go to make room.
fn can_make_room(older: &DbReaderPostForImages, removed: &HashSet<RowId>) -> bool {
    older.images == DbReaderOfflineImages::Stored && !removed.contains(&older.post)
}

/// A failure to reach the server stops a download, with its reason. Any other is the download's
/// own failure.
fn stopped_or_failed(error: ReaderSavedError) -> Result<Option<String>, ReaderSavedError> {
    match error {
        ReaderSavedError::Api(error) => Ok(Some(error.to_string())),
        other => Err(other),
    }
}

/// The addresses of a post's images to store: the ones in its content, and the one its card
/// shows. Each once.
fn image_urls(post: &DbReaderPostForImages) -> Vec<String> {
    let mut urls = post
        .content
        .as_deref()
        .map(reader_post_content_image_urls)
        .unwrap_or_default();
    for url in [&post.featured_image, &post.display_image_url]
        .into_iter()
        .flatten()
    {
        let url = url.trim();
        let lowercase = url.to_ascii_lowercase();
        if (lowercase.starts_with("https://") || lowercase.starts_with("http://"))
            && !urls.iter().any(|known| known == url)
        {
            urls.push(url.to_string());
        }
    }
    urls
}

/// Whether a file in the directory is one this library stored: `rdr-`, 32 hexadecimal digits, and
/// an extension.
fn is_stored_file_name(name: &str) -> bool {
    name.strip_prefix(FILE_PREFIX)
        .and_then(|rest| rest.split_once('.'))
        .is_some_and(|(id, extension)| {
            id.len() == 32
                && id.bytes().all(|byte| byte.is_ascii_hexdigit())
                && !extension.is_empty()
                && extension.bytes().all(|byte| byte.is_ascii_alphanumeric())
        })
}

/// Deletes stored files by name. One that's already gone is fine: it was to be deleted.
fn delete_files(directory: &Path, names: &[String]) -> Result<(), ReaderSavedError> {
    for name in names {
        // A name comes from the cache or from the directory's own listing, and is checked all
        // the same, so that nothing outside the directory can be named.
        if !is_stored_file_name(name) {
            continue;
        }
        let path = directory.join(name);
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(files_error("delete", &path, error)),
        }
    }
    Ok(())
}

fn files_error(action: &str, path: &Path, error: std::io::Error) -> ReaderSavedError {
    ReaderSavedError::OfflineFiles {
        reason: format!("Couldn't {action} {}: {error}", path.display()),
    }
}

#[cfg(test)]
#[path = "offline_tests.rs"]
mod tests;

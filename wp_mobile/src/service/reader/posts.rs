//! One post at a time: fetched by its IDs, by its slug or by its address, and kept in the cache.

use super::{ReaderContext, ReaderError, ReaderService};
use wp_api::comments::CommentId;
use wp_api::posts::PostId;
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::reader_likes::{ReaderLikesParams, ReaderLikesResponse};
use wp_api::wp_com::reader_post::{ReaderPostBySlugParams, ReaderPostParams, ReaderPostSlug};
use wp_api::wp_com::reader_posts::ReaderPost;
use wp_api::wp_com::reader_saved::{ReaderFeedItemId, ReaderPostSaveIdentity};
use wp_api::wp_com::sites::WpComSiteIdentifier;
use wp_mobile_cache::SqliteDbError;
use wp_mobile_cache::db_types::reader::ReaderPostRecord;
use wp_mobile_cache::repository::reader::ReaderPostRepository;
use wp_mobile_cache::repository::wordpress_com_accounts::ReaderOwner;

/// What a single post is asked for with, as a stream's posts are: its site, and its feed.
const POST_META: &str = "site,feed";

impl ReaderContext {
    /// Fetches a post by its IDs: an RSS post from its feed, and any other from its site.
    pub(crate) async fn fetch_reader_post(
        &self,
        post: &ReaderPostSaveIdentity,
    ) -> Result<ReaderPost, ReaderError> {
        let params = ReaderPostParams {
            meta: Some(POST_META.to_string()),
            fields: None,
        };
        let posts = self.client.reader_post();
        match (post.is_external, post.feed_id, post.site_id) {
            (true, Some(feed_id), _) => {
                let item = post.feed_item_id.unwrap_or(ReaderFeedItemId(post.id));
                Ok(posts.feed_post(&feed_id, &item, &params).await?.data)
            }
            (_, _, Some(site_id)) if site_id.0 != 0 => Ok(posts
                .site_post(&site_id, &PostId(post.id as i64), &params)
                .await?
                .data),
            _ => Err(ReaderError::MissingIds),
        }
    }

    /// A post as the cache keeps it for this reader, or `None` if it has no IDs to keep it by.
    ///
    /// The API sends a signed-out reader `is_seen: false` for every post, though nobody is there
    /// for a post to have been seen by. It's kept as unknown, which is what says a post can't be
    /// marked as seen.
    pub(crate) fn record_of(&self, post: &ReaderPost) -> Option<ReaderPostRecord> {
        let mut record = ReaderPostRecord::from_reader_post(post)?;
        if self.owner == ReaderOwner::SignedOut {
            record.is_seen = None;
        }
        Some(record)
    }

    /// Stores a fetched post, and returns it as the cache now has it, with its content.
    pub(crate) fn store_reader_post(
        &self,
        post: &ReaderPost,
    ) -> Result<ReaderPostRecord, ReaderError> {
        let record = self.record_of(post).ok_or(ReaderError::MissingIds)?;
        Ok(self.cache.execute(|connection| {
            let transaction = connection.transaction()?;
            ReaderPostRepository::upsert(&transaction, self.owner_row, &record)?;
            // The post may be older than something the reader turned on or off since.
            super::actions::show_pending(&transaction, self.owner_row)?;
            let stored =
                ReaderPostRepository::select_by_key(&transaction, self.owner_row, &record.key)?
                    .map(|(_, post)| post)
                    .unwrap_or(record);
            transaction.commit()?;
            Ok::<_, SqliteDbError>(stored)
        })?)
    }
}

#[uniffi::export]
impl ReaderService {
    /// Fetches a post by its IDs, stores it in the cache, and returns it with its content.
    ///
    /// For a post to show from a link or a notification, and to bring a cached post up to date.
    /// A public site's or feed's post needs no account.
    pub async fn fetch_post(
        &self,
        post: ReaderPostSaveIdentity,
    ) -> Result<ReaderPostRecord, ReaderError> {
        let fetched = self.context.fetch_reader_post(&post).await?;
        self.context.store_reader_post(&fetched)
    }

    /// Fetches a site's post by its slug, stores it in the cache, and returns it with its content.
    /// `site` is the site's ID or its domain.
    ///
    /// The slug finds the post's IDs, and the post is then fetched as [`Self::fetch_post`] fetches
    /// it, because a post found by its slug comes without what the Reader shows of its site.
    pub async fn fetch_post_by_slug(
        &self,
        site: WpComSiteIdentifier,
        slug: String,
    ) -> Result<ReaderPostRecord, ReaderError> {
        let found = self
            .context
            .client
            .reader_post()
            .post_by_slug(
                &site,
                &ReaderPostSlug(slug),
                &ReaderPostBySlugParams::default(),
            )
            .await?
            .data;
        self.fetch_post(ReaderPostSaveIdentity {
            id: found.post_id.0 as u64,
            site_id: Some(found.site_id),
            feed_id: None,
            feed_item_id: None,
            is_external: false,
            url: found.url,
        })
        .await
    }

    /// The users who liked a post, newest like first, a page at a time. Pass the default `params`
    /// for the first page, and the previous page's `next_page_params` for the next. A public
    /// site's needs no account. Not cached.
    pub async fn post_likes(
        &self,
        site_id: WpComSiteId,
        post_id: PostId,
        params: ReaderLikesParams,
    ) -> Result<ReaderLikesResponse, ReaderError> {
        Ok(self
            .context
            .client
            .reader_likes()
            .post_likes(&site_id, &post_id, &params)
            .await?
            .data)
    }

    /// The users who liked a comment, newest like first, a page at a time, as
    /// [`Self::post_likes`]. Not cached.
    pub async fn comment_likes(
        &self,
        site_id: WpComSiteId,
        comment_id: CommentId,
        params: ReaderLikesParams,
    ) -> Result<ReaderLikesResponse, ReaderError> {
        Ok(self
            .context
            .client
            .reader_likes()
            .comment_likes(&site_id, &comment_id, &params)
            .await?
            .data)
    }
}

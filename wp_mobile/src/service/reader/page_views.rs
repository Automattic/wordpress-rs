//! Counting a view when a post is opened in the Reader.
//!
//! The app says a post was opened. Whether that counts as a view, and the request that counts it,
//! are decided here, so both apps count the same views.
//!
//! The rule:
//!
//! - Only a post of a WordPress.com or Jetpack site counts. An RSS post has no site stats to count
//!   it in. (The iOS app's rule. The Android app sends an RSS post's feed ID as a site ID, which
//!   counts a view for whichever site has that ID.)
//! - A view of a public site's post by a member of the site doesn't count, as it doesn't on the
//!   site itself. (The Android app's rule. The iOS app only leaves out the site's admins.)
//! - A view of a private site's post counts whoever reads it, and is marked as one by a reader
//!   allowed to see it, as the web Reader marks it.
//!
//! A view that can't be sent, such as offline, isn't kept to send later.
//!
//! A view is sent with the app's `User-Agent`, which the app passes each time: stats don't count
//! one sent with the library's own. See
//! [`reader_page_view_user_agent_counts`](wp_api::wp_com::reader_page_views::reader_page_view_user_agent_counts).

use super::{ReaderContext, ReaderError, ReaderService};
use std::time::Duration;
use wp_api::posts::PostId;
use wp_api::prelude::WpApiError;
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::reader_page_views::{ReaderPageView, reader_page_view_user_agent_counts};
use wp_api::wp_com::reader_saved::ReaderSavedKey;
use wp_api::wp_com::sites::SitesListParams;
use wp_mobile_cache::SqliteDbError;
use wp_mobile_cache::db_types::reader::{reader_date_from_text, reader_date_text};
use wp_mobile_cache::repository::reader::ReaderPostRepository;
use wp_mobile_cache::repository::reader_actions::{
    ReaderMemberSiteRepository, ReaderSyncTimeRepository,
};
use wp_mobile_cache::repository::wordpress_com_accounts::ReaderOwner;

/// The name the member sites' fetch time is kept under.
const MEMBER_SITES: &str = "member_sites";

/// How long the member sites are used before they're fetched again.
const MEMBER_SITES_LIFETIME: Duration = Duration::from_secs(24 * 60 * 60);

/// What became of an opened post's view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ReaderPageViewOutcome {
    /// The view was counted.
    Counted,
    /// Not sent: an RSS post has no site stats to count it in.
    NotCountedForRssPost,
    /// Not sent: the reader is a member of the post's site, and the site is public.
    NotCountedForSiteMember,
    /// Not sent: the cache doesn't have the post, or what it has lacks the site's ID or URL.
    NotCountedForMissingDetails,
    /// Not sent: the `User-Agent` passed is blank, or has `wordpress` in it as the library's own
    /// has, and stats drop a view sent with one like that.
    NotCountedForUserAgent,
}

#[uniffi::export]
impl ReaderService {
    /// Reports that a post was opened, and counts a view of it if it's one that counts. `key` is
    /// the post's key in the cache. Call it once each time the reader opens the post.
    ///
    /// `user_agent` is the `User-Agent` the view is sent with: the one the app's web view sends,
    /// which the WordPress apps follow with their own name, as in
    /// `… Mobile/15E148 wp-iphone/26.4`. Not the library's own, which stats don't count a view
    /// with: that's [`ReaderPageViewOutcome::NotCountedForUserAgent`], whatever the post.
    ///
    /// Works signed out. A view that fails to send returns the error, and isn't sent again later.
    pub async fn post_opened(
        &self,
        key: ReaderSavedKey,
        user_agent: String,
    ) -> Result<ReaderPageViewOutcome, ReaderError> {
        if !reader_page_view_user_agent_counts(&user_agent) {
            return Ok(ReaderPageViewOutcome::NotCountedForUserAgent);
        }
        let context = &self.context;
        let post = context
            .cache
            .execute(|connection| {
                ReaderPostRepository::select_by_key(connection, context.owner_row, &key)
            })?
            .map(|(_, post)| post);
        let Some(post) = post else {
            return Ok(ReaderPageViewOutcome::NotCountedForMissingDetails);
        };
        if post.identity.is_external {
            return Ok(ReaderPageViewOutcome::NotCountedForRssPost);
        }
        let (Some(site_id), Some(site_url)) = (
            post.identity.site_id.filter(|site| site.0 != 0),
            post.site_url.clone(),
        ) else {
            return Ok(ReaderPageViewOutcome::NotCountedForMissingDetails);
        };
        let site_is_private = post.site_is_private == Some(true);
        if !site_is_private && context.is_member_of(site_id).await? {
            return Ok(ReaderPageViewOutcome::NotCountedForSiteMember);
        }
        let view = ReaderPageView {
            site_id,
            post_id: PostId(post.identity.id as i64),
            site_url,
            site_is_private,
            user_agent,
        };
        let Some(request) = view.request() else {
            return Ok(ReaderPageViewOutcome::NotCountedForMissingDetails);
        };
        let response = context
            .request_executor
            .execute(request.into())
            .await
            .map_err(WpApiError::from)?;
        if !(200..300).contains(&response.status_code) {
            return Err(ReaderError::Api(WpApiError::InvalidHttpStatusCode {
                status_code: response.status_code,
                request_url: response.request_url.0,
                request_method: response.request_method,
            }));
        }
        Ok(ReaderPageViewOutcome::Counted)
    }

    /// Tells the Reader which sites the account is a member of, for an app that already has
    /// them. Without it, the Reader fetches them itself the first time a post is opened, and
    /// again once they're a day old.
    pub fn set_member_sites(&self, site_ids: Vec<WpComSiteId>) -> Result<(), ReaderError> {
        Ok(self.context.store_member_sites(&site_ids)?)
    }

    /// Fetches the sites the account is a member of. Needs an account.
    pub async fn refresh_member_sites(&self) -> Result<(), ReaderError> {
        if self.context.owner == ReaderOwner::SignedOut {
            return Err(ReaderError::NeedsAccount);
        }
        self.context.fetch_member_sites().await
    }
}

impl ReaderContext {
    fn store_member_sites(&self, site_ids: &[WpComSiteId]) -> Result<(), SqliteDbError> {
        let now = reader_date_text(&self.now());
        self.cache.execute(|connection| {
            let transaction = connection.transaction()?;
            ReaderMemberSiteRepository::replace(&transaction, self.owner_row, site_ids)?;
            ReaderSyncTimeRepository::set(&transaction, self.owner_row, MEMBER_SITES, &now)?;
            transaction.commit().map_err(SqliteDbError::from)
        })
    }

    async fn fetch_member_sites(&self) -> Result<(), ReaderError> {
        let sites = self
            .client
            .sites()
            .get(&SitesListParams::default())
            .await?
            .data
            .sites;
        let ids: Vec<WpComSiteId> = sites.iter().map(|site| site.id).collect();
        Ok(self.store_member_sites(&ids)?)
    }

    /// Whether the reader is a member of a site. Fetches the account's sites first if they've
    /// never been fetched, or were fetched over a day ago. If that fetch fails, an older list
    /// answers, and with no list at all the failure is returned.
    async fn is_member_of(&self, site_id: WpComSiteId) -> Result<bool, ReaderError> {
        if self.owner == ReaderOwner::SignedOut {
            return Ok(false);
        }
        let fetched_at = self
            .cache
            .execute(|connection| {
                ReaderSyncTimeRepository::select(connection, self.owner_row, MEMBER_SITES)
            })?
            .map(|text| reader_date_from_text(&text))
            .transpose()?;
        let stale = fetched_at.is_none_or(|at| {
            self.now().0 - at.0
                > chrono::Duration::from_std(MEMBER_SITES_LIFETIME).unwrap_or_default()
        });
        if stale && let Err(error) = self.fetch_member_sites().await {
            if fetched_at.is_none() {
                return Err(error);
            }
            log::warn!("Couldn't refresh the account's sites, so the older list is used: {error}");
        }
        Ok(self.cache.execute(|connection| {
            ReaderMemberSiteRepository::is_member(connection, self.owner_row, site_id)
        })?)
    }
}

//! What opening a post in the Reader does: it marks the post as seen, and may count a view of it.
//!
//! The app says a post was opened. What follows is decided here, so both apps do the same.
//!
//! A post is marked as seen when the reader has an account, the post has a seen state and it isn't
//! seen yet. The mark goes through the same path as [`ReaderService::set_post_seen`]: it shows in
//! every stream that has the post straight away, and one made offline is sent later. (Both
//! WordPress apps mark an opened post this way, and the Android one only when online.)
//!
//! Whether opening counts as a view, the rule:
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
//! Liking a post counts a view of it too, under the same rule, as both WordPress apps count one:
//! iOS in `ReaderLikeAction.execute` and Android in `PostLikeUseCase.trackEvent`, each "to solve a
//! long-standing question from folks who ask 'why do I have more likes than page views?'". From
//! what the apps do:
//!
//! - Only a like counts one, not an unlike, from a stream or the post's screen alike.
//! - A like of a post the cache shows liked already counts none, as iOS checks (`!post.isLiked`).
//!   Android counts it before it checks whether the like changes anything; iOS's rule is the one
//!   that matches "a like counts a view".
//! - Neither app skips a view because one was counted lately, such as when the post was opened,
//!   and neither does the library.
//! - The view is sent when the reader likes the post, alongside the like. A like made offline
//!   waits and is sent by [`ReaderService::sync_actions`], but its view isn't: it fails at the tap
//!   and is dropped, like any view that can't be sent. iOS sends its view at the tap whether or not
//!   the like gets through, and Android sends neither offline: neither ever counts one later.
//!
//! A view is sent with the app's `User-Agent`, which the app passes each time: stats don't count
//! one sent with the library's own. See
//! [`reader_page_view_user_agent_counts`](wp_api::wp_com::reader_page_views::reader_page_view_user_agent_counts).

use super::actions::{Action, ReaderActionOutcome};
use super::{ReaderContext, ReaderError, ReaderService};
use std::time::Duration;
use wp_api::prelude::WpApiError;
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::reader_page_views::{ReaderPageView, reader_page_view_user_agent_counts};
use wp_api::wp_com::reader_saved::ReaderSavedKey;
use wp_api::wp_com::sites::SitesListParams;
use wp_mobile_cache::SqliteDbError;
use wp_mobile_cache::db_types::reader::ReaderPostRecord;
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

/// Whether opening a post counts a view of it.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum ReaderPageViewChoice {
    /// Count a view, if it's one that counts, sent with `user_agent`: the one the app's web view
    /// sends, which the WordPress apps follow with their own name, as in
    /// `… Mobile/15E148 wp-iphone/26.4`. Not the library's own, which stats don't count a view
    /// with: that's [`ReaderPageViewOutcome::NotCountedForUserAgent`], whatever the post.
    Count { user_agent: String },
    /// Don't count one, such as when the reader turned counting off: nothing is sent for the view.
    /// A counted view on someone's post can't be taken back.
    DontCount,
}

/// What opening a post did. Each part is reported on its own, so one failing doesn't hide the
/// other.
#[derive(Debug, uniffi::Record)]
pub struct ReaderPostOpenedOutcome {
    pub seen: ReaderOpenedSeen,
    pub page_view: ReaderOpenedPageView,
}

/// What became of marking an opened post as seen.
#[allow(clippy::large_enum_variant)] // Boxing doesn't cross the FFI boundary.
#[derive(Debug, uniffi::Enum)]
pub enum ReaderOpenedSeen {
    /// Marked as seen, which shows straight away. `outcome` says whether the server has it yet,
    /// or it waits to be sent with [`ReaderService::sync_actions`].
    Marked { outcome: ReaderActionOutcome },
    /// It was seen already. Nothing was sent.
    AlreadySeen,
    /// The post has no seen state, so it isn't marked. A signed-out reader's posts never have one,
    /// and a post from a stream that doesn't send one, such as a tag's, has none. Nothing was sent.
    NoSeenState,
    /// The cache doesn't have the post. Nothing was sent.
    NotCached,
    /// The server refused the mark, so the post shows as not seen again, or the cache couldn't
    /// keep it.
    Failed { error: ReaderError },
}

/// What became of the view an opened post may count.
#[allow(clippy::large_enum_variant)] // Boxing doesn't cross the FFI boundary.
#[derive(Debug, uniffi::Enum)]
pub enum ReaderOpenedPageView {
    /// It was decided as `outcome` says: counted, or not counted and why.
    Reported { outcome: ReaderPageViewOutcome },
    /// Not counted, as asked with [`ReaderPageViewChoice::DontCount`]. Nothing was sent.
    NotAsked,
    /// The view couldn't be sent, such as offline. It isn't sent again later.
    Failed { error: ReaderError },
}

/// What liking or unliking a post did. Each part is reported on its own, so one failing doesn't
/// hide the other.
#[derive(Debug, uniffi::Record)]
pub struct ReaderPostLikedOutcome {
    pub like: ReaderPostLikeResult,
    pub page_view: ReaderLikePageView,
}

/// What became of the like or unlike itself.
#[allow(clippy::large_enum_variant)] // Boxing doesn't cross the FFI boundary.
#[derive(Debug, uniffi::Enum)]
pub enum ReaderPostLikeResult {
    /// It shows, and `outcome` says whether the server has it yet, or it waits to be sent with
    /// [`ReaderService::sync_actions`].
    Done { outcome: ReaderActionOutcome },
    /// It wasn't done: the server refused it, so the post shows as it was, or it can't be sent,
    /// such as a feed's post ([`ReaderError::FeedPost`]), or the cache couldn't keep it.
    Failed { error: ReaderError },
}

/// What became of the view a like may count.
#[allow(clippy::large_enum_variant)] // Boxing doesn't cross the FFI boundary.
#[derive(Debug, uniffi::Enum)]
pub enum ReaderLikePageView {
    /// It was decided as `outcome` says: counted, or not counted and why.
    Reported { outcome: ReaderPageViewOutcome },
    /// Not counted, as asked with [`ReaderPageViewChoice::DontCount`]. Nothing was sent.
    NotAsked,
    /// Not sent: an unlike counts no view.
    NotForUnlike,
    /// Not sent: the cache showed the post liked already, so the like is no new one.
    NotForAlreadyLiked,
    /// The view couldn't be sent, such as offline. It isn't sent again later, even when the like
    /// is.
    Failed { error: ReaderError },
}

#[uniffi::export]
impl ReaderService {
    /// Reports that a post was opened: marks it as seen, and counts a view of it when `page_view`
    /// asks and it's one that counts. `key` is the post's key in the cache. Call it once each time
    /// the reader opens the post.
    ///
    /// The mark is only for an account, and only for a post that has a seen state and isn't seen
    /// yet. It shows straight away in every stream that has the post, and one made offline is sent
    /// later. A view works signed out.
    ///
    /// The two are done together and reported apart: an error of one is in its own part of the
    /// outcome. The call itself only fails when the cache can't be read.
    pub async fn post_opened(
        &self,
        key: ReaderSavedKey,
        page_view: ReaderPageViewChoice,
    ) -> Result<ReaderPostOpenedOutcome, ReaderError> {
        let context = &self.context;
        let post = context
            .cache
            .execute(|connection| {
                ReaderPostRepository::select_by_key(connection, context.owner_row, &key)
            })?
            .map(|(_, post)| post);
        let (seen, page_view) =
            futures::join!(context.mark_opened_post_seen(post.as_ref()), async {
                match page_view {
                    ReaderPageViewChoice::DontCount => ReaderOpenedPageView::NotAsked,
                    ReaderPageViewChoice::Count { user_agent } => {
                        match context.count_view(post.as_ref(), user_agent).await {
                            Ok(outcome) => ReaderOpenedPageView::Reported { outcome },
                            Err(error) => ReaderOpenedPageView::Failed { error },
                        }
                    }
                }
            });
        Ok(ReaderPostOpenedOutcome { seen, page_view })
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
    async fn mark_opened_post_seen(&self, post: Option<&ReaderPostRecord>) -> ReaderOpenedSeen {
        let Some(post) = post else {
            return ReaderOpenedSeen::NotCached;
        };
        if self.owner == ReaderOwner::SignedOut {
            return ReaderOpenedSeen::NoSeenState;
        }
        match post.is_seen {
            None => ReaderOpenedSeen::NoSeenState,
            Some(true) => ReaderOpenedSeen::AlreadySeen,
            Some(false) => {
                let action = Action::Seen {
                    post: post.identity.clone(),
                };
                match self.set_action(action, true).await {
                    Ok(outcome) => ReaderOpenedSeen::Marked { outcome },
                    Err(error) => ReaderOpenedSeen::Failed { error },
                }
            }
        }
    }

    /// The view a like or an unlike of `post` counts, as the module's documentation says. `post`
    /// is the cache's, as it was before the like.
    pub(super) async fn like_page_view(
        &self,
        post: Option<&ReaderPostRecord>,
        liked: bool,
        choice: ReaderPageViewChoice,
    ) -> ReaderLikePageView {
        if !liked {
            return ReaderLikePageView::NotForUnlike;
        }
        let ReaderPageViewChoice::Count { user_agent } = choice else {
            return ReaderLikePageView::NotAsked;
        };
        if post.and_then(|post| post.i_like) == Some(true) {
            return ReaderLikePageView::NotForAlreadyLiked;
        }
        match self.count_view(post, user_agent).await {
            Ok(outcome) => ReaderLikePageView::Reported { outcome },
            Err(error) => ReaderLikePageView::Failed { error },
        }
    }

    /// Counts a view of a post, if it's one that counts.
    async fn count_view(
        &self,
        post: Option<&ReaderPostRecord>,
        user_agent: String,
    ) -> Result<ReaderPageViewOutcome, ReaderError> {
        if !reader_page_view_user_agent_counts(&user_agent) {
            return Ok(ReaderPageViewOutcome::NotCountedForUserAgent);
        }
        let Some(post) = post else {
            return Ok(ReaderPageViewOutcome::NotCountedForMissingDetails);
        };
        if post.identity.is_feed_item() {
            return Ok(ReaderPageViewOutcome::NotCountedForRssPost);
        }
        let (Some((site_id, post_id)), Some(site_url)) =
            (post.identity.site_post(), post.site_url.clone())
        else {
            return Ok(ReaderPageViewOutcome::NotCountedForMissingDetails);
        };
        let site_is_private = post.site_is_private == Some(true);
        if !site_is_private && self.is_member_of(site_id).await? {
            return Ok(ReaderPageViewOutcome::NotCountedForSiteMember);
        }
        let view = ReaderPageView {
            site_id,
            post_id,
            site_url,
            site_is_private,
            user_agent,
        };
        let Some(request) = view.request() else {
            return Ok(ReaderPageViewOutcome::NotCountedForMissingDetails);
        };
        let response = self
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

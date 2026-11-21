//! What the user turns on or off in the Reader: a post marked as seen, a like, a follow, a block.
//!
//! Each works the same way. A tap changes the cache straight away, so it shows everywhere the cache
//! is read, and adds a row to `reader_actions`: what the user wants, and what the cache showed
//! before. Then every waiting row is sent, oldest tap first, one at a time.
//!
//! - When the server agrees, the row goes.
//! - When the server refuses, the cache goes back to what it showed before, the row goes, and the
//!   tap's call returns the error.
//! - When no answer comes, such as offline, or the server rejects the account's token, the row
//!   stays and [`ReaderService::sync_actions`] sends it later. Turning something on or off twice
//!   is the same as once, so sending again is safe.
//!
//! A refresh that brings the server's older state for something still waiting doesn't undo the
//! tap: what's waiting is shown again after every page is stored.

use super::page_views::{ReaderPageViewChoice, ReaderPostLikeResult, ReaderPostLikedOutcome};
use super::{ReaderContext, ReaderError, ReaderService};
use futures::lock::Mutex as AsyncMutex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use wp_api::comments::CommentId;
use wp_api::prelude::{WpApiError, WpErrorCode};
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::me::WpComUserId;
use wp_api::wp_com::reader_blocks::{ReaderBlockedSite, ReaderBlockedSitesParams};
use wp_api::wp_com::reader_comments::{
    ReaderCommentsNotificationsParams, ReaderSubscribeToCommentsParams,
};
use wp_api::wp_com::reader_posts::ReaderTagSlug;
use wp_api::wp_com::reader_saved::{
    ReaderFeedId, ReaderFeedItemId, ReaderPostSaveIdentity, ReaderSavedSource,
    reader_saved_key_for_post,
};
use wp_api::wp_com::reader_seen::{
    ReaderSeenFeedItemsParams, ReaderSeenSitePostsParams, ReaderSeenSource,
};
use wp_api::wp_com::reader_sites::ReaderSiteParams;
use wp_api::wp_com::reader_subscriptions::{
    ReaderFollowUrlParams, ReaderPostEmailSubscriptionParams, ReaderSubscriptionEmailFrequency,
    ReaderUnfollowUrlParams,
};
use wp_api::wp_com::reader_tags::{
    ReaderFollowTagsParams, ReaderTag, ReaderTagId, ReaderUnreadableTag,
};
use wp_api::wp_com::sites::WpComSiteIdentifier;
use wp_mobile_cache::db_types::reader::reader_saved_key_text;
use wp_mobile_cache::repository::QueryExecutor;
use wp_mobile_cache::repository::reader::{ReaderPostFlag, ReaderPostRepository};
use wp_mobile_cache::repository::reader_actions::{
    DbReaderAction, DbReaderBlockedSite, ReaderActionRepository, ReaderBlockRepository,
};
use wp_mobile_cache::repository::reader_comments::ReaderCommentRepository;
use wp_mobile_cache::repository::reader_sites::{
    ReaderDeliverySetting, ReaderSiteRepository, ReaderSubscriptionRecord,
    ReaderSubscriptionRepository,
};
use wp_mobile_cache::repository::wordpress_com_accounts::ReaderOwner;
use wp_mobile_cache::{RowId, SqliteDbError};

/// How many blocked sites a page of the server's list is asked for.
const BLOCKED_SITES_PAGE_SIZE: u32 = 100;

/// What became of something the user turned on or off. It shows straight away either way.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum ReaderActionOutcome {
    /// The server has it.
    Synced,
    /// It's on this device, and goes to the server with a later
    /// [`ReaderService::sync_actions`]: the device is offline, or no answer arrived. `reason`
    /// says why.
    Pending { reason: String },
}

/// Something the user turns on or off, with what's needed to send it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub(crate) enum Action {
    /// A post is seen, or not.
    Seen { post: ReaderPostSaveIdentity },
    /// A site is blocked, or not.
    BlockSite { site_id: WpComSiteId },
    /// The user likes a post, or not.
    LikePost { post: ReaderPostSaveIdentity },
    /// The user follows a site or a feed, or not.
    Follow {
        site_id: Option<WpComSiteId>,
        feed_id: Option<ReaderFeedId>,
        /// The address to follow: the site's, or the feed's.
        url: Option<String>,
        name: Option<String>,
    },
    /// The user follows a tag, or not.
    FollowTag {
        slug: ReaderTagSlug,
        title: Option<String>,
    },
    /// The user likes a comment, or not.
    LikeComment {
        site_id: WpComSiteId,
        comment_id: CommentId,
    },
    /// A followed site's new posts, new comments or notifications are sent to the user, or not.
    Delivery {
        site_id: WpComSiteId,
        setting: DeliverySetting,
    },
    /// The user is subscribed to a post's comments, or not.
    SubscribeComments { post: ReaderPostSaveIdentity },
    /// The user gets notifications for the comments of a post they're subscribed to, or not.
    CommentNotifications { post: ReaderPostSaveIdentity },
}

/// A way a followed site's news reaches the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DeliverySetting {
    /// New posts are emailed.
    PostEmails,
    /// New comments are emailed.
    CommentEmails,
    /// New posts send a notification.
    Notifications,
}

impl DeliverySetting {
    fn column(self) -> ReaderDeliverySetting {
        match self {
            Self::PostEmails => ReaderDeliverySetting::EmailPosts,
            Self::CommentEmails => ReaderDeliverySetting::EmailComments,
            Self::Notifications => ReaderDeliverySetting::NotifyPosts,
        }
    }
}

/// What the server said when it took an action, where there's something to keep.
#[derive(Debug, Default)]
struct Confirmation {
    /// How many likes the post or comment now has.
    like_count: Option<u64>,
    /// The subscription a follow made.
    subscription: Option<ReaderSubscriptionRecord>,
    /// How often a site's new posts are emailed, once the emails are started. The server picks
    /// it when the request doesn't, and drops it when the emails stop.
    email_frequency: Option<ReaderSubscriptionEmailFrequency>,
    /// A post's comment subscription as the server answered a change to it.
    comment_subscription: Option<CommentSubscription>,
    /// Whether the tags the server answered a follow or unfollow with have the tag, when that
    /// isn't what was asked.
    tag_listed: Option<bool>,
    /// Why the server's answer, though a success, says it didn't do what was asked. The cache
    /// then shows what the server answered, and the tap is refused with this reason.
    contradicted: Option<String>,
}

impl Confirmation {
    /// Whether the server's answer shows `action` on, when the answer says.
    fn shows(&self, action: &Action) -> Option<bool> {
        self.comment_subscription
            .map(|answer| answer.shows(action))
            .or(self.tag_listed)
    }
}

/// A post's comment subscription, as the server answers a change to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CommentSubscription {
    /// Whether the user is subscribed to the post's comments.
    subscribed: bool,
    /// Whether new comments notify the user, when the answer says.
    notified: Option<bool>,
}

impl CommentSubscription {
    /// Whether it shows `action` on: the subscription for [`Action::SubscribeComments`], and its
    /// notifications for [`Action::CommentNotifications`], which are off when the answer doesn't
    /// say they're on.
    fn shows(self, action: &Action) -> bool {
        match action {
            Action::CommentNotifications { .. } => self.subscribed && self.notified == Some(true),
            _ => self.subscribed,
        }
    }

    /// Why this answer to turning `action` on or off says the server didn't, or `None` when it
    /// did.
    fn contradiction(self, action: &Action, on: bool) -> Option<String> {
        if self.shows(action) == on {
            return None;
        }
        Some(
            match (action, on) {
                (Action::SubscribeComments { .. }, true) => {
                    "The server didn't subscribe you to the post's comments."
                }
                (Action::SubscribeComments { .. }, false) => {
                    "The server kept your subscription to the post's comments. While a site's new \
                     comments are emailed to you, its posts' comments stay subscribed."
                }
                (_, true) if !self.subscribed => {
                    "The server has no subscription to the post's comments to notify you of. \
                     Subscribe to them first."
                }
                (_, true) => "The server didn't turn on notifications for the post's comments.",
                (_, false) => "The server didn't turn off notifications for the post's comments.",
            }
            .to_string(),
        )
    }
}

/// The app, as the Reader's requests name it.
fn source_name(source: ReaderSavedSource) -> &'static str {
    match source {
        ReaderSavedSource::Android => "reader-android",
        ReaderSavedSource::Ios => "reader-ios",
    }
}

/// Whether an error is the server saying the thing was already as asked, by one of `codes`. Such
/// an answer to turning something off means it's off.
fn says_already(error: &WpApiError, codes: &[&str]) -> bool {
    matches!(
        error,
        WpApiError::WpError { error_code: WpErrorCode::CustomError(code), .. }
            if codes.contains(&code.as_str())
    )
}

impl Action {
    /// The action's number in `reader_actions.kind`. It never changes for an action, since rows
    /// outlive a launch.
    fn kind(&self) -> i64 {
        match self {
            Self::Seen { .. } => 1,
            Self::BlockSite { .. } => 2,
            Self::LikePost { .. } => 3,
            Self::Follow { .. } => 4,
            Self::FollowTag { .. } => 5,
            Self::LikeComment { .. } => 6,
            Self::Delivery { setting, .. } => match setting {
                DeliverySetting::PostEmails => 7,
                DeliverySetting::CommentEmails => 8,
                DeliverySetting::Notifications => 9,
            },
            Self::SubscribeComments { .. } => 10,
            Self::CommentNotifications { .. } => 11,
        }
    }

    /// Why the action has nothing to send it with, when [`Self::target`] is `None`: a like or a
    /// comment subscription of a feed's post, or an action missing the IDs or the address it needs.
    fn unsendable(&self) -> ReaderError {
        match self {
            Self::LikePost { post }
            | Self::SubscribeComments { post }
            | Self::CommentNotifications { post }
                if post.is_feed_item() =>
            {
                ReaderError::FeedPost
            }
            _ => ReaderError::MissingIds,
        }
    }

    /// What the action is about, in `reader_actions.target`, or `None` when it's missing the IDs
    /// that say.
    fn target(&self) -> Option<String> {
        match self {
            Self::Seen { post } => reader_saved_key_for_post(post.clone())
                .map(|key| reader_saved_key_text(&key.normalized())),
            // Only a post of a WordPress.com or Jetpack site can be liked, or have its comments
            // followed, so a feed item has nothing to send them with.
            Self::LikePost { post }
            | Self::SubscribeComments { post }
            | Self::CommentNotifications { post } => post
                .site_post()
                .and(reader_saved_key_for_post(post.clone()))
                .map(|key| reader_saved_key_text(&key.normalized())),
            Self::BlockSite { site_id } | Self::Delivery { site_id, .. } => {
                Some(format!("site:{}", site_id.0))
            }
            Self::Follow {
                site_id,
                feed_id,
                url,
                ..
            } => match (site_id, feed_id, url) {
                (Some(site), _, _) => Some(format!("site:{}", site.0)),
                (None, Some(feed), _) => Some(format!("feed:{}", feed.0)),
                (None, None, Some(url)) if !url.is_empty() => Some(format!("url:{url}")),
                _ => None,
            },
            Self::FollowTag { slug, .. } => Some(format!("tag:{}", slug.0)),
            Self::LikeComment {
                site_id,
                comment_id,
            } => Some(format!("comment:{}:{}", site_id.0, comment_id.0)),
        }
    }

    /// The post the action shows on, if the cache has it.
    fn post_row(
        post: &ReaderPostSaveIdentity,
        executor: &impl QueryExecutor,
        owner: RowId,
    ) -> Result<Option<RowId>, SqliteDbError> {
        let Some(key) = reader_saved_key_for_post(post.clone()) else {
            return Ok(None);
        };
        ReaderPostRepository::select_row_id_by_key(executor, owner, &key.normalized())
    }

    /// Whether the cache shows it on, or `None` if the cache doesn't say.
    fn shown(
        &self,
        executor: &impl QueryExecutor,
        owner: RowId,
    ) -> Result<Option<bool>, SqliteDbError> {
        let post_flag =
            |post: &ReaderPostSaveIdentity, flag| match Self::post_row(post, executor, owner)? {
                Some(row) => ReaderPostRepository::select_flag(executor, row, flag),
                None => Ok(None),
            };
        match self {
            Self::Seen { post } => post_flag(post, ReaderPostFlag::IsSeen),
            Self::LikePost { post } => post_flag(post, ReaderPostFlag::ILike),
            Self::SubscribeComments { post } => {
                post_flag(post, ReaderPostFlag::IsSubscribedComments)
            }
            Self::CommentNotifications { post } => {
                post_flag(post, ReaderPostFlag::ReceivesCommentNotifications)
            }
            Self::BlockSite { site_id } => Ok(Some(ReaderBlockRepository::is_site_blocked(
                executor, owner, *site_id,
            )?)),
            Self::Follow {
                site_id, feed_id, ..
            } => {
                // Followed if the subscriptions list it, or what's cached of the site says so.
                let listed = ReaderSubscriptionRepository::is_following(
                    executor, owner, *site_id, *feed_id,
                )?;
                let site_says = ReaderSiteRepository::select(executor, owner, *site_id, *feed_id)?
                    .and_then(|site| site.is_following);
                Ok(Some(listed || site_says == Some(true)))
            }
            Self::FollowTag { slug, .. } => {
                Ok(Some(super::tags::is_tag_followed(executor, owner, slug)?))
            }
            Self::LikeComment {
                site_id,
                comment_id,
            } => Ok(
                ReaderCommentRepository::select_like(executor, owner, *site_id, *comment_id)?
                    .and_then(|like| like.i_like),
            ),
            Self::Delivery { site_id, setting } => Ok(
                ReaderSubscriptionRepository::select_for_site(executor, owner, *site_id)?.and_then(
                    |subscription| match setting {
                        DeliverySetting::PostEmails => subscription.email_posts,
                        DeliverySetting::CommentEmails => subscription.email_comments,
                        DeliverySetting::Notifications => subscription.notify_posts,
                    },
                ),
            ),
        }
    }

    /// Makes the cache show it on or off.
    fn show(
        &self,
        executor: &impl QueryExecutor,
        owner: RowId,
        on: bool,
    ) -> Result<(), SqliteDbError> {
        match self {
            Self::Seen { post } => {
                if let Some(row) = Self::post_row(post, executor, owner)? {
                    ReaderPostRepository::set_flag(executor, row, ReaderPostFlag::IsSeen, on)?;
                }
            }
            Self::LikePost { post } => {
                if let Some(row) = Self::post_row(post, executor, owner)? {
                    ReaderPostRepository::set_liked(executor, row, on)?;
                }
            }
            Self::SubscribeComments { post } => {
                if let Some(row) = Self::post_row(post, executor, owner)? {
                    ReaderPostRepository::set_flag(
                        executor,
                        row,
                        ReaderPostFlag::IsSubscribedComments,
                        on,
                    )?;
                    if !on {
                        // Unsubscribing ends the notifications too.
                        ReaderPostRepository::set_flag(
                            executor,
                            row,
                            ReaderPostFlag::ReceivesCommentNotifications,
                            false,
                        )?;
                    }
                }
            }
            Self::CommentNotifications { post } => {
                if let Some(row) = Self::post_row(post, executor, owner)? {
                    ReaderPostRepository::set_flag(
                        executor,
                        row,
                        ReaderPostFlag::ReceivesCommentNotifications,
                        on,
                    )?;
                }
            }
            Self::BlockSite { site_id } => {
                if on {
                    let (name, url) =
                        ReaderPostRepository::select_site_name_and_url(executor, owner, *site_id)?;
                    ReaderBlockRepository::block_site(
                        executor,
                        owner,
                        &DbReaderBlockedSite {
                            site_id: *site_id,
                            name,
                            url,
                        },
                    )?;
                } else {
                    ReaderBlockRepository::unblock_site(executor, owner, *site_id)?;
                }
            }
            Self::Follow {
                site_id,
                feed_id,
                url,
                name,
            } => {
                ReaderPostRepository::set_following(executor, owner, *site_id, *feed_id, on)?;
                ReaderSiteRepository::set_following(executor, owner, *site_id, *feed_id, on)?;
                let following = ReaderSubscriptionRepository::is_following(
                    executor, owner, *site_id, *feed_id,
                )?;
                if on && !following {
                    // A stand-in until the server answers with the subscription it made.
                    let site = ReaderSiteRepository::select(executor, owner, *site_id, *feed_id)?;
                    let site = site.as_ref();
                    ReaderSubscriptionRepository::add_first(
                        executor,
                        owner,
                        &ReaderSubscriptionRecord {
                            subscription_id: None,
                            site_id: *site_id,
                            feed_id: *feed_id,
                            url: url.clone().or(site.and_then(|site| site.url.clone())),
                            feed_url: site.and_then(|site| site.feed_url.clone()),
                            name: name.clone().or(site.and_then(|site| site.name.clone())),
                            icon_url: site.and_then(|site| site.icon_url.clone()),
                            date_subscribed: None,
                            unseen_count: None,
                            organization_id: site.and_then(|site| site.organization_id),
                            is_owner: None,
                            is_rss: Some(site_id.is_none()),
                            email_posts: None,
                            email_comments: None,
                            email_frequency: None,
                            notify_posts: None,
                        },
                    )?;
                } else if !on {
                    ReaderSubscriptionRepository::remove(
                        executor,
                        owner,
                        *site_id,
                        *feed_id,
                        url.as_deref(),
                    )?;
                }
            }
            Self::FollowTag { slug, title } => {
                super::tags::show_tag_followed(executor, owner, slug, title.clone(), on)?;
            }
            Self::LikeComment {
                site_id,
                comment_id,
            } => {
                if let Some(like) =
                    ReaderCommentRepository::select_like(executor, owner, *site_id, *comment_id)?
                {
                    // The count moves by one only when the like is a change.
                    let like_count = match (like.i_like == Some(on), on) {
                        (true, _) => like.like_count,
                        (false, true) => Some(like.like_count.unwrap_or(0) + 1),
                        (false, false) => Some(like.like_count.unwrap_or(0).saturating_sub(1)),
                    };
                    ReaderCommentRepository::set_like(
                        executor,
                        owner,
                        *site_id,
                        *comment_id,
                        on,
                        like_count,
                    )?;
                }
            }
            Self::Delivery { site_id, setting } => {
                ReaderSubscriptionRepository::set_delivery(
                    executor,
                    owner,
                    *site_id,
                    setting.column(),
                    on,
                )?;
            }
        }
        Ok(())
    }

    /// Moves the followed site's or feed's count of unseen posts when marking a post seen or unseen
    /// changes what the cache showed: `was` is what it showed, and `on` what it shows now. Any
    /// other action, or a mark that changes nothing, leaves the count.
    ///
    /// As Android does (`ReaderSeenStatusToggleUseCase`, which calls `ReaderBlogTable`'s
    /// `decrementUnseenCount` when a post is marked seen and `incrementUnseenCount` when it's
    /// marked unseen, only when the post's seen state changes). Android moves it once the server
    /// has taken the mark; here it moves with the tap, as the post's seen state does, and goes back
    /// with it if the server refuses. An RSS post moves its feed's count, as the subscriptions list
    /// counts a feed's unseen posts too; Android keys the count by the post's site only.
    ///
    /// It isn't done in [`Self::show`], which [`show_pending`] runs again after every fetch of a
    /// stream, and which would count a mark that waits to be sent each time.
    fn move_unseen_count(
        &self,
        executor: &impl QueryExecutor,
        owner: RowId,
        was: Option<bool>,
        on: bool,
    ) -> Result<(), SqliteDbError> {
        let Self::Seen { post } = self else {
            return Ok(());
        };
        if was != Some(!on) {
            return Ok(());
        }
        let (site_id, feed_id) = if post.is_feed_item() {
            (None, post.feed_id)
        } else {
            (post.site_id, None)
        };
        let change = if on { -1 } else { 1 };
        ReaderSubscriptionRepository::add_to_unseen_count(executor, owner, site_id, feed_id, change)
    }

    /// Keeps what the server said when it took the action.
    fn confirm(
        &self,
        executor: &impl QueryExecutor,
        owner: RowId,
        on: bool,
        confirmation: &Confirmation,
    ) -> Result<(), SqliteDbError> {
        match self {
            Self::LikePost { post } => {
                if let (Some(row), Some(like_count)) = (
                    Self::post_row(post, executor, owner)?,
                    confirmation.like_count,
                ) {
                    ReaderPostRepository::set_like_count(executor, row, like_count)?;
                }
            }
            Self::LikeComment {
                site_id,
                comment_id,
            } => {
                if confirmation.like_count.is_some() {
                    ReaderCommentRepository::set_like(
                        executor,
                        owner,
                        *site_id,
                        *comment_id,
                        on,
                        confirmation.like_count,
                    )?;
                }
            }
            // The post shows the subscription the server answered with, which is the server's
            // choice of notifications for a new one, and can differ from what was asked.
            Self::SubscribeComments { post } | Self::CommentNotifications { post } => {
                if let (Some(row), Some(answer)) = (
                    Self::post_row(post, executor, owner)?,
                    confirmation.comment_subscription,
                ) {
                    ReaderPostRepository::set_flag(
                        executor,
                        row,
                        ReaderPostFlag::IsSubscribedComments,
                        answer.subscribed,
                    )?;
                    let notified = match answer.notified {
                        Some(notified) => Some(answer.subscribed && notified),
                        None if !answer.subscribed => Some(false),
                        None => None,
                    };
                    if let Some(notified) = notified {
                        ReaderPostRepository::set_flag(
                            executor,
                            row,
                            ReaderPostFlag::ReceivesCommentNotifications,
                            notified,
                        )?;
                    }
                    // The answer covers both of the post's settings, so a change to the other one
                    // that's still waiting to be sent is shown again.
                    show_pending(executor, owner)?;
                }
            }
            // The tag shows as the server's answer lists it, when that isn't what was asked. A
            // newer tap that's still waiting to be sent is shown over it.
            Self::FollowTag { slug, title } => {
                if let Some(listed) = confirmation.tag_listed {
                    super::tags::show_tag_followed(executor, owner, slug, title.clone(), listed)?;
                    show_pending(executor, owner)?;
                }
            }
            Self::Follow { .. } => {
                if let (true, Some(subscription)) = (on, &confirmation.subscription) {
                    ReaderSubscriptionRepository::add_first(executor, owner, subscription)?;
                }
            }
            // Emails started again come at the frequency the server answers with, which a
            // subscriptions list fetched while they were off doesn't have.
            Self::Delivery {
                site_id,
                setting: DeliverySetting::PostEmails,
            } => {
                if let (true, Some(frequency)) = (on, &confirmation.email_frequency) {
                    ReaderSubscriptionRepository::set_email_frequency(
                        executor, owner, *site_id, frequency,
                    )?;
                }
            }
            // While a site is blocked the server says it isn't followed, but it keeps the
            // subscription, and the unblock brings the follow back. A fetch made during the block
            // stored the server's "not followed", so a site the subscriptions list is followed
            // again.
            Self::BlockSite { site_id } => {
                let site_id = Some(*site_id);
                if !on
                    && ReaderSubscriptionRepository::is_following(executor, owner, site_id, None)?
                {
                    ReaderPostRepository::set_following(executor, owner, site_id, None, true)?;
                    ReaderSiteRepository::set_following(executor, owner, site_id, None, true)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Asks the server to turn it on or off.
    async fn send(&self, context: &ReaderContext, on: bool) -> Result<Confirmation, ReaderError> {
        let mut confirmation = Confirmation::default();
        match self {
            Self::Seen { post } => {
                let source = match context.source {
                    ReaderSavedSource::Android => ReaderSeenSource::Android,
                    ReaderSavedSource::Ios => ReaderSeenSource::Ios,
                };
                let seen = context.client.reader_seen();
                // An RSS post is marked by its feed's IDs, and any other by its site's.
                let marked = if let (true, Some(feed_id)) = (post.is_feed_item(), post.feed_id) {
                    let params = ReaderSeenFeedItemsParams {
                        feed_id,
                        feed_item_ids: vec![post.feed_item_id.unwrap_or(ReaderFeedItemId(post.id))],
                        source,
                    };
                    if on {
                        seen.feed_items_seen(&params).await?.data.status
                    } else {
                        seen.feed_items_unseen(&params).await?.data.status
                    }
                } else {
                    let (blog_id, post_id) = post.site_post().ok_or(ReaderError::MissingIds)?;
                    let params = ReaderSeenSitePostsParams {
                        blog_id,
                        post_ids: vec![post_id],
                        source,
                    };
                    if on {
                        seen.site_posts_seen(&params).await?.data.status
                    } else {
                        seen.site_posts_unseen(&params).await?.data.status
                    }
                };
                // The API answers 200 with `status: false` when it didn't mark the post.
                if !marked {
                    return Err(ReaderError::Refused {
                        reason: format!(
                            "The server didn't mark the post as {}.",
                            if on { "seen" } else { "unseen" }
                        ),
                    });
                }
            }
            Self::BlockSite { site_id } => {
                let blocks = context.client.reader_blocks();
                // The answer says whether the block changed. Either way the site is now as asked.
                if on {
                    blocks.block_site(site_id).await?;
                } else {
                    blocks.unblock_site(site_id).await?;
                }
            }
            Self::LikePost { post } => {
                // Only a post of a WordPress.com or Jetpack site can be liked.
                let (site_id, post_id) = post.site_post().ok_or_else(|| self.unsendable())?;
                let likes = context.client.reader_likes();
                confirmation.like_count = if on {
                    likes.like_post(&site_id, &post_id).await?.data.like_count
                } else {
                    likes.unlike_post(&site_id, &post_id).await?.data.like_count
                };
            }
            Self::Follow {
                site_id,
                feed_id,
                url,
                ..
            } => {
                let subscriptions = context.client.reader_subscriptions();
                // The address comes with the action, or from what the cache has of the site.
                let url = match url.clone().filter(|url| !url.is_empty()) {
                    Some(url) => Some(url),
                    None => context.cache.execute(|connection| {
                        Ok::<_, SqliteDbError>(
                            ReaderSiteRepository::select(
                                &*connection,
                                context.owner_row,
                                *site_id,
                                *feed_id,
                            )?
                            .and_then(|site| site.url.or(site.feed_url)),
                        )
                    })?,
                };
                if on {
                    let params = ReaderFollowUrlParams {
                        url: url.ok_or(ReaderError::MissingIds)?,
                        source: Some(source_name(context.source).to_string()),
                    };
                    let answer = subscriptions.follow_url(&params).await?.data;
                    if !answer.subscribed {
                        // The API answers 200 for an address it can't follow.
                        return Err(ReaderError::Refused {
                            reason: answer
                                .info
                                .unwrap_or_else(|| "The address can't be followed.".to_string()),
                        });
                    }
                    confirmation.subscription = answer
                        .subscription
                        .as_ref()
                        .map(super::sites::subscription_record);
                } else {
                    // Unfollowing something that isn't followed is refused, and leaves it unfollowed.
                    let result = match site_id {
                        Some(site_id) => subscriptions.unfollow_site(site_id).await.map(|_| ()),
                        None => {
                            let params = ReaderUnfollowUrlParams {
                                sub_id: None,
                                url: Some(url.ok_or(ReaderError::MissingIds)?),
                                blog_id: None,
                                email_id: None,
                            };
                            subscriptions.unfollow_url(&params).await.map(|_| ())
                        }
                    };
                    match result {
                        Err(error) if !says_already(&error, &["are_not_subscribed"]) => {
                            return Err(error.into());
                        }
                        _ => {}
                    }
                }
            }
            // Each answers with every tag the user follows. The server can answer 200 to a change
            // it doesn't keep (see `ReaderService::set_tag_followed`), and its list then most
            // often says so: a follow's added tag isn't in it, or an unfollowed tag still is.
            Self::FollowTag { slug, .. } => {
                let tags = context.client.reader_tags();
                if on {
                    let answer = tags
                        .follow_tags(&ReaderFollowTagsParams {
                            tags: vec![slug.clone()],
                        })
                        .await?
                        .data;
                    // `added_tags` is empty when the tag was followed already, which says nothing
                    // about this follow.
                    if !answer.added_tags.is_empty()
                        && answer.added_tags.iter().all(|id| {
                            tag_listed(&answer.tags, &answer.unreadable_tags, *id, slug)
                                == Some(false)
                        })
                    {
                        confirmation.tag_listed = Some(false);
                        confirmation.contradicted = Some(
                            "The server answered that it followed the tag, but the tags it \
                             says you follow don't include it."
                                .to_string(),
                        );
                    }
                } else {
                    match tags.unfollow_tag(slug).await {
                        Ok(answer) => {
                            let answer = answer.data;
                            if let Some(id) = answer.removed_tag
                                && tag_listed(&answer.tags, &answer.unreadable_tags, id, slug)
                                    == Some(true)
                            {
                                confirmation.tag_listed = Some(true);
                                confirmation.contradicted = Some(
                                    "The server answered that it unfollowed the tag, but the \
                                     tags it says you follow still include it."
                                        .to_string(),
                                );
                            }
                        }
                        // Unfollowing a tag that isn't followed is refused, and leaves it
                        // unfollowed.
                        Err(error) if !says_already(&error, &["not_subscribed"]) => {
                            return Err(error.into());
                        }
                        Err(_) => {}
                    }
                }
            }
            Self::LikeComment {
                site_id,
                comment_id,
            } => {
                let likes = context.client.reader_likes();
                confirmation.like_count = if on {
                    likes
                        .like_comment(site_id, comment_id)
                        .await?
                        .data
                        .like_count
                } else {
                    likes
                        .unlike_comment(site_id, comment_id)
                        .await?
                        .data
                        .like_count
                };
            }
            Self::Delivery { site_id, setting } => {
                let subscriptions = context.client.reader_subscriptions();
                match (setting, on) {
                    (DeliverySetting::PostEmails, true) => {
                        let params = ReaderPostEmailSubscriptionParams::default();
                        confirmation.email_frequency = subscriptions
                            .subscribe_to_post_emails(site_id, &params)
                            .await?
                            .data
                            .subscription
                            .and_then(|subscription| subscription.delivery_frequency);
                    }
                    (DeliverySetting::PostEmails, false) => {
                        let params = ReaderPostEmailSubscriptionParams::default();
                        subscriptions
                            .unsubscribe_from_post_emails(site_id, &params)
                            .await?;
                    }
                    (DeliverySetting::CommentEmails, true) => {
                        subscriptions.subscribe_to_comment_emails(site_id).await?;
                    }
                    (DeliverySetting::CommentEmails, false) => {
                        subscriptions
                            .unsubscribe_from_comment_emails(site_id)
                            .await?;
                    }
                    (DeliverySetting::Notifications, true) => {
                        subscriptions.subscribe_to_notifications(site_id).await?;
                    }
                    (DeliverySetting::Notifications, false) => {
                        subscriptions
                            .unsubscribe_from_notifications(site_id)
                            .await?;
                    }
                }
            }
            Self::SubscribeComments { post } | Self::CommentNotifications { post } => {
                // Only a post of a WordPress.com or Jetpack site has comments to subscribe to.
                let (site_id, post_id) = post.site_post().ok_or_else(|| self.unsendable())?;
                let comments = context.client.reader_comments();
                // Each answers 200 with the subscription as the server now has it, which isn't
                // always what was asked: turning notifications on for a post the user isn't
                // subscribed to answers "not subscribed".
                let answer = match (self, on) {
                    (Self::SubscribeComments { .. }, true) => {
                        let params = ReaderSubscribeToCommentsParams {
                            receive_notifications: None,
                        };
                        let answer = comments.subscribe(&site_id, &post_id, &params).await?.data;
                        CommentSubscription {
                            subscribed: answer.i_subscribe,
                            notified: answer.receives_notifications,
                        }
                    }
                    (Self::SubscribeComments { .. }, false) => {
                        let answer = comments.unsubscribe(&site_id, &post_id).await?.data;
                        CommentSubscription {
                            subscribed: answer.i_subscribe,
                            notified: answer.receives_notifications,
                        }
                    }
                    _ => {
                        let params = ReaderCommentsNotificationsParams {
                            receive_notifications: on,
                        };
                        let answer = comments
                            .update_subscription(&site_id, &post_id, &params)
                            .await?
                            .data;
                        CommentSubscription {
                            subscribed: answer.i_subscribe,
                            notified: answer.receives_notifications,
                        }
                    }
                };
                confirmation.comment_subscription = Some(answer);
                confirmation.contradicted = answer.contradiction(self, on);
            }
        }
        Ok(confirmation)
    }
}

/// Makes the cache show everything that's waiting to be sent. Call it in the transaction that
/// stores what the server sent, which may be older than a tap.
pub(crate) fn show_pending(
    executor: &impl QueryExecutor,
    owner: RowId,
) -> Result<(), SqliteDbError> {
    for row in ReaderActionRepository::select_all(executor, owner)? {
        if let Ok(action) = serde_json::from_str::<Action>(&row.payload) {
            action.show(executor, owner, row.wanted)?;
        }
    }
    Ok(())
}

/// Whether the tags a follow or unfollow answered with have the tag `id`, `slug`. `None` when a
/// tag the answer couldn't read could be it.
fn tag_listed(
    listed: &[ReaderTag],
    unreadable: &[ReaderUnreadableTag],
    id: ReaderTagId,
    slug: &ReaderTagSlug,
) -> Option<bool> {
    if listed
        .iter()
        .any(|tag| tag.id == Some(id) || tag.slug == *slug)
        || unreadable.iter().any(|tag| tag.id == Some(id))
    {
        Some(true)
    } else if unreadable.iter().any(|tag| tag.id.is_none()) {
        None
    } else {
        Some(false)
    }
}

/// A target a tap waits on: its action's kind and target.
type Target = (i64, String);

/// The sending of what the user turned on or off, one request at a time.
#[derive(Default)]
pub(crate) struct ActionQueue {
    /// Held while rows are sent, so they go in order; while the service's other changes are sent
    /// (a site's email frequency, a comment), so no two of its changes overlap; and by the
    /// refreshes whose reads must not overlap one: see [`ReaderService::set_tag_followed`],
    /// [`ReaderService::refresh_blocked_sites`] and [`ReaderService::refresh_menu`].
    pub(super) lane: AsyncMutex<()>,
    /// The targets taps are waiting on, and how many taps wait on each.
    pub(super) waiting: Mutex<HashMap<Target, usize>>,
    /// Why the server refused a target a tap is waiting on, until the tap takes it. The call that
    /// sends a row isn't always its tap's: whichever call holds the lane sends every row.
    pub(super) refusals: Mutex<HashMap<Target, ReaderError>>,
    /// Blocks and unblocks the server took from this service, by site, until the server's list
    /// of blocked sites agrees: whether the site was blocked. See
    /// [`ReaderService::refresh_blocked_sites`].
    confirmed_blocks: Mutex<HashMap<WpComSiteId, bool>>,
}

/// Marks a target a tap waits on, while it waits, so a refusal is kept for it. Dropping it,
/// including when the call is cancelled, drops the refusal if no other tap waits.
struct Waiting<'a> {
    queue: &'a ActionQueue,
    target: Target,
}

impl<'a> Waiting<'a> {
    fn new(queue: &'a ActionQueue, target: Target) -> Self {
        *queue
            .waiting
            .lock()
            .expect("waiting lock poisoned")
            .entry(target.clone())
            .or_default() += 1;
        Self { queue, target }
    }
}

impl Drop for Waiting<'_> {
    fn drop(&mut self) {
        let mut waiting = self.queue.waiting.lock().expect("waiting lock poisoned");
        if let Some(count) = waiting.get_mut(&self.target) {
            *count -= 1;
            if *count == 0 {
                waiting.remove(&self.target);
                self.queue
                    .refusals
                    .lock()
                    .expect("refusals lock poisoned")
                    .remove(&self.target);
            }
        }
    }
}

impl ReaderContext {
    /// Turns something on or off: shows it, then sends it with everything else that's waiting.
    pub(crate) async fn set_action(
        &self,
        action: Action,
        on: bool,
    ) -> Result<ReaderActionOutcome, ReaderError> {
        if self.owner == ReaderOwner::SignedOut {
            return Err(ReaderError::NeedsAccount);
        }
        let target = self.cache.execute(|connection| {
            let transaction = connection.transaction().map_err(SqliteDbError::from)?;
            let target = self.record_action(&transaction, &action, on)?;
            transaction.commit().map_err(SqliteDbError::from)?;
            Ok::<_, ReaderError>(target)
        })?;
        let _waiting = self.wait_on(target.clone());

        let unsent = {
            let _lane = self.actions.lane.lock().await;
            self.send_pending_actions().await?
        };
        if let Some(error) = self.take_refusal(&target) {
            return Err(error);
        }
        let row = self.cache.execute(|connection| {
            ReaderActionRepository::select(connection, self.owner_row, target.0, &target.1)
        })?;
        Ok(match row {
            None => ReaderActionOutcome::Synced,
            Some(row) => ReaderActionOutcome::Pending {
                reason: row
                    .last_error
                    .or(unsent)
                    .unwrap_or_else(|| "It wasn't sent yet.".to_string()),
            },
        })
    }

    /// Records a tap in `executor`, a transaction: shows it, and stores its row to be sent.
    /// Returns its target. Fails for an action that can't be sent.
    fn record_action(
        &self,
        executor: &impl QueryExecutor,
        action: &Action,
        on: bool,
    ) -> Result<Target, ReaderError> {
        let target = (
            action.kind(),
            action.target().ok_or_else(|| action.unsendable())?,
        );
        let payload = serde_json::to_string(action)
            .map_err(|error| SqliteDbError::SqliteError(error.to_string()))?;
        let was = action.shown(executor, self.owner_row)?;
        action.show(executor, self.owner_row, on)?;
        action.move_unseen_count(executor, self.owner_row, was, on)?;
        ReaderActionRepository::tap(
            executor,
            self.owner_row,
            target.0,
            &target.1,
            on,
            was,
            &payload,
        )?;
        Ok(target)
    }

    /// Marks `target` as waited on, so a refusal of the tap just recorded for it is kept for
    /// [`Self::take_refusal`].
    fn wait_on(&self, target: Target) -> Waiting<'_> {
        // A refusal from before this tap was of an older one.
        self.actions
            .refusals
            .lock()
            .expect("refusals lock poisoned")
            .remove(&target);
        Waiting::new(&self.actions, target)
    }

    /// Takes why the server refused `target`, if it did while it was waited on.
    fn take_refusal(&self, target: &Target) -> Option<ReaderError> {
        self.actions
            .refusals
            .lock()
            .expect("refusals lock poisoned")
            .remove(target)
    }

    /// Sends every waiting row, oldest tap first. Stops at the first that gets no answer, since
    /// the next would fare no better. Returns why it stopped, if it did. Call it holding the lane.
    async fn send_pending_actions(&self) -> Result<Option<String>, SqliteDbError> {
        // Rows sent this round, as they were when sent. A row tapped since is sent again.
        let mut sent: HashSet<(RowId, i64)> = HashSet::new();
        loop {
            let rows = self.cache.execute(|connection| {
                ReaderActionRepository::select_all(connection, self.owner_row)
            })?;
            let Some(row) = rows
                .into_iter()
                .find(|row| !sent.contains(&(row.row_id, row.seq)))
            else {
                return Ok(None);
            };
            sent.insert((row.row_id, row.seq));
            let Ok(action) = serde_json::from_str::<Action>(&row.payload) else {
                // A row this version can't read can't be sent. It's dropped rather than kept
                // forever.
                log::error!(
                    "Dropping a Reader action that can't be read: {}",
                    row.payload
                );
                self.cache.execute(|connection| {
                    ReaderActionRepository::finish(connection, row.row_id, row.seq)
                })?;
                continue;
            };

            match action.send(self, row.wanted).await {
                Ok(confirmation) => self.action_answered(&row, &action, Ok(confirmation))?,
                // A rejected sign-in is no answer to the change: it's kept and sent again after
                // the reader signs in again.
                Err(error) if is_refused(&error) && !is_sign_in_rejected(&error) => {
                    self.action_answered(&row, &action, Err(error))?
                }
                Err(error) => {
                    let message = error.to_string();
                    self.cache.execute(|connection| {
                        ReaderActionRepository::record_attempt(connection, row.row_id, &message)
                    })?;
                    return Ok(Some(message));
                }
            }
        }
    }

    /// The sites a list of blocked sites from the server is stale about: a block or unblock the
    /// server took from this service that the list disagrees with, and that `read/sites/{id}`
    /// says still holds. What the cache has of them is kept. A change the list agrees with, or
    /// that the site says no longer holds, is forgotten. Call it holding the lane, so no change is
    /// sent meanwhile.
    async fn sites_the_list_is_stale_about(
        &self,
        listed: &[DbReaderBlockedSite],
    ) -> Result<Vec<WpComSiteId>, ReaderError> {
        let confirmed: Vec<(WpComSiteId, bool)> = self
            .actions
            .confirmed_blocks
            .lock()
            .expect("confirmed blocks lock poisoned")
            .iter()
            .map(|(site_id, blocked)| (*site_id, *blocked))
            .collect();
        let mut stale = Vec::new();
        for (site_id, blocked) in confirmed {
            let is_listed = listed.iter().any(|site| site.site_id == site_id);
            let still_holds = is_listed != blocked
                && match self
                    .client
                    .reader_sites()
                    // The whole site, as `fetch_site` asks for it. With `fields`, the server
                    // keeps an answer for each value of it, and a kept answer is what this check
                    // gets around.
                    .site(
                        &WpComSiteIdentifier::Id { value: site_id.0 },
                        &ReaderSiteParams::default(),
                    )
                    .await
                {
                    Ok(site) => site.data.is_blocked == Some(blocked),
                    Err(error) => {
                        let error = ReaderError::from(error);
                        // The site can't be asked about, such as a deleted one, so the list
                        // decides. A rejected sign-in says nothing about the site.
                        if !is_refused(&error) || is_sign_in_rejected(&error) {
                            return Err(error);
                        }
                        false
                    }
                };
            if still_holds {
                stale.push(site_id);
            } else {
                self.actions
                    .confirmed_blocks
                    .lock()
                    .expect("confirmed blocks lock poisoned")
                    .remove(&site_id);
            }
        }
        Ok(stale)
    }

    /// Records the server's answer to a row: it took it, or refused it.
    fn action_answered(
        &self,
        row: &DbReaderAction,
        action: &Action,
        answer: Result<Confirmation, ReaderError>,
    ) -> Result<(), SqliteDbError> {
        let target = (row.kind, row.target.clone());
        // A success whose answer says the server didn't do what was asked is a refusal, but the
        // cache takes what the server answered rather than going back to what it showed before.
        let answer = match answer {
            Ok(mut confirmation) => match confirmation.contradicted.take() {
                Some(reason) => Err((Some(confirmation), ReaderError::Refused { reason })),
                None => Ok(confirmation),
            },
            Err(error) => Err((None, error)),
        };
        let finished = self.cache.execute(|connection| {
            let transaction = connection.transaction()?;
            let finished = ReaderActionRepository::finish(&transaction, row.row_id, row.seq)?;
            match (&answer, finished) {
                // Taken, with no tap since: the row is done, and what the server said is kept.
                (Ok(confirmation), true) => {
                    action.confirm(&transaction, self.owner_row, row.wanted, confirmation)?;
                }
                // Taken, and the user tapped again since: the server now has what was sent,
                // which is what the newer tap goes back to if it's refused. A comment
                // subscription's answer is kept too, and the newer tap shown over it.
                (Ok(confirmation), false) => {
                    ReaderActionRepository::set_was(&transaction, row.row_id, row.wanted)?;
                    if confirmation.comment_subscription.is_some() {
                        action.confirm(&transaction, self.owner_row, row.wanted, confirmation)?;
                    }
                }
                // Answered with something other than what was asked, with no tap since: the
                // cache shows what the server answered.
                (Err((Some(confirmation), _)), true) => {
                    action.confirm(&transaction, self.owner_row, row.wanted, confirmation)?;
                }
                // Answered with something other than what was asked, and the user tapped again
                // since: what the server answered is kept, with the newer tap shown over it, and
                // is what the newer tap goes back to.
                (Err((Some(confirmation), _)), false) => {
                    if let Some(answered) = confirmation.shows(action) {
                        ReaderActionRepository::set_was(&transaction, row.row_id, answered)?;
                    }
                    action.confirm(&transaction, self.owner_row, row.wanted, confirmation)?;
                }
                // Refused, with no tap since: the cache goes back to what it showed before.
                (Err((None, _)), true) => {
                    if let Some(was) = row.was {
                        let shown = action.shown(&transaction, self.owner_row)?;
                        action.show(&transaction, self.owner_row, was)?;
                        action.move_unseen_count(&transaction, self.owner_row, shown, was)?;
                    }
                }
                // Refused, with a tap since, which is newer than what was refused and is sent
                // next.
                (Err((None, _)), false) => {}
            }
            transaction.commit()?;
            Ok::<_, SqliteDbError>(finished)
        })?;
        let answer = answer.map_err(|(_, error)| error);
        if let (Ok(_), Action::BlockSite { site_id }) = (&answer, action) {
            self.actions
                .confirmed_blocks
                .lock()
                .expect("confirmed blocks lock poisoned")
                .insert(*site_id, row.wanted);
        }
        if let (Err(error), true) = (answer, finished) {
            let is_waited_on = self
                .actions
                .waiting
                .lock()
                .expect("waiting lock poisoned")
                .contains_key(&target);
            if is_waited_on {
                self.actions
                    .refusals
                    .lock()
                    .expect("refusals lock poisoned")
                    .insert(target, error);
            } else {
                log::warn!("The server refused a Reader action sent later: {error}");
            }
        }
        Ok(())
    }
}

/// Whether the server refused a request for good, rather than it failing on the way or for now.
/// A request timeout and a rate limit pass, so their rows are sent again later. Any other answer
/// in the 400s won't change by asking again, and nor will an action that's missing what it needs
/// to be sent. A rejected sign-in is in the 400s too: a caller that keeps the reader's change
/// through one checks [`is_sign_in_rejected`] first.
pub(super) fn is_refused(error: &ReaderError) -> bool {
    /// Request timeout, too many requests.
    const TEMPORARY: [u32; 2] = [408, 429];
    match error {
        ReaderError::Api(
            WpApiError::WpError { status_code, .. }
            | WpApiError::InvalidHttpStatusCode { status_code, .. }
            | WpApiError::UnknownError { status_code, .. },
        ) => (400..500).contains(status_code) && !TEMPORARY.contains(status_code),
        ReaderError::Refused { .. } | ReaderError::MissingIds | ReaderError::FeedPost => true,
        _ => false,
    }
}

/// Whether the server rejected the account's token rather than the request: see
/// [`crate::service::is_sign_in_rejected`].
pub(super) fn is_sign_in_rejected(error: &ReaderError) -> bool {
    matches!(error, ReaderError::Api(error) if crate::service::is_sign_in_rejected(error))
}

#[uniffi::export]
impl ReaderService {
    /// Marks a post as seen or unseen. It shows straight away. Needs an account.
    ///
    /// A post has a seen state when the stream that sent it gave it one, which the Following
    /// stream does: [`wp_mobile_cache::db_types::reader::ReaderPostRecord::is_seen`] is `None`
    /// for a post that has none, and marking such a post is refused by the server.
    ///
    /// When it changes the cached post's seen state, the followed site's (or an RSS post's feed's)
    /// [`ReaderSubscriptionRecord::unseen_count`] goes down by one for seen, never below zero, and
    /// up by one for unseen, as Android moves it, and goes back if the server refuses.
    pub async fn set_post_seen(
        &self,
        post: ReaderPostSaveIdentity,
        seen: bool,
    ) -> Result<ReaderActionOutcome, ReaderError> {
        self.context.set_action(Action::Seen { post }, seen).await
    }

    /// Blocks or unblocks a site. The Reader's streams leave out a blocked site's posts, straight
    /// away. Needs an account.
    ///
    /// A block doesn't unfollow the site. While it's blocked, the server describes the site and
    /// its posts as not followed, with no email or notification settings, and leaves the reader
    /// out of its subscriber count, but it keeps the subscription: an unblock brings the follow
    /// back with its delivery settings as they were. So a fetch made during the block stores "not
    /// followed", and when the server takes an unblock, a site the cached subscriptions list is
    /// shown as followed again, on the site and on its posts.
    ///
    /// The server answers a block or unblock with `success: false` when the site was already as
    /// asked, which is taken as done. It doesn't read the list of blocked sites back: the cache
    /// already has the change. The server's list can go on listing a site after an unblock it
    /// took, when reads of the list were on their way meanwhile, until the next block or unblock;
    /// [`Self::refresh_blocked_sites`] says how it guards against that. Unblocking the site
    /// again, a no-op, clears it.
    pub async fn set_site_blocked(
        &self,
        site_id: WpComSiteId,
        blocked: bool,
    ) -> Result<ReaderActionOutcome, ReaderError> {
        self.context
            .set_action(Action::BlockSite { site_id }, blocked)
            .await
    }

    /// The sites the user blocked, as the cache has them, by name.
    pub fn blocked_sites(&self) -> Result<Vec<ReaderBlockedSite>, ReaderError> {
        let sites = self.context.cache.execute(|connection| {
            ReaderBlockRepository::select_sites(connection, self.context.owner_row)
        })?;
        Ok(sites
            .into_iter()
            .map(|site| ReaderBlockedSite {
                id: site.site_id,
                name: site.name,
                url: site.url,
            })
            .collect())
    }

    /// Fetches the sites the user blocked, so a block made on another device shows on this one. A
    /// block or unblock made here that's still waiting to be sent stays as the user left it. Needs
    /// an account.
    ///
    /// The server's list can be wrong about a site just unblocked, and stay wrong. As measured on
    /// 2026-11-20:
    ///
    /// - The server keeps its answer for each page and page size. Page 1 and 100 a page are the
    ///   defaults, so `page=1&per_page=100`, `per_page=100` and no parameters share one answer;
    ///   another page size has its own. A cache-busting parameter doesn't change it.
    /// - When reads of the list are on their way while the server takes an unblock, the server
    ///   can go on listing the site for that page size: 3 times in 3 tries, for at least 10
    ///   minutes (14 in an earlier run), until the next block or unblock, which a second, no-op
    ///   unblock of the site is. `read/sites/{id}`'s `is_blocked` was right throughout.
    /// - A read that starts after the unblock is answered was right (6 tries), as were reads
    ///   during a block (no list left the site out, 3 tries) and reads of the Following stream
    ///   on their way during an unblock (3 tries). One live run did get a stale list with no other
    ///   read of the list found to overlap the unblock, so not every cause is known.
    ///
    /// So this doesn't read the list while a block or unblock of this service's is being sent,
    /// and doesn't send one while it reads. And when the list disagrees with a block or unblock
    /// the server took from this service, the site is asked about by `read/sites/{id}`, and its
    /// `is_blocked` decides: what the cache has is kept when it agrees with that change, and the
    /// list is taken when it doesn't (blocked or unblocked elsewhere since). A site that can't be
    /// asked about, such as a deleted one (410), is taken as the list has it. A site is asked
    /// about only until the list agrees with the change, and only by the service that made it: a
    /// new service, such as after a restart, takes the list as it is.
    ///
    /// The site is asked for whole, with no `fields`, as [`Self::fetch_site`] asks: the server
    /// keeps an answer for each value of `fields`, and a kept answer is what the check gets
    /// around. On 2026-11-20 that request answered `is_blocked: false` for a site that wasn't
    /// blocked, and 410 `site_gone` for the only site the account had blocked, a deleted one, so
    /// its `is_blocked: true` for a blocked site rests on the measurements above.
    pub async fn refresh_blocked_sites(&self) -> Result<(), ReaderError> {
        let context = &self.context;
        if context.owner == ReaderOwner::SignedOut {
            return Err(ReaderError::NeedsAccount);
        }
        // Held from the first read to the store, so no block or unblock of this service's is sent
        // while the list is being read. It also keeps a list read before an unblock from being
        // stored after the unblock is confirmed, which would block the site again here.
        let _lane = context.actions.lane.lock().await;
        let mut sites = Vec::new();
        let mut page = 1;
        loop {
            let params = ReaderBlockedSitesParams {
                page: Some(page),
                per_page: Some(BLOCKED_SITES_PAGE_SIZE),
            };
            let response = context
                .client
                .reader_blocks()
                .list_sites(&params)
                .await?
                .data;
            let count = response.sites.len();
            sites.extend(response.sites.into_iter().map(|site| DbReaderBlockedSite {
                site_id: site.id,
                name: site.name,
                url: site.url,
            }));
            if count < BLOCKED_SITES_PAGE_SIZE as usize {
                break;
            }
            page += 1;
        }
        let kept = context.sites_the_list_is_stale_about(&sites).await?;
        context.cache.execute(|connection| {
            let transaction = connection.transaction()?;
            let mut keep: Vec<WpComSiteId> =
                ReaderActionRepository::select_all(&transaction, context.owner_row)?
                    .iter()
                    .filter_map(|row| serde_json::from_str::<Action>(&row.payload).ok())
                    .filter_map(|action| match action {
                        Action::BlockSite { site_id } => Some(site_id),
                        _ => None,
                    })
                    .collect();
            keep.extend(kept);
            ReaderBlockRepository::replace_sites(&transaction, context.owner_row, &sites, &keep)?;
            transaction.commit().map_err(SqliteDbError::from)
        })?;
        Ok(())
    }

    /// Blocks or unblocks a user. The Reader's streams leave out a blocked user's posts, and a
    /// post's comments leave out their comments and the replies under them. Nothing is deleted, so
    /// an unblock shows them again.
    ///
    /// WordPress.com keeps no list of blocked users, so this one lives on this device only, and
    /// works signed out too.
    pub fn set_user_blocked(&self, user_id: WpComUserId, blocked: bool) -> Result<(), ReaderError> {
        self.context.cache.execute(|connection| {
            ReaderBlockRepository::set_user_blocked(
                connection,
                self.context.owner_row,
                user_id,
                blocked,
            )
        })?;
        Ok(())
    }

    /// The users the reader blocked on this device, in the order they were blocked.
    pub fn blocked_users(&self) -> Result<Vec<WpComUserId>, ReaderError> {
        Ok(self.context.cache.execute(|connection| {
            ReaderBlockRepository::select_users(connection, self.context.owner_row)
        })?)
    }

    /// Sends what the user turned on or off that the server doesn't have yet, such as a post
    /// marked as seen while offline. Returns how many are still waiting. Call it at launch, when
    /// the app comes to the foreground, and when the device comes back online.
    pub async fn sync_actions(&self) -> Result<u64, ReaderError> {
        let context = &self.context;
        if context.owner == ReaderOwner::SignedOut {
            return Ok(0);
        }
        {
            let _lane = context.actions.lane.lock().await;
            context.send_pending_actions().await?;
        }
        self.pending_action_count()
    }

    /// How many things the user turned on or off are waiting to be sent. Saves aren't among
    /// them: [`ReaderSavedService::pending_count`](crate::service::reader_saved::ReaderSavedService::pending_count)
    /// counts those.
    pub fn pending_action_count(&self) -> Result<u64, ReaderError> {
        Ok(self.context.cache.execute(|connection| {
            ReaderActionRepository::count(connection, self.context.owner_row)
        })?)
    }
}

/// A site or feed to follow or unfollow. Give every ID the caller has: a site's ID, its feed's ID,
/// or both.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ReaderFollowTarget {
    /// The site's ID, for a WordPress.com or Jetpack site.
    #[uniffi(default = None)]
    pub site_id: Option<WpComSiteId>,
    /// The feed's ID.
    #[uniffi(default = None)]
    pub feed_id: Option<ReaderFeedId>,
    /// The address to follow: the site's, or the feed's. Without it, the address the cache has
    /// for the site or feed is used, and a follow of one the cache doesn't know is refused.
    #[uniffi(default = None)]
    pub url: Option<String>,
    /// The site's or feed's name, to show in the list of subscriptions until the server answers.
    #[uniffi(default = None)]
    pub name: Option<String>,
}

/// What [`ReaderService::adopt_signed_out_tags`] did.
#[derive(Debug, Default, uniffi::Record)]
pub struct ReaderTagAdoptionReport {
    /// Tags this account now follows: the server has the follow, or it waits to be sent with
    /// [`ReaderService::sync_actions`].
    pub adopted: u64,
    /// Tags the server refused to follow. Each shows for this account as it did before, which is
    /// unfollowed unless the account followed it already, and isn't put back on the device.
    pub refused: Vec<ReaderRefusedTag>,
}

/// A tag the server refused to follow.
#[derive(Debug, uniffi::Record)]
pub struct ReaderRefusedTag {
    pub slug: ReaderTagSlug,
    /// The tag's name, when the device had one.
    pub title: Option<String>,
    /// Why the server refused it.
    pub error: ReaderError,
}

/// What becomes of something a signed-out reader turned on or off: it only happens on this
/// device.
fn only_on_this_device(on: bool) -> ReaderActionOutcome {
    let reason = if on {
        "It's kept on this device until you sign in to WordPress.com."
    } else {
        "It's removed from this device. Nothing was sent: you aren't signed in to WordPress.com."
    };
    ReaderActionOutcome::Pending {
        reason: reason.to_string(),
    }
}

#[uniffi::export]
impl ReaderService {
    /// Likes or unlikes a post, and counts a view of it for a like when `page_view` asks and it's
    /// one that counts. The post's like count moves with it, straight away, and is set to the
    /// server's once it answers. Only a post of a WordPress.com or Jetpack site can be liked: a
    /// feed's post is refused with [`ReaderError::FeedPost`].
    ///
    /// Pass the same `page_view` as to [`Self::post_opened`]. A like counts a view under the same
    /// rules as opening the post, as both WordPress apps count one: an unlike, or a like of a post
    /// the cache shows liked already, counts none, and a like made offline is sent later but its
    /// view isn't. See the `page_views` module.
    ///
    /// The like and the view are done together and reported apart: an error of one is in its own
    /// part of the outcome. The call fails with [`ReaderError::NeedsAccount`] signed out, sending
    /// nothing, and otherwise only when the cache can't be read.
    pub async fn set_post_liked(
        &self,
        post: ReaderPostSaveIdentity,
        liked: bool,
        page_view: ReaderPageViewChoice,
    ) -> Result<ReaderPostLikedOutcome, ReaderError> {
        let context = &self.context;
        if context.owner == ReaderOwner::SignedOut {
            return Err(ReaderError::NeedsAccount);
        }
        let cached = match reader_saved_key_for_post(post.clone()) {
            Some(key) => context
                .cache
                .execute(|connection| {
                    ReaderPostRepository::select_by_key(
                        connection,
                        context.owner_row,
                        &key.normalized(),
                    )
                })?
                .map(|(_, post)| post),
            None => None,
        };
        let (like, page_view) = futures::join!(
            context.set_action(Action::LikePost { post }, liked),
            context.like_page_view(cached.as_ref(), liked, page_view)
        );
        let like = match like {
            Ok(outcome) => ReaderPostLikeResult::Done { outcome },
            Err(error) => ReaderPostLikeResult::Failed { error },
        };
        Ok(ReaderPostLikedOutcome { like, page_view })
    }

    /// Follows or unfollows a site or a feed. Every cached post of it, what's cached of the site,
    /// and the list of subscriptions show it straight away. Needs an account.
    ///
    /// When it changes whether the cached site is followed, the site's subscriber count counts
    /// the reader in or out with it, and goes back with it if the server refuses.
    ///
    /// A follow is sent by the site's or feed's address, and an unfollow by the site's ID when
    /// there is one.
    pub async fn set_followed(
        &self,
        target: ReaderFollowTarget,
        followed: bool,
    ) -> Result<ReaderActionOutcome, ReaderError> {
        let action = Action::Follow {
            site_id: target.site_id.filter(|site| site.0 != 0),
            feed_id: target.feed_id.filter(|feed| feed.0 != 0),
            url: target.url,
            name: target.name,
        };
        self.context.set_action(action, followed).await
    }

    /// Follows or unfollows a tag. `title` is the tag's name, to show with it. It shows in
    /// [`Self::followed_tags`] straight away.
    ///
    /// Signed out, the tag is followed on this device only, and nothing is sent:
    /// [`Self::adopt_signed_out_tags`] takes such tags over after sign-in.
    ///
    /// The server can answer 200 to a follow or unfollow it doesn't keep. As measured on
    /// 2026-11-20 with one account:
    ///
    /// - Two of the account's tag changes sent at the same moment keep only one: 6 times in 6,
    ///   whether a follow and an unfollow or two follows, each answered 200 with the tag it
    ///   added or removed. The next change of the tags is kept.
    /// - Follows sent while the account's other changes were being sent answered 200 with the
    ///   tag in `added_tags`, and weren't kept: beside the live trial that likes a post alone, 2
    ///   of 2; beside the one that marks posts seen alone, 2 of 2; beside the seven trials that
    ///   make a change and put it back, 13 of 14; beside every Reader trial, 7 of 19. Every read
    ///   then said the tag wasn't followed, and unfollowing it was refused as `not_subscribed`.
    ///   None was lost beside the block trial alone (4 follows) or beside trials that only read
    ///   (4 follows).
    /// - Once, during such a run, an unfollow left the tag listed as followed by every read
    ///   (`read/tags` in `rest/v1.1` and `rest/v1.2`, `read/tags/{slug}/mine`, the menu) for 13
    ///   to 16 minutes. Seven more unfollows each answered 200 with the tag in `removed_tag`, and
    ///   a follow answered with nothing added, and none of them changed it; it ended on its own.
    ///   That wasn't seen again.
    /// - Reads of the tags or the menu on their way during a follow or unfollow, or made straight
    ///   after one, never left the tags wrong (21 tries).
    ///
    /// The answer lists every tag the user then follows. When that list doesn't have a tag the
    /// follow says it added, or still has the tag an unfollow says it removed, the change is
    /// refused with [`ReaderError::Refused`] and the tag shows as the list has it. A tag in the
    /// list that couldn't be read is taken as the added or removed one when its ID matches, and
    /// leaves the follow unchecked when its ID couldn't be read either. That caught 22 of the 24
    /// follows that weren't kept: the other two answers listed the tag and the server still
    /// dropped it. No read says otherwise once the server has dropped a change, so nothing else
    /// is checked.
    ///
    /// This service sends its own changes one at a time, in one lane: everything it turns on or
    /// off (tags, likes, seen, follows, blocks, a comment's like, comment subscriptions and a
    /// site's delivery settings), [`Self::set_post_email_frequency`], and the comments posted
    /// through [`Self::comments`]. So a tag change of this service's never overlaps another of
    /// its changes, and [`Self::refresh_menu`] doesn't read the menu while one is being sent.
    /// Saves aren't in that lane ([`Self::saved`] sends them in its own), nor are page views,
    /// which count a view rather than change the account; neither was measured beside a tag
    /// follow. Nothing here holds back another device or app signed in to the account.
    pub async fn set_tag_followed(
        &self,
        slug: ReaderTagSlug,
        title: Option<String>,
        followed: bool,
    ) -> Result<ReaderActionOutcome, ReaderError> {
        let context = &self.context;
        if context.owner == ReaderOwner::SignedOut {
            context.cache.execute(|connection| {
                super::tags::show_tag_followed(
                    &*connection,
                    context.owner_row,
                    &slug,
                    title,
                    followed,
                )
            })?;
            return Ok(only_on_this_device(followed));
        }
        context
            .set_action(Action::FollowTag { slug, title }, followed)
            .await
    }

    /// Follows, for this account, the tags that were followed on this device while nobody was
    /// signed in, and empties the signed-out reader's list. Call it after sign-in, on the
    /// account's service.
    ///
    /// Every tag is a follow of this account's before anything is sent, as if the reader had
    /// tapped it, so none is lost if the call stops part-way: what isn't sent waits for
    /// [`Self::sync_actions`]. A tag the server refuses goes back to how this account had it, and
    /// is listed in [`ReaderTagAdoptionReport::refused`]; the others are still sent. A tag the
    /// account already follows stays followed.
    pub async fn adopt_signed_out_tags(&self) -> Result<ReaderTagAdoptionReport, ReaderError> {
        use wp_api::wp_com::reader_tags::ReaderMenuSection;
        use wp_mobile_cache::repository::reader_sites::ReaderMenuRepository;
        use wp_mobile_cache::repository::wordpress_com_accounts::WordPressComAccountRepository;

        let context = &self.context;
        if context.owner == ReaderOwner::SignedOut {
            return Ok(ReaderTagAdoptionReport::default());
        }
        let tags = context.cache.execute(|connection| {
            let transaction = connection.transaction().map_err(SqliteDbError::from)?;
            let Some(signed_out) =
                WordPressComAccountRepository::select_owner(&transaction, ReaderOwner::SignedOut)?
            else {
                return Ok::<_, ReaderError>(Vec::new());
            };
            let mut tags: Vec<(Target, ReaderTagSlug, Option<String>)> = Vec::new();
            for tag in ReaderMenuRepository::select_section(
                &transaction,
                signed_out,
                ReaderMenuSection::Subscribed,
            )? {
                let slug = ReaderTagSlug(tag.slug.unwrap_or(tag.key));
                let action = Action::FollowTag {
                    slug: slug.clone(),
                    title: tag.title.clone(),
                };
                let target = context.record_action(&transaction, &action, true)?;
                if !tags.iter().any(|(listed, _, _)| *listed == target) {
                    tags.push((target, slug, tag.title));
                }
            }
            // They're the account's now.
            ReaderMenuRepository::replace_section(
                &transaction,
                signed_out,
                ReaderMenuSection::Subscribed,
                &[],
            )?;
            transaction.commit().map_err(SqliteDbError::from)?;
            Ok(tags)
        })?;
        let _waiting: Vec<_> = tags
            .iter()
            .map(|(target, _, _)| context.wait_on(target.clone()))
            .collect();

        {
            let _lane = context.actions.lane.lock().await;
            context.send_pending_actions().await?;
        }
        let mut report = ReaderTagAdoptionReport::default();
        for (target, slug, title) in tags {
            match context.take_refusal(&target) {
                Some(error) => report.refused.push(ReaderRefusedTag { slug, title, error }),
                // Followed, or waiting to be sent.
                None => report.adopted += 1,
            }
        }
        Ok(report)
    }

    /// Likes or unlikes a comment. Needs an account.
    pub async fn set_comment_liked(
        &self,
        site_id: WpComSiteId,
        comment_id: CommentId,
        liked: bool,
    ) -> Result<ReaderActionOutcome, ReaderError> {
        self.context
            .set_action(
                Action::LikeComment {
                    site_id,
                    comment_id,
                },
                liked,
            )
            .await
    }

    /// Subscribes the user to a post's comments, or unsubscribes them. Unsubscribing ends the
    /// post's comment notifications too. Needs an account, and a post of a WordPress.com or
    /// Jetpack site: a feed's post is refused with [`ReaderError::FeedPost`].
    ///
    /// The server answers with the subscription as it now has it, and the cached post shows that
    /// answer: a new subscription's notifications are off unless the server says otherwise. An
    /// answer that isn't what was asked is refused with [`ReaderError::Refused`], and the post
    /// shows what the server answered rather than going back.
    ///
    /// A post's subscription follows the site's comment emails ([`Self::set_comment_emails`]).
    /// As measured on 2026-11-20, on a site the account owns with one post: turning the site's
    /// comment emails on made the post read as subscribed with notifications off, and while they
    /// were on, an unsubscribe answered `success: true` and still subscribed (so it's refused
    /// here). Turning them off unsubscribed the post, including a subscription made before they
    /// were turned on. Turning them off when they were already off left the post's subscription
    /// alone. Posting a comment on the post didn't subscribe the user to it.
    pub async fn set_comments_subscribed(
        &self,
        post: ReaderPostSaveIdentity,
        subscribed: bool,
    ) -> Result<ReaderActionOutcome, ReaderError> {
        self.context
            .set_action(Action::SubscribeComments { post }, subscribed)
            .await
    }

    /// Turns notifications for the comments of a post the user is subscribed to on or off. Needs
    /// an account, and a post of a WordPress.com or Jetpack site: a feed's post is refused with
    /// [`ReaderError::FeedPost`].
    ///
    /// The server answers with the subscription as it now has it, and the cached post shows that
    /// answer. For a post the user isn't subscribed to, turning them on still answers 200, with
    /// "not subscribed": that's refused with [`ReaderError::Refused`], and the post shows as not
    /// subscribed. While the site's comment emails are on, turning them on answers with them
    /// off, and is refused the same way. See [`Self::set_comments_subscribed`].
    pub async fn set_comment_notifications(
        &self,
        post: ReaderPostSaveIdentity,
        enabled: bool,
    ) -> Result<ReaderActionOutcome, ReaderError> {
        self.context
            .set_action(Action::CommentNotifications { post }, enabled)
            .await
    }

    /// Turns emails for a followed site's new posts on or off. Needs an account.
    pub async fn set_post_emails(
        &self,
        site_id: WpComSiteId,
        enabled: bool,
    ) -> Result<ReaderActionOutcome, ReaderError> {
        self.set_delivery(site_id, DeliverySetting::PostEmails, enabled)
            .await
    }

    /// Turns emails for a followed site's new comments on or off. Needs an account.
    ///
    /// The server ties it to the site's posts' comment subscriptions: turning it on subscribes
    /// the user to them, with notifications off, and turning it off unsubscribes them, as
    /// measured on a site with one post (see [`Self::set_comments_subscribed`]). Posts already in
    /// the cache keep the subscription they had until they're fetched again.
    pub async fn set_comment_emails(
        &self,
        site_id: WpComSiteId,
        enabled: bool,
    ) -> Result<ReaderActionOutcome, ReaderError> {
        self.set_delivery(site_id, DeliverySetting::CommentEmails, enabled)
            .await
    }

    /// Turns notifications for a followed site's new posts on or off. Needs an account.
    pub async fn set_post_notifications(
        &self,
        site_id: WpComSiteId,
        enabled: bool,
    ) -> Result<ReaderActionOutcome, ReaderError> {
        self.set_delivery(site_id, DeliverySetting::Notifications, enabled)
            .await
    }

    /// Sets how often a followed site's new posts are emailed. Unlike the on/off settings, this
    /// one isn't kept to send later: it returns the error if it can't be sent now. Needs an
    /// account. It's sent once no other change of this service's is being sent.
    pub async fn set_post_email_frequency(
        &self,
        site_id: WpComSiteId,
        frequency: wp_api::wp_com::reader_subscriptions::ReaderSubscriptionEmailFrequency,
    ) -> Result<(), ReaderError> {
        use wp_api::wp_com::reader_subscriptions::ReaderPostEmailFrequencyParams;

        let context = &self.context;
        if context.owner == ReaderOwner::SignedOut {
            return Err(ReaderError::NeedsAccount);
        }
        let params = ReaderPostEmailFrequencyParams {
            delivery_frequency: frequency.clone(),
            track_source: None,
        };
        // Sent in the lane, so it doesn't overlap another change of this service's: see
        // `Self::set_tag_followed`.
        let _lane = context.actions.lane.lock().await;
        context
            .client
            .reader_subscriptions()
            .update_post_emails(&site_id, &params)
            .await?;
        context.cache.execute(|connection| {
            ReaderSubscriptionRepository::set_email_frequency(
                connection,
                context.owner_row,
                site_id,
                &frequency,
            )
        })?;
        Ok(())
    }
}

impl ReaderService {
    async fn set_delivery(
        &self,
        site_id: WpComSiteId,
        setting: DeliverySetting,
        enabled: bool,
    ) -> Result<ReaderActionOutcome, ReaderError> {
        self.context
            .set_action(Action::Delivery { site_id, setting }, enabled)
            .await
    }
}

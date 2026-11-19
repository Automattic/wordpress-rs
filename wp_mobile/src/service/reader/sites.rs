//! Sites and feeds: what the Reader shows of one besides its posts, the ones the account follows,
//! and the search that finds them.

use super::{ReaderContext, ReaderError, ReaderService};
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::reader_saved::ReaderFeedId;
use wp_api::wp_com::reader_sites::{
    ReaderFeed, ReaderFeedSearchParams, ReaderFeedSearchResponse, ReaderSite, ReaderSiteParams,
};
use wp_api::wp_com::reader_subscriptions::{ReaderSubscription, ReaderSubscriptionsParams};
use wp_api::wp_com::reader_users::{ReaderUserProfile, WpComUserIdentifier};
use wp_api::wp_com::sites::WpComSiteIdentifier;
use wp_mobile_cache::SqliteDbError;
use wp_mobile_cache::db_types::reader::reader_date_text;
use wp_mobile_cache::repository::reader_actions::ReaderSyncTimeRepository;
use wp_mobile_cache::repository::reader_sites::{
    ReaderSiteRecord, ReaderSiteRepository, ReaderSubscriptionRecord, ReaderSubscriptionRepository,
};
use wp_mobile_cache::repository::wordpress_com_accounts::ReaderOwner;

/// The name the subscriptions' fetch time is kept under.
const SUBSCRIPTIONS: &str = "subscriptions";

/// How many subscriptions a page of the account's list is asked for, which is the most the API
/// gives.
const SUBSCRIPTIONS_PAGE_SIZE: u32 = 100;

/// The most pages of subscriptions one refresh asks for, so a list that never ends can't keep it
/// going.
const MAX_SUBSCRIPTION_PAGES: u32 = 50;

fn site_record(site: &ReaderSite) -> ReaderSiteRecord {
    ReaderSiteRecord {
        site_id: Some(site.site_id),
        feed_id: site.feed_id,
        name: site.name.clone(),
        description: site.description.clone(),
        url: site.url.clone(),
        feed_url: site.feed_url.clone(),
        icon_url: site.icon_url.clone(),
        subscribers_count: site.subscribers_count,
        post_count: site.post_count,
        is_private: site.is_private,
        is_jetpack: site.is_jetpack,
        organization_id: site.organization_id,
        is_following: site.is_following,
    }
}

fn feed_record(feed: &ReaderFeed) -> ReaderSiteRecord {
    ReaderSiteRecord {
        site_id: feed.site_id,
        feed_id: Some(feed.feed_id),
        name: feed.name.clone(),
        description: feed.description.clone(),
        url: feed.url.clone(),
        feed_url: feed.feed_url.clone(),
        icon_url: feed.image_url.clone(),
        subscribers_count: feed.subscribers_count,
        post_count: None,
        is_private: None,
        is_jetpack: None,
        organization_id: feed.organization_id,
        is_following: feed.is_following,
    }
}

pub(crate) fn subscription_record(subscription: &ReaderSubscription) -> ReaderSubscriptionRecord {
    let email = subscription.delivery_methods.email.as_ref();
    ReaderSubscriptionRecord {
        subscription_id: subscription.id,
        site_id: subscription.blog_id,
        feed_id: subscription.feed_id,
        url: subscription.url.clone(),
        feed_url: subscription.feed_url.clone(),
        name: subscription.name.clone(),
        icon_url: subscription.site_icon.clone(),
        date_subscribed: subscription.date_subscribed,
        unseen_count: subscription.unseen_count,
        organization_id: subscription.organization_id,
        is_owner: subscription.is_owner,
        is_rss: subscription.is_rss,
        email_posts: email.and_then(|email| email.send_posts),
        email_comments: email.and_then(|email| email.send_comments),
        email_frequency: email.and_then(|email| email.post_delivery_frequency.clone()),
        notify_posts: subscription
            .delivery_methods
            .notification
            .as_ref()
            .and_then(|notification| notification.send_posts),
    }
}

impl ReaderContext {
    /// Stores what was fetched of a site or feed, and returns it as the cache now has it.
    fn store_site(&self, site: ReaderSiteRecord) -> Result<ReaderSiteRecord, SqliteDbError> {
        let now = self.now();
        self.cache.execute(|connection| {
            let transaction = connection.transaction()?;
            ReaderSiteRepository::upsert(&transaction, self.owner_row, &site, &now)?;
            // What was fetched may be older than a follow or unfollow the reader made since.
            super::actions::show_pending(&transaction, self.owner_row)?;
            let stored = ReaderSiteRepository::select(
                &transaction,
                self.owner_row,
                site.site_id,
                site.feed_id,
            )?
            .unwrap_or(site);
            transaction.commit()?;
            Ok(stored)
        })
    }
}

#[uniffi::export]
impl ReaderService {
    /// Fetches what the Reader shows of a site besides its posts, such as its name, its
    /// description and how many follow it, and keeps it in the cache. A public site needs no
    /// account.
    pub async fn fetch_site(&self, site_id: WpComSiteId) -> Result<ReaderSiteRecord, ReaderError> {
        let site = self
            .context
            .client
            .reader_sites()
            .site(
                &WpComSiteIdentifier::Id { value: site_id.0 },
                &ReaderSiteParams::default(),
            )
            .await?
            .data;
        Ok(self.context.store_site(site_record(&site))?)
    }

    /// Fetches what the Reader shows of a feed besides its posts, and keeps it in the cache.
    /// Needs no account.
    pub async fn fetch_feed(&self, feed_id: ReaderFeedId) -> Result<ReaderSiteRecord, ReaderError> {
        let feed = self
            .context
            .client
            .reader_sites()
            .feed(&feed_id)
            .await?
            .data;
        Ok(self.context.store_site(feed_record(&feed))?)
    }

    /// What the cache has of a site, or of a feed when there's no site ID to look by. Doesn't
    /// touch the network.
    pub fn cached_site(
        &self,
        site_id: Option<WpComSiteId>,
        feed_id: Option<ReaderFeedId>,
    ) -> Result<Option<ReaderSiteRecord>, ReaderError> {
        Ok(self.context.cache.execute(|connection| {
            ReaderSiteRepository::select(connection, self.context.owner_row, site_id, feed_id)
        })?)
    }

    /// Searches sites and feeds, best match first. `offset` is where the page starts: `None` for
    /// the first, and the previous page's `next_offset` after it. Needs no account. Not cached.
    ///
    /// A query that looks like an address finds the feed at that address.
    pub async fn search_sites(
        &self,
        query: String,
        offset: Option<u32>,
    ) -> Result<ReaderFeedSearchResponse, ReaderError> {
        let params = ReaderFeedSearchParams {
            q: Some(query),
            offset,
            // Each result's site and feed, which have what a row of results shows.
            meta: Some("site,feed".to_string()),
            ..Default::default()
        };
        Ok(self
            .context
            .client
            .reader_sites()
            .search_feeds(&params)
            .await?
            .data)
    }

    /// The sites and feeds the account follows, from the cache, newest follow first.
    pub fn subscriptions(&self) -> Result<Vec<ReaderSubscriptionRecord>, ReaderError> {
        Ok(self.context.cache.execute(|connection| {
            ReaderSubscriptionRepository::select_all(connection, self.context.owner_row)
        })?)
    }

    /// Fetches every site and feed the account follows, a page at a time, and replaces the cached
    /// list with them. A follow or unfollow made here that's still waiting to be sent stays as
    /// the reader left it. Returns how many there are. Needs an account.
    pub async fn refresh_subscriptions(&self) -> Result<u64, ReaderError> {
        let context = &self.context;
        if context.owner == ReaderOwner::SignedOut {
            return Err(ReaderError::NeedsAccount);
        }
        let mut subscriptions = Vec::new();
        for page in 1..=MAX_SUBSCRIPTION_PAGES {
            let params = ReaderSubscriptionsParams {
                page: Some(page),
                number: Some(SUBSCRIPTIONS_PAGE_SIZE),
                ..Default::default()
            };
            let response = context
                .client
                .reader_subscriptions()
                .list(&params)
                .await?
                .data;
            subscriptions.extend(response.subscriptions.iter().map(subscription_record));
            if !response.has_next_page(SUBSCRIPTIONS_PAGE_SIZE) {
                break;
            }
        }
        let now = reader_date_text(&context.now());
        context.cache.execute(|connection| {
            let transaction = connection.transaction()?;
            ReaderSubscriptionRepository::replace_all(
                &transaction,
                context.owner_row,
                &subscriptions,
            )?;
            super::actions::show_pending(&transaction, context.owner_row)?;
            ReaderSyncTimeRepository::set(&transaction, context.owner_row, SUBSCRIPTIONS, &now)?;
            transaction.commit().map_err(SqliteDbError::from)
        })?;
        Ok(subscriptions.len() as u64)
    }

    /// A user's public profile, by their login or their user ID. Prefer the login when it's known:
    /// the API reads a number as a login first. Needs an account. Not cached.
    pub async fn user_profile(
        &self,
        user: WpComUserIdentifier,
    ) -> Result<ReaderUserProfile, ReaderError> {
        if self.context.owner == ReaderOwner::SignedOut {
            return Err(ReaderError::NeedsAccount);
        }
        Ok(self
            .context
            .client
            .reader_users()
            .profile(&user)
            .await?
            .data)
    }
}

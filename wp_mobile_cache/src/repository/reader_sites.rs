//! The Reader menu with the tags a reader follows, the sites and feeds an account follows, and
//! what the Reader shows of a site or a feed besides its posts.

use crate::{
    RowId, SqliteDbError,
    db_types::reader::{reader_date_from_text, reader_date_text},
    repository::QueryExecutor,
};
use rusqlite::{OptionalExtension, Row, params};
use wp_api::date::WpGmtDateTime;
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::reader_saved::ReaderFeedId;
use wp_api::wp_com::reader_subscriptions::{
    ReaderSubscriptionEmailFrequency, ReaderSubscriptionId,
};
use wp_api::wp_com::reader_tags::{
    ReaderMenuItem, ReaderMenuItemKind, ReaderMenuSection, ReaderTagId,
};

fn section_number(section: ReaderMenuSection) -> i64 {
    match section {
        ReaderMenuSection::Default => 0,
        ReaderMenuSection::Subscribed => 1,
        ReaderMenuSection::Recommended => 2,
    }
}

fn kind_text(kind: &ReaderMenuItemKind) -> &str {
    match kind {
        ReaderMenuItemKind::Stream => "stream",
        ReaderMenuItemKind::Tag => "tag",
        ReaderMenuItemKind::List => "list",
        ReaderMenuItemKind::Organization => "organization",
        ReaderMenuItemKind::Other(kind) => kind,
    }
}

fn kind_from_text(kind: String) -> ReaderMenuItemKind {
    match kind.as_str() {
        "stream" => ReaderMenuItemKind::Stream,
        "tag" => ReaderMenuItemKind::Tag,
        "list" => ReaderMenuItemKind::List,
        "organization" => ReaderMenuItemKind::Organization,
        _ => ReaderMenuItemKind::Other(kind),
    }
}

const MENU_COLUMNS: &str = "key, kind, title, url, slug, display_name, tag_id, organization_id";

fn menu_item_from_row(row: &Row) -> rusqlite::Result<ReaderMenuItem> {
    Ok(ReaderMenuItem {
        key: row.get(0)?,
        kind: kind_from_text(row.get(1)?),
        title: row.get(2)?,
        url: row.get(3)?,
        slug: row.get(4)?,
        display_name: row.get(5)?,
        tag_id: row.get::<_, Option<u64>>(6)?.map(ReaderTagId),
        organization_id: row.get(7)?,
    })
}

/// The Reader menu. The tags a reader follows are its `Subscribed` section.
pub struct ReaderMenuRepository;

impl ReaderMenuRepository {
    /// Makes a section hold exactly `items`, in that order. Call it inside a transaction.
    pub fn replace_section(
        executor: &impl QueryExecutor,
        account: RowId,
        section: ReaderMenuSection,
        items: &[ReaderMenuItem],
    ) -> Result<(), SqliteDbError> {
        executor.execute(
            "DELETE FROM reader_menu_items WHERE account_id = ?1 AND section = ?2",
            params![account, section_number(section)],
        )?;
        for item in items {
            Self::upsert_item(executor, account, section, item)?;
        }
        Ok(())
    }

    /// Adds an entry at the end of a section, or updates it where it is.
    pub fn upsert_item(
        executor: &impl QueryExecutor,
        account: RowId,
        section: ReaderMenuSection,
        item: &ReaderMenuItem,
    ) -> Result<(), SqliteDbError> {
        executor.execute(
            "INSERT INTO reader_menu_items (
                account_id, section, key, kind, position, title, url, slug, display_name, tag_id,
                organization_id
            ) VALUES (?1, ?2, ?3, ?4,
                (SELECT COALESCE(MAX(position), -1) + 1 FROM reader_menu_items
                    WHERE account_id = ?1 AND section = ?2),
                ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(account_id, section, key) DO UPDATE SET
                kind = excluded.kind,
                title = COALESCE(excluded.title, title),
                url = COALESCE(excluded.url, url),
                slug = COALESCE(excluded.slug, slug),
                display_name = COALESCE(excluded.display_name, display_name),
                tag_id = COALESCE(excluded.tag_id, tag_id),
                organization_id = COALESCE(excluded.organization_id, organization_id)",
            params![
                account,
                section_number(section),
                item.key,
                kind_text(&item.kind),
                item.title,
                item.url,
                item.slug,
                item.display_name,
                item.tag_id.map(|id| id.0),
                item.organization_id,
            ],
        )?;
        Ok(())
    }

    /// Removes a section's entries for a tag, found by its slug as [`Self::select_tag`] finds
    /// them. Returns whether there was one.
    pub fn delete_tag(
        executor: &impl QueryExecutor,
        account: RowId,
        section: ReaderMenuSection,
        slug: &str,
    ) -> Result<bool, SqliteDbError> {
        let removed = executor.execute(
            "DELETE FROM reader_menu_items
            WHERE account_id = ?1 AND section = ?2 AND (slug = ?3 OR (slug IS NULL AND key = ?3))",
            params![account, section_number(section), slug],
        )?;
        Ok(removed > 0)
    }

    /// A section's entry for a tag, found by its slug. The server keys a tag in the menu by the
    /// tag's ID, and a tag followed on this device is keyed by its slug, so the slug is what both
    /// have. An entry without a slug is matched by its key.
    pub fn select_tag(
        executor: &impl QueryExecutor,
        account: RowId,
        section: ReaderMenuSection,
        slug: &str,
    ) -> Result<Option<ReaderMenuItem>, SqliteDbError> {
        let sql = format!(
            "SELECT {MENU_COLUMNS} FROM reader_menu_items
            WHERE account_id = ?1 AND section = ?2 AND (slug = ?3 OR (slug IS NULL AND key = ?3))
            ORDER BY position, rowid LIMIT 1"
        );
        let mut statement = executor.prepare(&sql)?;
        Ok(statement
            .query_row(
                params![account, section_number(section), slug],
                menu_item_from_row,
            )
            .optional()?)
    }

    /// A section's entries, in order.
    pub fn select_section(
        executor: &impl QueryExecutor,
        account: RowId,
        section: ReaderMenuSection,
    ) -> Result<Vec<ReaderMenuItem>, SqliteDbError> {
        let sql = format!(
            "SELECT {MENU_COLUMNS} FROM reader_menu_items
            WHERE account_id = ?1 AND section = ?2 ORDER BY position, rowid"
        );
        let mut statement = executor.prepare(&sql)?;
        let rows = statement.query_map(
            params![account, section_number(section)],
            menu_item_from_row,
        )?;
        rows.map(|row| row.map_err(SqliteDbError::from)).collect()
    }
}

/// A site or feed the account follows, as the cache has it.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ReaderSubscriptionRecord {
    /// The subscription's own ID. `None` for a follow made on this device that the server hasn't
    /// answered yet.
    #[uniffi(default = None)]
    pub subscription_id: Option<ReaderSubscriptionId>,
    /// `None` for a feed that belongs to no WordPress.com or Jetpack site.
    #[uniffi(default = None)]
    pub site_id: Option<WpComSiteId>,
    #[uniffi(default = None)]
    pub feed_id: Option<ReaderFeedId>,
    /// The address that was followed.
    #[uniffi(default = None)]
    pub url: Option<String>,
    #[uniffi(default = None)]
    pub feed_url: Option<String>,
    #[uniffi(default = None)]
    pub name: Option<String>,
    #[uniffi(default = None)]
    pub icon_url: Option<String>,
    #[uniffi(default = None)]
    pub date_subscribed: Option<WpGmtDateTime>,
    #[uniffi(default = None)]
    pub unseen_count: Option<u64>,
    /// Set for an Automattic (`1`) or P2 (`2`) site.
    #[uniffi(default = None)]
    pub organization_id: Option<u64>,
    #[uniffi(default = None)]
    pub is_owner: Option<bool>,
    #[uniffi(default = None)]
    pub is_rss: Option<bool>,
    /// Whether new posts are emailed.
    #[uniffi(default = None)]
    pub email_posts: Option<bool>,
    /// Whether new comments are emailed.
    #[uniffi(default = None)]
    pub email_comments: Option<bool>,
    /// How often new posts are emailed.
    #[uniffi(default = None)]
    pub email_frequency: Option<ReaderSubscriptionEmailFrequency>,
    /// Whether new posts send a notification.
    #[uniffi(default = None)]
    pub notify_posts: Option<bool>,
}

fn frequency_text(frequency: &ReaderSubscriptionEmailFrequency) -> &str {
    match frequency {
        ReaderSubscriptionEmailFrequency::Instantly => "instantly",
        ReaderSubscriptionEmailFrequency::Daily => "daily",
        ReaderSubscriptionEmailFrequency::Weekly => "weekly",
        ReaderSubscriptionEmailFrequency::Other(frequency) => frequency,
    }
}

fn frequency_from_text(frequency: String) -> ReaderSubscriptionEmailFrequency {
    match frequency.as_str() {
        "instantly" => ReaderSubscriptionEmailFrequency::Instantly,
        "daily" => ReaderSubscriptionEmailFrequency::Daily,
        "weekly" => ReaderSubscriptionEmailFrequency::Weekly,
        _ => ReaderSubscriptionEmailFrequency::Other(frequency),
    }
}

const SUBSCRIPTION_COLUMNS: &str = "subscription_id, site_id, feed_id, url, feed_url, name, \
    icon_url, date_subscribed, unseen_count, organization_id, is_owner, is_rss, email_posts, \
    email_comments, email_frequency, notify_posts";

fn subscription_from_row(row: &Row) -> Result<ReaderSubscriptionRecord, SqliteDbError> {
    Ok(ReaderSubscriptionRecord {
        subscription_id: row.get::<_, Option<u64>>(0)?.map(ReaderSubscriptionId),
        site_id: row.get::<_, Option<u64>>(1)?.map(WpComSiteId),
        feed_id: row.get::<_, Option<u64>>(2)?.map(ReaderFeedId),
        url: row.get(3)?,
        feed_url: row.get(4)?,
        name: row.get(5)?,
        icon_url: row.get(6)?,
        date_subscribed: row
            .get::<_, Option<String>>(7)?
            .map(|text| reader_date_from_text(&text))
            .transpose()?,
        unseen_count: row.get(8)?,
        organization_id: row.get(9)?,
        is_owner: row.get(10)?,
        is_rss: row.get(11)?,
        email_posts: row.get(12)?,
        email_comments: row.get(13)?,
        email_frequency: row.get::<_, Option<String>>(14)?.map(frequency_from_text),
        notify_posts: row.get(15)?,
    })
}

/// Which delivery setting of a subscription to change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReaderDeliverySetting {
    EmailPosts,
    EmailComments,
    NotifyPosts,
}

/// The sites and feeds an account follows.
pub struct ReaderSubscriptionRepository;

impl ReaderSubscriptionRepository {
    fn insert(
        executor: &impl QueryExecutor,
        account: RowId,
        subscription: &ReaderSubscriptionRecord,
        position: i64,
    ) -> Result<(), SqliteDbError> {
        executor.execute(
            "INSERT INTO reader_subscriptions (
                account_id, subscription_id, site_id, feed_id, url, feed_url, name, icon_url,
                date_subscribed, unseen_count, organization_id, is_owner, is_rss, email_posts,
                email_comments, email_frequency, notify_posts, position
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17,
                ?18)",
            params![
                account,
                subscription.subscription_id.map(|id| id.0),
                subscription.site_id.map(|id| id.0),
                subscription.feed_id.map(|id| id.0),
                subscription.url,
                subscription.feed_url,
                subscription.name,
                subscription.icon_url,
                subscription.date_subscribed.as_ref().map(reader_date_text),
                subscription.unseen_count,
                subscription.organization_id,
                subscription.is_owner,
                subscription.is_rss,
                subscription.email_posts,
                subscription.email_comments,
                subscription.email_frequency.as_ref().map(frequency_text),
                subscription.notify_posts,
                position,
            ],
        )?;
        Ok(())
    }

    /// Makes the account's subscriptions be exactly `subscriptions`, in that order. Call it inside
    /// a transaction.
    pub fn replace_all(
        executor: &impl QueryExecutor,
        account: RowId,
        subscriptions: &[ReaderSubscriptionRecord],
    ) -> Result<(), SqliteDbError> {
        executor.execute(
            "DELETE FROM reader_subscriptions WHERE account_id = ?1",
            [account],
        )?;
        for (position, subscription) in subscriptions.iter().enumerate() {
            Self::insert(executor, account, subscription, position as i64)?;
        }
        Ok(())
    }

    /// Adds a subscription ahead of the others, as the newest is listed first. One already there
    /// for the same site or feed is replaced.
    pub fn add_first(
        executor: &impl QueryExecutor,
        account: RowId,
        subscription: &ReaderSubscriptionRecord,
    ) -> Result<(), SqliteDbError> {
        Self::remove(
            executor,
            account,
            subscription.site_id,
            subscription.feed_id,
            subscription.url.as_deref(),
        )?;
        let mut statement = executor.prepare(
            "SELECT COALESCE(MIN(position), 0) - 1 FROM reader_subscriptions WHERE account_id = ?1",
        )?;
        let position: i64 = statement.query_row([account], |row| row.get(0))?;
        Self::insert(executor, account, subscription, position)
    }

    /// Moves the count of posts the reader hasn't seen of a followed site or feed by `change`,
    /// never below zero, on the subscriptions to whichever of the two is given. A missing count
    /// stays missing. The next fetch of the subscriptions replaces it with the server's count.
    pub fn add_to_unseen_count(
        executor: &impl QueryExecutor,
        account: RowId,
        site_id: Option<WpComSiteId>,
        feed_id: Option<ReaderFeedId>,
        change: i64,
    ) -> Result<(), SqliteDbError> {
        executor.execute(
            "UPDATE reader_subscriptions SET unseen_count = MAX(unseen_count + ?4, 0)
            WHERE account_id = ?1 AND unseen_count IS NOT NULL AND (
                (?2 IS NOT NULL AND site_id = ?2) OR (?3 IS NOT NULL AND feed_id = ?3))",
            params![
                account,
                site_id.map(|id| id.0),
                feed_id.map(|id| id.0),
                change
            ],
        )?;
        Ok(())
    }

    /// Removes the subscriptions to a site, a feed or an address: whichever of the three is
    /// given and matches. Returns how many it removed.
    pub fn remove(
        executor: &impl QueryExecutor,
        account: RowId,
        site_id: Option<WpComSiteId>,
        feed_id: Option<ReaderFeedId>,
        url: Option<&str>,
    ) -> Result<u64, SqliteDbError> {
        let removed = executor.execute(
            "DELETE FROM reader_subscriptions
            WHERE account_id = ?1 AND (
                (?2 IS NOT NULL AND site_id = ?2)
                OR (?3 IS NOT NULL AND feed_id = ?3)
                OR (?4 IS NOT NULL AND url = ?4))",
            params![account, site_id.map(|id| id.0), feed_id.map(|id| id.0), url],
        )?;
        Ok(removed as u64)
    }

    /// Whether the account follows a site or a feed, as far as the list says.
    pub fn is_following(
        executor: &impl QueryExecutor,
        account: RowId,
        site_id: Option<WpComSiteId>,
        feed_id: Option<ReaderFeedId>,
    ) -> Result<bool, SqliteDbError> {
        let mut statement = executor.prepare(
            "SELECT EXISTS(SELECT 1 FROM reader_subscriptions
            WHERE account_id = ?1 AND (
                (?2 IS NOT NULL AND site_id = ?2) OR (?3 IS NOT NULL AND feed_id = ?3)))",
        )?;
        Ok(statement.query_row(
            params![account, site_id.map(|id| id.0), feed_id.map(|id| id.0)],
            |row| row.get(0),
        )?)
    }

    /// The account's subscriptions, in the order the server lists them: newest first.
    pub fn select_all(
        executor: &impl QueryExecutor,
        account: RowId,
    ) -> Result<Vec<ReaderSubscriptionRecord>, SqliteDbError> {
        let sql = format!(
            "SELECT {SUBSCRIPTION_COLUMNS} FROM reader_subscriptions
            WHERE account_id = ?1 ORDER BY position, rowid"
        );
        let mut statement = executor.prepare(&sql)?;
        let rows = statement.query_map([account], |row| Ok(subscription_from_row(row)))?;
        rows.map(|row| row.map_err(SqliteDbError::from).and_then(|row| row))
            .collect()
    }

    /// The subscription to a site, if the account follows it.
    pub fn select_for_site(
        executor: &impl QueryExecutor,
        account: RowId,
        site_id: WpComSiteId,
    ) -> Result<Option<ReaderSubscriptionRecord>, SqliteDbError> {
        let sql = format!(
            "SELECT {SUBSCRIPTION_COLUMNS} FROM reader_subscriptions
            WHERE account_id = ?1 AND site_id = ?2 ORDER BY position LIMIT 1"
        );
        let mut statement = executor.prepare(&sql)?;
        statement
            .query_row(params![account, site_id.0], |row| {
                Ok(subscription_from_row(row))
            })
            .optional()?
            .transpose()
    }

    /// Turns a delivery setting of a site's subscription on or off. Returns whether the account
    /// has a subscription to the site.
    pub fn set_delivery(
        executor: &impl QueryExecutor,
        account: RowId,
        site_id: WpComSiteId,
        setting: ReaderDeliverySetting,
        on: bool,
    ) -> Result<bool, SqliteDbError> {
        let column = match setting {
            ReaderDeliverySetting::EmailPosts => "email_posts",
            ReaderDeliverySetting::EmailComments => "email_comments",
            ReaderDeliverySetting::NotifyPosts => "notify_posts",
        };
        let sql = format!(
            "UPDATE reader_subscriptions SET {column} = ?3 WHERE account_id = ?1 AND site_id = ?2"
        );
        Ok(executor.execute(&sql, params![account, site_id.0, on])? > 0)
    }

    /// Sets how often a site's new posts are emailed.
    pub fn set_email_frequency(
        executor: &impl QueryExecutor,
        account: RowId,
        site_id: WpComSiteId,
        frequency: &ReaderSubscriptionEmailFrequency,
    ) -> Result<bool, SqliteDbError> {
        Ok(executor.execute(
            "UPDATE reader_subscriptions SET email_frequency = ?3
            WHERE account_id = ?1 AND site_id = ?2",
            params![account, site_id.0, frequency_text(frequency)],
        )? > 0)
    }
}

/// What the Reader shows of a site or a feed besides its posts, as the cache has it.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ReaderSiteRecord {
    /// `None` for a feed that belongs to no WordPress.com or Jetpack site.
    #[uniffi(default = None)]
    pub site_id: Option<WpComSiteId>,
    /// `None` for a site the Reader has no feed for.
    #[uniffi(default = None)]
    pub feed_id: Option<ReaderFeedId>,
    #[uniffi(default = None)]
    pub name: Option<String>,
    #[uniffi(default = None)]
    pub description: Option<String>,
    #[uniffi(default = None)]
    pub url: Option<String>,
    #[uniffi(default = None)]
    pub feed_url: Option<String>,
    #[uniffi(default = None)]
    pub icon_url: Option<String>,
    #[uniffi(default = None)]
    pub subscribers_count: Option<u64>,
    #[uniffi(default = None)]
    pub post_count: Option<u64>,
    #[uniffi(default = None)]
    pub is_private: Option<bool>,
    #[uniffi(default = None)]
    pub is_jetpack: Option<bool>,
    /// Set for an Automattic (`1`) or P2 (`2`) site.
    #[uniffi(default = None)]
    pub organization_id: Option<u64>,
    #[uniffi(default = None)]
    pub is_following: Option<bool>,
}

impl ReaderSiteRecord {
    /// The key the record is stored under: `site:{id}` for a WordPress.com or Jetpack site, and
    /// `feed:{id}` for a feed that belongs to none. `None` with neither ID.
    pub fn key(&self) -> Option<String> {
        match (self.site_id, self.feed_id) {
            (Some(site), _) => Some(format!("site:{}", site.0)),
            (None, Some(feed)) => Some(format!("feed:{}", feed.0)),
            (None, None) => None,
        }
    }
}

const SITE_COLUMNS: &str = "site_id, feed_id, name, description, url, feed_url, icon_url, \
    subscribers_count, post_count, is_private, is_jetpack, organization_id, is_following";

fn site_from_row(row: &Row) -> rusqlite::Result<ReaderSiteRecord> {
    Ok(ReaderSiteRecord {
        site_id: row.get::<_, Option<u64>>(0)?.map(WpComSiteId),
        feed_id: row.get::<_, Option<u64>>(1)?.map(ReaderFeedId),
        name: row.get(2)?,
        description: row.get(3)?,
        url: row.get(4)?,
        feed_url: row.get(5)?,
        icon_url: row.get(6)?,
        subscribers_count: row.get(7)?,
        post_count: row.get(8)?,
        is_private: row.get(9)?,
        is_jetpack: row.get(10)?,
        organization_id: row.get(11)?,
        is_following: row.get(12)?,
    })
}

/// What the Reader shows of sites and feeds besides their posts.
pub struct ReaderSiteRepository;

impl ReaderSiteRepository {
    /// Stores a site or feed as it was fetched at `now`. A field it doesn't have keeps what's
    /// stored. Does nothing for a record with neither a site ID nor a feed ID.
    pub fn upsert(
        executor: &impl QueryExecutor,
        account: RowId,
        site: &ReaderSiteRecord,
        now: &WpGmtDateTime,
    ) -> Result<(), SqliteDbError> {
        let Some(key) = site.key() else {
            return Ok(());
        };
        executor.execute(
            "INSERT INTO reader_sites (
                account_id, key, site_id, feed_id, name, description, url, feed_url, icon_url,
                subscribers_count, post_count, is_private, is_jetpack, organization_id,
                is_following, fetched_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
            ON CONFLICT(account_id, key) DO UPDATE SET
                feed_id = COALESCE(excluded.feed_id, feed_id),
                name = COALESCE(excluded.name, name),
                description = COALESCE(excluded.description, description),
                url = COALESCE(excluded.url, url),
                feed_url = COALESCE(excluded.feed_url, feed_url),
                icon_url = COALESCE(excluded.icon_url, icon_url),
                subscribers_count = COALESCE(excluded.subscribers_count, subscribers_count),
                post_count = COALESCE(excluded.post_count, post_count),
                is_private = COALESCE(excluded.is_private, is_private),
                is_jetpack = COALESCE(excluded.is_jetpack, is_jetpack),
                organization_id = COALESCE(excluded.organization_id, organization_id),
                is_following = COALESCE(excluded.is_following, is_following),
                fetched_at = excluded.fetched_at",
            params![
                account,
                key,
                site.site_id.map(|id| id.0),
                site.feed_id.map(|id| id.0),
                site.name,
                site.description,
                site.url,
                site.feed_url,
                site.icon_url,
                site.subscribers_count,
                site.post_count,
                site.is_private,
                site.is_jetpack,
                site.organization_id,
                site.is_following,
                reader_date_text(now),
            ],
        )?;
        Ok(())
    }

    /// The stored site, or the stored feed when there's no site ID to look by.
    pub fn select(
        executor: &impl QueryExecutor,
        account: RowId,
        site_id: Option<WpComSiteId>,
        feed_id: Option<ReaderFeedId>,
    ) -> Result<Option<ReaderSiteRecord>, SqliteDbError> {
        let sql = format!(
            "SELECT {SITE_COLUMNS} FROM reader_sites
            WHERE account_id = ?1 AND (
                (?2 IS NOT NULL AND site_id = ?2) OR (?2 IS NULL AND feed_id = ?3))
            ORDER BY site_id IS NULL LIMIT 1"
        );
        let mut statement = executor.prepare(&sql)?;
        Ok(statement
            .query_row(
                params![account, site_id.map(|id| id.0), feed_id.map(|id| id.0)],
                site_from_row,
            )
            .optional()?)
    }

    /// Records whether the account follows a site or a feed, on every stored record of either.
    ///
    /// A record whose stored follow this changes counts the account in or out of its subscriber
    /// count, never below zero. A record that was already as asked, or that doesn't say whether
    /// it's followed, keeps its count, and a missing count stays missing. The record's next fetch
    /// replaces it with the server's count.
    pub fn set_following(
        executor: &impl QueryExecutor,
        account: RowId,
        site_id: Option<WpComSiteId>,
        feed_id: Option<ReaderFeedId>,
        following: bool,
    ) -> Result<(), SqliteDbError> {
        // `SET` reads the row as it was, so `is_following` here is the follow before this one.
        executor.execute(
            "UPDATE reader_sites SET
                subscribers_count = CASE
                    WHEN is_following IS NULL OR is_following = ?4 THEN subscribers_count
                    WHEN ?4 THEN subscribers_count + 1
                    ELSE MAX(subscribers_count - 1, 0)
                END,
                is_following = ?4
            WHERE account_id = ?1 AND (
                (?2 IS NOT NULL AND site_id = ?2) OR (?3 IS NOT NULL AND feed_id = ?3))",
            params![
                account,
                site_id.map(|id| id.0),
                feed_id.map(|id| id.0),
                following
            ],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MigrationManager;
    use crate::repository::wordpress_com_accounts::WordPressComAccountRepository;
    use rusqlite::Connection;
    use wp_api::wp_com::me::WpComUserId;

    fn conn() -> (Connection, RowId) {
        let conn = Connection::open_in_memory().unwrap();
        MigrationManager::new(&conn)
            .unwrap()
            .perform_migrations()
            .unwrap();
        let account = WordPressComAccountRepository::get_or_create(&conn, WpComUserId(1)).unwrap();
        (conn, account)
    }

    fn tag(slug: &str) -> ReaderMenuItem {
        ReaderMenuItem {
            key: slug.to_string(),
            kind: ReaderMenuItemKind::Tag,
            title: Some(slug.to_uppercase()),
            url: None,
            slug: Some(slug.to_string()),
            display_name: None,
            tag_id: None,
            organization_id: None,
        }
    }

    fn keys(conn: &Connection, account: RowId, section: ReaderMenuSection) -> Vec<String> {
        ReaderMenuRepository::select_section(conn, account, section)
            .unwrap()
            .into_iter()
            .map(|item| item.key)
            .collect()
    }

    #[test]
    fn test_a_menu_section_keeps_its_order_and_its_own_entries() {
        let (conn, account) = conn();
        let subscribed = ReaderMenuSection::Subscribed;
        ReaderMenuRepository::replace_section(&conn, account, subscribed, &[tag("b"), tag("a")])
            .unwrap();
        ReaderMenuRepository::replace_section(
            &conn,
            account,
            ReaderMenuSection::Recommended,
            &[tag("a")],
        )
        .unwrap();
        assert_eq!(keys(&conn, account, subscribed), ["b", "a"]);

        // A tag followed on this device goes last, and one already there stays where it is.
        ReaderMenuRepository::upsert_item(&conn, account, subscribed, &tag("c")).unwrap();
        ReaderMenuRepository::upsert_item(&conn, account, subscribed, &tag("b")).unwrap();
        assert_eq!(keys(&conn, account, subscribed), ["b", "a", "c"]);

        assert!(ReaderMenuRepository::delete_tag(&conn, account, subscribed, "a").unwrap());
        assert!(!ReaderMenuRepository::delete_tag(&conn, account, subscribed, "a").unwrap());
        assert_eq!(keys(&conn, account, subscribed), ["b", "c"]);
        assert_eq!(keys(&conn, account, ReaderMenuSection::Recommended), ["a"]);
        let item = ReaderMenuRepository::select_tag(&conn, account, subscribed, "c")
            .unwrap()
            .unwrap();
        assert_eq!(item, tag("c"));
    }

    #[test]
    fn test_a_tag_is_found_and_removed_by_its_slug_whatever_its_key() {
        let (conn, account) = conn();
        let subscribed = ReaderMenuSection::Subscribed;
        // As the server lists it: keyed by the tag's ID.
        let listed = ReaderMenuItem {
            key: "9002".to_string(),
            ..tag("knitting")
        };
        // An entry without a slug is known by its key.
        let without_slug = ReaderMenuItem {
            slug: None,
            ..tag("travel")
        };
        ReaderMenuRepository::replace_section(
            &conn,
            account,
            subscribed,
            &[listed.clone(), without_slug.clone()],
        )
        .unwrap();

        let found = |slug: &str| ReaderMenuRepository::select_tag(&conn, account, subscribed, slug);
        assert_eq!(found("knitting").unwrap(), Some(listed));
        assert_eq!(found("9002").unwrap(), None);
        assert_eq!(found("travel").unwrap(), Some(without_slug));

        assert!(ReaderMenuRepository::delete_tag(&conn, account, subscribed, "knitting").unwrap());
        assert!(ReaderMenuRepository::delete_tag(&conn, account, subscribed, "travel").unwrap());
        assert!(keys(&conn, account, subscribed).is_empty());
    }

    #[test]
    fn test_a_menu_entry_of_an_unknown_kind_is_kept_as_it_was_named() {
        let (conn, account) = conn();
        let item = ReaderMenuItem {
            kind: ReaderMenuItemKind::Other("something_new".to_string()),
            organization_id: Some(2),
            tag_id: Some(ReaderTagId(9)),
            ..tag("x")
        };
        ReaderMenuRepository::replace_section(
            &conn,
            account,
            ReaderMenuSection::Default,
            std::slice::from_ref(&item),
        )
        .unwrap();
        assert_eq!(
            ReaderMenuRepository::select_section(&conn, account, ReaderMenuSection::Default)
                .unwrap(),
            [item]
        );
    }

    fn subscription(id: u64, site: Option<u64>, feed: u64) -> ReaderSubscriptionRecord {
        ReaderSubscriptionRecord {
            subscription_id: Some(ReaderSubscriptionId(id)),
            site_id: site.map(WpComSiteId),
            feed_id: Some(ReaderFeedId(feed)),
            url: Some(format!("https://site{feed}.example")),
            feed_url: None,
            name: Some(format!("Site {feed}")),
            icon_url: None,
            date_subscribed: Some("2026-11-18T10:00:00Z".parse().unwrap()),
            unseen_count: Some(3),
            organization_id: None,
            is_owner: Some(false),
            is_rss: Some(site.is_none()),
            email_posts: Some(false),
            email_comments: None,
            email_frequency: Some(ReaderSubscriptionEmailFrequency::Weekly),
            notify_posts: Some(false),
        }
    }

    #[test]
    fn test_subscriptions_are_listed_in_order_and_found_by_site_or_feed() {
        let (conn, account) = conn();
        let listed = [subscription(1, Some(10), 100), subscription(2, None, 200)];
        ReaderSubscriptionRepository::replace_all(&conn, account, &listed).unwrap();
        assert_eq!(
            ReaderSubscriptionRepository::select_all(&conn, account).unwrap(),
            listed
        );
        let following = |site: Option<u64>, feed: Option<u64>| {
            ReaderSubscriptionRepository::is_following(
                &conn,
                account,
                site.map(WpComSiteId),
                feed.map(ReaderFeedId),
            )
            .unwrap()
        };
        assert!(following(Some(10), None));
        assert!(following(None, Some(200)));
        assert!(!following(Some(11), None));
        assert!(!following(None, None));

        // A new follow goes first, and replaces what was there for the same site.
        let new = ReaderSubscriptionRecord {
            subscription_id: None,
            ..subscription(3, Some(10), 100)
        };
        ReaderSubscriptionRepository::add_first(&conn, account, &new).unwrap();
        let all = ReaderSubscriptionRepository::select_all(&conn, account).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].subscription_id, None);

        assert_eq!(
            ReaderSubscriptionRepository::remove(
                &conn,
                account,
                None,
                None,
                Some("https://site200.example")
            )
            .unwrap(),
            1
        );
        assert!(!following(None, Some(200)));
    }

    #[test]
    fn test_a_subscriptions_delivery_settings_change() {
        let (conn, account) = conn();
        ReaderSubscriptionRepository::replace_all(
            &conn,
            account,
            &[subscription(1, Some(10), 100)],
        )
        .unwrap();
        let site = WpComSiteId(10);
        assert!(
            ReaderSubscriptionRepository::set_delivery(
                &conn,
                account,
                site,
                ReaderDeliverySetting::EmailPosts,
                true
            )
            .unwrap()
        );
        assert!(
            ReaderSubscriptionRepository::set_delivery(
                &conn,
                account,
                site,
                ReaderDeliverySetting::NotifyPosts,
                true
            )
            .unwrap()
        );
        ReaderSubscriptionRepository::set_email_frequency(
            &conn,
            account,
            site,
            &ReaderSubscriptionEmailFrequency::Daily,
        )
        .unwrap();
        let stored = ReaderSubscriptionRepository::select_for_site(&conn, account, site)
            .unwrap()
            .unwrap();
        assert_eq!(stored.email_posts, Some(true));
        assert_eq!(stored.notify_posts, Some(true));
        assert_eq!(stored.email_comments, None);
        assert_eq!(
            stored.email_frequency,
            Some(ReaderSubscriptionEmailFrequency::Daily)
        );
        // A site the account doesn't follow has no settings to change.
        assert!(
            !ReaderSubscriptionRepository::set_delivery(
                &conn,
                account,
                WpComSiteId(11),
                ReaderDeliverySetting::EmailPosts,
                true
            )
            .unwrap()
        );
    }

    #[test]
    fn test_a_site_is_stored_and_a_later_fetch_keeps_what_it_leaves_out() {
        let (conn, account) = conn();
        let now: WpGmtDateTime = "2026-11-18T10:00:00Z".parse().unwrap();
        let site = ReaderSiteRecord {
            site_id: Some(WpComSiteId(10)),
            feed_id: Some(ReaderFeedId(100)),
            name: Some("A site".to_string()),
            description: Some("About it".to_string()),
            url: Some("https://site.example".to_string()),
            feed_url: None,
            icon_url: None,
            subscribers_count: Some(12),
            post_count: Some(34),
            is_private: Some(false),
            is_jetpack: Some(false),
            organization_id: None,
            is_following: Some(false),
        };
        ReaderSiteRepository::upsert(&conn, account, &site, &now).unwrap();
        let select = |site: Option<u64>, feed: Option<u64>| {
            ReaderSiteRepository::select(
                &conn,
                account,
                site.map(WpComSiteId),
                feed.map(ReaderFeedId),
            )
            .unwrap()
        };
        assert_eq!(select(Some(10), None), Some(site.clone()));
        // Signed out, the same site comes without its post count.
        let partial = ReaderSiteRecord {
            post_count: None,
            subscribers_count: Some(13),
            ..site.clone()
        };
        ReaderSiteRepository::upsert(&conn, account, &partial, &now).unwrap();
        let stored = select(Some(10), None).unwrap();
        assert_eq!(stored.post_count, Some(34));
        assert_eq!(stored.subscribers_count, Some(13));

        // A feed with no site is stored by its feed ID, and isn't found by a site's.
        let feed = ReaderSiteRecord {
            site_id: None,
            feed_id: Some(ReaderFeedId(200)),
            ..site.clone()
        };
        ReaderSiteRepository::upsert(&conn, account, &feed, &now).unwrap();
        assert_eq!(select(None, Some(200)), Some(feed));
        assert_eq!(select(Some(200), None), None);

        ReaderSiteRepository::set_following(&conn, account, None, Some(ReaderFeedId(200)), true)
            .unwrap();
        assert_eq!(select(None, Some(200)).unwrap().is_following, Some(true));
        assert_eq!(select(Some(10), None).unwrap().is_following, Some(false));
        // The feed's follow changed, so it counts one more subscriber; the site's didn't.
        assert_eq!(select(None, Some(200)).unwrap().subscribers_count, Some(13));
        assert_eq!(select(Some(10), None).unwrap().subscribers_count, Some(13));
    }

    #[test]
    fn a_follow_moves_the_subscriber_count_only_when_it_changes() {
        let (conn, account) = conn();
        let now: WpGmtDateTime = "2026-11-18T10:00:00Z".parse().unwrap();
        let site = ReaderSiteRecord {
            site_id: Some(WpComSiteId(10)),
            feed_id: None,
            name: None,
            description: None,
            url: None,
            feed_url: None,
            icon_url: None,
            subscribers_count: Some(0),
            post_count: None,
            is_private: None,
            is_jetpack: None,
            organization_id: None,
            is_following: Some(true),
        };
        ReaderSiteRepository::upsert(&conn, account, &site, &now).unwrap();
        let set = |following| {
            ReaderSiteRepository::set_following(
                &conn,
                account,
                Some(WpComSiteId(10)),
                None,
                following,
            )
            .unwrap();
            let stored = ReaderSiteRepository::select(&conn, account, Some(WpComSiteId(10)), None)
                .unwrap()
                .unwrap();
            (stored.is_following, stored.subscribers_count)
        };
        // A count that's already 0 doesn't go below it.
        assert_eq!(set(false), (Some(false), Some(0)));
        assert_eq!(set(true), (Some(true), Some(1)));
        assert_eq!(set(true), (Some(true), Some(1)));
        assert_eq!(set(false), (Some(false), Some(0)));

        // A missing count stays missing.
        conn.execute("UPDATE reader_sites SET subscribers_count = NULL", [])
            .unwrap();
        assert_eq!(set(true), (Some(true), None));

        // A record that doesn't say whether it's followed keeps its count.
        conn.execute(
            "UPDATE reader_sites SET subscribers_count = 5, is_following = NULL",
            [],
        )
        .unwrap();
        assert_eq!(set(true), (Some(true), Some(5)));
    }
}

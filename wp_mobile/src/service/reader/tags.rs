//! The Reader menu and the tags a reader follows.
//!
//! The menu is the streams, tags, lists and organizations a reader chooses from. It answers
//! signed out too, with the default streams and the recommended tags. The tags a signed-out reader
//! follows are kept on this device only: no server has them.

use super::{ReaderContext, ReaderError, ReaderService};
use wp_api::wp_com::reader_posts::ReaderTagSlug;
use wp_api::wp_com::reader_tags::{
    ReaderInterest, ReaderMenuItem, ReaderMenuItemKind, ReaderMenuSection, ReaderTag,
};
use wp_mobile_cache::RowId;
use wp_mobile_cache::SqliteDbError;
use wp_mobile_cache::db_types::reader::reader_date_text;
use wp_mobile_cache::repository::QueryExecutor;
use wp_mobile_cache::repository::reader_actions::ReaderSyncTimeRepository;
use wp_mobile_cache::repository::reader_sites::ReaderMenuRepository;
use wp_mobile_cache::repository::wordpress_com_accounts::ReaderOwner;

/// The name the menu's fetch time is kept under.
const MENU: &str = "menu";

/// The Reader menu, as the cache has it.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ReaderCachedMenu {
    /// The streams, lists and organizations the reader chooses from: Following, Discover and
    /// Liked, a list for each Reader list, and an organization for Automattic or P2 when the
    /// account belongs to it. Signed out, only Discover.
    pub items: Vec<ReaderMenuItem>,
    /// The tags the reader follows, in the order the server lists them, then the order they were
    /// followed in on this device.
    pub followed_tags: Vec<ReaderMenuItem>,
    /// Tags WordPress.com recommends. The same for every reader.
    pub recommended_tags: Vec<ReaderMenuItem>,
}

/// A followed tag's entry in the menu.
pub(crate) fn tag_menu_item(slug: &ReaderTagSlug, title: Option<String>) -> ReaderMenuItem {
    ReaderMenuItem {
        key: slug.0.clone(),
        kind: ReaderMenuItemKind::Tag,
        title: title.clone(),
        url: None,
        slug: Some(slug.0.clone()),
        display_name: title,
        tag_id: None,
        organization_id: None,
    }
}

/// Makes the cache show a tag as followed, or not.
pub(crate) fn show_tag_followed(
    executor: &impl QueryExecutor,
    owner: RowId,
    slug: &ReaderTagSlug,
    title: Option<String>,
    followed: bool,
) -> Result<(), SqliteDbError> {
    let subscribed = ReaderMenuSection::Subscribed;
    if followed {
        // The tag's name comes from the recommended tags when the caller didn't have it.
        let title = match title {
            Some(title) => Some(title),
            None => ReaderMenuRepository::select_item(
                executor,
                owner,
                ReaderMenuSection::Recommended,
                &slug.0,
            )?
            .and_then(|item| item.title),
        };
        ReaderMenuRepository::upsert_item(executor, owner, subscribed, &tag_menu_item(slug, title))
    } else {
        ReaderMenuRepository::delete_item(executor, owner, subscribed, &slug.0).map(|_| ())
    }
}

/// Whether the cache shows a tag as followed.
pub(crate) fn is_tag_followed(
    executor: &impl QueryExecutor,
    owner: RowId,
    slug: &ReaderTagSlug,
) -> Result<bool, SqliteDbError> {
    Ok(
        ReaderMenuRepository::select_item(executor, owner, ReaderMenuSection::Subscribed, &slug.0)?
            .is_some(),
    )
}

impl ReaderContext {
    pub(crate) fn followed_tags(&self) -> Result<Vec<ReaderMenuItem>, SqliteDbError> {
        self.cache.execute(|connection| {
            ReaderMenuRepository::select_section(
                connection,
                self.owner_row,
                ReaderMenuSection::Subscribed,
            )
        })
    }
}

#[uniffi::export]
impl ReaderService {
    /// The Reader menu, from the cache. Empty until [`Self::refresh_menu`] has run once, apart
    /// from tags followed on this device.
    pub fn menu(&self) -> Result<ReaderCachedMenu, ReaderError> {
        let owner = self.context.owner_row;
        Ok(self.context.cache.execute(|connection| {
            let section =
                |section| ReaderMenuRepository::select_section(&*connection, owner, section);
            Ok::<_, SqliteDbError>(ReaderCachedMenu {
                items: section(ReaderMenuSection::Default)?,
                followed_tags: section(ReaderMenuSection::Subscribed)?,
                recommended_tags: section(ReaderMenuSection::Recommended)?,
            })
        })?)
    }

    /// Fetches the Reader menu. Works signed out, where it brings the default streams and the
    /// recommended tags, and leaves the tags followed on this device as they are.
    ///
    /// For an account, the followed tags become the server's, apart from a follow or unfollow
    /// made here that's still waiting to be sent.
    pub async fn refresh_menu(&self) -> Result<(), ReaderError> {
        let context = &self.context;
        let menu = context.client.reader_tags().menu().await?.data;
        let now = reader_date_text(&context.now());
        context.cache.execute(|connection| {
            let transaction = connection.transaction()?;
            let owner = context.owner_row;
            ReaderMenuRepository::replace_section(
                &transaction,
                owner,
                ReaderMenuSection::Default,
                &menu.default_items,
            )?;
            ReaderMenuRepository::replace_section(
                &transaction,
                owner,
                ReaderMenuSection::Recommended,
                &menu.recommended_items,
            )?;
            if context.owner != ReaderOwner::SignedOut {
                ReaderMenuRepository::replace_section(
                    &transaction,
                    owner,
                    ReaderMenuSection::Subscribed,
                    &menu.subscribed_items,
                )?;
                // The menu may be older than a tag the reader followed or unfollowed since.
                super::actions::show_pending(&transaction, owner)?;
            }
            ReaderSyncTimeRepository::set(&transaction, owner, MENU, &now)?;
            transaction.commit().map_err(SqliteDbError::from)
        })?;
        Ok(())
    }

    /// The tags the reader follows, from the cache.
    pub fn followed_tags(&self) -> Result<Vec<ReaderMenuItem>, ReaderError> {
        Ok(self.context.followed_tags()?)
    }

    /// The tags a new reader picks from. Works signed out. Not cached.
    pub async fn interests(&self) -> Result<Vec<ReaderInterest>, ReaderError> {
        Ok(self
            .context
            .client
            .reader_tags()
            .interests()
            .await?
            .data
            .interests)
    }

    /// A tag's name and description. Works signed out. Not cached. It doesn't say whether the
    /// reader follows the tag: [`Self::followed_tags`] does.
    pub async fn tag_info(&self, slug: ReaderTagSlug) -> Result<ReaderTag, ReaderError> {
        Ok(self.context.client.reader_tags().tag(&slug).await?.data.tag)
    }
}

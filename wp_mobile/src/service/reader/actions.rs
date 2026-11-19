//! What the user turns on or off in the Reader: a post marked as seen, a like, a follow, a block.
//!
//! Each works the same way. A tap changes the cache straight away, so it shows everywhere the cache
//! is read, and adds a row to `reader_actions`: what the user wants, and what the cache showed
//! before. Then every waiting row is sent, oldest tap first, one at a time.
//!
//! - When the server agrees, the row goes.
//! - When the server refuses, the cache goes back to what it showed before, the row goes, and the
//!   tap's call returns the error.
//! - When no answer comes, such as offline, the row stays and [`ReaderService::sync_actions`] sends
//!   it later. Turning something on or off twice is the same as once, so sending again is safe.
//!
//! A refresh that brings the server's older state for something still waiting doesn't undo the
//! tap: what's waiting is shown again after every page is stored.

use super::{ReaderContext, ReaderError, ReaderService};
use futures::lock::Mutex as AsyncMutex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use wp_api::posts::PostId;
use wp_api::prelude::WpApiError;
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::me::WpComUserId;
use wp_api::wp_com::reader_blocks::{ReaderBlockedSite, ReaderBlockedSitesParams};
use wp_api::wp_com::reader_saved::{
    ReaderFeedItemId, ReaderPostSaveIdentity, ReaderSavedSource, reader_saved_key_for_post,
};
use wp_api::wp_com::reader_seen::{
    ReaderSeenFeedItemsParams, ReaderSeenSitePostsParams, ReaderSeenSource,
};
use wp_mobile_cache::db_types::reader::reader_saved_key_text;
use wp_mobile_cache::repository::QueryExecutor;
use wp_mobile_cache::repository::reader::{ReaderPostFlag, ReaderPostRepository};
use wp_mobile_cache::repository::reader_actions::{
    DbReaderAction, DbReaderBlockedSite, ReaderActionRepository, ReaderBlockRepository,
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
}

impl Action {
    /// The action's number in `reader_actions.kind`. It never changes for an action, since rows
    /// outlive a launch.
    fn kind(&self) -> i64 {
        match self {
            Self::Seen { .. } => 1,
            Self::BlockSite { .. } => 2,
        }
    }

    /// What the action is about, in `reader_actions.target`, or `None` when it's missing the IDs
    /// that say.
    fn target(&self) -> Option<String> {
        match self {
            Self::Seen { post } => reader_saved_key_for_post(post.clone())
                .map(|key| reader_saved_key_text(&key.normalized())),
            Self::BlockSite { site_id } => Some(format!("site:{}", site_id.0)),
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
        match self {
            Self::Seen { post } => match Self::post_row(post, executor, owner)? {
                Some(row) => {
                    ReaderPostRepository::select_flag(executor, row, ReaderPostFlag::IsSeen)
                }
                None => Ok(None),
            },
            Self::BlockSite { site_id } => Ok(Some(ReaderBlockRepository::is_site_blocked(
                executor, owner, *site_id,
            )?)),
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
        }
        Ok(())
    }

    /// Asks the server to turn it on or off.
    async fn send(&self, context: &ReaderContext, on: bool) -> Result<(), WpApiError> {
        match self {
            Self::Seen { post } => {
                let source = match context.source {
                    ReaderSavedSource::Android => ReaderSeenSource::Android,
                    ReaderSavedSource::Ios => ReaderSeenSource::Ios,
                };
                let seen = context.client.reader_seen();
                // An RSS post is marked by its feed's IDs, and any other by its site's.
                if let (true, Some(feed_id)) = (post.is_external, post.feed_id) {
                    let params = ReaderSeenFeedItemsParams {
                        feed_id,
                        feed_item_ids: vec![post.feed_item_id.unwrap_or(ReaderFeedItemId(post.id))],
                        source,
                    };
                    if on {
                        seen.feed_items_seen(&params).await?;
                    } else {
                        seen.feed_items_unseen(&params).await?;
                    }
                } else {
                    let params = ReaderSeenSitePostsParams {
                        blog_id: post.site_id.unwrap_or(WpComSiteId(0)),
                        post_ids: vec![PostId(post.id as i64)],
                        source,
                    };
                    if on {
                        seen.site_posts_seen(&params).await?;
                    } else {
                        seen.site_posts_unseen(&params).await?;
                    }
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
        }
        Ok(())
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

/// A target a tap waits on: its action's kind and target.
type Target = (i64, String);

/// The sending of what the user turned on or off, one request at a time.
#[derive(Default)]
pub(crate) struct ActionQueue {
    /// Held while rows are sent, so they go in order.
    lane: AsyncMutex<()>,
    /// The targets taps are waiting on, and how many taps wait on each.
    waiting: Mutex<HashMap<Target, usize>>,
    /// Why the server refused a target a tap is waiting on, until the tap takes it. The call that
    /// sends a row isn't always its tap's: whichever call holds the lane sends every row.
    refusals: Mutex<HashMap<Target, WpApiError>>,
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
        let target = (
            action.kind(),
            action.target().ok_or(ReaderError::MissingIds)?,
        );
        let payload = serde_json::to_string(&action)
            .map_err(|error| SqliteDbError::SqliteError(error.to_string()))?;
        self.cache.execute(|connection| {
            let transaction = connection.transaction()?;
            let was = action.shown(&transaction, self.owner_row)?;
            action.show(&transaction, self.owner_row, on)?;
            ReaderActionRepository::tap(
                &transaction,
                self.owner_row,
                target.0,
                &target.1,
                on,
                was,
                &payload,
            )?;
            transaction.commit().map_err(SqliteDbError::from)
        })?;
        // A refusal from before this tap was of an older one.
        self.actions
            .refusals
            .lock()
            .expect("refusals lock poisoned")
            .remove(&target);
        let _waiting = Waiting::new(&self.actions, target.clone());

        let unsent = {
            let _lane = self.actions.lane.lock().await;
            self.send_pending_actions().await?
        };
        if let Some(error) = self
            .actions
            .refusals
            .lock()
            .expect("refusals lock poisoned")
            .remove(&target)
        {
            return Err(ReaderError::Api(error));
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
                Ok(()) => self.action_answered(&row, &action, None)?,
                Err(error) if is_refused(&error) => {
                    self.action_answered(&row, &action, Some(error))?
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

    /// Records the server's answer to a row: it agreed, or refused with `refusal`.
    fn action_answered(
        &self,
        row: &DbReaderAction,
        action: &Action,
        refusal: Option<WpApiError>,
    ) -> Result<(), SqliteDbError> {
        let target = (row.kind, row.target.clone());
        let finished = self.cache.execute(|connection| {
            let transaction = connection.transaction()?;
            let finished = ReaderActionRepository::finish(&transaction, row.row_id, row.seq)?;
            match (&refusal, finished) {
                // Refused, with no tap since: the cache goes back to what it showed before.
                (Some(_), true) => {
                    if let Some(was) = row.was {
                        action.show(&transaction, self.owner_row, was)?;
                    }
                }
                // Agreed, and the user tapped again since: the server now has what was sent,
                // which is what the newer tap goes back to if it's refused.
                (None, false) => {
                    ReaderActionRepository::set_was(&transaction, row.row_id, row.wanted)?;
                }
                // Agreed with no tap since, so the row is done. Or refused with a tap since,
                // which is newer than what was refused, and is sent next.
                (None, true) | (Some(_), false) => {}
            }
            transaction.commit()?;
            Ok::<_, SqliteDbError>(finished)
        })?;
        if let (Some(error), true) = (refusal, finished) {
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
/// in the 400s won't change by asking again.
fn is_refused(error: &WpApiError) -> bool {
    /// Request timeout, too many requests.
    const TEMPORARY: [u32; 2] = [408, 429];
    match error {
        WpApiError::WpError { status_code, .. }
        | WpApiError::InvalidHttpStatusCode { status_code, .. }
        | WpApiError::UnknownError { status_code, .. } => {
            (400..500).contains(status_code) && !TEMPORARY.contains(status_code)
        }
        _ => false,
    }
}

#[uniffi::export]
impl ReaderService {
    /// Marks a post as seen or unseen. It shows straight away. Needs an account.
    ///
    /// A post has a seen state when the stream that sent it gave it one, which the Following
    /// stream does: [`wp_mobile_cache::db_types::reader::ReaderPostRecord::is_seen`] is `None`
    /// for a post that has none, and marking such a post is refused by the server.
    pub async fn set_post_seen(
        &self,
        post: ReaderPostSaveIdentity,
        seen: bool,
    ) -> Result<ReaderActionOutcome, ReaderError> {
        self.context.set_action(Action::Seen { post }, seen).await
    }

    /// Blocks or unblocks a site. The Reader's streams leave out a blocked site's posts, straight
    /// away. Needs an account.
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
    pub async fn refresh_blocked_sites(&self) -> Result<(), ReaderError> {
        let context = &self.context;
        if context.owner == ReaderOwner::SignedOut {
            return Err(ReaderError::NeedsAccount);
        }
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
        context.cache.execute(|connection| {
            let transaction = connection.transaction()?;
            let waiting: Vec<WpComSiteId> =
                ReaderActionRepository::select_all(&transaction, context.owner_row)?
                    .iter()
                    .filter_map(|row| serde_json::from_str::<Action>(&row.payload).ok())
                    .filter_map(|action| match action {
                        Action::BlockSite { site_id } => Some(site_id),
                        _ => None,
                    })
                    .collect();
            ReaderBlockRepository::replace_sites(
                &transaction,
                context.owner_row,
                &sites,
                &waiting,
            )?;
            transaction.commit().map_err(SqliteDbError::from)
        })?;
        Ok(())
    }

    /// Blocks or unblocks a user. The Reader's streams leave out a blocked user's posts.
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

    /// How many things the user turned on or off are waiting to be sent.
    pub fn pending_action_count(&self) -> Result<u64, ReaderError> {
        Ok(self.context.cache.execute(|connection| {
            ReaderActionRepository::count(connection, self.context.owner_row)
        })?)
    }
}

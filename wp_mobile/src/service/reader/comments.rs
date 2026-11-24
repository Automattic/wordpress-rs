//! A post's comments, kept in the cache.
//!
//! Comments are fetched newest first, a page at a time, and shown oldest first with each reply
//! pointing at the comment it answers. A comment the reader writes is kept as a row of its own
//! until the server takes it: see [`ReaderCommentsCollection::post_comment`].
//!
//! A user blocked with [`ReaderService::set_user_blocked`] has their comments hidden, with every
//! reply under them, at any depth: a reply answers something the reader chose not to see, and
//! without it the reply would read as an answer to nothing. They're hidden when read, not deleted,
//! so an unblock shows them again with no fetch. A comment written on this device that the server
//! hasn't taken stays shown, so it can be sent again or discarded. The server's count,
//! [`ReaderCommentsInfo::found`], still counts them, and [`ReaderCommentsInfo::hidden`] says how
//! many of the cached ones are hidden. (Neither WordPress app hides a blocked user's comments, so
//! there's no behaviour of theirs to match.)

use super::{ReaderContext, ReaderError, ReaderService};
use futures::lock::Mutex as AsyncMutex;
use std::sync::Arc;
use wp_api::comments::CommentId;
use wp_api::date::WpGmtDateTime;
use wp_api::posts::PostId;
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::reader_comments::{
    ReaderComment, ReaderCommentStatus, ReaderCommentType, ReaderCommentsOrder,
    ReaderCommentsParams, ReaderNewCommentParams,
};
use wp_api::wp_com::reader_users::{ReaderUserSuggestionsParams, ReaderUserSuggestionsResponse};
use wp_mobile_cache::list_metadata::ListState;
use wp_mobile_cache::repository::reader::ReaderPostRepository;
use wp_mobile_cache::repository::reader_comments::{
    ReaderCommentRecord, ReaderCommentRepository, ReaderCommentSendState,
    ReaderCommentThreadRepository,
};
use wp_mobile_cache::repository::wordpress_com_accounts::ReaderOwner;
use wp_mobile_cache::{DbTable, SqliteDbError, UpdateHook};

/// How many comments a page asks for, which is the most the API gives.
const COMMENTS_PAGE_SIZE: u32 = 100;

/// Sent with every request for comments. Without it a Jetpack site's comments come without their
/// likes.
const FORCE_WPCOM: &str = "wpcom";

/// The tables a post's comments are read from: the blocked users hide some.
const COMMENT_TABLES: [DbTable; 3] = [
    DbTable::ReaderComments,
    DbTable::ReaderCommentThreads,
    DbTable::ReaderBlockedUsers,
];

/// Why a comment written here wasn't taken, when its call was dropped before an answer arrived.
const SEND_STOPPED: &str = "The send stopped before an answer arrived.";

/// Marks a comment as not taken if the call sending it is dropped before an answer arrives, which
/// is how Kotlin cancels a call. The server may have taken it, so it isn't sent again by itself:
/// the reader sends it again or discards it, as with one left being sent when the app closed.
struct Unanswered<'a> {
    context: &'a ReaderContext,
    local_id: &'a str,
    answered: bool,
}

impl Drop for Unanswered<'_> {
    fn drop(&mut self) {
        if self.answered {
            return;
        }
        let marked = self.context.cache.execute(|connection| {
            ReaderCommentRepository::set_send_state(
                connection,
                self.context.owner_row,
                self.local_id,
                ReaderCommentSendState::Failed,
                Some(SEND_STOPPED),
            )
        });
        if let Err(error) = marked {
            log::error!("Couldn't mark a Reader comment whose send stopped as not taken: {error}");
        }
    }
}

/// Whether a post's comments are loading, and how many there are.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ReaderCommentsInfo {
    pub state: ListState,
    #[uniffi(default = None)]
    pub error_message: Option<String>,
    /// How many comments the server has, as of the newest page. `None` until a page loads. It
    /// counts the hidden ones.
    #[uniffi(default = None)]
    pub found: Option<u64>,
    /// How many of the cached comments are hidden because of a blocked user: the user's own, and
    /// the replies under them. [`ReaderCommentsCollection::load_items`] leaves them out.
    pub hidden: u64,
    /// Whether the server has comments the cache doesn't.
    pub has_more: bool,
    #[uniffi(default = None)]
    pub last_refreshed_at: Option<WpGmtDateTime>,
}

/// What loading a page of comments did.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ReaderCommentsPageResult {
    /// The comments the page had.
    pub comment_count: u64,
    /// Whether the server has comments the cache doesn't.
    pub has_more: bool,
}

pub(crate) fn comment_record(
    site_id: WpComSiteId,
    post_id: PostId,
    comment: &ReaderComment,
) -> ReaderCommentRecord {
    let author = comment.author.as_ref();
    ReaderCommentRecord {
        site_id,
        post_id: comment.post_id.unwrap_or(post_id),
        comment_id: Some(comment.id),
        local_id: None,
        parent_id: comment.parent_id,
        author_id: author.and_then(|author| author.id).map(|id| id.0),
        author_name: author.and_then(|author| author.name.clone()),
        author_avatar_url: author.and_then(|author| author.avatar_url.clone()),
        author_url: author.and_then(|author| author.url.clone()),
        date: comment.date,
        content: comment.content.clone(),
        status: comment.status.clone(),
        comment_type: comment.comment_type.clone(),
        like_count: comment.like_count,
        i_like: comment.i_like,
        send_state: ReaderCommentSendState::Sent,
        send_error: None,
    }
}

/// A post's comments, as the cache has them.
///
/// Read with [`Self::load_items`], which doesn't touch the network, and reload when
/// [`Self::is_relevant_update`] says a change to the cache is one it shows.
#[derive(uniffi::Object)]
pub struct ReaderCommentsCollection {
    pub(crate) context: Arc<ReaderContext>,
    pub(crate) site_id: WpComSiteId,
    pub(crate) post_id: PostId,
    /// Held while a page is fetched and stored, so two pages don't overlap.
    lane: AsyncMutex<()>,
}

#[uniffi::export]
impl ReaderCommentsCollection {
    /// The post's comments from the cache, oldest first, with the ones written on this device
    /// that the server hasn't taken. A reply's `parent_id` is the comment it answers, at any
    /// depth. A blocked user's comments, and the replies under them, are left out.
    pub async fn load_items(&self) -> Result<Vec<ReaderCommentRecord>, ReaderError> {
        Ok(self.context.cache.execute(|connection| {
            ReaderCommentRepository::select_post(
                connection,
                self.context.owner_row,
                self.site_id,
                self.post_id,
            )
        })?)
    }

    /// Whether the comments are loading, and how many there are.
    pub fn info(&self) -> Result<ReaderCommentsInfo, ReaderError> {
        let owner = self.context.owner_row;
        Ok(self.context.cache.execute(|connection| {
            let thread = ReaderCommentThreadRepository::select(
                &*connection,
                owner,
                self.site_id,
                self.post_id,
            )?;
            let stored = ReaderCommentRepository::count_sent(
                &*connection,
                owner,
                self.site_id,
                self.post_id,
            )?;
            let hidden = ReaderCommentRepository::count_hidden(
                &*connection,
                owner,
                self.site_id,
                self.post_id,
            )?;
            Ok::<_, SqliteDbError>(match thread {
                Some(thread) => ReaderCommentsInfo {
                    has_more: thread.found.is_some_and(|found| stored < found),
                    state: thread.state,
                    error_message: thread.error_message,
                    found: thread.found,
                    hidden,
                    last_refreshed_at: thread.last_refreshed_at,
                },
                None => ReaderCommentsInfo {
                    state: ListState::Idle,
                    error_message: None,
                    found: None,
                    hidden,
                    has_more: false,
                    last_refreshed_at: None,
                },
            })
        })?)
    }

    /// Fetches the newest comments. When they're all the post has, they replace what's cached, so
    /// a comment that was deleted goes. Otherwise they're added to it. A public site's comments
    /// need no account.
    pub async fn refresh(&self) -> Result<ReaderCommentsPageResult, ReaderError> {
        let _lane = self.lane.lock().await;
        self.load_page(0).await
    }

    /// Fetches the comments before the oldest the cache has, or refreshes if none have loaded.
    /// Does nothing when the cache has them all.
    pub async fn load_more(&self) -> Result<ReaderCommentsPageResult, ReaderError> {
        let _lane = self.lane.lock().await;
        let info = self.info()?;
        if info.found.is_some() && !info.has_more {
            return Ok(ReaderCommentsPageResult {
                comment_count: 0,
                has_more: false,
            });
        }
        let stored = self.context.cache.execute(|connection| {
            ReaderCommentRepository::count_sent(
                connection,
                self.context.owner_row,
                self.site_id,
                self.post_id,
            )
        })?;
        let offset = if info.found.is_some() { stored } else { 0 };
        self.load_page(offset as u32).await
    }

    /// Writes a comment on the post, or a reply to the comment `parent`. Needs an account.
    ///
    /// The comment shows among the post's comments straight away, as one being sent. It's sent
    /// once. When the server takes it, the server's comment replaces it. When it doesn't, the
    /// comment stays, marked as not taken with the reason, and the error is returned: nothing
    /// sends it again until [`Self::retry_comment`] is called, so a comment is never posted twice
    /// by itself. A call dropped before the answer leaves it marked as not taken the same way. It's
    /// sent once no other change of the Reader's is being sent: see
    /// [`ReaderService::set_tag_followed`](super::ReaderService::set_tag_followed).
    pub async fn post_comment(
        &self,
        content: String,
        parent: Option<CommentId>,
    ) -> Result<ReaderCommentRecord, ReaderError> {
        if self.context.owner == ReaderOwner::SignedOut {
            return Err(ReaderError::NeedsAccount);
        }
        if content.trim().is_empty() {
            return Err(ReaderError::Refused {
                reason: "A comment needs some text.".to_string(),
            });
        }
        let local_id = format!("local-{:032x}", rand::random::<u128>());
        let written = ReaderCommentRecord {
            site_id: self.site_id,
            post_id: self.post_id,
            comment_id: None,
            local_id: Some(local_id.clone()),
            parent_id: parent,
            author_id: None,
            author_name: None,
            author_avatar_url: None,
            author_url: None,
            date: Some(self.context.now()),
            content: Some(content),
            status: None,
            comment_type: Some(ReaderCommentType::Comment),
            like_count: None,
            i_like: None,
            send_state: ReaderCommentSendState::Sending,
            send_error: None,
        };
        self.context.cache.execute(|connection| {
            ReaderCommentRepository::upsert(connection, self.context.owner_row, &written)
        })?;
        self.send_comment(written).await
    }

    /// Sends again a comment that wasn't taken, by its
    /// [`ReaderCommentRecord::local_id`]. Returns the server's comment, as [`Self::post_comment`]
    /// does.
    pub async fn retry_comment(
        &self,
        local_id: String,
    ) -> Result<ReaderCommentRecord, ReaderError> {
        let owner = self.context.owner_row;
        let written = self.context.cache.execute(|connection| {
            let Some(written) =
                ReaderCommentRepository::select_local(&*connection, owner, &local_id)?
            else {
                return Ok::<_, SqliteDbError>(None);
            };
            if written.send_state == ReaderCommentSendState::Sending {
                // It's being sent already, and sending it again could post it twice.
                return Ok(None);
            }
            ReaderCommentRepository::set_send_state(
                &*connection,
                owner,
                &local_id,
                ReaderCommentSendState::Sending,
                None,
            )?;
            Ok(Some(written))
        })?;
        let written = written.ok_or(ReaderError::Refused {
            reason: "There's no comment waiting to be sent again.".to_string(),
        })?;
        self.send_comment(written).await
    }

    /// Discards a comment that wasn't taken, by its [`ReaderCommentRecord::local_id`]. Returns
    /// whether it was there. One that's being sent can't be discarded: it may already be posted.
    pub fn discard_comment(&self, local_id: String) -> Result<bool, ReaderError> {
        let owner = self.context.owner_row;
        Ok(self
            .context
            .cache
            .execute(|connection| {
                match ReaderCommentRepository::select_local(&*connection, owner, &local_id)? {
                    Some(comment) if comment.send_state == ReaderCommentSendState::Failed => {
                        ReaderCommentRepository::delete_local(&*connection, owner, &local_id)
                    }
                    _ => Ok::<_, SqliteDbError>(false),
                }
            })?)
    }

    /// The people of the post's site to suggest when the reader types `@` in a comment: the
    /// site's users, as both WordPress apps ask for them
    /// (`GET rest/v1.1/users/suggest?site_id=`, iOS's `SuggestionService`, Android's
    /// `SuggestionService.updateSuggestions`). Every one is fetched each time, and nothing is
    /// kept: filter them as the reader types, by the start of `user_login` or of `display_name`,
    /// in any case, which both apps do (iOS's `SuggestionsListViewModel`; Android's
    /// `SuggestionAdapter` also matches the start of any word of the name).
    ///
    /// The server only suggests the users of a site the account is a member of. For any other
    /// site it answers with no one, which isn't an error.
    ///
    /// Needs an account: signed out, the server answers with no one, so it fails with
    /// [`ReaderError::NeedsAccount`] and sends nothing.
    pub async fn mention_suggestions(&self) -> Result<ReaderUserSuggestionsResponse, ReaderError> {
        if self.context.owner == ReaderOwner::SignedOut {
            return Err(ReaderError::NeedsAccount);
        }
        let params = ReaderUserSuggestionsParams {
            site_id: Some(self.site_id),
            ..Default::default()
        };
        Ok(self
            .context
            .client
            .reader_users()
            .suggestions(&params)
            .await?
            .data)
    }

    /// Whether a change to the cache is one the comments show, a user blocked or unblocked among
    /// them.
    pub fn is_relevant_update(&self, hook: &UpdateHook) -> bool {
        COMMENT_TABLES.contains(&hook.table)
    }
}

impl ReaderCommentsCollection {
    pub(crate) fn new(context: Arc<ReaderContext>, site_id: WpComSiteId, post_id: PostId) -> Self {
        Self {
            context,
            site_id,
            post_id,
            lane: AsyncMutex::new(()),
        }
    }

    /// Sends a comment written here, once, and records what became of it.
    async fn send_comment(
        &self,
        written: ReaderCommentRecord,
    ) -> Result<ReaderCommentRecord, ReaderError> {
        let owner = self.context.owner_row;
        let local_id = written.local_id.clone().unwrap_or_default();
        let params = ReaderNewCommentParams {
            content: written.content.clone().unwrap_or_default(),
        };
        let comments = self.context.client.reader_comments();
        let mut unanswered = Unanswered {
            context: &self.context,
            local_id: &local_id,
            answered: false,
        };
        let sent = {
            // Sent in the lane the account's other changes are sent in, so it doesn't overlap
            // one: see `ReaderService::set_tag_followed`.
            let _lane = self.context.actions.hold_lane().await;
            match written.parent_id {
                Some(parent) => comments
                    .new_reply(&self.site_id, &parent, &params)
                    .await
                    .map(|response| response.data),
                None => comments
                    .new_comment(&self.site_id, &self.post_id, &params)
                    .await
                    .map(|response| response.data),
            }
        };
        unanswered.answered = true;
        match sent {
            Ok(comment) => {
                let taken = comment_record(self.site_id, self.post_id, &comment);
                self.context.cache.execute(|connection| {
                    let transaction = connection.transaction()?;
                    ReaderCommentRepository::delete_local(&transaction, owner, &local_id)?;
                    ReaderCommentRepository::upsert(&transaction, owner, &taken)?;
                    // The post has one more comment than the server last said.
                    let thread = ReaderCommentThreadRepository::get_or_create(
                        &transaction,
                        owner,
                        self.site_id,
                        self.post_id,
                    )?;
                    ReaderCommentThreadRepository::finish_page(
                        &transaction,
                        thread.row_id,
                        thread.found.map(|found| found + 1),
                        None,
                    )?;
                    // A post's count is of the comments anyone can read, so one held for a
                    // moderator isn't in it yet.
                    if taken.status == Some(ReaderCommentStatus::Approved) {
                        ReaderPostRepository::add_comment(
                            &transaction,
                            owner,
                            self.site_id,
                            self.post_id,
                        )?;
                    }
                    transaction.commit().map_err(SqliteDbError::from)
                })?;
                Ok(taken)
            }
            Err(error) => {
                let message = error.to_string();
                self.context.cache.execute(|connection| {
                    ReaderCommentRepository::set_send_state(
                        connection,
                        owner,
                        &local_id,
                        ReaderCommentSendState::Failed,
                        Some(&message),
                    )
                })?;
                Err(error.into())
            }
        }
    }

    fn set_state(&self, state: ListState, error: Option<&str>) -> Result<(), SqliteDbError> {
        self.context.cache.execute(|connection| {
            let thread = ReaderCommentThreadRepository::get_or_create(
                &*connection,
                self.context.owner_row,
                self.site_id,
                self.post_id,
            )?;
            ReaderCommentThreadRepository::set_state(&*connection, thread.row_id, state, error)
        })
    }

    /// Fetches the page of comments that starts `offset` comments from the newest, and stores it.
    async fn load_page(&self, offset: u32) -> Result<ReaderCommentsPageResult, ReaderError> {
        let first = offset == 0;
        self.set_state(
            if first {
                ListState::FetchingFirstPage
            } else {
                ListState::FetchingNextPage
            },
            None,
        )?;
        let result = self.fetch_and_store(offset).await;
        if let Err(error) = &result
            && let Err(record_error) = self.set_state(ListState::Error, Some(&error.to_string()))
        {
            // The page's error is the one to return, so a failure to record it is only logged.
            log::error!("Couldn't record the Reader comments' error: {record_error}");
        }
        result
    }

    async fn fetch_and_store(&self, offset: u32) -> Result<ReaderCommentsPageResult, ReaderError> {
        let params = ReaderCommentsParams {
            number: Some(COMMENTS_PAGE_SIZE),
            offset: (offset > 0).then_some(offset),
            order: Some(ReaderCommentsOrder::Desc),
            force: Some(FORCE_WPCOM.to_string()),
            ..Default::default()
        };
        let page = self
            .context
            .client
            .reader_comments()
            .list(&self.site_id, &self.post_id, &params)
            .await?
            .data;
        let comments: Vec<ReaderCommentRecord> = page
            .comments
            .iter()
            .map(|comment| comment_record(self.site_id, self.post_id, comment))
            .collect();
        let owner = self.context.owner_row;
        let now = self.context.now();
        let first = offset == 0;
        let has_more = self.context.cache.execute(|connection| {
            let transaction = connection.transaction()?;
            // The newest page holds every comment the post has, so it's the whole truth.
            let is_everything = first
                && page
                    .found
                    .is_some_and(|found| found <= comments.len() as u64);
            if is_everything {
                ReaderCommentRepository::replace_post(
                    &transaction,
                    owner,
                    self.site_id,
                    self.post_id,
                    &comments,
                )?;
            } else {
                for comment in &comments {
                    ReaderCommentRepository::upsert(&transaction, owner, comment)?;
                }
            }
            // A like the reader turned on or off since may be newer than the page.
            super::actions::show_pending(&transaction, owner)?;
            let thread = ReaderCommentThreadRepository::get_or_create(
                &transaction,
                owner,
                self.site_id,
                self.post_id,
            )?;
            // A page past the first with nothing on it means the cache has them all.
            let stored = ReaderCommentRepository::count_sent(
                &transaction,
                owner,
                self.site_id,
                self.post_id,
            )?;
            let found = match page.found {
                Some(found) if !first && comments.is_empty() => Some(found.min(stored)),
                Some(found) => Some(found),
                None if comments.is_empty() => Some(stored),
                None => None,
            };
            ReaderCommentThreadRepository::finish_page(
                &transaction,
                thread.row_id,
                found,
                first.then_some(&now),
            )?;
            transaction.commit()?;
            Ok::<_, SqliteDbError>(found.is_some_and(|found| stored < found))
        })?;
        Ok(ReaderCommentsPageResult {
            comment_count: comments.len() as u64,
            has_more,
        })
    }
}

#[uniffi::export]
impl ReaderService {
    /// A post's comments, as the cache has them. Getting it doesn't touch the network. Only a
    /// post of a WordPress.com or Jetpack site has comments the Reader can show.
    pub fn comments(&self, site_id: WpComSiteId, post_id: PostId) -> Arc<ReaderCommentsCollection> {
        Arc::new(ReaderCommentsCollection::new(
            self.context.clone(),
            site_id,
            post_id,
        ))
    }
}

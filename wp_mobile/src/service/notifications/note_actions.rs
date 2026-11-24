//! Liking the post or comment a note is about, and following a site a note offers.
//!
//! These go through the account's Reader action queue, so a like made from a note and a like
//! made in the Reader of the same post are one like, with one row waiting and one request.
//!
//! - While a like or follow waits to be sent, whoever made it, every cached note about the same
//!   post, comment or site shows it: [`NotificationService::load_notes`] puts it over the note's
//!   own state, so a sync that stores the server's older copy doesn't undo it.
//! - When the server takes it, the stored notes about it keep it, so they still show it once the
//!   row goes, until the next sync brings the server's own copy.
//! - When the server refuses it for good, the row goes and nothing was stored, so the notes show
//!   the server's state again, and the call fails with
//!   [`NotificationServiceError::ActionRefused`].
//!
//! Which IDs each action uses follows the WordPress apps and the web notifications panel: a like
//! is of the note's own post (`meta.ids.site` and `meta.ids.post`) or comment (`meta.ids.site`
//! and `meta.ids.comment`), and the note offers it when a body block has the `like-post` or
//! `like-comment` action. A follow is of the site of a body block that has the `follow` action:
//! the block's `meta.ids.site`, followed by its `meta.links.home`.

use super::{NotificationService, NotificationServiceError};
use crate::service::reader::{ReaderActionOutcome, ReaderError};
use serde_json::Value;
use wp_api::comments::CommentId;
use wp_api::posts::PostId;
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::notifications::{
    NotificationActionKind, NotificationBlock, NotificationId, WpComNotification,
};
use wp_localization::{WpLocale, WpMessages};
use wp_mobile_cache::repository::QueryExecutor;
use wp_mobile_cache::repository::notifications::NotificationRepository;
use wp_mobile_cache::{RowId, SqliteDbError};

/// What a like or follow a note can show is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NoteActionTarget {
    Post {
        site_id: WpComSiteId,
        post_id: PostId,
    },
    Comment {
        site_id: WpComSiteId,
        comment_id: CommentId,
    },
    Site {
        site_id: WpComSiteId,
    },
}

impl NoteActionTarget {
    /// The name of the note action that shows it, as the API sends it.
    fn action_name(&self) -> &'static str {
        match self {
            Self::Post { .. } => "like-post",
            Self::Comment { .. } => "like-comment",
            Self::Site { .. } => "follow",
        }
    }
}

/// A change the user makes from a note to what it's about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum NotificationNoteChange {
    LikePost,
    UnlikePost,
    LikeComment,
    UnlikeComment,
    FollowSite,
    UnfollowSite,
}

impl NotificationNoteChange {
    fn new(target: &NoteActionTarget, on: bool) -> Self {
        match (target, on) {
            (NoteActionTarget::Post { .. }, true) => Self::LikePost,
            (NoteActionTarget::Post { .. }, false) => Self::UnlikePost,
            (NoteActionTarget::Comment { .. }, true) => Self::LikeComment,
            (NoteActionTarget::Comment { .. }, false) => Self::UnlikeComment,
            (NoteActionTarget::Site { .. }, true) => Self::FollowSite,
            (NoteActionTarget::Site { .. }, false) => Self::UnfollowSite,
        }
    }

    /// What the user is told when the server refuses it, in `locale`.
    fn refused_message(self, locale: Option<WpLocale>) -> String {
        match self {
            Self::LikePost => WpMessages::notification_like_post_refused(),
            Self::UnlikePost => WpMessages::notification_unlike_post_refused(),
            Self::LikeComment => WpMessages::notification_like_comment_refused(),
            Self::UnlikeComment => WpMessages::notification_unlike_comment_refused(),
            Self::FollowSite => WpMessages::notification_follow_site_refused(),
            Self::UnfollowSite => WpMessages::notification_unfollow_site_refused(),
        }
        .localize(locale)
    }
}

/// Returns the message to show the user for `error`, in `locale`, or `None` for an error only a
/// developer should see, such as a note that isn't cached or doesn't offer the action.
///
/// Today only [`NotificationServiceError::ActionRefused`] has one: it says the like or follow
/// didn't happen, in plain words. The error's `error` field has the server's reason.
#[uniffi::export]
pub fn notification_service_error_user_message(
    error: &NotificationServiceError,
    locale: Option<WpLocale>,
) -> Option<String> {
    match error {
        NotificationServiceError::ActionRefused { change, .. } => {
            Some(change.refused_message(locale))
        }
        _ => None,
    }
}

/// A numeric ID as the API sends it: a number, or a number in a string.
fn value_id(value: &Value) -> Option<u64> {
    match value {
        Value::Number(number) => number.as_u64(),
        Value::String(text) => text.parse().ok(),
        _ => None,
    }
    .filter(|id| *id != 0)
}

/// What each like or follow action in a note's JSON is about.
fn action_targets(note: &Value) -> Vec<(usize, NoteActionTarget)> {
    let ids = &note["meta"]["ids"];
    let site = value_id(&ids["site"]).map(WpComSiteId);
    let post = value_id(&ids["post"]).and_then(|id| i64::try_from(id).ok().map(PostId));
    let comment = value_id(&ids["comment"]).and_then(|id| i64::try_from(id).ok().map(CommentId));
    let mut targets = Vec::new();
    let Some(body) = note["body"].as_array() else {
        return targets;
    };
    for (index, block) in body.iter().enumerate() {
        let Some(actions) = block["actions"].as_object() else {
            continue;
        };
        if let (true, Some(site_id), Some(post_id)) =
            (actions.contains_key("like-post"), site, post)
        {
            targets.push((index, NoteActionTarget::Post { site_id, post_id }));
        }
        if let (true, Some(site_id), Some(comment_id)) =
            (actions.contains_key("like-comment"), site, comment)
        {
            targets.push((
                index,
                NoteActionTarget::Comment {
                    site_id,
                    comment_id,
                },
            ));
        }
        if let (true, Some(site_id)) = (
            actions.contains_key("follow"),
            value_id(&block["meta"]["ids"]["site"]).map(WpComSiteId),
        ) {
            targets.push((index, NoteActionTarget::Site { site_id }));
        }
    }
    targets
}

/// Sets each like or follow action in a note's JSON that `states` has to the state it has there.
/// Returns whether it changed anything.
fn apply(note: &mut Value, states: &[(NoteActionTarget, bool)]) -> bool {
    let mut changed = false;
    for (index, target) in action_targets(note) {
        // The newest wins: rows come oldest first.
        if let Some((_, on)) = states.iter().rev().find(|(waiting, _)| *waiting == target) {
            let action = &mut note["body"][index]["actions"][target.action_name()];
            if action.as_bool() != Some(*on) {
                *action = Value::Bool(*on);
                changed = true;
            }
        }
    }
    changed
}

/// The likes and follows waiting to be sent, to show on cached notes as they're read.
pub(crate) struct WaitingNoteActions(pub(crate) Vec<(NoteActionTarget, bool)>);

impl WaitingNoteActions {
    /// Reads a stored note, with what's waiting shown over its own state.
    pub(crate) fn read(&self, json: &str) -> serde_json::Result<WpComNotification> {
        if self.0.is_empty() {
            return serde_json::from_str(json);
        }
        let mut note: Value = serde_json::from_str(json)?;
        apply(&mut note, &self.0);
        serde_json::from_value(note)
    }
}

/// Stores `on` as the state of `target` in every stored note of `account` that shows it: the
/// server has it now.
pub(crate) fn store_note_action(
    executor: &impl QueryExecutor,
    account: RowId,
    target: &NoteActionTarget,
    on: bool,
) -> Result<(), SqliteDbError> {
    let quoted = format!("\"{}\"", target.action_name());
    for (note_id, json) in
        NotificationRepository::select_json_containing(executor, account, &quoted)?
    {
        let Ok(mut note) = serde_json::from_str::<Value>(&json) else {
            continue;
        };
        if apply(&mut note, &[(*target, on)]) {
            NotificationRepository::set_json(executor, account, note_id, &note.to_string())?;
        }
    }
    Ok(())
}

/// The body blocks with an action of `kind`.
fn blocks_with<'a>(
    note: &'a WpComNotification,
    kind: &NotificationActionKind,
) -> impl Iterator<Item = &'a NotificationBlock> + 'a {
    let kind = kind.clone();
    note.body
        .iter()
        .filter(move |block| block.actions.iter().any(|action| action.kind == kind))
}

#[uniffi::export]
impl NotificationService {
    /// Likes or unlikes the post the note is about. Every cached note about the post shows it at
    /// once, and so does the Reader's cached post. It's sent with the Reader's other changes:
    /// see [`crate::service::reader::ReaderService::sync_actions`] for one that can't be sent yet.
    ///
    /// Returns [`ReaderActionOutcome::Pending`] when it couldn't be sent yet. Fails with
    /// [`NotificationServiceError::NoteNotCached`] or
    /// [`NotificationServiceError::ActionNotOffered`] without sending anything, and with
    /// [`NotificationServiceError::ActionRefused`] when the server refuses it.
    pub async fn set_post_liked(
        &self,
        note_id: NotificationId,
        liked: bool,
    ) -> Result<ReaderActionOutcome, NotificationServiceError> {
        let note = self.cached_note(note_id)?;
        let ids = note.meta.as_ref().and_then(|meta| meta.ids.as_ref());
        let target = match (
            ids.and_then(|ids| ids.site).filter(|site| site.0 != 0),
            ids.and_then(|ids| ids.post).filter(|post| post.0 > 0),
            blocks_with(&note, &NotificationActionKind::LikePost).next(),
        ) {
            (Some(site_id), Some(post_id), Some(_)) => NoteActionTarget::Post { site_id, post_id },
            _ => {
                return Err(NotificationServiceError::ActionNotOffered {
                    change: if liked {
                        NotificationNoteChange::LikePost
                    } else {
                        NotificationNoteChange::UnlikePost
                    },
                });
            }
        };
        self.send_note_action(target, None, None, liked).await
    }

    /// Likes or unlikes the comment the note is about, as [`Self::set_post_liked`] likes its post.
    pub async fn set_comment_liked(
        &self,
        note_id: NotificationId,
        liked: bool,
    ) -> Result<ReaderActionOutcome, NotificationServiceError> {
        let note = self.cached_note(note_id)?;
        let ids = note.meta.as_ref().and_then(|meta| meta.ids.as_ref());
        let target = match (
            ids.and_then(|ids| ids.site).filter(|site| site.0 != 0),
            ids.and_then(|ids| ids.comment)
                .filter(|comment| comment.0 > 0),
            blocks_with(&note, &NotificationActionKind::LikeComment).next(),
        ) {
            (Some(site_id), Some(comment_id), Some(_)) => NoteActionTarget::Comment {
                site_id,
                comment_id,
            },
            _ => {
                return Err(NotificationServiceError::ActionNotOffered {
                    change: if liked {
                        NotificationNoteChange::LikeComment
                    } else {
                        NotificationNoteChange::UnlikeComment
                    },
                });
            }
        };
        self.send_note_action(target, None, None, liked).await
    }

    /// Follows or unfollows `site_id`, a site the note offers to follow: the site of a body block
    /// with the `follow` action, such as the one in the note's
    /// [`wp_api::wp_com::notifications::NotificationFollowOffer`]. Every cached note that offers
    /// it shows it at once, and so does what the Reader caches of the site. A follow is sent by
    /// the block's site address. Otherwise as [`Self::set_post_liked`].
    pub async fn set_site_followed(
        &self,
        note_id: NotificationId,
        site_id: WpComSiteId,
        followed: bool,
    ) -> Result<ReaderActionOutcome, NotificationServiceError> {
        let note = self.cached_note(note_id)?;
        let target = NoteActionTarget::Site { site_id };
        let Some(meta) = blocks_with(&note, &NotificationActionKind::Follow)
            .filter_map(|block| block.meta.as_ref())
            .find(|meta| meta.ids.as_ref().and_then(|ids| ids.site) == Some(site_id))
        else {
            return Err(NotificationServiceError::ActionNotOffered {
                change: NotificationNoteChange::new(&target, followed),
            });
        };
        let url = meta.links.as_ref().and_then(|links| links.home.clone());
        let name = meta.titles.as_ref().and_then(|titles| titles.home.clone());
        self.send_note_action(target, url, name, followed).await
    }
}

impl NotificationService {
    /// The cached note, read as [`Self::load_notes`] reads it.
    fn cached_note(
        &self,
        note_id: NotificationId,
    ) -> Result<WpComNotification, NotificationServiceError> {
        let (row, waiting) = self.cache.execute(|connection| {
            Ok::<_, SqliteDbError>((
                NotificationRepository::select_one(&*connection, self.account, note_id)?,
                crate::service::reader::waiting_note_actions(&*connection, self.account)?,
            ))
        })?;
        row.and_then(|row| WaitingNoteActions(waiting).read(&row.json).ok())
            .ok_or(NotificationServiceError::NoteNotCached { note_id })
    }

    async fn send_note_action(
        &self,
        target: NoteActionTarget,
        url: Option<String>,
        name: Option<String>,
        on: bool,
    ) -> Result<ReaderActionOutcome, NotificationServiceError> {
        match self.reader.set_note_action(target, url, name, on).await {
            Ok(outcome) => Ok(outcome),
            Err(ReaderError::Database(error)) => Err(NotificationServiceError::Database(error)),
            Err(error) => Err(NotificationServiceError::ActionRefused {
                change: NotificationNoteChange::new(&target, on),
                error,
            }),
        }
    }
}

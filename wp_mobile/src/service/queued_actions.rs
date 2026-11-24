//! Events about what waits in the queues of a database's WordPress.com accounts: the likes,
//! follows and other changes the user turned on or off, which are shown at once and sent when
//! they can be.
//!
//! An event says that a change started waiting, that the server took it, or that the server
//! refused it. The last matters most: a change made offline and refused later has no call left to
//! fail, so an event is the only way the app hears of it.
//!
//! Listeners are the database's. A listener added through one [`WpApiCache`] object hears the
//! changes made through every object of the same database file in the process. A database with
//! no file, such as one in memory, has listeners of its own.
//!
//! Events are in memory only: one sent before a listener is added isn't sent to it later. A change
//! still waiting from before a restart sends no new `Waiting` event, though its answer sends one.
//! Marking notifications read or seen sends none.

use crate::service::notifications::{NoteActionKind, NotificationNoteChange};
use crate::service::reader::ReaderError;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use wp_api::comments::CommentId;
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::me::WpComUserId;
use wp_api::wp_com::reader_posts::ReaderTagSlug;
use wp_api::wp_com::reader_saved::{ReaderFeedId, ReaderPostSaveIdentity};
use wp_localization::WpLocale;
use wp_mobile_cache::WpApiCache;

/// Hears of every change to what waits in the queues of a database's accounts.
///
/// It's called on the thread that made the change, while the account's changes are being sent,
/// so it must return quickly and must not block: hand the event to another thread or queue to do
/// more. It may call back into the library, such as to count what's waiting.
#[uniffi::export(with_foreign)]
pub trait QueuedActionListener: Send + Sync {
    fn queued_action_changed(&self, event: QueuedActionEvent);
}

/// A change to something the user turned on or off.
#[derive(Debug, Clone, uniffi::Record)]
pub struct QueuedActionEvent {
    /// The account whose queue it's in.
    pub user_id: WpComUserId,
    /// What the user turned on or off.
    pub action: QueuedAction,
    /// Whether the user turned it on, or off.
    pub on: bool,
    pub state: QueuedActionState,
}

/// What became of a change the user made.
#[allow(clippy::large_enum_variant)] // Boxing doesn't cross the FFI boundary.
#[derive(Debug, Clone, uniffi::Enum)]
pub enum QueuedActionState {
    /// It's shown, and waits to be sent.
    Waiting,
    /// The server has it, and nothing waits for it.
    Sent,
    /// The server refused it for good, and nothing waits for it. What the user sees went back to
    /// what it showed before, or to what the server answered. `error` says why.
    Refused { error: ReaderError },
}

/// Something the user turns on or off, with the IDs that say what it's about.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum QueuedAction {
    /// A post is marked seen, or unseen.
    PostSeen { post: ReaderPostSaveIdentity },
    /// A site is blocked, or unblocked.
    SiteBlock { site_id: WpComSiteId },
    /// A post is liked, or unliked.
    PostLike { post: ReaderPostSaveIdentity },
    /// A site or feed is followed, or unfollowed: by its ID, or by its address when the app gave
    /// no ID.
    Follow {
        site_id: Option<WpComSiteId>,
        feed_id: Option<ReaderFeedId>,
        url: Option<String>,
    },
    /// A tag is followed, or unfollowed.
    TagFollow { slug: ReaderTagSlug },
    /// A comment is liked, or unliked.
    CommentLike {
        site_id: WpComSiteId,
        comment_id: CommentId,
    },
    /// A followed site's new posts are emailed, or not.
    PostEmails { site_id: WpComSiteId },
    /// A followed site's new comments are emailed, or not.
    CommentEmails { site_id: WpComSiteId },
    /// A followed site's new posts send a notification, or not.
    PostNotifications { site_id: WpComSiteId },
    /// The user is subscribed to a post's comments, or not.
    CommentsSubscription { post: ReaderPostSaveIdentity },
    /// A post's comments the user is subscribed to send notifications, or not.
    CommentNotifications { post: ReaderPostSaveIdentity },
}

impl QueuedAction {
    /// The kind of like or follow a note can show, which has a message for the user.
    fn note_action_kind(&self) -> Option<NoteActionKind> {
        match self {
            Self::PostLike { .. } => Some(NoteActionKind::PostLike),
            Self::CommentLike { .. } => Some(NoteActionKind::CommentLike),
            Self::Follow { .. } => Some(NoteActionKind::Follow),
            _ => None,
        }
    }
}

/// Starts sending `listener` every [`QueuedActionEvent`] of `cache`'s database, and returns the
/// ID that removes it.
#[uniffi::export]
pub fn add_queued_action_listener(
    cache: Arc<WpApiCache>,
    listener: Arc<dyn QueuedActionListener>,
) -> u64 {
    cache
        .database_shared_state::<QueuedActionListeners>()
        .add(listener)
}

/// Stops sending events to the listener that `listener_id` names. Returns `false` when there's no
/// such listener.
#[uniffi::export]
pub fn remove_queued_action_listener(cache: Arc<WpApiCache>, listener_id: u64) -> bool {
    cache
        .database_shared_state::<QueuedActionListeners>()
        .remove(listener_id)
}

/// Returns the message to show the user for `event`, in `locale`, or `None` when there's nothing
/// to tell them.
///
/// Only a refused like of a post or a comment, or a refused follow or unfollow, has one: it says
/// the change didn't happen, in plain words. The event's error has the server's reason.
#[uniffi::export]
pub fn queued_action_event_user_message(
    event: &QueuedActionEvent,
    locale: Option<WpLocale>,
) -> Option<String> {
    match event.state {
        QueuedActionState::Refused { .. } => event
            .action
            .note_action_kind()
            .map(|kind| NotificationNoteChange::of(kind, event.on).refused_message(locale)),
        _ => None,
    }
}

/// The listeners of one database.
#[derive(Default)]
pub(crate) struct QueuedActionListeners {
    listeners: Mutex<Vec<(u64, Arc<dyn QueuedActionListener>)>>,
    last_id: AtomicU64,
}

impl QueuedActionListeners {
    fn add(&self, listener: Arc<dyn QueuedActionListener>) -> u64 {
        let id = self.last_id.fetch_add(1, Ordering::Relaxed) + 1;
        self.listeners
            .lock()
            .expect("listeners lock poisoned")
            .push((id, listener));
        id
    }

    fn remove(&self, id: u64) -> bool {
        let mut listeners = self.listeners.lock().expect("listeners lock poisoned");
        let count = listeners.len();
        listeners.retain(|(listener_id, _)| *listener_id != id);
        listeners.len() != count
    }

    /// Sends `event` to every listener. Call it holding no lock: a listener may call back into
    /// the library.
    pub(crate) fn send(&self, event: QueuedActionEvent) {
        let listeners: Vec<_> = self
            .listeners
            .lock()
            .expect("listeners lock poisoned")
            .iter()
            .map(|(_, listener)| Arc::clone(listener))
            .collect();
        for listener in listeners {
            listener.queued_action_changed(event.clone());
        }
    }
}

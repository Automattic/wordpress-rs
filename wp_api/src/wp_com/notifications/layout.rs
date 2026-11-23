//! How to show a note: one of five layouts, built from the note context the way the web
//! notifications panel builds them.

use super::{
    NotificationActor, NotificationBlock, NotificationBlockKind, NotificationIds, NotificationMeta,
    NotificationParentComment, NotificationPostContext, NotificationRangeKind,
    NotificationSiteContext, NotificationText, NotificationType, WpComNotification,
    styled_runs::notification_styled_text,
};
use crate::{
    date::WpGmtDateTime,
    wp_com::{WpComSiteId, styled_text::StyledTextRun},
};
use wp_localization::{MessageBundle, WpMessages, WpSupportsLocalization};
use wp_localization_macro::WpDeriveLocalizable;

/// The Noticons characters the web panel reads to tell a fediverse like from a repost.
const STAR_NOTICON: &str = "\u{f408}";
const REBLOG_NOTICON: &str = "\u{f805}";

/// Picks the layout for a note and builds it, as the web notifications panel does.
///
/// A note needs its note context (`include_context` when it was fetched) to get one of the
/// four special layouts: without the person it's from, it's shown as a
/// [`NotificationLayoutContent::Notice`] built from its header and body, and
/// [`NotificationLayout::fallback`] says why. This never fails.
#[uniffi::export]
pub fn notification_layout(note: &WpComNotification) -> NotificationLayout {
    let preview = preview(note);
    let meta = note.meta.as_ref();
    let Some(actor) = meta.and_then(|meta| meta.actor.as_ref()) else {
        return NotificationLayout {
            title: note.title.clone().map(NotificationLayoutText::from),
            preview,
            content: NotificationLayoutContent::Notice {
                layout: notice_from_blocks(note),
            },
            fallback: Some(NotificationLayoutFallback::NoContext),
        };
    };
    let meta = meta.expect("a note with an actor has meta");
    let title = context_title(note);
    let mut fallback = None;
    let content = match &note.note_type {
        Some(NotificationType::Comment) => NotificationLayoutContent::Comment {
            layout: comment(note, meta, actor),
        },
        Some(NotificationType::Automattcher) => {
            let mentions_post = meta.post.is_some()
                && meta.site.is_some()
                && meta.ids.as_ref().and_then(|ids| ids.comment).is_none();
            if mentions_post {
                NotificationLayoutContent::NewPost {
                    layout: new_post(note, meta),
                }
            } else {
                NotificationLayoutContent::Comment {
                    layout: comment(note, meta, actor),
                }
            }
        }
        Some(NotificationType::Like | NotificationType::CommentLike | NotificationType::Reblog) => {
            NotificationLayoutContent::Reactions {
                layout: reactions(note, meta, &title),
            }
        }
        Some(
            NotificationType::Follow
            | NotificationType::ActivityPubFollowed
            | NotificationType::ActivityPubInteraction,
        ) => NotificationLayoutContent::Follow {
            layout: NotificationFollowLayout {
                site: header_site(&note.header),
                reactions: reactions(note, meta, &title),
            },
        },
        Some(NotificationType::NewPost) => NotificationLayoutContent::NewPost {
            layout: new_post(note, meta),
        },
        _ => {
            fallback = Some(NotificationLayoutFallback::NoLayoutForType);
            NotificationLayoutContent::Notice {
                layout: notice(note, meta),
            }
        }
    };
    NotificationLayout {
        title,
        preview,
        content,
        fallback,
    }
}

/// How to show a note. Build it with [`notification_layout`].
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct NotificationLayout {
    /// The title above the note, such as "12 likes on your post". `None` when the note has no
    /// title.
    pub title: Option<NotificationLayoutText>,
    /// A line to show under the note in a list: the comment, the liked comment, or the post's
    /// title. Text the API wrote.
    pub preview: Option<String>,
    pub content: NotificationLayoutContent,
    /// Why the note is shown as a notice rather than in a layout of its own. `None` for a note
    /// in its own layout.
    pub fallback: Option<NotificationLayoutFallback>,
}

/// The note's content, in one of five layouts.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum NotificationLayoutContent {
    /// A comment, under the comment it replies to. Comments, and mentions in a comment.
    Comment { layout: NotificationCommentLayout },
    /// People who liked or reblogged a post or comment.
    Reactions { layout: NotificationReactionsLayout },
    /// People who subscribed to a site, or followed or interacted from the fediverse.
    Follow { layout: NotificationFollowLayout },
    /// A post: a new post on a site the user follows, or a post that mentions them.
    NewPost { layout: NotificationNewPostLayout },
    /// Anything else: a headline over the note's body.
    Notice { layout: NotificationNoticeLayout },
}

/// Why a note is shown as a [`NotificationLayoutContent::Notice`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum NotificationLayoutFallback {
    /// The note has no note context: it wasn't asked for, or the note has nobody it's from.
    /// The notice is the note's header and its whole body.
    NoContext,
    /// The note's type has no layout of its own.
    NoLayoutForType,
}

/// Text to show: either what the API wrote, or a label the library wrote.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum NotificationLayoutText {
    /// Text the API wrote, as it sent it.
    Text { text: String },
    /// A label to show in the user's language: pass it to `localize_notification_label`.
    Label { label: NotificationLabel },
}

impl From<String> for NotificationLayoutText {
    fn from(text: String) -> Self {
        Self::Text { text }
    }
}

impl From<NotificationLabel> for NotificationLayoutText {
    fn from(label: NotificationLabel) -> Self {
        Self::Label { label }
    }
}

/// A comment, under the comment it replies to.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct NotificationCommentLayout {
    /// The post the comment is on, as a card above the thread.
    pub post: Option<NotificationPostCard>,
    /// The comment this one replies to, shown short above it.
    pub replied_to: Option<NotificationQuotedComment>,
    /// Who wrote the comment.
    pub commenter: NotificationActor,
    /// The time and follow offer beside the commenter's name.
    pub meta_line: NotificationMetaLine,
    /// A badge beside the commenter, such as "Pending" for a comment waiting for approval.
    pub status: Option<NotificationLabel>,
    /// The comment itself, without the blocks about people.
    pub body: Vec<NotificationLayoutBlock>,
}

/// People who reacted to a post or comment.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct NotificationReactionsLayout {
    /// The post they reacted to, as a card.
    pub post: Option<NotificationPostCard>,
    /// The comment they reacted to.
    pub comment: Option<NotificationQuotedComment>,
    /// Whether the body is a list of people, to space evenly. For likes and subscribers, whose
    /// count is in the title.
    pub is_people_list: bool,
    /// A heading over the body. Text the API wrote. Never set for a list of people.
    pub heading: Option<String>,
    /// The people, and what the API wrote about them. Without the post's block when there's a
    /// card for it, and without the text blocks when [`Self::others`] is set.
    pub body: Vec<NotificationLayoutBlock>,
    /// A line for the people the body doesn't list: the API lists ten at most.
    pub others: Option<NotificationOthers>,
}

/// The people a list leaves out.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct NotificationOthers {
    /// How many people aren't listed.
    pub count: u64,
    /// The line to show, such as "And 3 others liked your post.".
    pub label: NotificationLabel,
    /// Where the line leads: the page that lists everyone.
    pub url: Option<String>,
}

/// People who subscribed or followed, under the site they did it on.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct NotificationFollowLayout {
    /// The site, as a card above the people.
    pub site: Option<NotificationSiteCard>,
    pub reactions: NotificationReactionsLayout,
}

/// A post.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct NotificationNewPostLayout {
    /// What's above the post's title: its site and author, or a headline.
    pub byline: Option<NotificationNewPostByline>,
    /// The post's title, linking to it.
    pub post: Option<NotificationPostLink>,
    /// The post's content, without the block about its author when only one person is behind
    /// the note.
    pub body: Vec<NotificationLayoutBlock>,
}

/// What's above a post's title.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum NotificationNewPostByline {
    /// The site, linking to [`NotificationSiteContext::url`], with the author and time below it.
    Site {
        site: NotificationSiteContext,
        /// Who wrote the post. `None` when the API sent no name.
        author: Option<NotificationActor>,
        meta_line: NotificationMetaLine,
    },
    /// The note's headline, when it has no post or no site.
    Headline { headline: NotificationHeadline },
}

/// A post's title, linking to the post.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct NotificationPostLink {
    /// Text the API wrote. It can be missing or empty.
    pub title: Option<String>,
    pub url: Option<String>,
}

/// A headline over a note's body, the post or comment it's about, and the body.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct NotificationNoticeLayout {
    pub headline: Option<NotificationHeadline>,
    /// The post the note is about, as a card.
    pub post: Option<NotificationPostCard>,
    /// The comment the note is about.
    pub comment: Option<NotificationQuotedComment>,
    /// The note's body. For a note with note context, without the block about the person when
    /// only one person is behind the note, and without the post's block when there's a card
    /// for it.
    pub body: Vec<NotificationLayoutBlock>,
}

/// The line at the top of a note.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum NotificationHeadline {
    /// The note context's one-line summary, beside the note's icon.
    Summary {
        /// The note's icon, usually the avatar of who it's from.
        icon: Option<String>,
        /// Where the icon leads: the person's website.
        actor_url: Option<String>,
        /// The summary, which the API wrote, with its people and posts linked.
        summary: Vec<StyledTextRun>,
        meta_line: NotificationMetaLine,
        /// A badge beside it, such as "Pending".
        status: Option<NotificationLabel>,
    },
    /// A card for the site the note's header names.
    Site { card: NotificationSiteCard },
    /// The note's header lines, as the API wrote them.
    Header { lines: Vec<Vec<StyledTextRun>> },
}

/// The time a note happened, and an offer to follow the one person behind it.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct NotificationMetaLine {
    /// When the note happened. A time at the very start of 1970 means the note has no date.
    pub timestamp: Option<WpGmtDateTime>,
    /// Where the time leads.
    pub url: Option<String>,
    /// Set when one person is behind the note and the note offers to follow their site.
    pub follow: Option<NotificationFollowOffer>,
}

/// An offer to follow a person's site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct NotificationFollowOffer {
    pub site_id: WpComSiteId,
    /// Whether the user follows the site already.
    pub is_following: bool,
}

/// A card for the post a note is about, linking to it.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct NotificationPostCard {
    /// The post's title, or the "No title" label when it has none.
    pub title: NotificationLayoutText,
    pub url: Option<String>,
    /// The post's site, shown with the title. When it's `None`, the card is only the title,
    /// and has no author or date.
    pub site: Option<NotificationSiteContext>,
    pub author_name: Option<String>,
    pub author_avatar: Option<String>,
    /// When the post was published.
    pub date: Option<WpGmtDateTime>,
}

/// A card for a site, from the first line of a note's header.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct NotificationSiteCard {
    /// The site's name.
    pub name: String,
    pub icon: Option<String>,
    /// The site's address. The card leads here, or to the post when there is one.
    pub url: String,
    /// A post on the site the header's second line names, shown under the site.
    pub post: Option<NotificationPostLink>,
}

/// A comment shown above or beside a note: the one a reply answers, or the one people liked.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct NotificationQuotedComment {
    pub author_name: Option<String>,
    pub author_avatar: Option<String>,
    pub author_url: Option<String>,
    /// When the comment was written.
    pub date: Option<WpGmtDateTime>,
    pub url: Option<String>,
    /// The comment, which its author wrote, whole.
    pub content: Vec<StyledTextRun>,
    /// How much of the comment to show.
    pub style: NotificationQuoteStyle,
    /// A label for screen readers, such as "In reply to Sam".
    pub accessibility_label: Option<NotificationLabel>,
}

/// How much of a quoted comment to show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum NotificationQuoteStyle {
    /// A small preview on one or two lines, with its author's name and time on one line. The
    /// web panel leaves out the text the comment itself quotes, unless that's all it has.
    Preview,
    /// The comment, cut at a word after about 300 characters.
    Excerpt,
}

/// A block of a note's body, with what it is and its text as runs.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct NotificationLayoutBlock {
    pub role: NotificationBlockRole,
    pub block: NotificationBlock,
    /// The block's text as runs to show.
    pub runs: Vec<StyledTextRun>,
}

/// What a body block is, by what it refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum NotificationBlockRole {
    /// Text.
    Text,
    /// The user's own reply to the comment.
    Reply,
    /// A blogging prompt.
    Prompt,
    Comment,
    Post,
    /// A person.
    User,
    /// A block of a type this version doesn't know.
    Other,
}

/// A label the library writes for a note, in the words of the web notifications panel.
///
/// Show it with `localize_notification_label(value, locale)`, which returns it in the given
/// language, or English when there's no translation yet.
#[derive(Debug, Clone, PartialEq, Eq, Hash, uniffi::Enum, WpDeriveLocalizable)]
pub enum NotificationLabel {
    /// "Likes".
    Likes,
    /// "12 likes on your comment".
    LikesOnYourComment { count: u64 },
    /// "12 likes on your post".
    LikesOnYourPost { count: u64 },
    /// "12 people subscribed".
    PeopleSubscribed { count: u64 },
    /// "Subscribers".
    Subscribers,
    /// "Followers".
    Followers,
    /// "Reposts".
    Reposts,
    /// "And 3 others liked your comment."
    OthersLikedYourComment { count: u64 },
    /// "And 3 others subscribed."
    OthersSubscribed { count: u64 },
    /// "And 3 others liked your post."
    OthersLikedYourPost { count: u64 },
    /// "In reply to Sam".
    InReplyTo { name: String },
    /// "No title", for a post without one.
    NoTitle,
    /// "Pending", for a comment waiting for approval.
    Pending,
}

impl WpSupportsLocalization for NotificationLabel {
    fn message_bundle(&self) -> MessageBundle<'_> {
        match self {
            Self::Likes => WpMessages::notification_likes(),
            Self::LikesOnYourComment { count } => {
                WpMessages::notification_likes_on_your_comment(*count)
            }
            Self::LikesOnYourPost { count } => WpMessages::notification_likes_on_your_post(*count),
            Self::PeopleSubscribed { count } => WpMessages::notification_people_subscribed(*count),
            Self::Subscribers => WpMessages::notification_subscribers(),
            Self::Followers => WpMessages::notification_followers(),
            Self::Reposts => WpMessages::notification_reposts(),
            Self::OthersLikedYourComment { count } => {
                WpMessages::notification_others_liked_your_comment(*count)
            }
            Self::OthersSubscribed { count } => WpMessages::notification_others_subscribed(*count),
            Self::OthersLikedYourPost { count } => {
                WpMessages::notification_others_liked_your_post(*count)
            }
            Self::InReplyTo { name } => WpMessages::notification_in_reply_to(name.as_str()),
            Self::NoTitle => WpMessages::notification_no_title(),
            Self::Pending => WpMessages::notification_pending(),
        }
    }
}

/// The title for a note with note context.
fn context_title(note: &WpComNotification) -> Option<NotificationLayoutText> {
    let count = note
        .meta
        .as_ref()
        .and_then(|meta| meta.actor_count)
        .filter(|count| *count > 0);
    let label = match &note.note_type {
        Some(NotificationType::Like) => Some(count.map_or(NotificationLabel::Likes, |count| {
            NotificationLabel::LikesOnYourPost { count }
        })),
        Some(NotificationType::CommentLike) => {
            Some(count.map_or(NotificationLabel::Likes, |count| {
                NotificationLabel::LikesOnYourComment { count }
            }))
        }
        Some(NotificationType::Follow) => {
            Some(count.map_or(NotificationLabel::Subscribers, |count| {
                NotificationLabel::PeopleSubscribed { count }
            }))
        }
        Some(NotificationType::ActivityPubFollowed) => Some(NotificationLabel::Followers),
        Some(NotificationType::ActivityPubInteraction) => match note.noticon.as_deref() {
            Some(STAR_NOTICON) => Some(NotificationLabel::Likes),
            Some(REBLOG_NOTICON) => Some(NotificationLabel::Reposts),
            _ => None,
        },
        _ => None,
    };
    label
        .map(NotificationLayoutText::from)
        .or_else(|| note.title.clone().map(NotificationLayoutText::from))
}

fn preview(note: &WpComNotification) -> Option<String> {
    let meta = note.meta.as_ref();
    match &note.note_type {
        Some(NotificationType::Comment | NotificationType::Automattcher) => {
            note.subject.get(1).map(|line| line.text.clone())
        }
        Some(NotificationType::CommentLike) => meta
            .and_then(|meta| meta.parent_comment.as_ref())
            .and_then(|comment| comment.text.clone()),
        _ => meta
            .and_then(|meta| meta.post.as_ref())
            .and_then(|post| post.title.clone()),
    }
}

fn comment(
    note: &WpComNotification,
    meta: &NotificationMeta,
    actor: &NotificationActor,
) -> NotificationCommentLayout {
    let replied_to = meta.parent_comment.as_ref().map(|parent| {
        let mut quoted = quoted_comment(parent, NotificationQuoteStyle::Preview);
        quoted.accessibility_label = parent
            .author_name
            .clone()
            .map(|name| NotificationLabel::InReplyTo { name });
        quoted
    });
    NotificationCommentLayout {
        post: post_card(meta),
        replied_to,
        commenter: actor.clone(),
        meta_line: meta_line(note),
        status: is_pending_approval(note).then_some(NotificationLabel::Pending),
        body: body(note, Hide::PEOPLE),
    }
}

fn reactions(
    note: &WpComNotification,
    meta: &NotificationMeta,
    title: &Option<NotificationLayoutText>,
) -> NotificationReactionsLayout {
    let has_post_card = meta.post.is_some();
    // Likes and new subscribers have their count in the title, so they list the people.
    let people_list: Option<fn(u64) -> NotificationLabel> = match &note.note_type {
        Some(NotificationType::Like) => {
            Some(|count| NotificationLabel::OthersLikedYourPost { count })
        }
        Some(NotificationType::CommentLike) => {
            Some(|count| NotificationLabel::OthersLikedYourComment { count })
        }
        Some(NotificationType::Follow) => {
            Some(|count| NotificationLabel::OthersSubscribed { count })
        }
        _ => None,
    };
    let Some(others_label) = people_list else {
        let heading = meta
            .heading
            .clone()
            .or_else(|| note.title.clone())
            .filter(|heading| match title {
                Some(NotificationLayoutText::Text { text }) => heading != text,
                // The title is a label, so a heading that's only the note's own title would say
                // it again in the API's words.
                Some(NotificationLayoutText::Label { .. }) => {
                    meta.heading.is_some() && note.title.as_ref() != Some(heading)
                }
                None => true,
            });
        return NotificationReactionsLayout {
            post: post_card(meta),
            comment: meta
                .parent_comment
                .as_ref()
                .map(|comment| quoted_comment(comment, NotificationQuoteStyle::Excerpt)),
            is_people_list: false,
            heading,
            body: body(
                note,
                Hide {
                    post: has_post_card,
                    ..Hide::NOTHING
                },
            ),
            others: None,
        };
    };

    let people = note
        .body
        .iter()
        .filter(|block| block.kind == Some(NotificationBlockKind::User))
        .count() as u64;
    let others = meta.actor_count.unwrap_or(0).saturating_sub(people);
    let others = (others > 0).then(|| NotificationOthers {
        count: others,
        label: others_label(others),
        url: note
            .body
            .iter()
            .find(|block| block.kind != Some(NotificationBlockKind::User))
            .and_then(|block| block.ranges.iter().find_map(|range| range.url.clone()))
            .or_else(|| note.url.clone()),
    });
    NotificationReactionsLayout {
        post: post_card(meta),
        comment: meta
            .parent_comment
            .as_ref()
            .map(|comment| quoted_comment(comment, NotificationQuoteStyle::Preview)),
        is_people_list: true,
        heading: None,
        body: body(
            note,
            Hide {
                post: has_post_card,
                text: others.is_some(),
                ..Hide::NOTHING
            },
        ),
        others,
    }
}

fn new_post(note: &WpComNotification, meta: &NotificationMeta) -> NotificationNewPostLayout {
    let byline = match (&meta.post, &meta.site) {
        (Some(_), Some(site)) => Some(NotificationNewPostByline::Site {
            site: site.clone(),
            author: meta
                .actor
                .clone()
                .filter(|actor| actor.name.as_deref().is_some_and(|name| !name.is_empty())),
            meta_line: meta_line(note),
        }),
        _ => headline(note).map(|headline| NotificationNewPostByline::Headline { headline }),
    };
    NotificationNewPostLayout {
        byline,
        post: meta.post.as_ref().map(|post| NotificationPostLink {
            title: post.title.clone(),
            url: post.url.clone(),
        }),
        body: body(
            note,
            Hide {
                people: meta.actor_count == Some(1),
                ..Hide::NOTHING
            },
        ),
    }
}

fn notice(note: &WpComNotification, meta: &NotificationMeta) -> NotificationNoticeLayout {
    NotificationNoticeLayout {
        headline: headline(note),
        post: post_card(meta),
        comment: meta
            .parent_comment
            .as_ref()
            .map(|comment| quoted_comment(comment, NotificationQuoteStyle::Excerpt)),
        body: body(
            note,
            Hide {
                people: meta.actor_count == Some(1),
                post: meta.post.is_some(),
                ..Hide::NOTHING
            },
        ),
    }
}

/// A note without note context: its header over its whole body.
fn notice_from_blocks(note: &WpComNotification) -> NotificationNoticeLayout {
    NotificationNoticeLayout {
        headline: header_lines(&note.header),
        post: None,
        comment: None,
        body: body(note, Hide::NOTHING),
    }
}

fn headline(note: &WpComNotification) -> Option<NotificationHeadline> {
    let meta = note.meta.as_ref();
    let actor = meta.and_then(|meta| meta.actor.as_ref());
    let summary = meta.and_then(|meta| meta.summary.as_ref());
    if let (Some(actor), Some(summary)) = (actor, summary) {
        return Some(NotificationHeadline::Summary {
            icon: note.icon.clone(),
            actor_url: actor.url.clone(),
            summary: text_runs(summary),
            meta_line: meta_line(note),
            status: is_pending_approval(note).then_some(NotificationLabel::Pending),
        });
    }
    header_site(&note.header)
        .map(|card| NotificationHeadline::Site { card })
        .or_else(|| header_lines(&note.header))
}

fn header_lines(header: &[NotificationText]) -> Option<NotificationHeadline> {
    (!header.is_empty()).then(|| NotificationHeadline::Header {
        lines: header.iter().map(text_runs).collect(),
    })
}

/// The site a note's header names in its first line, with the post its second line links to.
fn header_site(header: &[NotificationText]) -> Option<NotificationSiteCard> {
    let site_line = header.first()?;
    let range = site_line.ranges.first()?;
    let url = range.url.clone()?;
    let is_site = match &range.kind {
        Some(NotificationRangeKind::Site) => true,
        // Some notes name a site with a person's range whose ID is the site's.
        Some(NotificationRangeKind::User) => range
            .id
            .as_deref()
            .and_then(|id| id.parse::<u64>().ok())
            .zip(range.site_id)
            .is_some_and(|(id, site_id)| id == site_id.0),
        _ => false,
    };
    if site_line.text.is_empty() || !is_site {
        return None;
    }
    let post = header.get(1).and_then(|post_line| {
        let url = post_line
            .ranges
            .iter()
            .find_map(|range| range.url.clone())?;
        (!post_line.text.is_empty()).then(|| NotificationPostLink {
            title: Some(post_line.text.clone()),
            url: Some(url),
        })
    });
    Some(NotificationSiteCard {
        name: site_line.text.clone(),
        icon: site_line
            .media
            .first()
            .and_then(|medium| medium.url.clone()),
        url,
        post,
    })
}

fn post_card(meta: &NotificationMeta) -> Option<NotificationPostCard> {
    let post: &NotificationPostContext = meta.post.as_ref()?;
    let title = post.title.clone().filter(|title| !title.is_empty()).map_or(
        NotificationLabel::NoTitle.into(),
        NotificationLayoutText::from,
    );
    let site = meta.site.clone();
    let has_site = site.is_some();
    Some(NotificationPostCard {
        title,
        url: post.url.clone(),
        site,
        author_name: post.author_name.clone().filter(|_| has_site),
        author_avatar: post.author_avatar.clone().filter(|_| has_site),
        date: post.date.filter(|_| has_site),
    })
}

fn quoted_comment(
    comment: &NotificationParentComment,
    style: NotificationQuoteStyle,
) -> NotificationQuotedComment {
    let content = match &comment.content {
        Some(content) => block_runs(content),
        None => {
            notification_styled_text(comment.text.as_deref().unwrap_or_default(), &[], &[]).runs
        }
    };
    NotificationQuotedComment {
        author_name: comment.author_name.clone(),
        author_avatar: comment.author_avatar.clone(),
        author_url: comment.author_url.clone(),
        date: comment.date,
        url: comment.url.clone(),
        content,
        style,
        accessibility_label: None,
    }
}

fn meta_line(note: &WpComNotification) -> NotificationMetaLine {
    NotificationMetaLine {
        timestamp: note.timestamp,
        url: note.url.clone(),
        follow: follow_offer(note),
    }
}

/// An offer to follow the one person behind a note, from the first block about a person.
fn follow_offer(note: &WpComNotification) -> Option<NotificationFollowOffer> {
    if note.note_type == Some(NotificationType::Comment)
        || note.meta.as_ref().and_then(|meta| meta.actor_count) != Some(1)
    {
        return None;
    }
    let person = note
        .body
        .iter()
        .find(|block| role(note, block) == NotificationBlockRole::User)?;
    let site_id = person.meta.as_ref()?.ids.as_ref()?.site?;
    let follow = person
        .actions
        .iter()
        .find(|action| action.kind == super::NotificationActionKind::Follow)?;
    Some(NotificationFollowOffer {
        site_id,
        is_following: follow.is_on.unwrap_or(false),
    })
}

/// Whether a comment note's comment waits for approval, as the note says it was when fetched.
fn is_pending_approval(note: &WpComNotification) -> bool {
    if note.note_type != Some(NotificationType::Comment) {
        return false;
    }
    let Some(actions) = note
        .body
        .iter()
        .rev()
        .find(|block| !block.actions.is_empty())
        .map(|block| &block.actions)
    else {
        return false;
    };
    actions
        .iter()
        .find(|action| action.kind == super::NotificationActionKind::ApproveComment)
        .is_some_and(|approve| approve.is_on != Some(true))
}

/// Which body blocks to leave out.
#[derive(Clone, Copy)]
struct Hide {
    people: bool,
    post: bool,
    text: bool,
}

impl Hide {
    const NOTHING: Self = Self {
        people: false,
        post: false,
        text: false,
    };
    const PEOPLE: Self = Self {
        people: true,
        ..Self::NOTHING
    };
}

fn body(note: &WpComNotification, hide: Hide) -> Vec<NotificationLayoutBlock> {
    note.body
        .iter()
        .filter_map(|block| {
            let role = role(note, block);
            let hidden = match role {
                NotificationBlockRole::User => hide.people,
                NotificationBlockRole::Post => hide.post,
                NotificationBlockRole::Text => hide.text,
                _ => false,
            };
            (!hidden).then(|| NotificationLayoutBlock {
                role,
                block: block.clone(),
                runs: block_runs(block),
            })
        })
        .collect()
}

/// What a block is: what its IDs refer to, or else its type.
fn role(note: &WpComNotification, block: &NotificationBlock) -> NotificationBlockRole {
    let mut role = match &block.kind {
        None => NotificationBlockRole::Text,
        Some(NotificationBlockKind::User) => NotificationBlockRole::User,
        Some(NotificationBlockKind::Comment) => NotificationBlockRole::Comment,
        Some(NotificationBlockKind::Post) => NotificationBlockRole::Post,
        Some(NotificationBlockKind::Other(_)) => NotificationBlockRole::Other,
    };
    let reply_comment = note
        .meta
        .as_ref()
        .and_then(|meta| meta.ids.as_ref())
        .and_then(|ids| ids.reply_comment);
    if let Some(reply_comment) = reply_comment
        && block.ranges.len() > 1
        && block.ranges[1].id.as_deref() == Some(reply_comment.0.to_string().as_str())
    {
        role = NotificationBlockRole::Reply;
    }
    let Some(ids) = block.meta.as_ref().and_then(|meta| meta.ids.as_ref()) else {
        return role;
    };
    if is_empty(ids) {
        role
    } else if ids.other.contains_key("prompt") {
        NotificationBlockRole::Prompt
    } else if ids.comment.is_some() {
        NotificationBlockRole::Comment
    } else if ids.post.is_some() {
        NotificationBlockRole::Post
    } else if ids.user.is_some() {
        NotificationBlockRole::User
    } else {
        role
    }
}

fn is_empty(ids: &NotificationIds) -> bool {
    ids.site.is_none()
        && ids.post.is_none()
        && ids.comment.is_none()
        && ids.user.is_none()
        && ids.parent_comment.is_none()
        && ids.reply_comment.is_none()
        && ids.other.is_empty()
}

fn text_runs(text: &NotificationText) -> Vec<StyledTextRun> {
    notification_styled_text(&text.text, &text.ranges, &text.media).runs
}

fn block_runs(block: &NotificationBlock) -> Vec<StyledTextRun> {
    notification_styled_text(&block.text, &block.ranges, &block.media).runs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        comments::CommentId,
        wp_com::{notifications::NotificationActionKind, styled_text::TextRunStyle},
    };
    use rstest::rstest;
    use serde_json::{Value, json};
    use wp_localization::{WpLocale, WpLocalizable};

    fn note(value: Value) -> WpComNotification {
        serde_json::from_value(value).expect("the note parses")
    }

    fn person(name: &str, user_id: u64, site_id: u64, following: Option<bool>) -> Value {
        let mut block = json!({
            "type": "user",
            "text": name,
            "ranges": [{ "type": "user", "indices": [0, name.encode_utf16().count()], "id": user_id }],
            "meta": { "ids": { "user": user_id, "site": site_id } }
        });
        if let Some(following) = following {
            block["actions"] = json!({ "follow": following });
        }
        block
    }

    fn actor(name: &str) -> Value {
        json!({ "name": name, "avatar": "https://avatar.example.com/a.png", "url": "https://sam.example.org" })
    }

    fn post_context() -> Value {
        json!({
            "title": "Winter garden",
            "url": "https://garden.example.com/winter",
            "date": "2026-01-02T03:04:05+00:00",
            "author_name": "Robin",
            "author_avatar": "https://avatar.example.com/r.png"
        })
    }

    fn site_context() -> Value {
        json!({ "name": "Garden Notes", "icon": "https://garden.example.com/icon.png", "url": "https://garden.example.com" })
    }

    fn summary() -> Value {
        json!({ "text": "Sam liked your post", "ranges": [{ "type": "user", "indices": [0, 3], "id": 701 }] })
    }

    fn label(text: &Option<NotificationLayoutText>) -> &NotificationLabel {
        match text {
            Some(NotificationLayoutText::Label { label }) => label,
            other => panic!("Expected a label, got {other:?}"),
        }
    }

    fn roles(body: &[NotificationLayoutBlock]) -> Vec<NotificationBlockRole> {
        body.iter().map(|block| block.role).collect()
    }

    fn text(runs: &[StyledTextRun]) -> String {
        runs.iter().map(|run| run.text.as_str()).collect()
    }

    fn comment_note(parent: Value, approved: bool) -> WpComNotification {
        note(json!({
            "id": 10_000_000_001u64,
            "type": "comment",
            "title": "Comment",
            "timestamp": "2026-01-03T10:20:30+00:00",
            "url": "https://garden.example.com/winter#comment-3",
            "subject": [{ "text": "Sam replied" }, { "text": "Lovely photos" }],
            "body": [
                person("Sam", 701, 801, Some(false)),
                {
                    "text": "Lovely photos",
                    "meta": { "ids": { "comment": 3, "post": 20, "site": 801 } },
                    "actions": { "approve-comment": approved, "replyto-comment": false }
                }
            ],
            "meta": {
                "ids": { "site": 801, "post": 20, "comment": 3, "parent_comment": 2 },
                "actor": actor("Sam"),
                "actor_count": 1,
                "summary": summary(),
                "post": post_context(),
                "site": site_context(),
                "parent_comment": parent
            }
        }))
    }

    #[test]
    fn comment_shows_the_reply_under_the_comment_it_answers() {
        let parent = json!({
            "text": "First!",
            "content": { "text": "First!", "ranges": [{ "type": "b", "indices": [0, 5] }] },
            "author_name": "Robin",
            "date": "2026-01-02T03:04:05+00:00",
            "url": "https://garden.example.com/winter#comment-2"
        });
        let layout = notification_layout(&comment_note(parent, false));

        assert_eq!(layout.fallback, None);
        assert_eq!(
            layout.title,
            Some(NotificationLayoutText::Text {
                text: "Comment".to_string()
            })
        );
        assert_eq!(layout.preview.as_deref(), Some("Lovely photos"));
        let NotificationLayoutContent::Comment { layout } = layout.content else {
            panic!("Expected the comment layout");
        };
        let post = layout.post.expect("a card for the post");
        assert_eq!(
            post.title,
            NotificationLayoutText::Text {
                text: "Winter garden".to_string()
            }
        );
        assert_eq!(post.author_name.as_deref(), Some("Robin"));
        let replied_to = layout.replied_to.expect("the comment it replies to");
        assert_eq!(replied_to.style, NotificationQuoteStyle::Preview);
        assert_eq!(
            replied_to.accessibility_label,
            Some(NotificationLabel::InReplyTo {
                name: "Robin".to_string()
            })
        );
        assert_eq!(text(&replied_to.content), "First!");
        assert_eq!(replied_to.content[0].styles, [TextRunStyle::Bold]);
        assert_eq!(layout.commenter.name.as_deref(), Some("Sam"));
        assert_eq!(layout.status, Some(NotificationLabel::Pending));
        assert_eq!(roles(&layout.body), [NotificationBlockRole::Comment]);
        assert_eq!(
            layout.meta_line.follow, None,
            "A comment's own note never offers to follow"
        );
    }

    #[test]
    fn comment_quotes_a_parent_without_content_from_its_text() {
        let parent =
            json!({ "text": "Plain words", "url": "https://garden.example.com/winter#comment-2" });
        let NotificationLayoutContent::Comment { layout } =
            notification_layout(&comment_note(parent, true)).content
        else {
            panic!("Expected the comment layout");
        };
        let replied_to = layout.replied_to.expect("the comment it replies to");
        assert_eq!(text(&replied_to.content), "Plain words");
        assert_eq!(
            replied_to.accessibility_label, None,
            "No name to say who it replies to"
        );
        assert_eq!(layout.status, None, "An approved comment isn't pending");
    }

    #[test]
    fn comment_without_a_parent_post_or_site() {
        let mut note = comment_note(json!(null), true);
        let meta = note.meta.as_mut().unwrap();
        meta.post = None;
        meta.site = None;
        let NotificationLayoutContent::Comment { layout } = notification_layout(&note).content
        else {
            panic!("Expected the comment layout");
        };
        assert_eq!(layout.post, None);
        assert_eq!(layout.replied_to, None);
    }

    #[test]
    fn comment_marks_the_users_own_reply() {
        let mut note = comment_note(json!(null), true);
        note.meta
            .as_mut()
            .unwrap()
            .ids
            .as_mut()
            .unwrap()
            .reply_comment = Some(CommentId(4));
        note.body.push(
            serde_json::from_value(json!({
                "text": "You replied to this comment.",
                "ranges": [
                    { "type": "b", "indices": [0, 3] },
                    { "type": "comment", "indices": [4, 11], "id": 4 }
                ]
            }))
            .unwrap(),
        );
        let NotificationLayoutContent::Comment { layout } = notification_layout(&note).content
        else {
            panic!("Expected the comment layout");
        };
        assert_eq!(
            roles(&layout.body),
            [NotificationBlockRole::Comment, NotificationBlockRole::Reply]
        );
    }

    fn likes_note(note_type: &str, actor_count: Option<u64>, people: usize) -> WpComNotification {
        let mut body: Vec<Value> = (0..people)
            .map(|index| person(&format!("Person {index}"), 700 + index as u64, 800, None))
            .collect();
        body.push(json!({
            "text": "and others liked your post",
            "ranges": [{ "type": "link", "indices": [4, 10], "url": "https://garden.example.com/likes" }]
        }));
        body.push(
            json!({ "text": "Winter garden", "meta": { "ids": { "post": 20, "site": 801 } } }),
        );
        let mut meta = json!({
            "ids": { "site": 801, "post": 20 },
            "actor": actor("Person 0"),
            "heading": "Likes on Winter garden",
            "summary": summary(),
            "post": post_context(),
            "site": site_context()
        });
        if let Some(actor_count) = actor_count {
            meta["actor_count"] = json!(actor_count);
        }
        note(json!({
            "id": 10_000_000_002u64,
            "type": note_type,
            "title": "Likes",
            "url": "https://garden.example.com/winter",
            "body": body,
            "meta": meta
        }))
    }

    #[test]
    fn likes_count_the_people_the_list_leaves_out() {
        let layout = notification_layout(&likes_note("like", Some(12), 2));

        assert_eq!(
            label(&layout.title),
            &NotificationLabel::LikesOnYourPost { count: 12 }
        );
        assert_eq!(layout.preview.as_deref(), Some("Winter garden"));
        let NotificationLayoutContent::Reactions { layout } = layout.content else {
            panic!("Expected the reactions layout");
        };
        assert!(layout.is_people_list);
        assert_eq!(layout.heading, None);
        assert!(layout.post.is_some());
        assert_eq!(
            layout.others,
            Some(NotificationOthers {
                count: 10,
                label: NotificationLabel::OthersLikedYourPost { count: 10 },
                url: Some("https://garden.example.com/likes".to_string()),
            })
        );
        assert_eq!(
            roles(&layout.body),
            [NotificationBlockRole::User, NotificationBlockRole::User],
            "The text is replaced by the count, and the post is on its card"
        );
    }

    #[test]
    fn likes_from_one_person_keep_the_text() {
        let layout = notification_layout(&likes_note("like", Some(1), 1));
        assert_eq!(
            label(&layout.title),
            &NotificationLabel::LikesOnYourPost { count: 1 }
        );
        let NotificationLayoutContent::Reactions { layout } = layout.content else {
            panic!("Expected the reactions layout");
        };
        assert_eq!(layout.others, None);
        assert_eq!(
            roles(&layout.body),
            [NotificationBlockRole::User, NotificationBlockRole::Text]
        );
    }

    #[rstest]
    #[case(None)]
    #[case(Some(0))]
    fn likes_without_a_count_are_titled_likes(#[case] actor_count: Option<u64>) {
        let layout = notification_layout(&likes_note("like", actor_count, 1));
        assert_eq!(label(&layout.title), &NotificationLabel::Likes);
        let NotificationLayoutContent::Reactions { layout } = layout.content else {
            panic!("Expected the reactions layout");
        };
        assert_eq!(layout.others, None);
    }

    #[test]
    fn comment_likes_quote_the_comment() {
        let mut note = likes_note("comment_like", Some(3), 1);
        let meta = note.meta.as_mut().unwrap();
        meta.post = None;
        meta.parent_comment =
            Some(serde_json::from_value(json!({ "text": "Nice", "author_name": "You" })).unwrap());
        let layout = notification_layout(&note);
        assert_eq!(
            label(&layout.title),
            &NotificationLabel::LikesOnYourComment { count: 3 }
        );
        assert_eq!(layout.preview.as_deref(), Some("Nice"));
        let NotificationLayoutContent::Reactions { layout } = layout.content else {
            panic!("Expected the reactions layout");
        };
        let comment = layout.comment.expect("the liked comment");
        assert_eq!(comment.style, NotificationQuoteStyle::Preview);
        assert_eq!(comment.accessibility_label, None);
        assert_eq!(
            layout.others.map(|others| others.label),
            Some(NotificationLabel::OthersLikedYourComment { count: 2 })
        );
        assert_eq!(
            roles(&layout.body),
            [NotificationBlockRole::User, NotificationBlockRole::Post],
            "Without a card, the post's block stays"
        );
    }

    #[test]
    fn reblogs_show_a_heading_that_differs_from_the_title() {
        let mut note = likes_note("reblog", Some(5), 1);
        note.title = Some("Reblog".to_string());
        let layout = notification_layout(&note);
        assert_eq!(
            layout.title,
            Some(NotificationLayoutText::Text {
                text: "Reblog".to_string()
            })
        );
        let NotificationLayoutContent::Reactions { layout } = layout.content else {
            panic!("Expected the reactions layout");
        };
        assert!(!layout.is_people_list);
        assert_eq!(layout.heading.as_deref(), Some("Likes on Winter garden"));
        assert_eq!(layout.others, None, "Only lists of people count the rest");
        assert_eq!(
            roles(&layout.body),
            [NotificationBlockRole::User, NotificationBlockRole::Text]
        );

        note.meta.as_mut().unwrap().heading = None;
        let NotificationLayoutContent::Reactions { layout } = notification_layout(&note).content
        else {
            panic!("Expected the reactions layout");
        };
        assert_eq!(layout.heading, None, "The heading would repeat the title");
    }

    fn site_header() -> Value {
        json!([
            {
                "text": "Garden Notes",
                "ranges": [{ "type": "user", "indices": [0, 12], "id": 801, "site_id": 801, "url": "https://garden.example.com" }],
                "media": [{ "type": "image", "indices": [0, 0], "url": "https://garden.example.com/icon.png" }]
            },
            { "text": "Winter garden", "ranges": [{ "type": "post", "indices": [0, 13], "url": "https://garden.example.com/winter" }] }
        ])
    }

    #[test]
    fn follows_show_the_site_above_the_people() {
        let mut value = serde_json::to_value(likes_note("follow", Some(7), 2)).unwrap();
        value["header"] = site_header();
        let layout = notification_layout(&note(value));

        assert_eq!(
            label(&layout.title),
            &NotificationLabel::PeopleSubscribed { count: 7 }
        );
        let NotificationLayoutContent::Follow { layout } = layout.content else {
            panic!("Expected the follow layout");
        };
        assert_eq!(
            layout.site,
            Some(NotificationSiteCard {
                name: "Garden Notes".to_string(),
                icon: Some("https://garden.example.com/icon.png".to_string()),
                url: "https://garden.example.com".to_string(),
                post: Some(NotificationPostLink {
                    title: Some("Winter garden".to_string()),
                    url: Some("https://garden.example.com/winter".to_string()),
                }),
            })
        );
        assert!(layout.reactions.is_people_list);
        assert_eq!(
            layout.reactions.others.map(|others| others.label),
            Some(NotificationLabel::OthersSubscribed { count: 5 })
        );
    }

    #[test]
    fn follows_without_a_count_are_titled_subscribers() {
        let layout = notification_layout(&likes_note("follow", None, 1));
        assert_eq!(label(&layout.title), &NotificationLabel::Subscribers);
    }

    #[rstest]
    #[case(
        "activitypub_followed_note",
        "\u{f801}",
        Some(NotificationLabel::Followers)
    )]
    #[case(
        "activitypub_interaction_note",
        "\u{f408}",
        Some(NotificationLabel::Likes)
    )]
    #[case(
        "activitypub_interaction_note",
        "\u{f805}",
        Some(NotificationLabel::Reposts)
    )]
    #[case("activitypub_interaction_note", "\u{f300}", None)]
    fn fediverse_notes_are_titled_by_what_happened(
        #[case] note_type: &str,
        #[case] noticon: &str,
        #[case] expected: Option<NotificationLabel>,
    ) {
        let mut note = likes_note(note_type, Some(2), 2);
        note.noticon = Some(noticon.to_string());
        note.title = Some("Fediverse".to_string());
        let layout = notification_layout(&note);
        match expected {
            Some(expected) => assert_eq!(label(&layout.title), &expected),
            None => assert_eq!(
                layout.title,
                Some(NotificationLayoutText::Text {
                    text: "Fediverse".to_string()
                })
            ),
        }
        let NotificationLayoutContent::Follow { layout } = layout.content else {
            panic!("Expected the follow layout");
        };
        assert!(!layout.reactions.is_people_list);
        assert_eq!(layout.site, None, "The note's header names no site");
    }

    fn new_post_note(note_type: &str, comment_id: Option<i64>) -> WpComNotification {
        let mut ids = json!({ "site": 801, "post": 20 });
        if let Some(comment_id) = comment_id {
            ids["comment"] = json!(comment_id);
        }
        note(json!({
            "id": 10_000_000_003u64,
            "type": note_type,
            "title": "New Post",
            "icon": "https://avatar.example.com/r.png",
            "timestamp": "2026-01-03T10:20:30+00:00",
            "url": "https://garden.example.com/winter",
            "subject": [{ "text": "Robin published" }, { "text": "A mention" }],
            "body": [
                person("Robin", 702, 802, Some(true)),
                { "text": "The snow came early.", "meta": { "ids": { "site": 801, "post": 20 } } }
            ],
            "meta": {
                "ids": ids,
                "actor": actor("Robin"),
                "actor_count": 1,
                "summary": summary(),
                "post": post_context(),
                "site": site_context()
            }
        }))
    }

    #[test]
    fn new_posts_show_the_site_author_and_title() {
        let layout = notification_layout(&new_post_note("new_post", None));
        assert_eq!(layout.fallback, None);
        let NotificationLayoutContent::NewPost { layout } = layout.content else {
            panic!("Expected the new post layout");
        };
        let Some(NotificationNewPostByline::Site {
            site,
            author,
            meta_line,
        }) = layout.byline
        else {
            panic!("Expected the site above the post");
        };
        assert_eq!(site.name.as_deref(), Some("Garden Notes"));
        assert_eq!(
            author.and_then(|author| author.name).as_deref(),
            Some("Robin")
        );
        assert_eq!(
            meta_line.follow,
            Some(NotificationFollowOffer {
                site_id: WpComSiteId(802),
                is_following: true,
            })
        );
        assert_eq!(
            layout.post.and_then(|post| post.title).as_deref(),
            Some("Winter garden")
        );
        assert_eq!(
            roles(&layout.body),
            [NotificationBlockRole::Post],
            "The one author's block is left out"
        );
    }

    #[test]
    fn new_posts_without_a_site_show_the_headline() {
        let mut note = new_post_note("new_post", None);
        note.meta.as_mut().unwrap().site = None;
        let NotificationLayoutContent::NewPost { layout } = notification_layout(&note).content
        else {
            panic!("Expected the new post layout");
        };
        let Some(NotificationNewPostByline::Headline {
            headline: NotificationHeadline::Summary { summary, icon, .. },
        }) = layout.byline
        else {
            panic!("Expected the summary above the post");
        };
        assert_eq!(text(&summary), "Sam liked your post");
        assert_eq!(icon.as_deref(), Some("https://avatar.example.com/r.png"));
    }

    #[test]
    fn mentions_in_a_post_take_the_new_post_layout_and_in_a_comment_the_comment_layout() {
        let in_post = notification_layout(&new_post_note("automattcher", None));
        assert!(matches!(
            in_post.content,
            NotificationLayoutContent::NewPost { .. }
        ));
        assert_eq!(in_post.preview.as_deref(), Some("A mention"));

        let in_comment = notification_layout(&new_post_note("automattcher", Some(3)));
        let NotificationLayoutContent::Comment { layout } = in_comment.content else {
            panic!("Expected the comment layout");
        };
        assert!(
            layout.meta_line.follow.is_some(),
            "A mention offers to follow"
        );
    }

    #[test]
    fn other_types_with_context_are_notices() {
        let layout = notification_layout(&new_post_note("invented_type", None));
        assert_eq!(
            layout.fallback,
            Some(NotificationLayoutFallback::NoLayoutForType)
        );
        assert_eq!(
            layout.title,
            Some(NotificationLayoutText::Text {
                text: "New Post".to_string()
            })
        );
        let NotificationLayoutContent::Notice { layout } = layout.content else {
            panic!("Expected the notice layout");
        };
        assert!(matches!(
            layout.headline,
            Some(NotificationHeadline::Summary { .. })
        ));
        assert!(layout.post.is_some());
        assert!(
            layout.body.is_empty(),
            "The one person's block and the post's block are shown elsewhere"
        );
    }

    #[test]
    fn notes_without_context_are_notices_built_from_their_blocks() {
        let mut value = serde_json::to_value(new_post_note("like", None)).unwrap();
        value["meta"] = json!({ "ids": { "site": 801 } });
        value["header"] = json!([{ "text": "Robin" }, { "text": "" }]);
        let layout = notification_layout(&note(value));

        assert_eq!(layout.fallback, Some(NotificationLayoutFallback::NoContext));
        assert_eq!(
            layout.title,
            Some(NotificationLayoutText::Text {
                text: "New Post".to_string()
            }),
            "Without context, the API's title"
        );
        let NotificationLayoutContent::Notice { layout } = layout.content else {
            panic!("Expected the notice layout");
        };
        let Some(NotificationHeadline::Header { lines }) = layout.headline else {
            panic!("Expected the header lines");
        };
        assert_eq!(lines.len(), 2);
        assert_eq!(
            roles(&layout.body),
            [NotificationBlockRole::User, NotificationBlockRole::Post]
        );
    }

    #[test]
    fn a_note_with_nothing_is_an_empty_notice() {
        let layout = notification_layout(&note(json!({ "id": 1 })));
        assert_eq!(layout.fallback, Some(NotificationLayoutFallback::NoContext));
        assert_eq!(layout.title, None);
        assert_eq!(layout.preview, None);
        assert_eq!(
            layout.content,
            NotificationLayoutContent::Notice {
                layout: NotificationNoticeLayout {
                    headline: None,
                    post: None,
                    comment: None,
                    body: vec![],
                }
            }
        );
    }

    #[test]
    fn a_post_without_a_title_or_site() {
        let mut note = comment_note(json!(null), true);
        let meta = note.meta.as_mut().unwrap();
        meta.post.as_mut().unwrap().title = Some(String::new());
        meta.site = None;
        let NotificationLayoutContent::Comment { layout } = notification_layout(&note).content
        else {
            panic!("Expected the comment layout");
        };
        let post = layout.post.expect("a card for the post");
        assert_eq!(
            post.title,
            NotificationLayoutText::Label {
                label: NotificationLabel::NoTitle
            }
        );
        assert_eq!(post.site, None);
        assert_eq!(post.author_name, None, "Without a site, only the title");
        assert_eq!(post.date, None);
    }

    #[test]
    fn a_header_that_names_a_person_is_not_a_site() {
        let mut header = site_header();
        header[0]["ranges"][0]["id"] = json!(701);
        let header: Vec<NotificationText> = serde_json::from_value(header).unwrap();
        assert_eq!(header_site(&header), None);
    }

    #[test]
    fn a_follow_offer_needs_the_follow_action() {
        let mut note = new_post_note("new_post", None);
        note.body[0]
            .actions
            .retain(|action| action.kind != NotificationActionKind::Follow);
        assert_eq!(follow_offer(&note), None);
    }

    fn english(label: NotificationLabel) -> String {
        label.localize(Some(WpLocale::from("en-US")))
    }

    #[rstest]
    #[case(NotificationLabel::Likes, "Likes")]
    #[case(NotificationLabel::LikesOnYourComment { count: 1 }, "\u{2068}1\u{2069} like on your comment")]
    #[case(NotificationLabel::LikesOnYourComment { count: 5 }, "\u{2068}5\u{2069} likes on your comment")]
    #[case(NotificationLabel::LikesOnYourPost { count: 1 }, "\u{2068}1\u{2069} like on your post")]
    #[case(NotificationLabel::LikesOnYourPost { count: 12 }, "\u{2068}12\u{2069} likes on your post")]
    #[case(NotificationLabel::PeopleSubscribed { count: 1 }, "\u{2068}1\u{2069} person subscribed")]
    #[case(NotificationLabel::PeopleSubscribed { count: 17 }, "\u{2068}17\u{2069} people subscribed")]
    #[case(NotificationLabel::Subscribers, "Subscribers")]
    #[case(NotificationLabel::Followers, "Followers")]
    #[case(NotificationLabel::Reposts, "Reposts")]
    #[case(NotificationLabel::OthersLikedYourComment { count: 1 }, "And \u{2068}1\u{2069} other liked your comment.")]
    #[case(NotificationLabel::OthersLikedYourComment { count: 2 }, "And \u{2068}2\u{2069} others liked your comment.")]
    #[case(NotificationLabel::OthersSubscribed { count: 1 }, "And \u{2068}1\u{2069} other subscribed.")]
    #[case(NotificationLabel::OthersSubscribed { count: 3 }, "And \u{2068}3\u{2069} others subscribed.")]
    #[case(NotificationLabel::OthersLikedYourPost { count: 1 }, "And \u{2068}1\u{2069} other liked your post.")]
    #[case(NotificationLabel::OthersLikedYourPost { count: 10 }, "And \u{2068}10\u{2069} others liked your post.")]
    #[case(NotificationLabel::InReplyTo { name: "Sam".to_string() }, "In reply to \u{2068}Sam\u{2069}")]
    #[case(NotificationLabel::NoTitle, "No title")]
    #[case(NotificationLabel::Pending, "Pending")]
    fn labels_in_english(#[case] label: NotificationLabel, #[case] expected: &str) {
        assert_eq!(english(label), expected);
    }

    #[test]
    fn labels_fall_back_to_english_until_translated() {
        assert_eq!(
            NotificationLabel::LikesOnYourPost { count: 2 }.localize(Some(WpLocale::from("de"))),
            "\u{2068}2\u{2069} likes on your post"
        );
        assert_eq!(
            localize_notification_label(&NotificationLabel::Pending, None),
            "Pending"
        );
    }
}

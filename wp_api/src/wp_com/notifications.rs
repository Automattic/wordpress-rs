//! The user's WordPress.com notifications: listing them and fetching one.
//!
//! A notification (the API calls it a note) is made of text blocks: a subject, an optional
//! header, and a body. Each block's ranges and media point into its text, counting positions in
//! UTF-16 units, and ranges can nest and can be empty.
//!
//! The API has many note types and adds more, so every kind of thing a note can hold — its type,
//! a block's type, a range's, a medium's, an action's — has a named case for the ones the apps
//! treat specially, and `Other` with the name as the API sent it for the rest.
//!
//! A value that's missing, empty or of an unexpected type reads as `None` (or an empty list)
//! rather than failing the note. A note that still can't be read doesn't fail its page: it's
//! listed in the page's `unreadable_notes`.
//!
//! Every request needs a token.

use crate::{
    comments::CommentId,
    date::WpGmtDateTime,
    posts::PostId,
    url_query::{AppendUrlQueryPairs, AsQueryValue, QueryPairs, QueryPairsExtension},
    wp_com::{WpComSiteId, language::WPComLanguage, me::WpComUserId},
    wp_content_u64_id,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wp_serde_helper::LenientVec;

wp_content_u64_id!(NotificationId);

/// Defines a kind the API sends as a name: a case for each name the apps treat specially, and
/// `Other` for the rest. It reads from and writes to the name.
macro_rules! notification_kind {
    (
        $(#[$enum_meta:meta])*
        $name:ident {
            $( $(#[$variant_meta:meta])* $variant:ident => $raw:literal, )*
        }
    ) => {
        $(#[$enum_meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, uniffi::Enum)]
        pub enum $name {
            $( $(#[$variant_meta])* $variant, )*
            /// A name this version doesn't treat specially, as the API sent it.
            Other(String),
        }

        impl $name {
            /// Returns the name the API uses for this kind.
            pub fn as_str(&self) -> &str {
                match self {
                    $( Self::$variant => $raw, )*
                    Self::Other(raw) => raw,
                }
            }
        }

        impl From<String> for $name {
            fn from(raw: String) -> Self {
                match raw.as_str() {
                    $( $raw => Self::$variant, )*
                    _ => Self::Other(raw),
                }
            }
        }

        impl From<&str> for $name {
            fn from(raw: &str) -> Self {
                Self::from(raw.to_string())
            }
        }

        impl Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                String::deserialize(deserializer).map(Self::from)
            }
        }
    };
}

notification_kind! {
    /// What a notification is about. These are the types the WordPress apps and the web
    /// notifications panel show in their own way; every other type is `Other`.
    NotificationType {
        /// A comment on one of the user's posts, or a reply to one of their comments.
        Comment => "comment",
        /// Someone liked the user's comment.
        CommentLike => "comment_like",
        /// Someone liked the user's post.
        Like => "like",
        /// Someone followed the user's site.
        Follow => "follow",
        /// Someone mentioned the user, or a term the user watches.
        Automattcher => "automattcher",
        /// A site the user follows published a post.
        NewPost => "new_post",
        /// Someone reblogged the user's post.
        Reblog => "reblog",
        /// The user's site reached a number of views.
        ViewMilestone => "view_milestone",
        /// A request to approve a sign-in.
        PushAuth => "push_auth",
        /// A blogging prompt.
        BloggingPrompt => "blogging_prompts_note",
        /// Someone on the fediverse followed the user's site.
        ActivityPubFollowed => "activitypub_followed_note",
        /// Someone on the fediverse interacted with the user's post.
        ActivityPubInteraction => "activitypub_interaction_note",
        Post => "post",
        User => "user",
    }
}

impl AsQueryValue for NotificationType {
    fn as_query_value(&self) -> impl AsRef<str> {
        self.as_str()
    }
}

notification_kind! {
    /// What a body block shows. Most blocks have no type: they're text.
    NotificationBlockKind {
        /// A person, such as who commented or liked.
        User => "user",
        /// A comment, which the actions in the block act on.
        Comment => "comment",
        /// A post, which the actions in the block act on.
        Post => "post",
    }
}

notification_kind! {
    /// What a range of a block's text is.
    ///
    /// Many ranges are named after HTML elements and mark up the text the way that element
    /// would. Those that change how the text looks have a case. Those that only give it
    /// structure, such as `p`, `div` and `span`, are `Other`: the text usually breaks its lines
    /// where they do already.
    NotificationRangeKind {
        /// A person. The range's `id` is their user ID, and its `site_id` their site.
        User => "user",
        /// A post. The range's `id` is the post's ID, and its `site_id` the post's site.
        Post => "post",
        /// A page, with the same IDs as a post.
        Page => "page",
        /// A site. The range's `id` is the site's ID.
        Site => "site",
        /// A comment. The range's `id` is the comment's ID, and its `post_id` and `site_id`
        /// the post and site it's on.
        Comment => "comment",
        /// A link to the site's stats.
        Stat => "stat",
        /// A link to a security scan's results.
        Scan => "scan",
        /// Quoted text, such as the comment being replied to.
        Blockquote => "blockquote",
        Follow => "follow",
        /// An icon from the Noticons font, whose character is the range's `value`.
        Noticon => "noticon",
        Like => "like",
        /// The text that matched a term the user watches.
        Match => "match",
        Media => "media",
        /// A link to the range's `url`.
        Link => "link",
        /// An HTML link to the range's `url`.
        Anchor => "a",
        Bold => "b",
        Italic => "i",
        Emphasis => "em",
        Code => "code",
        /// Text whose spaces and line breaks are kept.
        Preformatted => "pre",
        Strikethrough => "s",
        /// Deleted text, shown struck through.
        Deleted => "del",
        Superscript => "sup",
        Subscript => "sub",
        /// The title of a work.
        Citation => "cite",
        Heading1 => "h1",
        Heading2 => "h2",
        Heading3 => "h3",
        Heading4 => "h4",
        Heading5 => "h5",
        Heading6 => "h6",
        /// A numbered list, whose items are the [`Self::ListItem`] ranges inside it.
        OrderedList => "ol",
        /// A bulleted list, whose items are the [`Self::ListItem`] ranges inside it.
        UnorderedList => "ul",
        ListItem => "li",
        /// A line break. It covers no text.
        LineBreak => "br",
        /// A line across the text, between two parts of it. It covers no text.
        HorizontalRule => "hr",
        /// A button that opens the range's `url`.
        Button => "button",
        /// A link whose clicks the web panel records under the range's `context`.
        Tracks => "tracks",
        /// A link to a backup that's ready to download.
        RewindDownloadReady => "rewind_download_ready",
    }
}

notification_kind! {
    /// What a medium in a block is.
    NotificationMediaKind {
        Image => "image",
        /// An achievement's badge.
        Badge => "badge",
    }
}

notification_kind! {
    /// Something the user can do from a body block.
    NotificationActionKind {
        ApproveComment => "approve-comment",
        EditComment => "edit-comment",
        LikeComment => "like-comment",
        LikePost => "like-post",
        ReplyToComment => "replyto-comment",
        SpamComment => "spam-comment",
        TrashComment => "trash-comment",
        /// Follow the block's site.
        Follow => "follow",
        /// Write a post that answers a blogging prompt.
        AnswerPrompt => "answer-prompt",
    }
}

/// A notification.
///
/// Only [`Self::id`] is certain. What else it has depends on the fields asked for: without
/// `body` there's no header or body, and the note context in [`Self::meta`] is only there when
/// asked for.
//
// Not called `Notification`: in Swift that name would hide Foundation's `Notification`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct WpComNotification {
    #[serde(deserialize_with = "lenient::required_id")]
    pub id: NotificationId,
    #[serde(rename = "type", default, deserialize_with = "lenient::kind")]
    #[uniffi(default = None)]
    pub note_type: Option<NotificationType>,
    /// A finer type within some types, such as what kind of thing a review is about.
    #[serde(default, deserialize_with = "lenient::text")]
    #[uniffi(default = None)]
    pub subtype: Option<String>,
    /// Which version of a note's wording it has, for types that are worded more than one way.
    #[serde(default, deserialize_with = "lenient::text")]
    #[uniffi(default = None)]
    pub variant: Option<String>,
    /// Whether the user has read it.
    #[serde(default, deserialize_with = "lenient::flag")]
    #[uniffi(default = None)]
    pub read: Option<bool>,
    /// When it happened. A note with no date of its own has a time at the very start of 1970.
    #[serde(default, deserialize_with = "lenient::date")]
    #[uniffi(default = None)]
    pub timestamp: Option<WpGmtDateTime>,
    /// An image for the note, such as the avatar of who it's from.
    #[serde(default, deserialize_with = "lenient::text")]
    #[uniffi(default = None)]
    pub icon: Option<String>,
    /// A character in the Noticons font that shows the note's type.
    #[serde(default, deserialize_with = "lenient::text")]
    #[uniffi(default = None)]
    pub noticon: Option<String>,
    /// A short title for the note's type, such as "Comment".
    #[serde(default, deserialize_with = "lenient::text")]
    #[uniffi(default = None)]
    pub title: Option<String>,
    /// Where the note leads, such as the comment's address. Many notes have none.
    #[serde(default, deserialize_with = "lenient::text")]
    #[uniffi(default = None)]
    pub url: Option<String>,
    /// Changes when anything in the note changes, including that it was read. Only sent when
    /// asked for. It also changes between requests that ask for different fields or in a
    /// different language, so only compare hashes from requests made the same way.
    #[serde(default, deserialize_with = "lenient::unsigned")]
    #[uniffi(default = None)]
    pub note_hash: Option<u64>,
    /// What the note says, in a line or two.
    #[serde(default, deserialize_with = "lenient::list")]
    #[uniffi(default = [])]
    pub subject: Vec<NotificationText>,
    /// A line above the body, such as the post a comment is on. Most notes have none.
    #[serde(default, deserialize_with = "lenient::list")]
    #[uniffi(default = [])]
    pub header: Vec<NotificationText>,
    /// The note's details: who it's from, what they wrote, and what can be done about it.
    #[serde(default, deserialize_with = "lenient::list")]
    #[uniffi(default = [])]
    pub body: Vec<NotificationBlock>,
    #[serde(default, deserialize_with = "lenient::object")]
    #[uniffi(default = None)]
    pub meta: Option<NotificationMeta>,
}

/// A line of text with its ranges and media: a note's subject or header, or a context summary.
///
/// [`notification_text_styled_runs`] turns it into runs to show.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct NotificationText {
    /// Plain text. It can be empty while its ranges and media still say something.
    #[serde(default, deserialize_with = "lenient::text_or_empty")]
    pub text: String,
    #[serde(default, deserialize_with = "lenient::list")]
    #[uniffi(default = [])]
    pub ranges: Vec<NotificationRange>,
    #[serde(default, deserialize_with = "lenient::list")]
    #[uniffi(default = [])]
    pub media: Vec<NotificationMedia>,
}

/// A block of a note's body.
///
/// [`notification_block_styled_runs`] turns its text into runs to show.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct NotificationBlock {
    /// Plain text. It can be empty while its ranges and media still say something.
    #[serde(default, deserialize_with = "lenient::text_or_empty")]
    pub text: String,
    #[serde(default, deserialize_with = "lenient::list")]
    #[uniffi(default = [])]
    pub ranges: Vec<NotificationRange>,
    #[serde(default, deserialize_with = "lenient::list")]
    #[uniffi(default = [])]
    pub media: Vec<NotificationMedia>,
    /// `None` for a block of text.
    #[serde(rename = "type", default, deserialize_with = "lenient::kind")]
    #[uniffi(default = None)]
    pub kind: Option<NotificationBlockKind>,
    /// What the user can do with what the block shows, in the order the API sent them.
    #[serde(default, deserialize_with = "lenient::actions")]
    #[uniffi(default = [])]
    pub actions: Vec<NotificationAction>,
    #[serde(default, deserialize_with = "lenient::object")]
    #[uniffi(default = None)]
    pub meta: Option<NotificationBlockMeta>,
    /// How deep a comment is in its thread.
    #[serde(default, deserialize_with = "lenient::small_unsigned")]
    #[uniffi(default = None)]
    pub nest_level: Option<u32>,
    /// Where a comment can be edited in the site's dashboard.
    #[serde(default, deserialize_with = "lenient::text")]
    #[uniffi(default = None)]
    pub edit_comment_link: Option<String>,
    /// Where the site's comments can be moderated in its dashboard.
    #[serde(default, deserialize_with = "lenient::text")]
    #[uniffi(default = None)]
    pub moderate_comments_link: Option<String>,
    /// Where to start a new post, such as one that answers a blogging prompt.
    #[serde(default, deserialize_with = "lenient::text")]
    #[uniffi(default = None)]
    pub new_post_link: Option<String>,
}

/// Something the user can do from a block, and its state now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct NotificationAction {
    pub kind: NotificationActionKind,
    /// Whether it's done: the comment is approved or liked, the post liked, the site followed.
    /// `None` when the API sent something other than a yes or no.
    #[uniffi(default = None)]
    pub is_on: Option<bool>,
}

/// A part of a block's text that's styled, linked or stands for something.
///
/// Ranges can overlap and nest, and one can cover no text at all.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct NotificationRange {
    #[serde(rename = "type", default, deserialize_with = "lenient::kind")]
    #[uniffi(default = None)]
    pub kind: Option<NotificationRangeKind>,
    /// Where it is in the text.
    #[serde(default, deserialize_with = "lenient::indices")]
    #[uniffi(default = None)]
    pub indices: Option<NotificationIndices>,
    #[serde(default, deserialize_with = "lenient::text")]
    #[uniffi(default = None)]
    pub url: Option<String>,
    /// What the API calls the range's ID, as text. For a person, site, post or comment range,
    /// it's that thing's ID. For other ranges it's a number that changes from one request to the
    /// next and means nothing outside the response: don't keep it, and don't compare notes by it.
    #[serde(default, deserialize_with = "lenient::text")]
    #[uniffi(default = None)]
    pub id: Option<String>,
    /// The `id` of the range this one is inside, in the same response. Like `id`, it means
    /// nothing outside the response.
    #[serde(default, deserialize_with = "lenient::text")]
    #[uniffi(default = None)]
    pub parent: Option<String>,
    /// The site a site, post or comment range is on.
    #[serde(default, deserialize_with = "lenient::id")]
    #[uniffi(default = None)]
    pub site_id: Option<WpComSiteId>,
    /// The post a comment range is on.
    #[serde(default, deserialize_with = "lenient::id")]
    #[uniffi(default = None)]
    pub post_id: Option<PostId>,
    /// The character a [`NotificationRangeKind::Noticon`] range shows.
    #[serde(default, deserialize_with = "lenient::text")]
    #[uniffi(default = None)]
    pub value: Option<String>,
    /// CSS classes for the range's element.
    #[serde(rename = "class", default, deserialize_with = "lenient::text")]
    #[uniffi(default = None)]
    pub class_name: Option<String>,
    /// CSS for the range's element.
    #[serde(default, deserialize_with = "lenient::text")]
    #[uniffi(default = None)]
    pub style: Option<String>,
    /// The name the web panel records a click on a [`NotificationRangeKind::Tracks`] or
    /// [`NotificationRangeKind::Button`] range under.
    #[serde(default, deserialize_with = "lenient::text")]
    #[uniffi(default = None)]
    pub context: Option<String>,
}

/// Where a range or medium is in its block's text, in UTF-16 units: from `start` up to, but not
/// including, `end`. They're equal for one that covers no text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct NotificationIndices {
    pub start: u32,
    pub end: u32,
}

/// An image in a block, such as an avatar or a badge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct NotificationMedia {
    #[serde(rename = "type", default, deserialize_with = "lenient::kind")]
    #[uniffi(default = None)]
    pub kind: Option<NotificationMediaKind>,
    /// Where it goes in the text. It usually covers no text, or the text it replaces.
    #[serde(default, deserialize_with = "lenient::indices")]
    #[uniffi(default = None)]
    pub indices: Option<NotificationIndices>,
    #[serde(default, deserialize_with = "lenient::text")]
    #[uniffi(default = None)]
    pub url: Option<String>,
    /// As sent: usually a number of pixels, but it can be a percentage or a word.
    #[serde(default, deserialize_with = "lenient::text")]
    #[uniffi(default = None)]
    pub width: Option<String>,
    /// As sent: usually a number of pixels, but it can be a percentage or a word.
    #[serde(default, deserialize_with = "lenient::text")]
    #[uniffi(default = None)]
    pub height: Option<String>,
    /// Like a range's `id`, it means nothing outside the response.
    #[serde(default, deserialize_with = "lenient::text")]
    #[uniffi(default = None)]
    pub id: Option<String>,
    /// Like a range's `parent`, it means nothing outside the response.
    #[serde(default, deserialize_with = "lenient::text")]
    #[uniffi(default = None)]
    pub parent: Option<String>,
    /// Whether it's an emoji drawn as an image, to be shown at the size of the text.
    #[serde(default, deserialize_with = "lenient::flag")]
    #[uniffi(default = None)]
    pub emoji: Option<bool>,
}

/// What a body block refers to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct NotificationBlockMeta {
    #[serde(default, deserialize_with = "lenient::object")]
    #[uniffi(default = None)]
    pub ids: Option<NotificationIds>,
    /// Sent as an empty list when there are none, which reads as `None`.
    #[serde(default, deserialize_with = "lenient::object")]
    #[uniffi(default = None)]
    pub links: Option<NotificationLinks>,
    #[serde(default, deserialize_with = "lenient::object")]
    #[uniffi(default = None)]
    pub titles: Option<NotificationTitles>,
    /// Whether the block is a button that only the apps show.
    #[serde(default, deserialize_with = "lenient::flag")]
    #[uniffi(default = None)]
    pub is_mobile_button: Option<bool>,
}

/// The titles of a block's site.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct NotificationTitles {
    /// The site's name.
    #[serde(default, deserialize_with = "lenient::text")]
    #[uniffi(default = None)]
    pub home: Option<String>,
    #[serde(default, deserialize_with = "lenient::text")]
    #[uniffi(default = None)]
    pub tagline: Option<String>,
}

/// What a note, or one of its blocks, refers to, by ID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
#[serde(from = "lenient::Entries")]
pub struct NotificationIds {
    #[uniffi(default = None)]
    pub site: Option<WpComSiteId>,
    #[uniffi(default = None)]
    pub post: Option<PostId>,
    #[uniffi(default = None)]
    pub comment: Option<CommentId>,
    #[uniffi(default = None)]
    pub user: Option<WpComUserId>,
    /// The comment that [`Self::comment`] replies to.
    #[uniffi(default = None)]
    pub parent_comment: Option<CommentId>,
    /// The user's own reply to [`Self::comment`].
    #[uniffi(default = None)]
    pub reply_comment: Option<CommentId>,
    /// Other IDs some types send, such as an order's, by the API's name for them. Only those
    /// that are numbers.
    pub other: HashMap<String, u64>,
}

/// Addresses of what a note, or one of its blocks, refers to. These are API addresses for a
/// site, post, comment or user, and a web address for `home`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
#[serde(from = "lenient::Entries")]
pub struct NotificationLinks {
    #[uniffi(default = None)]
    pub site: Option<String>,
    #[uniffi(default = None)]
    pub post: Option<String>,
    #[uniffi(default = None)]
    pub comment: Option<String>,
    #[uniffi(default = None)]
    pub user: Option<String>,
    #[uniffi(default = None)]
    pub parent_comment: Option<String>,
    #[uniffi(default = None)]
    pub reply_comment: Option<String>,
    /// The site's home page.
    #[uniffi(default = None)]
    pub home: Option<String>,
    /// Other addresses some types send, by the API's name for them.
    pub other: HashMap<String, String>,
}

/// What a note refers to, and, when the note context was asked for, a summary of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct NotificationMeta {
    #[serde(default, deserialize_with = "lenient::object")]
    #[uniffi(default = None)]
    pub ids: Option<NotificationIds>,
    #[serde(default, deserialize_with = "lenient::object")]
    #[uniffi(default = None)]
    pub links: Option<NotificationLinks>,
    /// Who the note is from: the first of them when there's more than one. Note context.
    #[serde(default, deserialize_with = "lenient::object")]
    #[uniffi(default = None)]
    pub actor: Option<NotificationActor>,
    /// How many people the note is from. Note context.
    #[serde(default, deserialize_with = "lenient::unsigned")]
    #[uniffi(default = None)]
    pub actor_count: Option<u64>,
    /// A line that sums up the note. Note context.
    #[serde(default, deserialize_with = "lenient::object")]
    #[uniffi(default = None)]
    pub summary: Option<NotificationText>,
    /// A heading for the note. Note context, and only some types have one.
    #[serde(default, deserialize_with = "lenient::text")]
    #[uniffi(default = None)]
    pub heading: Option<String>,
    /// The post the note is about. Note context.
    #[serde(default, deserialize_with = "lenient::object")]
    #[uniffi(default = None)]
    pub post: Option<NotificationPostContext>,
    /// The site the note is about. Note context.
    #[serde(default, deserialize_with = "lenient::object")]
    #[uniffi(default = None)]
    pub site: Option<NotificationSiteContext>,
    /// The comment that a comment note replies to. Note context.
    #[serde(default, deserialize_with = "lenient::object")]
    #[uniffi(default = None)]
    pub parent_comment: Option<NotificationParentComment>,
}

/// Who a note is from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct NotificationActor {
    #[serde(default, deserialize_with = "lenient::text")]
    #[uniffi(default = None)]
    pub name: Option<String>,
    #[serde(default, deserialize_with = "lenient::text")]
    #[uniffi(default = None)]
    pub avatar: Option<String>,
    /// Their website. Many have none.
    #[serde(default, deserialize_with = "lenient::text")]
    #[uniffi(default = None)]
    pub url: Option<String>,
}

/// The post a note is about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct NotificationPostContext {
    #[serde(default, deserialize_with = "lenient::text")]
    #[uniffi(default = None)]
    pub title: Option<String>,
    #[serde(default, deserialize_with = "lenient::text")]
    #[uniffi(default = None)]
    pub url: Option<String>,
    /// When it was published.
    #[serde(default, deserialize_with = "lenient::date")]
    #[uniffi(default = None)]
    pub date: Option<WpGmtDateTime>,
    #[serde(default, deserialize_with = "lenient::text")]
    #[uniffi(default = None)]
    pub author_name: Option<String>,
    #[serde(default, deserialize_with = "lenient::text")]
    #[uniffi(default = None)]
    pub author_avatar: Option<String>,
}

/// The site a note is about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct NotificationSiteContext {
    #[serde(default, deserialize_with = "lenient::text")]
    #[uniffi(default = None)]
    pub name: Option<String>,
    #[serde(default, deserialize_with = "lenient::text")]
    #[uniffi(default = None)]
    pub icon: Option<String>,
    #[serde(default, deserialize_with = "lenient::text")]
    #[uniffi(default = None)]
    pub url: Option<String>,
}

/// The comment a comment note replies to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct NotificationParentComment {
    /// The comment as plain text.
    #[serde(default, deserialize_with = "lenient::text")]
    #[uniffi(default = None)]
    pub text: Option<String>,
    /// The comment with its ranges and media.
    #[serde(default, deserialize_with = "lenient::object")]
    #[uniffi(default = None)]
    pub content: Option<NotificationBlock>,
    #[serde(default, deserialize_with = "lenient::text")]
    #[uniffi(default = None)]
    pub url: Option<String>,
    #[serde(default, deserialize_with = "lenient::date")]
    #[uniffi(default = None)]
    pub date: Option<WpGmtDateTime>,
    #[serde(default, deserialize_with = "lenient::text")]
    #[uniffi(default = None)]
    pub author_name: Option<String>,
    #[serde(default, deserialize_with = "lenient::text")]
    #[uniffi(default = None)]
    pub author_avatar: Option<String>,
    #[serde(default, deserialize_with = "lenient::text")]
    #[uniffi(default = None)]
    pub author_url: Option<String>,
}

/// Which notes to list, newest first.
///
/// A page with fewer notes than asked for, or none, doesn't mean there are no older ones: the
/// API can leave out notes it can't show after it has picked the page. To ask for older notes,
/// set `before` to the oldest note's time. `before` includes that second, so the oldest note
/// comes back again.
#[derive(Debug, Default, Clone, PartialEq, Eq, uniffi::Record)]
pub struct NotificationsQuery {
    /// How many notes to return: 200 at most.
    #[uniffi(default = None)]
    pub number: Option<u32>,
    /// Only notes from this second or earlier. Sent as seconds since 1970.
    #[uniffi(default = None)]
    pub before: Option<WpGmtDateTime>,
    /// Only notes from this time on. Sent as seconds since 1970.
    #[uniffi(default = None)]
    pub since: Option<WpGmtDateTime>,
    /// Only these notes.
    #[uniffi(default = [])]
    pub ids: Vec<NotificationId>,
    /// Only notes of these types.
    #[uniffi(default = [])]
    pub types: Vec<NotificationType>,
    /// Only notes the user hasn't read.
    #[uniffi(default = false)]
    pub unread: bool,
    /// Only notes the user has read.
    #[uniffi(default = false)]
    pub read: bool,
    /// The language to write the notes in, instead of the client's. A request in a different
    /// language from the one before it changes every note's hash.
    #[uniffi(default = None)]
    pub locale: Option<WPComLanguage>,
    /// A name for the app making the request. What it changes wasn't checked.
    #[uniffi(default = None)]
    pub source: Option<String>,
}

impl AppendUrlQueryPairs for NotificationsQuery {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut
            .append_option_query_value_pair("number", self.number.as_ref())
            .append_option_query_value_pair(
                "before",
                self.before.map(|before| before.0.timestamp()).as_ref(),
            )
            .append_option_query_value_pair(
                "since",
                self.since.map(|since| since.0.timestamp()).as_ref(),
            )
            .append_vec_query_value_pair("ids", &self.ids)
            .append_vec_query_value_pair("type", &self.types)
            .append_if_true("unread", self.unread)
            .append_if_true("read", self.read)
            .append_option_query_value_pair("locale", self.locale.as_ref())
            .append_option_query_value_pair("source", self.source.as_ref());
    }
}

/// Which notes to list, and what to send of each.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct NotificationsParams {
    pub query: NotificationsQuery,
    /// Send each note's header and body. Without them a page is much smaller, which is enough
    /// for a list.
    #[uniffi(default = true)]
    pub include_body: bool,
    /// Send the note context in each note's [`WpComNotification::meta`]: who it's from, and a
    /// summary of the post, site and comment it's about.
    #[uniffi(default = false)]
    pub include_context: bool,
}

impl Default for NotificationsParams {
    fn default() -> Self {
        Self {
            query: NotificationsQuery::default(),
            include_body: true,
            include_context: false,
        }
    }
}

impl AppendUrlQueryPairs for NotificationsParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        self.query.append_query_pairs(query_pairs_mut);
        query_pairs_mut.append_pair(
            "fields",
            &note_fields(self.include_body, self.include_context),
        );
    }
}

/// What to send of the one note asked for.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct NotificationParams {
    /// Send the note's header and body.
    #[uniffi(default = true)]
    pub include_body: bool,
    /// Send the note context in [`WpComNotification::meta`].
    #[uniffi(default = false)]
    pub include_context: bool,
    /// The language to write the note in, instead of the client's.
    #[uniffi(default = None)]
    pub locale: Option<WPComLanguage>,
    /// A name for the app making the request. What it changes wasn't checked.
    #[uniffi(default = None)]
    pub source: Option<String>,
}

impl Default for NotificationParams {
    fn default() -> Self {
        Self {
            include_body: true,
            include_context: false,
            locale: None,
            source: None,
        }
    }
}

impl AppendUrlQueryPairs for NotificationParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut
            .append_option_query_value_pair("locale", self.locale.as_ref())
            .append_option_query_value_pair("source", self.source.as_ref())
            .append_pair(
                "fields",
                &note_fields(self.include_body, self.include_context),
            );
    }
}

/// The `fields` value for whole notes. `note_hash` and `note_context` aren't fields: each turns
/// on what it names, and only when asked for.
pub(crate) fn note_fields(include_body: bool, include_context: bool) -> String {
    let mut fields = String::from(
        "id,type,subtype,variant,read,timestamp,icon,noticon,title,url,subject,meta,note_hash",
    );
    if include_body {
        fields.push_str(",header,body");
    }
    if include_context {
        fields.push_str(",note_context");
    }
    fields
}

/// The `fields` value for a list of IDs.
pub(crate) const NOTE_ID_FIELDS: &str = "id";
/// The `fields` value for a list of IDs and hashes.
pub(crate) const NOTE_HASH_FIELDS: &str = "id,note_hash";

/// A page of notes, or the one note asked for.
///
/// Asking for a note that doesn't exist, or that the user can't see, isn't an error: the page
/// has no notes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
#[serde(from = "RawNotificationsResponse")]
pub struct NotificationsResponse {
    /// The notes, newest first, without any in [`Self::unreadable_notes`].
    pub notes: Vec<WpComNotification>,
    /// The notes on the page that couldn't be read. Almost always empty.
    #[uniffi(default = [])]
    pub unreadable_notes: Vec<UnreadableNotification>,
    /// When the user last looked at their notifications.
    #[uniffi(default = None)]
    pub last_seen_time: Option<WpGmtDateTime>,
}

#[derive(Deserialize)]
struct RawNotificationsResponse {
    #[serde(default)]
    notes: Option<LenientVec<WpComNotification>>,
    #[serde(default, deserialize_with = "lenient::date")]
    last_seen_time: Option<WpGmtDateTime>,
}

impl From<RawNotificationsResponse> for NotificationsResponse {
    fn from(raw: RawNotificationsResponse) -> Self {
        let (notes, unreadable_notes) = sort_notes(raw.notes);
        Self {
            notes,
            unreadable_notes,
            last_seen_time: raw.last_seen_time,
        }
    }
}

/// A page of notes' IDs and hashes, newest first. Comparing them with the ones kept shows which
/// notes changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
#[serde(from = "RawNotificationHashesResponse")]
pub struct NotificationHashesResponse {
    pub notes: Vec<NotificationHash>,
    /// The notes on the page that couldn't be read. Almost always empty.
    #[uniffi(default = [])]
    pub unreadable_notes: Vec<UnreadableNotification>,
    /// When the user last looked at their notifications.
    #[uniffi(default = None)]
    pub last_seen_time: Option<WpGmtDateTime>,
}

/// A note's ID and hash.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct NotificationHash {
    #[serde(deserialize_with = "lenient::required_id")]
    pub id: NotificationId,
    /// See [`WpComNotification::note_hash`].
    #[serde(default, deserialize_with = "lenient::unsigned")]
    #[uniffi(default = None)]
    pub note_hash: Option<u64>,
}

#[derive(Deserialize)]
struct RawNotificationHashesResponse {
    #[serde(default)]
    notes: Option<LenientVec<NotificationHash>>,
    #[serde(default, deserialize_with = "lenient::date")]
    last_seen_time: Option<WpGmtDateTime>,
}

impl From<RawNotificationHashesResponse> for NotificationHashesResponse {
    fn from(raw: RawNotificationHashesResponse) -> Self {
        let (notes, unreadable_notes) = sort_notes(raw.notes);
        Self {
            notes,
            unreadable_notes,
            last_seen_time: raw.last_seen_time,
        }
    }
}

/// A page of notes' IDs, newest first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
#[serde(from = "RawNotificationIdsResponse")]
pub struct NotificationIdsResponse {
    pub ids: Vec<NotificationId>,
    /// The notes on the page that couldn't be read. Almost always empty.
    #[uniffi(default = [])]
    pub unreadable_notes: Vec<UnreadableNotification>,
    /// When the user last looked at their notifications.
    #[uniffi(default = None)]
    pub last_seen_time: Option<WpGmtDateTime>,
}

#[derive(Deserialize)]
struct NoteIdOnly {
    #[serde(deserialize_with = "lenient::required_id")]
    id: NotificationId,
}

#[derive(Deserialize)]
struct RawNotificationIdsResponse {
    #[serde(default)]
    notes: Option<LenientVec<NoteIdOnly>>,
    #[serde(default, deserialize_with = "lenient::date")]
    last_seen_time: Option<WpGmtDateTime>,
}

impl From<RawNotificationIdsResponse> for NotificationIdsResponse {
    fn from(raw: RawNotificationIdsResponse) -> Self {
        let (notes, unreadable_notes) = sort_notes(raw.notes);
        Self {
            ids: notes.into_iter().map(|note| note.id).collect(),
            unreadable_notes,
            last_seen_time: raw.last_seen_time,
        }
    }
}

/// A page of whole notes, each with its JSON as the API sent it, for a cache that keeps notes to
/// read again later. Keeping the JSON lets a later version of the library read fields this one
/// doesn't model from notes already stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
#[serde(from = "RawNotificationsWithJsonResponse")]
pub struct NotificationsWithJsonResponse {
    /// The notes, newest first, without any in [`Self::unreadable_notes`].
    pub notes: Vec<NotificationWithJson>,
    /// The notes on the page that couldn't be read. Almost always empty.
    #[uniffi(default = [])]
    pub unreadable_notes: Vec<UnreadableNotification>,
    /// When the user last looked at their notifications.
    #[uniffi(default = None)]
    pub last_seen_time: Option<WpGmtDateTime>,
}

/// A note, and the JSON it was read from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, uniffi::Record)]
pub struct NotificationWithJson {
    pub note: WpComNotification,
    /// The note's JSON object, as the API sent it.
    pub json: String,
}

impl<'de> Deserialize<'de> for NotificationWithJson {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let json = Box::<serde_json::value::RawValue>::deserialize(deserializer)?;
        let note = serde_json::from_str(json.get()).map_err(serde::de::Error::custom)?;
        Ok(Self {
            note,
            json: json.get().to_string(),
        })
    }
}

#[derive(Deserialize)]
struct RawNotificationsWithJsonResponse {
    #[serde(default)]
    notes: Option<LenientVec<NotificationWithJson>>,
    #[serde(default, deserialize_with = "lenient::date")]
    last_seen_time: Option<WpGmtDateTime>,
}

impl From<RawNotificationsWithJsonResponse> for NotificationsWithJsonResponse {
    fn from(raw: RawNotificationsWithJsonResponse) -> Self {
        let (notes, unreadable_notes) = sort_notes(raw.notes);
        Self {
            notes,
            unreadable_notes,
            last_seen_time: raw.last_seen_time,
        }
    }
}

/// A note on a page that couldn't be read. It doesn't fail the page: the page keeps the notes
/// it can read, and lists the others as these.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct UnreadableNotification {
    /// Which note it is. `None` when its ID couldn't be read either.
    #[uniffi(default = None)]
    pub id: Option<NotificationId>,
    /// The note's type, as the API sent it, when that could be read.
    #[uniffi(default = None)]
    pub note_type: Option<String>,
    /// Why it couldn't be read. A line and column in it count from the start of the note.
    pub error: String,
}

/// The ID and type of a note that couldn't be read, where those can be read.
#[derive(Deserialize)]
struct NoteIdentity {
    #[serde(default, deserialize_with = "lenient::id")]
    id: Option<NotificationId>,
    #[serde(rename = "type", default, deserialize_with = "lenient::text")]
    note_type: Option<String>,
}

/// Sorts a page's notes into the ones that were read and the ones that weren't.
fn sort_notes<T>(notes: Option<LenientVec<T>>) -> (Vec<T>, Vec<UnreadableNotification>) {
    let notes = notes.unwrap_or_default();
    let unreadable = notes
        .invalid
        .into_iter()
        .map(|note| {
            let identity = serde_json::from_str::<NoteIdentity>(note.json.get()).ok();
            UnreadableNotification {
                id: identity.as_ref().and_then(|identity| identity.id),
                note_type: identity.and_then(|identity| identity.note_type),
                error: note.error.to_string(),
            }
        })
        .collect();
    (notes.valid, unreadable)
}

/// Readers that take whatever the API sends for a value and make the best of it: a value of the
/// wrong type reads as absent rather than failing the note.
mod lenient {
    use super::{NotificationAction, NotificationIds, NotificationIndices, NotificationLinks};
    use crate::{
        comments::CommentId, date::WpGmtDateTime, posts::PostId, wp_com::WpComSiteId,
        wp_com::me::WpComUserId,
    };
    use serde::{
        Deserialize, Deserializer,
        de::{self, IgnoredAny, MapAccess, SeqAccess, Visitor, value::MapAccessDeserializer},
    };
    use std::{fmt, marker::PhantomData, str::FromStr};

    /// Any JSON value, with only scalars kept.
    pub(super) enum Scalar {
        Text(String),
        Unsigned(u64),
        Signed(i64),
        Float(f64),
        Bool(bool),
        Null,
        /// A list or an object, which was skipped.
        Composite,
    }

    impl Scalar {
        /// A non-empty string, or a number written out.
        fn into_text(self) -> Option<String> {
            match self {
                Scalar::Text(text) if !text.is_empty() => Some(text),
                Scalar::Unsigned(number) => Some(number.to_string()),
                Scalar::Signed(number) => Some(number.to_string()),
                Scalar::Float(number) => Some(number.to_string()),
                _ => None,
            }
        }

        fn as_u64(&self) -> Option<u64> {
            match self {
                Scalar::Unsigned(number) => Some(*number),
                Scalar::Text(text) => text.trim().parse().ok(),
                _ => None,
            }
        }

        fn as_bool(&self) -> Option<bool> {
            match self {
                Scalar::Bool(flag) => Some(*flag),
                Scalar::Unsigned(0) => Some(false),
                Scalar::Unsigned(1) => Some(true),
                Scalar::Text(text) => match text.as_str() {
                    "0" | "false" => Some(false),
                    "1" | "true" => Some(true),
                    _ => None,
                },
                _ => None,
            }
        }
    }

    impl<'de> Deserialize<'de> for Scalar {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            deserializer.deserialize_any(ScalarVisitor)
        }
    }

    struct ScalarVisitor;

    impl<'de> Visitor<'de> for ScalarVisitor {
        type Value = Scalar;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("any JSON value")
        }

        fn visit_bool<E>(self, value: bool) -> Result<Scalar, E> {
            Ok(Scalar::Bool(value))
        }

        fn visit_i64<E>(self, value: i64) -> Result<Scalar, E> {
            Ok(u64::try_from(value).map_or(Scalar::Signed(value), Scalar::Unsigned))
        }

        fn visit_u64<E>(self, value: u64) -> Result<Scalar, E> {
            Ok(Scalar::Unsigned(value))
        }

        fn visit_f64<E>(self, value: f64) -> Result<Scalar, E> {
            Ok(Scalar::Float(value))
        }

        fn visit_str<E>(self, value: &str) -> Result<Scalar, E> {
            Ok(Scalar::Text(value.to_string()))
        }

        fn visit_string<E>(self, value: String) -> Result<Scalar, E> {
            Ok(Scalar::Text(value))
        }

        fn visit_unit<E>(self) -> Result<Scalar, E> {
            Ok(Scalar::Null)
        }

        fn visit_none<E>(self) -> Result<Scalar, E> {
            Ok(Scalar::Null)
        }

        fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Scalar, D::Error> {
            Scalar::deserialize(deserializer)
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Scalar, A::Error> {
            while seq.next_element::<IgnoredAny>()?.is_some() {}
            Ok(Scalar::Composite)
        }

        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Scalar, A::Error> {
            while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
            Ok(Scalar::Composite)
        }
    }

    pub(super) fn text<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
        Ok(Scalar::deserialize(d)?.into_text())
    }

    pub(super) fn text_or_empty<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
        Ok(Scalar::deserialize(d)?.into_text().unwrap_or_default())
    }

    pub(super) fn kind<'de, D, T>(d: D) -> Result<Option<T>, D::Error>
    where
        D: Deserializer<'de>,
        T: From<String>,
    {
        Ok(text(d)?.map(T::from))
    }

    pub(super) fn unsigned<'de, D: Deserializer<'de>>(d: D) -> Result<Option<u64>, D::Error> {
        Ok(Scalar::deserialize(d)?.as_u64())
    }

    pub(super) fn small_unsigned<'de, D: Deserializer<'de>>(d: D) -> Result<Option<u32>, D::Error> {
        Ok(unsigned(d)?.and_then(|number| u32::try_from(number).ok()))
    }

    pub(super) fn flag<'de, D: Deserializer<'de>>(d: D) -> Result<Option<bool>, D::Error> {
        Ok(Scalar::deserialize(d)?.as_bool())
    }

    /// A date as text in any form [`WpGmtDateTime`] reads, or seconds since 1970 as a number.
    pub(super) fn date<'de, D: Deserializer<'de>>(d: D) -> Result<Option<WpGmtDateTime>, D::Error> {
        Ok(match Scalar::deserialize(d)? {
            Scalar::Text(text) => WpGmtDateTime::from_str(text.trim()).ok(),
            Scalar::Unsigned(seconds) => i64::try_from(seconds).ok().and_then(from_seconds),
            Scalar::Signed(seconds) => from_seconds(seconds),
            _ => None,
        })
    }

    fn from_seconds(seconds: i64) -> Option<WpGmtDateTime> {
        wp_serde_helper::wp_date_time_from_timestamp(seconds)
            .ok()
            .map(WpGmtDateTime)
    }

    /// The ID types a note holds.
    pub(super) trait FromId: Sized {
        fn from_id(id: u64) -> Option<Self>;
    }

    impl FromId for super::NotificationId {
        fn from_id(id: u64) -> Option<Self> {
            Some(Self(id))
        }
    }

    impl FromId for WpComSiteId {
        fn from_id(id: u64) -> Option<Self> {
            Some(Self(id))
        }
    }

    impl FromId for WpComUserId {
        fn from_id(id: u64) -> Option<Self> {
            Some(Self(id))
        }
    }

    impl FromId for PostId {
        fn from_id(id: u64) -> Option<Self> {
            i64::try_from(id).ok().map(Self)
        }
    }

    impl FromId for CommentId {
        fn from_id(id: u64) -> Option<Self> {
            i64::try_from(id).ok().map(Self)
        }
    }

    fn scalar_id<T: FromId>(scalar: &Scalar) -> Option<T> {
        scalar.as_u64().filter(|id| *id != 0).and_then(T::from_id)
    }

    /// An ID as a number or a string. `0` and anything else read as `None`.
    pub(super) fn id<'de, D, T>(d: D) -> Result<Option<T>, D::Error>
    where
        D: Deserializer<'de>,
        T: FromId,
    {
        Ok(scalar_id(&Scalar::deserialize(d)?))
    }

    /// An ID the value can't do without.
    pub(super) fn required_id<'de, D, T>(d: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        T: FromId,
    {
        id(d)?.ok_or_else(|| de::Error::custom("expected an ID that is a number above 0"))
    }

    /// `[start, end]`. Anything else reads as `None`.
    pub(super) fn indices<'de, D: Deserializer<'de>>(
        d: D,
    ) -> Result<Option<NotificationIndices>, D::Error> {
        struct IndicesVisitor;

        impl<'de> Visitor<'de> for IndicesVisitor {
            type Value = Option<NotificationIndices>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("any JSON value")
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut positions = Vec::new();
                while let Some(position) = seq.next_element::<Scalar>()? {
                    positions.push(position.as_u64().and_then(|p| u32::try_from(p).ok()));
                }
                Ok(match positions[..] {
                    [Some(start), Some(end)] => Some(NotificationIndices { start, end }),
                    _ => None,
                })
            }

            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
                ScalarVisitor.visit_map(map).map(|_| None)
            }

            fn visit_bool<E>(self, _: bool) -> Result<Self::Value, E> {
                Ok(None)
            }

            fn visit_i64<E>(self, _: i64) -> Result<Self::Value, E> {
                Ok(None)
            }

            fn visit_u64<E>(self, _: u64) -> Result<Self::Value, E> {
                Ok(None)
            }

            fn visit_f64<E>(self, _: f64) -> Result<Self::Value, E> {
                Ok(None)
            }

            fn visit_str<E>(self, _: &str) -> Result<Self::Value, E> {
                Ok(None)
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E> {
                Ok(None)
            }

            fn visit_none<E>(self) -> Result<Self::Value, E> {
                Ok(None)
            }

            fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
                d.deserialize_any(IndicesVisitor)
            }
        }

        d.deserialize_any(IndicesVisitor)
    }

    /// Reads an object as `T`, and anything else as `None`.
    struct ObjectVisitor<T>(PhantomData<T>);

    impl<'de, T: Deserialize<'de>> Visitor<'de> for ObjectVisitor<T> {
        type Value = Option<T>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("any JSON value")
        }

        fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
            T::deserialize(MapAccessDeserializer::new(map)).map(Some)
        }

        fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<Self::Value, A::Error> {
            ScalarVisitor.visit_seq(seq).map(|_| None)
        }

        fn visit_bool<E>(self, _: bool) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_i64<E>(self, _: i64) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_u64<E>(self, _: u64) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_f64<E>(self, _: f64) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_str<E>(self, _: &str) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_none<E>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
            d.deserialize_any(self)
        }
    }

    /// An object, or `None` for anything else, such as the empty list the API sends for an
    /// object with nothing in it.
    pub(super) fn object<'de, D, T>(d: D) -> Result<Option<T>, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        d.deserialize_any(ObjectVisitor(PhantomData))
    }

    /// One element of a [`list`].
    struct Element<T>(Option<T>);

    impl<'de, T: Deserialize<'de>> Deserialize<'de> for Element<T> {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            object(d).map(Element)
        }
    }

    /// A list of objects. An element that isn't an object is left out, and anything that isn't
    /// a list reads as an empty one.
    pub(super) fn list<'de, D, T>(d: D) -> Result<Vec<T>, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        struct ListVisitor<T>(PhantomData<T>);

        impl<'de, T: Deserialize<'de>> Visitor<'de> for ListVisitor<T> {
            type Value = Vec<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("any JSON value")
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut elements = Vec::new();
                while let Some(Element(element)) = seq.next_element::<Element<T>>()? {
                    elements.extend(element);
                }
                Ok(elements)
            }

            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
                ScalarVisitor.visit_map(map).map(|_| Vec::new())
            }

            fn visit_bool<E>(self, _: bool) -> Result<Self::Value, E> {
                Ok(Vec::new())
            }

            fn visit_i64<E>(self, _: i64) -> Result<Self::Value, E> {
                Ok(Vec::new())
            }

            fn visit_u64<E>(self, _: u64) -> Result<Self::Value, E> {
                Ok(Vec::new())
            }

            fn visit_f64<E>(self, _: f64) -> Result<Self::Value, E> {
                Ok(Vec::new())
            }

            fn visit_str<E>(self, _: &str) -> Result<Self::Value, E> {
                Ok(Vec::new())
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E> {
                Ok(Vec::new())
            }

            fn visit_none<E>(self) -> Result<Self::Value, E> {
                Ok(Vec::new())
            }

            fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
                d.deserialize_any(self)
            }
        }

        d.deserialize_any(ListVisitor(PhantomData))
    }

    /// An object's entries in the order they arrived, with only scalar values kept. Anything
    /// that isn't an object has none.
    pub(super) struct Entries(Vec<(String, Scalar)>);

    impl<'de> Deserialize<'de> for Entries {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            struct EntriesVisitor;

            impl<'de> Visitor<'de> for EntriesVisitor {
                type Value = Entries;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("any JSON value")
                }

                fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Entries, A::Error> {
                    let mut entries = Vec::new();
                    while let Some(entry) = map.next_entry::<String, Scalar>()? {
                        entries.push(entry);
                    }
                    Ok(Entries(entries))
                }

                fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<Entries, A::Error> {
                    ScalarVisitor.visit_seq(seq).map(|_| Entries(Vec::new()))
                }

                fn visit_bool<E>(self, _: bool) -> Result<Entries, E> {
                    Ok(Entries(Vec::new()))
                }

                fn visit_i64<E>(self, _: i64) -> Result<Entries, E> {
                    Ok(Entries(Vec::new()))
                }

                fn visit_u64<E>(self, _: u64) -> Result<Entries, E> {
                    Ok(Entries(Vec::new()))
                }

                fn visit_f64<E>(self, _: f64) -> Result<Entries, E> {
                    Ok(Entries(Vec::new()))
                }

                fn visit_str<E>(self, _: &str) -> Result<Entries, E> {
                    Ok(Entries(Vec::new()))
                }

                fn visit_unit<E>(self) -> Result<Entries, E> {
                    Ok(Entries(Vec::new()))
                }

                fn visit_none<E>(self) -> Result<Entries, E> {
                    Ok(Entries(Vec::new()))
                }

                fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<Entries, D::Error> {
                    d.deserialize_any(self)
                }
            }

            d.deserialize_any(EntriesVisitor)
        }
    }

    impl From<Entries> for NotificationIds {
        fn from(Entries(entries): Entries) -> Self {
            let mut ids = NotificationIds {
                site: None,
                post: None,
                comment: None,
                user: None,
                parent_comment: None,
                reply_comment: None,
                other: Default::default(),
            };
            for (key, value) in entries {
                match key.as_str() {
                    "site" => ids.site = scalar_id(&value),
                    "post" => ids.post = scalar_id(&value),
                    "comment" => ids.comment = scalar_id(&value),
                    "user" => ids.user = scalar_id(&value),
                    "parent_comment" => ids.parent_comment = scalar_id(&value),
                    "reply_comment" => ids.reply_comment = scalar_id(&value),
                    _ => {
                        if let Some(id) = value.as_u64() {
                            ids.other.insert(key, id);
                        }
                    }
                }
            }
            ids
        }
    }

    impl From<Entries> for NotificationLinks {
        fn from(Entries(entries): Entries) -> Self {
            let mut links = NotificationLinks {
                site: None,
                post: None,
                comment: None,
                user: None,
                parent_comment: None,
                reply_comment: None,
                home: None,
                other: Default::default(),
            };
            for (key, value) in entries {
                let Some(link) = value.into_text() else {
                    continue;
                };
                match key.as_str() {
                    "site" => links.site = Some(link),
                    "post" => links.post = Some(link),
                    "comment" => links.comment = Some(link),
                    "user" => links.user = Some(link),
                    "parent_comment" => links.parent_comment = Some(link),
                    "reply_comment" => links.reply_comment = Some(link),
                    "home" => links.home = Some(link),
                    _ => {
                        links.other.insert(key, link);
                    }
                }
            }
            links
        }
    }

    /// A block's actions: an object of each action's name and whether it's on.
    pub(super) fn actions<'de, D: Deserializer<'de>>(
        d: D,
    ) -> Result<Vec<NotificationAction>, D::Error> {
        let Entries(entries) = Entries::deserialize(d)?;
        Ok(entries
            .into_iter()
            .map(|(name, value)| NotificationAction {
                kind: name.into(),
                is_on: value.as_bool(),
            })
            .collect())
    }
}

mod styled_runs;
pub use styled_runs::{notification_block_styled_runs, notification_text_styled_runs};

mod layout;
pub use layout::{
    NotificationBlockRole, NotificationCommentLayout, NotificationFollowLayout,
    NotificationFollowOffer, NotificationHeadline, NotificationLabel, NotificationLayout,
    NotificationLayoutBlock, NotificationLayoutContent, NotificationLayoutFallback,
    NotificationLayoutText, NotificationMetaLine, NotificationNewPostByline,
    NotificationNewPostLayout, NotificationNoticeLayout, NotificationOthers, NotificationPostCard,
    NotificationPostLink, NotificationQuoteStyle, NotificationQuotedComment,
    NotificationReactionsLayout, NotificationSiteCard, notification_layout,
};

mod tab;
pub use tab::NotificationTab;

#[cfg(test)]
mod corpus;

#[cfg(test)]
mod fixture_check;

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use serde_json::json;

    fn page(json: &str) -> NotificationsResponse {
        serde_json::from_str(json).expect("the page parses")
    }

    fn note(value: serde_json::Value) -> WpComNotification {
        serde_json::from_value(value).expect("the note parses")
    }

    fn date(text: &str) -> Option<WpGmtDateTime> {
        Some(text.parse().unwrap())
    }

    /// A hand-built page with a comment, a like, a type the apps don't treat specially, and a
    /// follow with no date, asked for with the note context.
    #[test]
    fn test_deserialize_notes() {
        let page = page(include_str!("../../tests/wpcom/notifications/notes.json"));

        assert!(page.unreadable_notes.is_empty());
        assert_eq!(page.last_seen_time, date("2026-01-02T00:00:00Z"));
        let [comment, like, other, follow] = &page.notes[..] else {
            panic!("Expected four notes");
        };

        assert_eq!(comment.id, NotificationId(10_000_000_004));
        assert_eq!(comment.note_type, Some(NotificationType::Comment));
        assert_eq!(comment.read, Some(false));
        assert_eq!(comment.timestamp, date("2026-01-03T10:20:30Z"));
        assert_eq!(comment.noticon.as_deref(), Some("\u{f300}"));
        assert_eq!(comment.title.as_deref(), Some("Comment"));
        assert_eq!(comment.note_hash, Some(4_000_000_001));
        assert_eq!(comment.subject.len(), 2);
        let user_range = &comment.subject[0].ranges[0];
        assert_eq!(user_range.kind, Some(NotificationRangeKind::User));
        assert_eq!(
            user_range.indices,
            Some(NotificationIndices { start: 0, end: 11 })
        );
        assert_eq!(user_range.id.as_deref(), Some("701"), "Sent as a number");
        assert_eq!(user_range.parent, None);
        assert_eq!(user_range.site_id, Some(WpComSiteId(801)));
        let comment_range = &comment.subject[1].ranges[0];
        assert_eq!(comment_range.kind, Some(NotificationRangeKind::Comment));
        assert_eq!(comment_range.post_id, Some(PostId(601)));
        assert_eq!(comment.header.len(), 2);
        assert_eq!(
            comment.header[0].media[0].kind,
            Some(NotificationMediaKind::Image)
        );
        assert_eq!(comment.header[0].media[0].width.as_deref(), Some("256"));

        let [user_block, comment_block] = &comment.body[..] else {
            panic!("Expected two blocks");
        };
        assert_eq!(user_block.kind, Some(NotificationBlockKind::User));
        assert_eq!(
            user_block.actions,
            [NotificationAction {
                kind: NotificationActionKind::Follow,
                is_on: Some(false),
            }]
        );
        let user_meta = user_block.meta.as_ref().expect("block meta");
        assert_eq!(
            user_meta.ids.as_ref().and_then(|ids| ids.user),
            Some(WpComUserId(701))
        );
        assert_eq!(
            user_meta
                .links
                .as_ref()
                .and_then(|links| links.home.as_deref()),
            Some("https://sam.example.org")
        );
        assert_eq!(
            user_meta
                .titles
                .as_ref()
                .and_then(|titles| titles.tagline.as_deref()),
            Some("Notes from the garden")
        );
        assert_eq!(comment_block.kind, Some(NotificationBlockKind::Comment));
        assert_eq!(comment_block.nest_level, Some(0));
        assert!(comment_block.edit_comment_link.is_some());
        let actions: Vec<_> = comment_block
            .actions
            .iter()
            .map(|action| (action.kind.clone(), action.is_on))
            .collect();
        assert_eq!(
            actions,
            [
                (NotificationActionKind::SpamComment, Some(false)),
                (NotificationActionKind::TrashComment, Some(false)),
                (NotificationActionKind::ApproveComment, Some(true)),
                (NotificationActionKind::EditComment, Some(true)),
                (NotificationActionKind::ReplyToComment, Some(true)),
                (NotificationActionKind::LikeComment, Some(false)),
            ],
            "In the order the API sent them"
        );
        let [paragraph, bold] = &comment_block.ranges[..] else {
            panic!("Expected two ranges");
        };
        assert_eq!(
            paragraph.kind,
            Some(NotificationRangeKind::Other("p".to_string()))
        );
        assert_eq!(bold.kind, Some(NotificationRangeKind::Bold));
        assert_eq!(bold.parent.as_deref(), Some("10"));

        let meta = comment.meta.as_ref().expect("meta");
        let ids = meta.ids.as_ref().expect("ids");
        assert_eq!(ids.site, Some(WpComSiteId(801)));
        assert_eq!(ids.post, Some(PostId(601)));
        assert_eq!(ids.comment, Some(CommentId(501)));
        assert_eq!(ids.parent_comment, Some(CommentId(500)));
        assert!(ids.other.is_empty());
        let actor = meta.actor.as_ref().expect("an actor");
        assert_eq!(actor.name.as_deref(), Some("Sam Example"));
        assert_eq!(actor.url, None, "Sent as `null`");
        assert_eq!(meta.actor_count, Some(1));
        assert_eq!(
            meta.summary.as_ref().map(|summary| summary.text.as_str()),
            Some("Sam Example commented on your post")
        );
        let post = meta.post.as_ref().expect("a post");
        assert_eq!(post.title.as_deref(), Some("Hello World"));
        assert_eq!(post.date, date("2026-01-02T08:00:00Z"));
        assert_eq!(
            meta.site.as_ref().and_then(|site| site.name.as_deref()),
            Some("Example Site")
        );
        let parent = meta.parent_comment.as_ref().expect("a parent comment");
        assert_eq!(parent.text.as_deref(), Some("What did everyone think?"));
        assert_eq!(
            parent.content.as_ref().map(|content| content.ranges.len()),
            Some(1)
        );
        assert_eq!(parent.date, date("2026-01-03T09:00:00Z"));

        assert_eq!(like.note_type, Some(NotificationType::Like));
        assert_eq!(like.read, Some(true));
        assert!(like.header.is_empty());
        let like_meta = like.body[0].meta.as_ref().expect("block meta");
        assert_eq!(like_meta.links, None, "Sent as an empty list");
        assert_eq!(like.body[0].actions[0].is_on, Some(true));
        assert_eq!(
            like.meta.as_ref().and_then(|meta| meta.actor_count),
            Some(3)
        );

        assert_eq!(
            other.note_type,
            Some(NotificationType::Other("example_reminder_note".to_string()))
        );
        assert_eq!(other.subtype.as_deref(), Some("monthly"));
        assert_eq!(other.variant.as_deref(), Some("v2"));
        assert_eq!(other.url, None, "Sent as an empty string");
        assert!(other.subject[0].ranges.is_empty());
        let reminder = &other.body[0];
        assert_eq!(reminder.kind, None);
        assert_eq!(reminder.media[0].kind, Some(NotificationMediaKind::Badge));
        assert_eq!(reminder.media[0].height.as_deref(), Some("100%"));
        assert_eq!(reminder.ranges[0].kind, Some(NotificationRangeKind::Tracks));
        assert_eq!(
            reminder.ranges[0].context.as_deref(),
            Some("example_reminder_clicked")
        );
        assert_eq!(
            reminder.ranges[1].kind,
            Some(NotificationRangeKind::Other(
                "example_highlight".to_string()
            ))
        );
        let button = &other.body[1];
        assert_eq!(button.ranges[0].kind, Some(NotificationRangeKind::Button));
        assert_eq!(
            button.ranges[0].class_name.as_deref(),
            Some("example-button")
        );
        assert_eq!(
            button.meta.as_ref().and_then(|meta| meta.is_mobile_button),
            Some(true)
        );
        let other_meta = other.meta.as_ref().expect("meta");
        assert_eq!(
            other_meta.ids.as_ref().map(|ids| ids.other.clone()),
            Some(HashMap::from([("order".to_string(), 901)]))
        );
        assert_eq!(
            other_meta
                .links
                .as_ref()
                .and_then(|links| links.other.get("order").cloned()),
            Some("https://example.com/orders/901".to_string())
        );

        assert_eq!(follow.note_type, Some(NotificationType::Follow));
        assert_eq!(
            follow.timestamp,
            date("1970-01-01T00:00:00Z"),
            "A note with no date"
        );
        assert!(follow.body.is_empty());
        assert_eq!(
            follow.meta.as_ref().and_then(|meta| meta.actor.clone()),
            None
        );
    }

    #[test]
    fn test_deserialize_note_hashes() {
        let page: NotificationHashesResponse = serde_json::from_str(include_str!(
            "../../tests/wpcom/notifications/note_hashes.json"
        ))
        .unwrap();
        assert_eq!(
            page.notes,
            [
                NotificationHash {
                    id: NotificationId(10_000_000_003),
                    note_hash: Some(4_000_000_011),
                },
                NotificationHash {
                    id: NotificationId(10_000_000_002),
                    note_hash: Some(4_000_000_012),
                },
                NotificationHash {
                    id: NotificationId(10_000_000_001),
                    note_hash: Some(4_000_000_013),
                },
            ]
        );
        assert!(page.unreadable_notes.is_empty());
        assert_eq!(page.last_seen_time, date("2026-01-02T00:00:00Z"));
    }

    #[test]
    fn test_deserialize_note_ids() {
        let page: NotificationIdsResponse = serde_json::from_str(include_str!(
            "../../tests/wpcom/notifications/note_ids.json"
        ))
        .unwrap();
        assert_eq!(
            page.ids,
            [
                NotificationId(10_000_000_003),
                NotificationId(10_000_000_002),
                NotificationId(10_000_000_001),
            ]
        );
        assert!(page.unreadable_notes.is_empty());
    }

    /// Fetching a note that doesn't exist is a success with no notes.
    #[test]
    fn test_deserialize_note_not_found() {
        let page = page(include_str!(
            "../../tests/wpcom/notifications/note_not_found.json"
        ));
        assert!(page.notes.is_empty());
        assert!(page.unreadable_notes.is_empty());
        assert_eq!(page.last_seen_time, date("2026-01-02T00:00:00Z"));
    }

    /// A note without an ID can't be read, nor can something that isn't a note, but neither
    /// fails the notes around them.
    #[test]
    fn test_one_unreadable_note_does_not_fail_the_page() {
        let page = page(
            &json!({
                "last_seen_time": "1767312000",
                "notes": [
                    { "id": 10000000003u64, "type": "like" },
                    { "id": "not a number", "type": "example_type" },
                    { "type": "comment" },
                    "not a note",
                    { "id": 10000000001u64, "type": "follow" }
                ]
            })
            .to_string(),
        );

        let ids: Vec<_> = page.notes.iter().map(|note| note.id).collect();
        assert_eq!(
            ids,
            [
                NotificationId(10_000_000_003),
                NotificationId(10_000_000_001)
            ]
        );
        let unreadable: Vec<_> = page
            .unreadable_notes
            .iter()
            .map(|note| (note.id, note.note_type.as_deref()))
            .collect();
        assert_eq!(
            unreadable,
            [
                (None, Some("example_type")),
                (None, Some("comment")),
                (None, None)
            ]
        );
        assert!(
            page.unreadable_notes
                .iter()
                .all(|note| !note.error.is_empty())
        );
    }

    /// Each note keeps its JSON exactly as it came, fields the model doesn't know included, and a
    /// note that can't be read is listed as unreadable, as on a page without JSON.
    #[test]
    fn test_deserialize_notes_with_json() {
        let note_json = r#"{"id":10000000002,"type":"like","read":1,"a_later_field":{"x":[1,2]}}"#;
        let page: NotificationsWithJsonResponse = serde_json::from_str(&format!(
            r#"{{"last_seen_time":"1767312000","notes":[{note_json},{{"id":"not a number","type":"example_type"}}]}}"#
        ))
        .unwrap();

        let [kept] = &page.notes[..] else {
            panic!("Expected one note");
        };
        assert_eq!(kept.json, note_json);
        assert_eq!(kept.note.id, NotificationId(10_000_000_002));
        assert_eq!(kept.note.read, Some(true));
        assert_eq!(
            serde_json::from_str::<WpComNotification>(&kept.json).unwrap(),
            kept.note
        );
        assert_eq!(page.unreadable_notes.len(), 1);
        assert_eq!(
            page.unreadable_notes[0].note_type.as_deref(),
            Some("example_type")
        );
        assert_eq!(page.last_seen_time, date("2026-01-02T00:00:00Z"));
    }

    /// A note with only an ID is what `fields=id` sends, and it's still a note.
    #[test]
    fn test_note_with_only_an_id() {
        let note = note(json!({ "id": 10000000001u64 }));
        assert_eq!(note.id, NotificationId(10_000_000_001));
        assert_eq!(note.note_type, None);
        assert_eq!(note.read, None);
        assert_eq!(note.timestamp, None);
        assert!(note.subject.is_empty() && note.header.is_empty() && note.body.is_empty());
        assert_eq!(note.meta, None);
    }

    /// Note IDs are above ten billion, and can come as strings.
    #[rstest]
    #[case(json!(12_345_678_901u64))]
    #[case(json!("12345678901"))]
    fn test_note_id_needs_64_bits(#[case] id: serde_json::Value) {
        assert_eq!(note(json!({ "id": id })).id, NotificationId(12_345_678_901));
    }

    #[rstest]
    #[case(json!(0), Some(false))]
    #[case(json!(1), Some(true))]
    #[case(json!(true), Some(true))]
    #[case(json!("1"), Some(true))]
    #[case(json!("0"), Some(false))]
    #[case(json!(2), None)]
    #[case(json!(null), None)]
    #[case(json!({}), None)]
    fn test_read_state(#[case] read: serde_json::Value, #[case] expected: Option<bool>) {
        assert_eq!(note(json!({ "id": 1, "read": read })).read, expected);
    }

    /// Missing, empty and oddly typed values read as absent rather than failing the note.
    #[test]
    fn test_odd_values_do_not_fail_a_note() {
        let note = note(json!({
            "id": 1,
            "type": 42,
            "read": "yes",
            "timestamp": "not a date",
            "icon": false,
            "title": "",
            "url": [],
            "note_hash": "not a number",
            "subject": { "text": "an object, not a list" },
            "header": null,
            "body": [
                "not a block",
                null,
                {
                    "text": 5,
                    "type": "",
                    "ranges": [
                        { "type": "user", "indices": [3], "id": null, "site_id": "0" },
                        { "type": "post", "indices": "0,4", "site_id": "801", "post_id": -1 },
                        { "indices": [1, "two"] }
                    ],
                    "media": [ { "type": "image", "width": 640, "emoji": "maybe" } ],
                    "actions": [],
                    "meta": { "ids": { "site": "801", "prompt": true }, "links": "none" },
                    "nest_level": -1
                }
            ],
            "meta": "not an object"
        }));

        assert_eq!(
            note.note_type,
            Some(NotificationType::Other("42".to_string()))
        );
        assert_eq!(note.read, None);
        assert_eq!(note.timestamp, None);
        assert_eq!(note.icon, None);
        assert_eq!(note.title, None);
        assert_eq!(note.url, None);
        assert_eq!(note.note_hash, None);
        assert!(note.subject.is_empty());
        assert!(note.header.is_empty());
        assert_eq!(note.meta, None);

        let [block] = &note.body[..] else {
            panic!("Expected the one block that is an object");
        };
        assert_eq!(block.text, "5");
        assert_eq!(block.kind, None);
        assert!(block.actions.is_empty());
        assert_eq!(block.nest_level, None);
        let [user, post, untyped] = &block.ranges[..] else {
            panic!("Expected three ranges");
        };
        assert_eq!(user.indices, None);
        assert_eq!(user.id, None);
        assert_eq!(user.site_id, None, "Sent as \"0\"");
        assert_eq!(post.indices, None);
        assert_eq!(post.site_id, Some(WpComSiteId(801)), "Sent as a string");
        assert_eq!(post.post_id, None);
        assert_eq!(untyped.kind, None);
        assert_eq!(untyped.indices, None);
        assert_eq!(block.media[0].width.as_deref(), Some("640"));
        assert_eq!(block.media[0].emoji, None);
        let meta = block.meta.as_ref().expect("block meta");
        let ids = meta.ids.as_ref().expect("ids");
        assert_eq!(ids.site, Some(WpComSiteId(801)));
        assert!(ids.other.is_empty(), "A flag isn't an ID");
        assert_eq!(meta.links, None);
    }

    /// A note's time can come as seconds since 1970, and the list's last-seen time as text.
    #[rstest]
    #[case(json!("2026-01-02T03:04:05+00:00"))]
    #[case(json!("2026-01-02T04:04:05+01:00"))]
    #[case(json!(1767323045))]
    #[case(json!("1767323045"))]
    fn test_timestamp_forms(#[case] timestamp: serde_json::Value) {
        assert_eq!(
            note(json!({ "id": 1, "timestamp": timestamp })).timestamp,
            date("2026-01-02T03:04:05Z")
        );
    }

    #[rstest]
    #[case(json!({ "notes": [] }), None)]
    #[case(json!({ "last_seen_time": 1767312000, "notes": [] }), date("2026-01-02T00:00:00Z"))]
    #[case(json!({ "last_seen_time": "", "notes": [] }), None)]
    #[case(json!({ "last_seen_time": false }), None)]
    fn test_last_seen_time(
        #[case] page: serde_json::Value,
        #[case] expected: Option<WpGmtDateTime>,
    ) {
        let page: NotificationsResponse = serde_json::from_value(page).unwrap();
        assert_eq!(page.last_seen_time, expected);
    }

    #[test]
    fn test_kinds_read_and_write_their_names() {
        assert_eq!(
            NotificationType::from("comment_like"),
            NotificationType::CommentLike
        );
        assert_eq!(
            NotificationType::from("example_type").as_str(),
            "example_type"
        );
        assert_eq!(
            serde_json::to_value(NotificationActionKind::ReplyToComment).unwrap(),
            json!("replyto-comment")
        );
        assert_eq!(
            serde_json::from_value::<NotificationRangeKind>(json!("a")).unwrap(),
            NotificationRangeKind::Anchor
        );
    }
}

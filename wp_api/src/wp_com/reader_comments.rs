//! A post's comments, writing one, and the user's subscription to them.
//!
//! A comment's ID is only unique within its site: comment `10` is one comment on one site and
//! another on the next. With the site's ID it names one comment.
//!
//! The comments answer without a token for a public site. The subscription needs one, and so does
//! everything that changes something: a new comment, a reply, and a change to the subscription.

use crate::{
    comments::CommentId,
    date::{WpGmtDateTime, deserialize_optional_wp_gmt_date_time},
    impl_as_query_value_from_to_string,
    posts::PostId,
    url_query::{
        AppendUrlQueryPairs, FromUrlQueryPairs, QueryPairs, QueryPairsExtension, UrlQueryPairsMap,
    },
    wp_com::{WpComSiteId, me::WpComUserId, reader_saved::deserialize_optional_text},
};
use serde::{Deserialize, Deserializer, Serialize};
use wp_derive::WpDeriveParamsField;
use wp_serde_helper::{
    LenientVec, deserialize_bool_or_int, deserialize_optional_bool_or_int,
    deserialize_u64_or_string_or_none,
};

/// The order of a post's comments.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    uniffi::Enum,
    strum_macros::EnumString,
    strum_macros::Display,
)]
#[serde(rename_all = "UPPERCASE")]
#[strum(serialize_all = "UPPERCASE")]
pub enum ReaderCommentsOrder {
    /// Newest first. The API's default.
    Desc,
    /// Oldest first.
    Asc,
}

impl_as_query_value_from_to_string!(ReaderCommentsOrder);

/// The kinds of comment to return.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    uniffi::Enum,
    strum_macros::EnumString,
    strum_macros::Display,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ReaderCommentsTypeFilter {
    /// Every kind. The API's default.
    Any,
    /// Only what readers wrote.
    Comment,
    Trackback,
    Pingback,
    /// Trackbacks and pingbacks.
    Pings,
}

impl_as_query_value_from_to_string!(ReaderCommentsTypeFilter);

/// The moderation statuses of comment to return.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    uniffi::Enum,
    strum_macros::EnumString,
    strum_macros::Display,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ReaderCommentsStatusFilter {
    /// The API's default, and the only one that can be asked for signed out.
    Approved,
    Unapproved,
    Spam,
    Trash,
    All,
}

impl_as_query_value_from_to_string!(ReaderCommentsStatusFilter);

/// Query parameters for a post's comments.
///
/// The list pages three ways. For a cache, page a flat list by `offset` or `page`: it ends with an
/// empty page, and [`ReaderCommentsResponse::found`] says how many there are. A reply names its
/// parent in [`ReaderComment::parent_id`], so threads can be built from the flat list.
#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(false)] // The response has no `Link` header to read the next page from.
pub struct ReaderCommentsParams {
    /// The number of comments to return: 20 by default, and 100 at most. More is a 400
    /// `invalid_number`. With `hierarchical` it counts top-level comments.
    #[uniffi(default = None)]
    pub number: Option<u32>,
    /// The number of comments to skip. Ignored with `hierarchical`.
    #[uniffi(default = None)]
    pub offset: Option<u32>,
    /// The page to return, counting from 1. It wins over `offset`.
    #[uniffi(default = None)]
    pub page: Option<u32>,
    /// The order of the comments. The API's default is [`ReaderCommentsOrder::Desc`].
    #[uniffi(default = None)]
    pub order: Option<ReaderCommentsOrder>,
    /// Return whole threads: each top-level comment is followed by every reply under it, at any
    /// depth, and a page holds `number` top-level comments with all of theirs. The comments are
    /// still a flat list.
    ///
    /// Only `page` moves through it: `offset`, `before` and `after` are ignored. A page past the
    /// last one isn't empty: the API sends every comment again. So stop at a page with fewer
    /// than `number` top-level comments, or once the pages hold
    /// [`ReaderCommentsResponse::found`] comments.
    #[uniffi(default = None)]
    pub hierarchical: Option<bool>,
    /// Only return comments from this time or later. Ignored with `hierarchical`.
    #[uniffi(default = None)]
    pub after: Option<WpGmtDateTime>,
    /// Only return comments from this time or earlier. Ignored with `hierarchical`.
    #[uniffi(default = None)]
    pub before: Option<WpGmtDateTime>,
    /// The kinds of comment to return. [`ReaderCommentsResponse::found`] counts only those.
    #[uniffi(default = None)]
    #[field_name("type")]
    pub comment_type: Option<ReaderCommentsTypeFilter>,
    /// The moderation statuses to return. Signed out, anything but approved comments is a 403
    /// `unauthorized`.
    #[uniffi(default = None)]
    pub status: Option<ReaderCommentsStatusFilter>,
    /// Add [`ReaderCommentAuthor::wpcom_id`] and [`ReaderCommentAuthor::wpcom_login`] to each
    /// author with a WordPress.com account. On a WordPress.com site they repeat the author's ID
    /// and login.
    #[uniffi(default = None)]
    pub author_wpcom_data: Option<bool>,
    /// `wpcom` reads a Jetpack site's comments from WordPress.com's copy of them, as the apps and
    /// the web Reader do. Without it a Jetpack site's comments have no
    /// [`ReaderComment::like_count`] or [`ReaderComment::i_like`]. It changes nothing for a
    /// WordPress.com site.
    #[uniffi(default = None)]
    pub force: Option<String>,
    /// The fields of each comment to return, comma-separated, such as `ID,date,parent`. `ID` has
    /// to be among them: a comment without one can't be read, and is listed in
    /// [`ReaderCommentsResponse::unreadable_comments`].
    #[uniffi(default = None)]
    pub fields: Option<String>,
}

/// A page of a post's comments.
///
/// It doesn't say whether the post takes new comments: a post with comments closed returns the
/// same as one with them open. That's on the post, as its `comments_open`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
#[serde(from = "RawReaderCommentsResponse")]
pub struct ReaderCommentsResponse {
    /// The page's comments, without any in [`Self::unreadable_comments`].
    pub comments: Vec<ReaderComment>,
    /// The comments on the page that couldn't be read. Almost always empty.
    #[uniffi(default = [])]
    pub unreadable_comments: Vec<ReaderUnreadableComment>,
    /// How many comments match the request, on every page. It counts replies as well as
    /// top-level comments, with `hierarchical` too.
    #[uniffi(default = None)]
    pub found: Option<u64>,
}

#[derive(Deserialize)]
struct RawReaderCommentsResponse {
    #[serde(default)]
    comments: Option<LenientVec<ReaderComment>>,
    #[serde(default, deserialize_with = "deserialize_u64_or_string_or_none")]
    found: Option<u64>,
}

impl From<RawReaderCommentsResponse> for ReaderCommentsResponse {
    fn from(raw: RawReaderCommentsResponse) -> Self {
        let comments = raw.comments.unwrap_or_default();
        let unreadable_comments = comments
            .invalid
            .into_iter()
            .map(|comment| ReaderUnreadableComment {
                id: serde_json::from_str::<CommentIdOnly>(comment.json.get())
                    .ok()
                    .and_then(|comment| comment.id),
                error: comment.error.to_string(),
            })
            .collect();
        Self {
            comments: comments.valid,
            unreadable_comments,
            found: raw.found,
        }
    }
}

#[derive(Deserialize)]
struct CommentIdOnly {
    #[serde(rename = "ID", default, deserialize_with = "deserialize_optional_id")]
    id: Option<CommentId>,
}

/// A comment on a page that couldn't be read. It doesn't fail the page: the page keeps the
/// comments it can read, and lists the others as these.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, uniffi::Record)]
pub struct ReaderUnreadableComment {
    /// Which comment it is. `None` when its ID couldn't be read either.
    #[uniffi(default = None)]
    pub id: Option<CommentId>,
    /// Why it couldn't be read. A line and column in it count from the start of the comment.
    pub error: String,
}

/// A comment on a post.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
#[serde(try_from = "RawReaderComment")]
pub struct ReaderComment {
    /// The comment's ID, which is only unique within its site.
    pub id: CommentId,
    /// The post it's on.
    #[uniffi(default = None)]
    pub post_id: Option<PostId>,
    /// The comment it replies to, which can itself be a reply. `None` for a top-level comment.
    #[uniffi(default = None)]
    pub parent_id: Option<CommentId>,
    #[uniffi(default = None)]
    pub author: Option<ReaderCommentAuthor>,
    /// When it was written. It's the only date a comment has.
    #[uniffi(default = None)]
    pub date: Option<WpGmtDateTime>,
    /// The comment's address on the site.
    #[uniffi(default = None)]
    pub url: Option<String>,
    #[uniffi(default = None)]
    pub short_url: Option<String>,
    /// HTML, ready to show: paragraphs are marked up, and characters such as `’` are character
    /// references.
    #[uniffi(default = None)]
    pub content: Option<String>,
    /// What the author wrote, before it was made into [`Self::content`]. It can still hold HTML
    /// the author typed.
    #[uniffi(default = None)]
    pub raw_content: Option<String>,
    #[uniffi(default = None)]
    pub status: Option<ReaderCommentStatus>,
    #[uniffi(default = None)]
    pub comment_type: Option<ReaderCommentType>,
    /// `None` for a Jetpack site's comment asked for without `force=wpcom`.
    #[uniffi(default = None)]
    pub like_count: Option<u64>,
    /// Whether the user likes it. `false` signed out, and `None` wherever
    /// [`Self::like_count`] is.
    #[uniffi(default = None)]
    pub i_like: Option<bool>,
    /// What the API calls the user having replied to it. Don't rely on it: signed out it's
    /// `false`, except that one comment on a Jetpack site had it `true`.
    #[uniffi(default = None)]
    pub i_replied: Option<bool>,
    /// Whether the user can moderate it. `false` signed out.
    #[uniffi(default = None)]
    pub can_moderate: Option<bool>,
}

/// Who wrote a [`ReaderComment`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderCommentAuthor {
    /// The author's WordPress.com user ID. `None` for someone who commented without an account,
    /// and for a pingback, both of which the API sends as `0`.
    #[serde(rename = "ID", default, deserialize_with = "deserialize_optional_id")]
    #[uniffi(default = None)]
    pub id: Option<WpComUserId>,
    /// The author's username. Only an author with an account has one.
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    #[uniffi(default = None)]
    pub login: Option<String>,
    /// The name to show. A pingback's is the title of the page that links to the post, which is
    /// HTML: it can carry character references such as `&laquo;`.
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    #[uniffi(default = None)]
    pub name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    #[uniffi(default = None)]
    pub first_name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    #[uniffi(default = None)]
    pub last_name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    #[uniffi(default = None)]
    pub nice_name: Option<String>,
    /// The author's website, as they gave it. A pingback's is the page that links to the post.
    #[serde(
        rename = "URL",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    #[uniffi(default = None)]
    pub url: Option<String>,
    /// `None` for more than half of the pingbacks seen.
    #[serde(
        rename = "avatar_URL",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    #[uniffi(default = None)]
    pub avatar_url: Option<String>,
    /// The author's Gravatar profile.
    #[serde(
        rename = "profile_URL",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    #[uniffi(default = None)]
    pub profile_url: Option<String>,
    /// The author's main site. Only some authors with an account have one.
    #[serde(
        rename = "site_ID",
        default,
        deserialize_with = "deserialize_optional_id"
    )]
    #[uniffi(default = None)]
    pub site_id: Option<WpComSiteId>,
    /// Whether the author's main site can be shown.
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    #[uniffi(default = None)]
    pub site_visible: Option<bool>,
    /// The author's WordPress.com user ID. Only sent when asked for with `author_wpcom_data`,
    /// for an author with an account.
    #[serde(default, deserialize_with = "deserialize_optional_id")]
    #[uniffi(default = None)]
    pub wpcom_id: Option<WpComUserId>,
    /// The author's WordPress.com username. Sent with [`Self::wpcom_id`].
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    #[uniffi(default = None)]
    pub wpcom_login: Option<String>,
}

/// A comment's moderation status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, uniffi::Enum)]
pub enum ReaderCommentStatus {
    Approved,
    /// Waiting for a moderator.
    Unapproved,
    Spam,
    Trash,
    /// A status this version doesn't know, as the API sent it.
    Other(String),
}

/// What kind of comment a [`ReaderComment`] is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, uniffi::Enum)]
pub enum ReaderCommentType {
    /// What a reader wrote.
    Comment,
    /// A link to the post from another page, which that page's site sent.
    Pingback,
    Trackback,
    /// A kind this version doesn't know, as the API sent it.
    Other(String),
}

/// Every field a comment can arrive with, before the ones that are nested are flattened.
#[derive(Deserialize)]
struct RawReaderComment {
    #[serde(rename = "ID", deserialize_with = "deserialize_id")]
    id: CommentId,
    #[serde(default, deserialize_with = "deserialize_object_or_none")]
    post: Option<RawReference<PostId>>,
    /// `false` for a top-level comment.
    #[serde(default, deserialize_with = "deserialize_object_or_none")]
    parent: Option<RawReference<CommentId>>,
    #[serde(default, deserialize_with = "deserialize_object_or_none")]
    author: Option<ReaderCommentAuthor>,
    #[serde(default, deserialize_with = "deserialize_optional_wp_gmt_date_time")]
    date: Option<WpGmtDateTime>,
    #[serde(
        rename = "URL",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    url: Option<String>,
    #[serde(
        rename = "short_URL",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    short_url: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    content: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    raw_content: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    status: Option<String>,
    #[serde(
        rename = "type",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    comment_type: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_count")]
    like_count: Option<u64>,
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    i_like: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    i_replied: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    can_moderate: Option<bool>,
}

/// `post` and `parent`, of which only the ID is kept.
#[derive(Deserialize)]
#[serde(bound(deserialize = "T: FromId"))]
struct RawReference<T> {
    #[serde(rename = "ID", default, deserialize_with = "deserialize_optional_id")]
    id: Option<T>,
}

impl TryFrom<RawReaderComment> for ReaderComment {
    type Error = String;

    fn try_from(raw: RawReaderComment) -> Result<Self, Self::Error> {
        Ok(Self {
            id: raw.id,
            post_id: raw.post.and_then(|post| post.id),
            parent_id: raw.parent.and_then(|parent| parent.id),
            author: raw.author,
            date: raw.date,
            url: raw.url,
            short_url: raw.short_url,
            content: raw.content,
            raw_content: raw.raw_content,
            status: raw.status.map(|status| match status.as_str() {
                "approved" => ReaderCommentStatus::Approved,
                "unapproved" => ReaderCommentStatus::Unapproved,
                "spam" => ReaderCommentStatus::Spam,
                "trash" => ReaderCommentStatus::Trash,
                _ => ReaderCommentStatus::Other(status),
            }),
            comment_type: raw.comment_type.map(|kind| match kind.as_str() {
                "comment" => ReaderCommentType::Comment,
                "pingback" => ReaderCommentType::Pingback,
                "trackback" => ReaderCommentType::Trackback,
                _ => ReaderCommentType::Other(kind),
            }),
            like_count: raw.like_count,
            i_like: raw.i_like,
            i_replied: raw.i_replied,
            can_moderate: raw.can_moderate,
        })
    }
}

/// Request body for a new comment, on a post or in reply to another comment.
///
/// The answer is the new comment, as a [`ReaderComment`]: the shape the list of comments has it
/// in. Its author is the user. A top-level comment's `parent_id` is `None`, and a reply's is the
/// comment it was sent to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, uniffi::Record)]
pub struct ReaderNewCommentParams {
    /// What the comment says. The answer has it as written in [`ReaderComment::raw_content`], and
    /// made into HTML in [`ReaderComment::content`]. An empty one is a 400 `invalid_input`.
    pub content: String,
}

/// Whether the user is subscribed to a post's comments, from
/// `GET /rest/v1.1/sites/{site}/posts/{post}/subscribers/mine`. Turning the subscription's
/// notifications on or off answers with it too.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderPostCommentsSubscription {
    /// Whether the user is subscribed to the post's comments.
    pub i_subscribe: bool,
    /// Whether new comments send the user a notification. It can be `false` for a post the user
    /// is subscribed to.
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    #[uniffi(default = None)]
    pub receives_notifications: Option<bool>,
}

/// Request body to subscribe to a post's comments.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, uniffi::Record)]
pub struct ReaderSubscribeToCommentsParams {
    /// Whether new comments should send the user a notification as well. It wasn't tried, and
    /// the apps don't send it: a subscription made without it had notifications off.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[uniffi(default = None)]
    pub receive_notifications: Option<bool>,
}

/// Request body to turn the notifications of a subscription to a post's comments on or off.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, uniffi::Record)]
pub struct ReaderCommentsNotificationsParams {
    /// Whether new comments should send the user a notification. The answer calls it
    /// `receives_notifications`.
    pub receive_notifications: bool,
}

/// The answer to subscribing to a post's comments, or unsubscribing from them. It's a
/// [`ReaderPostCommentsSubscription`] with a `success` as well.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderPostCommentsSubscriptionResponse {
    /// Whether the user is subscribed to the post's comments now.
    #[serde(deserialize_with = "deserialize_bool_or_int")]
    pub i_subscribe: bool,
    /// Whether new comments send the user a notification now.
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    #[uniffi(default = None)]
    pub receives_notifications: Option<bool>,
    /// `true` in every answer seen, including an unsubscribe with nothing to remove. The iOS app
    /// reads it as whether the change was made.
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    #[uniffi(default = None)]
    pub success: Option<bool>,
}

/// An ID the API sends as a number or a string.
fn deserialize_id<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromId,
{
    deserialize_optional_id(deserializer)?
        .ok_or_else(|| serde::de::Error::custom("expected an ID that is a number above 0"))
}

/// An ID the API sends as a number or a string, and as `0`, `false`, `""` or `null` when there
/// isn't one.
fn deserialize_optional_id<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: FromId,
{
    Ok(deserialize_optional_count(deserializer)?
        .filter(|id| *id != 0)
        .and_then(T::from_id))
}

/// The ID types a comment holds: comment and post IDs are signed, and user and site IDs aren't.
trait FromId: Sized {
    fn from_id(id: u64) -> Option<Self>;
}

impl FromId for CommentId {
    fn from_id(id: u64) -> Option<Self> {
        i64::try_from(id).ok().map(Self)
    }
}

impl FromId for PostId {
    fn from_id(id: u64) -> Option<Self> {
        i64::try_from(id).ok().map(Self)
    }
}

impl FromId for WpComUserId {
    fn from_id(id: u64) -> Option<Self> {
        Some(Self(id))
    }
}

impl FromId for WpComSiteId {
    fn from_id(id: u64) -> Option<Self> {
        Some(Self(id))
    }
}

/// A count the API sends as a number or a string. Anything that isn't one reads as `None` rather
/// than failing the comment.
fn deserialize_optional_count<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(
        match Option::<serde_json::Value>::deserialize(deserializer)? {
            Some(serde_json::Value::Number(number)) => number.as_u64(),
            Some(serde_json::Value::String(text)) => text.parse().ok(),
            _ => None,
        },
    )
}

/// An object, or `None` for `false`, which the API sends for a comment with no parent, and for
/// `null` or an empty list.
fn deserialize_object_or_none<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: serde::de::DeserializeOwned,
{
    match Option::<serde_json::Value>::deserialize(deserializer)? {
        Some(value @ serde_json::Value::Object(_)) => serde_json::from_value(value)
            .map(Some)
            .map_err(serde::de::Error::custom),
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use serde_json::json;

    fn page(json: &str) -> ReaderCommentsResponse {
        serde_json::from_str(json).expect("the page parses")
    }

    fn comment(value: serde_json::Value) -> ReaderComment {
        serde_json::from_value(value).expect("the comment parses")
    }

    /// A page recorded from WordPress.com with `author_wpcom_data`, with the authors and what
    /// they wrote replaced. It has one comment of each kind seen: a top-level comment, a reply,
    /// a comment from someone with no account, and a pingback.
    #[test]
    fn test_deserialize_comments() {
        let page = page(include_str!(
            "../../tests/wpcom/reader_comments/comments.json"
        ));

        assert_eq!(page.found, Some(825));
        assert!(page.unreadable_comments.is_empty());
        let [reply, top_level, guest, pingback] = &page.comments[..] else {
            panic!("Expected four comments");
        };

        assert_eq!(reply.id, CommentId(3002));
        assert_eq!(reply.post_id, Some(PostId(6014)));
        assert_eq!(reply.parent_id, Some(CommentId(3001)));
        assert_eq!(
            reply.date,
            Some("2026-05-26T21:16:31+00:00".parse().unwrap()),
            "Sent with the site's offset, +01:00"
        );
        assert_eq!(
            reply.url.as_deref(),
            Some("https://example.blog/2026/04/28/example-post/#comment-3002")
        );
        assert_eq!(
            reply.content.as_deref(),
            Some("<p>Hadn&#8217;t thought of that. I like it!</p>\n")
        );
        assert_eq!(
            reply.raw_content.as_deref(),
            Some("Hadn't thought of that. I like it!")
        );
        assert_eq!(reply.status, Some(ReaderCommentStatus::Approved));
        assert_eq!(reply.comment_type, Some(ReaderCommentType::Comment));
        assert_eq!(reply.like_count, Some(1));
        assert_eq!(reply.i_like, Some(false));
        assert_eq!(reply.i_replied, Some(false));
        assert_eq!(reply.can_moderate, Some(false));
        let author = reply.author.as_ref().expect("an author");
        assert_eq!(author.id, Some(WpComUserId(1001)));
        assert_eq!(author.login.as_deref(), Some("exampleuser1"));
        assert_eq!(author.name.as_deref(), Some("Example User One"));
        assert_eq!(author.site_id, Some(WpComSiteId(2001)));
        assert_eq!(author.site_visible, Some(true));
        assert_eq!(author.wpcom_id, Some(WpComUserId(1001)));
        assert_eq!(author.wpcom_login.as_deref(), Some("exampleuser1"));

        assert_eq!(top_level.id, CommentId(3001));
        assert_eq!(top_level.parent_id, None, "Sent as `false`");

        // Someone who commented without an account: no ID or login, but a name and a picture.
        let author = guest.author.as_ref().expect("an author");
        assert_eq!(author.id, None);
        assert_eq!(author.login, None);
        assert_eq!(author.name.as_deref(), Some("Example Guest"));
        assert!(author.avatar_url.is_some());
        assert_eq!(author.wpcom_id, None);

        assert_eq!(pingback.comment_type, Some(ReaderCommentType::Pingback));
        let author = pingback.author.as_ref().expect("an author");
        assert_eq!(author.id, None);
        assert_eq!(author.name.as_deref(), Some("An Example Page"));
        assert_eq!(author.url.as_deref(), Some("https://example.org/a-page/"));
        assert_eq!(author.avatar_url, None);
    }

    /// What the API sends for a post with no comments, and past the last page of a flat list.
    #[test]
    fn test_deserialize_no_comments() {
        let page = page(r#"{ "found": 0, "site_ID": 3584907, "comments": [] }"#);
        assert!(page.comments.is_empty());
        assert_eq!(page.found, Some(0));
    }

    /// A Jetpack site's comment, asked for without `force=wpcom`, has no likes.
    #[test]
    fn test_a_comment_without_likes() {
        let comment = comment(json!({
            "ID": 5,
            "post": { "ID": 9, "type": "post" },
            "parent": false,
            "status": "approved",
            "type": "comment",
            "i_replied": false,
        }));
        assert_eq!(comment.like_count, None);
        assert_eq!(comment.i_like, None);
        assert_eq!(comment.i_replied, Some(false));
    }

    /// With `fields`, a comment arrives with only the fields asked for.
    #[test]
    fn test_a_comment_with_few_fields() {
        let comment = comment(json!({ "ID": 5, "date": "2011-01-13T15:27:31+00:00" }));
        assert_eq!(comment.id, CommentId(5));
        assert!(comment.date.is_some());
        assert_eq!(comment.author, None);
        assert_eq!(comment.parent_id, None);
        assert_eq!(comment.status, None);
    }

    #[rstest]
    #[case::number(json!(451843), 451843)]
    #[case::string(json!("451843"), 451843)]
    fn test_a_comment_id(#[case] sent: serde_json::Value, #[case] expected: i64) {
        let comment =
            comment(json!({ "ID": sent, "parent": { "ID": sent }, "post": { "ID": sent } }));
        assert_eq!(comment.id, CommentId(expected));
        assert_eq!(comment.parent_id, Some(CommentId(expected)));
        assert_eq!(comment.post_id, Some(PostId(expected)));
    }

    #[rstest]
    #[case::left_out(json!({}))]
    #[case::zero(json!({ "ID": 0 }))]
    #[case::false_(json!({ "ID": false }))]
    #[case::negative(json!({ "ID": -4 }))]
    #[case::not_a_number(json!({ "ID": "four" }))]
    fn test_a_comment_without_an_id_is_an_error(#[case] comment: serde_json::Value) {
        assert!(serde_json::from_value::<ReaderComment>(comment).is_err());
    }

    #[rstest]
    #[case::false_(json!(false))]
    #[case::null(json!(null))]
    #[case::empty_list(json!([]))]
    #[case::zero(json!({ "ID": 0 }))]
    fn test_a_comment_without_a_parent(#[case] parent: serde_json::Value) {
        let comment = comment(json!({ "ID": 5, "parent": parent }));
        assert_eq!(comment.parent_id, None);
    }

    #[rstest]
    #[case("approved", ReaderCommentStatus::Approved)]
    #[case("unapproved", ReaderCommentStatus::Unapproved)]
    #[case("spam", ReaderCommentStatus::Spam)]
    #[case("trash", ReaderCommentStatus::Trash)]
    #[case("held", ReaderCommentStatus::Other("held".to_string()))]
    fn test_comment_status(#[case] sent: &str, #[case] expected: ReaderCommentStatus) {
        let comment = comment(json!({ "ID": 5, "status": sent }));
        assert_eq!(comment.status, Some(expected));
    }

    #[rstest]
    #[case("comment", ReaderCommentType::Comment)]
    #[case("pingback", ReaderCommentType::Pingback)]
    #[case("trackback", ReaderCommentType::Trackback)]
    #[case("like", ReaderCommentType::Other("like".to_string()))]
    fn test_comment_type(#[case] sent: &str, #[case] expected: ReaderCommentType) {
        let comment = comment(json!({ "ID": 5, "type": sent }));
        assert_eq!(comment.comment_type, Some(expected));
    }

    #[test]
    fn test_author_fields_that_vary_in_shape() {
        let comment = comment(json!({
            "ID": 5,
            "author": {
                "ID": "7",
                "login": "",
                "email": false,
                "name": "Example",
                "URL": "",
                "avatar_URL": "",
                "ip_address": false,
                "site_ID": -1,
                "site_visible": 0,
                "wpcom_id": 7,
            },
            "like_count": "3",
            "i_like": 1,
        }));
        let author = comment.author.expect("an author");
        assert_eq!(author.id, Some(WpComUserId(7)));
        assert_eq!(author.login, None);
        assert_eq!(author.url, None);
        assert_eq!(author.avatar_url, None);
        assert_eq!(author.site_id, None);
        assert_eq!(author.site_visible, Some(false));
        assert_eq!(author.wpcom_id, Some(WpComUserId(7)));
        assert_eq!(comment.like_count, Some(3));
        assert_eq!(comment.i_like, Some(true));
    }

    #[test]
    fn test_a_page_keeps_its_comments_when_one_cannot_be_read() {
        let page = page(
            r#"{
                "found": 4,
                "comments": [
                    { "ID": 1, "date": "2026-11-17T09:00:00+00:00" },
                    { "ID": 2, "date": "2026-11-16T16:45:34+91:00" },
                    { "content": "No ID" },
                    { "ID": 4, "parent": { "ID": 1 } }
                ]
            }"#,
        );

        let ids: Vec<_> = page.comments.iter().map(|comment| comment.id).collect();
        assert_eq!(ids, [CommentId(1), CommentId(4)]);
        assert_eq!(page.found, Some(4));
        let unreadable: Vec<_> = page.unreadable_comments.iter().map(|c| c.id).collect();
        assert_eq!(unreadable, [Some(CommentId(2)), None]);
        assert!(
            page.unreadable_comments[0]
                .error
                .starts_with("Invalid date format"),
            "{}",
            page.unreadable_comments[0].error
        );
    }

    #[rstest]
    #[case(json!({ "comments": null }))]
    #[case(json!({}))]
    fn test_a_page_without_comments(#[case] page: serde_json::Value) {
        let page: ReaderCommentsResponse = serde_json::from_value(page).expect("the page parses");
        assert!(page.comments.is_empty());
        assert!(page.unreadable_comments.is_empty());
    }

    #[test]
    fn test_a_page_that_is_not_one_is_an_error() {
        let result = serde_json::from_value::<ReaderCommentsResponse>(json!({ "comments": 7 }));
        assert!(result.is_err());
    }

    #[rstest]
    #[case(r#"{"i_subscribe":true,"receives_notifications":false,"meta":{"links":{"site":"https://public-api.wordpress.com/rest/v1.1/sites/3584907"}}}"#, true, Some(false))]
    #[case(
        r#"{"i_subscribe":false,"receives_notifications":false,"meta":{"links":{}}}"#,
        false,
        Some(false)
    )]
    #[case(
        r#"{"i_subscribe":true,"receives_notifications":true}"#,
        true,
        Some(true)
    )]
    #[case(r#"{"i_subscribe":false}"#, false, None)]
    fn test_deserialize_post_comments_subscription(
        #[case] json: &str,
        #[case] i_subscribe: bool,
        #[case] receives_notifications: Option<bool>,
    ) {
        let subscription: ReaderPostCommentsSubscription =
            serde_json::from_str(json).expect("it parses");
        assert_eq!(subscription.i_subscribe, i_subscribe);
        assert_eq!(subscription.receives_notifications, receives_notifications);
    }

    /// The answer to a new comment on a post, recorded from WordPress.com, with the author, the
    /// site and what was written replaced. The author is the user, on a site of their own, so it
    /// keeps the types the API sent for them: `email` and `ip_address` are strings, where a list
    /// of comments sends `false`.
    #[test]
    fn test_deserialize_new_comment() {
        let comment: ReaderComment = serde_json::from_str(include_str!(
            "../../tests/wpcom/reader_comments/new_comment.json"
        ))
        .expect("the comment parses");

        assert_eq!(comment.id, CommentId(3001));
        assert_eq!(comment.post_id, Some(PostId(3)));
        assert_eq!(comment.parent_id, None, "Sent as `false`");
        assert_eq!(
            comment.date,
            Some("2026-11-19T07:06:59+00:00".parse().unwrap())
        );
        assert_eq!(
            comment.content.as_deref(),
            Some("<p>An example comment.</p>\n")
        );
        assert_eq!(comment.raw_content.as_deref(), Some("An example comment."));
        assert_eq!(comment.status, Some(ReaderCommentStatus::Approved));
        assert_eq!(comment.comment_type, Some(ReaderCommentType::Comment));
        assert_eq!(comment.like_count, Some(0));
        assert_eq!(comment.i_like, Some(false));
        assert_eq!(comment.can_moderate, Some(true));
        let author = comment.author.expect("an author");
        assert_eq!(author.id, Some(WpComUserId(1001)));
        assert_eq!(author.login.as_deref(), Some("exampleuser1"));
        assert_eq!(author.site_id, Some(WpComSiteId(2001)));
    }

    /// The answer to a reply to that comment. Its `parent` is the comment it was sent to.
    #[test]
    fn test_deserialize_new_reply() {
        let reply: ReaderComment = serde_json::from_str(include_str!(
            "../../tests/wpcom/reader_comments/new_reply.json"
        ))
        .expect("the reply parses");

        assert_eq!(reply.id, CommentId(3002));
        assert_eq!(reply.post_id, Some(PostId(3)));
        assert_eq!(reply.parent_id, Some(CommentId(3001)));
        assert_eq!(reply.raw_content.as_deref(), Some("An example reply."));
        assert_eq!(
            reply.url.as_deref(),
            Some(
                "https://example.wordpress.com/2026/11/06/example-post/comment-page-1/#comment-3002"
            )
        );
    }

    #[test]
    fn test_the_body_for_a_new_comment() {
        let params = ReaderNewCommentParams {
            content: "A <em>fine</em> post.".to_string(),
        };
        assert_eq!(
            serde_json::to_value(&params).unwrap(),
            json!({ "content": "A <em>fine</em> post." })
        );
    }

    /// What the API sent for a subscribe and for an unsubscribe. A second unsubscribe, with
    /// nothing to remove, answered the same as the first.
    #[rstest]
    #[case::subscribed(
        r#"{"success":true,"i_subscribe":true,"receives_notifications":false,"meta":{"links":{"site":"https://public-api.wordpress.com/rest/v1.1/sites/100000001"}}}"#,
        true
    )]
    #[case::unsubscribed(
        r#"{"success":true,"i_subscribe":false,"receives_notifications":false,"meta":{"links":{"site":"https://public-api.wordpress.com/rest/v1.1/sites/100000001"}}}"#,
        false
    )]
    fn test_deserialize_post_comments_subscription_response(
        #[case] json: &str,
        #[case] i_subscribe: bool,
    ) {
        let answer: ReaderPostCommentsSubscriptionResponse =
            serde_json::from_str(json).expect("it parses");
        assert_eq!(
            answer,
            ReaderPostCommentsSubscriptionResponse {
                i_subscribe,
                receives_notifications: Some(false),
                success: Some(true),
            }
        );
    }

    #[test]
    fn test_a_post_comments_subscription_response_with_little_in_it() {
        let answer: ReaderPostCommentsSubscriptionResponse =
            serde_json::from_str(r#"{ "i_subscribe": 1 }"#).expect("it parses");
        assert!(answer.i_subscribe);
        assert_eq!(answer.receives_notifications, None);
        assert_eq!(answer.success, None);

        let without = r#"{ "success": true }"#;
        assert!(serde_json::from_str::<ReaderPostCommentsSubscriptionResponse>(without).is_err());
    }

    /// Turning notifications on answers with the subscription, and no `success`.
    #[test]
    fn test_deserialize_notifications_turned_on() {
        let answer: ReaderPostCommentsSubscription = serde_json::from_str(
            r#"{"i_subscribe":true,"receives_notifications":true,"meta":{"links":{"site":"https://public-api.wordpress.com/rest/v1.1/sites/100000001"}}}"#,
        )
        .expect("it parses");
        assert!(answer.i_subscribe);
        assert_eq!(answer.receives_notifications, Some(true));
    }

    #[test]
    fn test_the_bodies_for_a_comments_subscription() {
        // With nothing set it's an empty object.
        assert_eq!(
            serde_json::to_value(ReaderSubscribeToCommentsParams::default()).unwrap(),
            json!({})
        );
        assert_eq!(
            serde_json::to_value(ReaderSubscribeToCommentsParams {
                receive_notifications: Some(true),
            })
            .unwrap(),
            json!({ "receive_notifications": true })
        );
        assert_eq!(
            serde_json::to_value(ReaderCommentsNotificationsParams {
                receive_notifications: false,
            })
            .unwrap(),
            json!({ "receive_notifications": false })
        );
    }

    /// The API reads `order` in upper case only: `asc` is ignored.
    #[rstest]
    #[case(ReaderCommentsOrder::Asc, "ASC")]
    #[case(ReaderCommentsOrder::Desc, "DESC")]
    fn test_order_is_sent_in_upper_case(#[case] order: ReaderCommentsOrder, #[case] sent: &str) {
        assert_eq!(order.to_string(), sent);
        assert_eq!(sent.parse::<ReaderCommentsOrder>().unwrap(), order);
    }
}

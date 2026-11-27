//! The sites and feeds the user is subscribed to in the Reader, and following and unfollowing one.
//!
//! The list and the follow check need a token: signed out, each is a 403
//! `authorization_required`. Following and unfollowing act for the user, so they need one too.
//!
//! There are two ways to follow. By its ID, for a WordPress.com or Jetpack site, which the apps
//! use when they have a site's ID. By its address, for anything with a feed, which is the only way
//! to follow an RSS feed, and how the web Reader follows.
//!
//! A subscription has three delivery settings, each turned on and off by its own pair of requests:
//! emails of new posts, emails of new comments, and notifications of new posts. They take the
//! site's ID, so they're for a WordPress.com or Jetpack site. What a subscription has turned on is
//! in its [`ReaderSubscriptionDeliveryMethods`].

use crate::{
    date::{WpGmtDateTime, deserialize_optional_wp_gmt_date_time},
    url_query::{
        AppendUrlQueryPairs, FromUrlQueryPairs, QueryPairs, QueryPairsExtension, UrlQueryPairsMap,
    },
    wp_com::{
        WpComSiteId,
        reader_saved::{ReaderFeedId, deserialize_optional_text},
    },
    wp_content_u64_id,
};
use serde::{Deserialize, Deserializer, Serialize};
use wp_derive::WpDeriveParamsField;
use wp_serde_helper::{
    LenientVec, deserialize_bool_or_int, deserialize_optional_bool_or_int,
    deserialize_u64_or_string_or_none,
};

wp_content_u64_id!(ReaderSubscriptionId);

/// The most subscriptions a page covers. The web Reader asks for this many, and says the API
/// returns no more. It wasn't seen in trials: the account they ran on has fewer subscriptions.
const MAX_PAGE_SIZE: u32 = 100;

/// Query parameters for the user's subscriptions. For the next page, add 1 to `page`, and see
/// [`ReaderSubscriptionsResponse::has_next_page`] for when to stop.
#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(false)] // The response has no `Link` header to read the next page from.
pub struct ReaderSubscriptionsParams {
    /// The page to return, counting from 1. `0` returns the first page.
    #[uniffi(default = None)]
    pub page: Option<u32>,
    /// The number of subscriptions a page covers. Ask for 100 at most, which is what the apps and
    /// the web Reader ask for. Without it, or with `0`, the API returned every subscription of an
    /// account that has 37.
    #[uniffi(default = None)]
    pub number: Option<u32>,
    /// Text to search the subscriptions for. In trials every search came back empty, even for the
    /// whole name or address of a subscription, so don't rely on it. An empty `q` is ignored.
    #[uniffi(default = None)]
    pub q: Option<String>,
    /// Only return some subscriptions. `a8c` returns the ones to Automattic's sites: it drops the
    /// others from each page after the page is cut, so a page of 4 can come back with 2. No other
    /// value tried changed the list.
    #[uniffi(default = None)]
    pub filter: Option<String>,
    /// Extra data to include, comma-separated. `site,feed`, which the apps send, makes the API add
    /// each subscription's whole site and feed, which [`ReaderSubscription`] doesn't keep.
    #[uniffi(default = None)]
    pub meta: Option<String>,
}

/// A page of the user's subscriptions, newest subscription first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
#[serde(from = "RawReaderSubscriptionsResponse")]
pub struct ReaderSubscriptionsResponse {
    /// The page's subscriptions, without any in [`Self::unreadable_subscriptions`].
    pub subscriptions: Vec<ReaderSubscription>,
    /// The subscriptions on the page that couldn't be read. Almost always empty.
    #[uniffi(default = [])]
    pub unreadable_subscriptions: Vec<ReaderUnreadableSubscription>,
    /// The page this is, counting from 1.
    #[uniffi(default = None)]
    pub page: Option<u32>,
    /// How many subscriptions the page has, which can be fewer than were asked for.
    #[uniffi(default = None)]
    pub number: Option<u32>,
    /// How many subscriptions the pages are cut from. It counts some that no page lists: an
    /// account with 39 gets 37 of them, whatever the page size. A first page that holds the whole
    /// list says 37 instead.
    #[uniffi(default = None)]
    pub total_subscriptions: Option<u64>,
}

impl ReaderSubscriptionsResponse {
    /// Whether the list has a page after this one. `number` is how many subscriptions a page
    /// were asked for.
    ///
    /// A short page isn't the end, and neither is an empty one. The API cuts each page from every
    /// subscription the user has, and then leaves out the ones it doesn't list. So the list ends
    /// once the pages asked for cover [`Self::total_subscriptions`], however many they returned.
    ///
    /// When the response doesn't say which page it is or how many there are, a page with nothing
    /// on it is taken as the end.
    pub fn has_next_page(&self, number: u32) -> bool {
        match (self.page, self.total_subscriptions) {
            (Some(page), Some(total)) if number > 0 => {
                u64::from(page) * u64::from(number.min(MAX_PAGE_SIZE)) < total
            }
            _ => !(self.subscriptions.is_empty() && self.unreadable_subscriptions.is_empty()),
        }
    }
}

#[derive(Deserialize)]
struct RawReaderSubscriptionsResponse {
    #[serde(default)]
    subscriptions: Option<LenientVec<ReaderSubscription>>,
    #[serde(default, deserialize_with = "deserialize_optional_u32")]
    page: Option<u32>,
    #[serde(default, deserialize_with = "deserialize_optional_u32")]
    number: Option<u32>,
    #[serde(default, deserialize_with = "deserialize_u64_or_string_or_none")]
    total_subscriptions: Option<u64>,
}

impl From<RawReaderSubscriptionsResponse> for ReaderSubscriptionsResponse {
    fn from(raw: RawReaderSubscriptionsResponse) -> Self {
        let subscriptions = raw.subscriptions.unwrap_or_default();
        let unreadable_subscriptions = subscriptions
            .invalid
            .into_iter()
            .map(|subscription| ReaderUnreadableSubscription {
                id: serde_json::from_str::<SubscriptionIdOnly>(subscription.json.get())
                    .ok()
                    .and_then(|subscription| subscription.id),
                error: subscription.error.to_string(),
            })
            .collect();
        Self {
            subscriptions: subscriptions.valid,
            unreadable_subscriptions,
            page: raw.page,
            number: raw.number,
            total_subscriptions: raw.total_subscriptions,
        }
    }
}

#[derive(Deserialize)]
struct SubscriptionIdOnly {
    #[serde(rename = "ID", default, deserialize_with = "deserialize_optional_id")]
    id: Option<ReaderSubscriptionId>,
}

/// A subscription on a page that couldn't be read. It doesn't fail the page: the page keeps the
/// subscriptions it can read, and lists the others as these.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, uniffi::Record)]
pub struct ReaderUnreadableSubscription {
    /// Which subscription it is. `None` when its ID couldn't be read either.
    #[uniffi(default = None)]
    pub id: Option<ReaderSubscriptionId>,
    /// Why it couldn't be read. A line and column in it count from the start of the subscription.
    pub error: String,
}

/// A subscription to a site or a feed.
///
/// What it's a subscription to is its [`Self::feed_id`], which every subscription has, and its
/// [`Self::blog_id`], which only a WordPress.com or Jetpack site has. [`Self::url`] is the address
/// that was followed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderSubscription {
    /// The subscription's own ID. It isn't the site's or the feed's: unfollowing by subscription
    /// takes it, and so does asking for one subscription's details.
    #[serde(rename = "ID", default, deserialize_with = "deserialize_optional_id")]
    #[uniffi(default = None)]
    pub id: Option<ReaderSubscriptionId>,
    /// The site, for a WordPress.com or Jetpack site. `None` for anything else, which the API
    /// sends as `0` for a site and `false` for an RSS feed.
    #[serde(
        rename = "blog_ID",
        default,
        deserialize_with = "deserialize_optional_id"
    )]
    #[uniffi(default = None)]
    pub blog_id: Option<WpComSiteId>,
    /// The feed. Every subscription seen has one, a WordPress.com site's too.
    #[serde(
        rename = "feed_ID",
        default,
        deserialize_with = "deserialize_optional_id"
    )]
    #[uniffi(default = None)]
    pub feed_id: Option<ReaderFeedId>,
    /// The address that was followed: a site's, or a feed's.
    #[serde(
        rename = "URL",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    #[uniffi(default = None)]
    pub url: Option<String>,
    /// The feed's address. Often not the same as [`Self::url`].
    #[serde(
        rename = "feed_URL",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    #[uniffi(default = None)]
    pub feed_url: Option<String>,
    /// The site's or feed's name.
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    #[uniffi(default = None)]
    pub name: Option<String>,
    /// The address of the site's icon. `None` for a site or feed without one.
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    #[uniffi(default = None)]
    pub site_icon: Option<String>,
    /// When the user subscribed.
    #[serde(default, deserialize_with = "deserialize_optional_wp_gmt_date_time")]
    #[uniffi(default = None)]
    pub date_subscribed: Option<WpGmtDateTime>,
    /// When the site or feed last had something new.
    #[serde(default, deserialize_with = "deserialize_optional_wp_gmt_date_time")]
    #[uniffi(default = None)]
    pub last_updated: Option<WpGmtDateTime>,
    /// How many of its posts the user hasn't seen.
    #[serde(default, deserialize_with = "deserialize_optional_count")]
    #[uniffi(default = None)]
    pub unseen_count: Option<u64>,
    /// `1` for an Automattic site, including its P2s, and `0` for a site in no organization.
    #[serde(default, deserialize_with = "deserialize_optional_count")]
    #[uniffi(default = None)]
    pub organization_id: Option<u64>,
    /// What the API calls the user owning the site. It's set for an Automattician's P2s too, so
    /// it isn't only the site's owner.
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    #[uniffi(default = None)]
    pub is_owner: Option<bool>,
    /// Whether it's an RSS feed rather than a site.
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    #[uniffi(default = None)]
    pub is_rss: Option<bool>,
    /// Whether the site is a P2.
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    #[uniffi(default = None)]
    pub is_wpforteams_site: Option<bool>,
    /// Whether the user pays for the subscription. `false` for every subscription seen.
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    #[uniffi(default = None)]
    pub is_paid_subscription: Option<bool>,
    /// Whether it's a complimentary subscription: a paid one the site gave the user for free.
    /// `false` for every subscription seen.
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    #[uniffi(default = None)]
    pub is_comp: Option<bool>,
    /// How the user hears about the subscription's new posts and comments.
    #[serde(default, deserialize_with = "deserialize_delivery_methods")]
    pub delivery_methods: ReaderSubscriptionDeliveryMethods,
}

/// How a subscription's new posts and comments reach the user.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderSubscriptionDeliveryMethods {
    /// By email. `None` when the subscription sends none: the API then leaves `email` out.
    #[serde(default, deserialize_with = "deserialize_object_or_none")]
    #[uniffi(default = None)]
    pub email: Option<ReaderSubscriptionEmailDelivery>,
    /// By notification.
    #[serde(default, deserialize_with = "deserialize_object_or_none")]
    #[uniffi(default = None)]
    pub notification: Option<ReaderSubscriptionNotificationDelivery>,
}

/// What a subscription emails the user.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderSubscriptionEmailDelivery {
    /// Whether new posts are emailed.
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    #[uniffi(default = None)]
    pub send_posts: Option<bool>,
    /// Whether new comments are emailed.
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    #[uniffi(default = None)]
    pub send_comments: Option<bool>,
    /// How often new posts are emailed.
    #[serde(default, deserialize_with = "deserialize_optional_frequency")]
    #[uniffi(default = None)]
    pub post_delivery_frequency: Option<ReaderSubscriptionEmailFrequency>,
    /// When the user subscribed by email, which can be later than
    /// [`ReaderSubscription::date_subscribed`].
    #[serde(default, deserialize_with = "deserialize_optional_wp_gmt_date_time")]
    #[uniffi(default = None)]
    pub date_subscribed: Option<WpGmtDateTime>,
}

/// How often a subscription emails new posts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Enum)]
#[serde(rename_all = "snake_case")]
pub enum ReaderSubscriptionEmailFrequency {
    /// As each post is published.
    Instantly,
    /// Once a day.
    Daily,
    /// Once a week.
    Weekly,
    /// A frequency this version doesn't know, as the API sent it.
    #[serde(untagged)]
    Other(String),
}

/// What a subscription notifies the user of.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderSubscriptionNotificationDelivery {
    /// Whether new posts send a notification.
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    #[uniffi(default = None)]
    pub send_posts: Option<bool>,
}

/// Whether the user follows a site, from `GET /rest/v1.1/sites/{site}/follows/mine`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderSiteFollowStatus {
    pub is_following: bool,
}

/// The answer to following or unfollowing a site by its ID. Both answer in this shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderFollowSiteResponse {
    /// Whether the user follows the site now.
    #[serde(deserialize_with = "deserialize_bool_or_int")]
    pub is_following: bool,
    /// `true` for an unfollow that removed the subscription. An unfollow with nothing to remove
    /// isn't a `false` here: it's a 400 `are_not_subscribed`.
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    #[uniffi(default = None)]
    pub success: Option<bool>,
}

/// Request body to follow a site or a feed by its address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, uniffi::Record)]
pub struct ReaderFollowUrlParams {
    /// The address of the site or feed to follow.
    pub url: String,
    /// Where the follow came from. The apps send `android` and `ios`. Other text is taken too,
    /// and none tried changed the answer.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[uniffi(default = None)]
    pub source: Option<String>,
}

/// The answer to following a site or a feed by its address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderFollowUrlResponse {
    /// Whether the user is subscribed now. `false`, in a 200, when the address wasn't followed:
    /// [`Self::info`] then says why.
    #[serde(deserialize_with = "deserialize_bool_or_int")]
    pub subscribed: bool,
    /// The new subscription. `None` when the address wasn't followed.
    ///
    /// It's the subscription the list returns, with fewer fields: its IDs, its
    /// [`ReaderSubscription::url`], when the user subscribed, and its delivery methods. It has no
    /// name, feed address, icon, counts or flags. Its `url` can differ from the address that was
    /// sent: one sent with `https` came back with `http`.
    #[serde(default, deserialize_with = "deserialize_object_or_none")]
    #[uniffi(default = None)]
    pub subscription: Option<ReaderSubscription>,
    /// Why the address wasn't followed: `invalid_feed` for one with no feed, such as a host that
    /// doesn't exist. `None` when it was followed.
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    #[uniffi(default = None)]
    pub info: Option<String>,
}

/// Request body to unfollow a site or a feed, by its subscription or by the address followed.
/// Send one of [`Self::sub_id`] and [`Self::url`].
///
/// Only an unfollow by `url` was tried. The other three are here because the API declares them
/// and the web Reader can send them: no effect of theirs was seen.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, uniffi::Record)]
pub struct ReaderUnfollowUrlParams {
    /// The subscription to remove: a [`ReaderSubscription::id`]. The web Reader sends it rather
    /// than `url` whenever it has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[uniffi(default = None)]
    pub sub_id: Option<ReaderSubscriptionId>,
    /// The address that was followed, for when the subscription's ID isn't known.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[uniffi(default = None)]
    pub url: Option<String>,
    /// The site to unsubscribe from.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[uniffi(default = None)]
    pub blog_id: Option<WpComSiteId>,
    /// An email's tracking ID, for an unfollow made from an email.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[uniffi(default = None)]
    pub email_id: Option<String>,
}

/// The answer to unfollowing by subscription or by address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderUnfollowUrlResponse {
    /// Whether the user is still subscribed: `false` for the one unfollow seen. The apps and the
    /// web Reader take anything else as a failure.
    #[serde(deserialize_with = "deserialize_bool_or_int")]
    pub subscribed: bool,
}

/// Request body to start or stop emailing the user a site's new posts.
///
/// The apps send none of these, and none was tried: every field is here because the API declares
/// it.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, uniffi::Record)]
pub struct ReaderPostEmailSubscriptionParams {
    /// How often to email the posts, for a subscription that's starting. The web Reader sends it
    /// when it starts one. A subscription started without it was
    /// [`ReaderSubscriptionEmailFrequency::Instantly`].
    #[serde(skip_serializing_if = "Option::is_none")]
    #[uniffi(default = None)]
    pub delivery_frequency: Option<ReaderSubscriptionEmailFrequency>,
    /// Where the change was made, which the API records.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[uniffi(default = None)]
    pub track_source: Option<String>,
    /// An email's tracking ID, for a change made from an email.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[uniffi(default = None)]
    pub email_id: Option<String>,
}

/// Request body to change how often a site's new posts are emailed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, uniffi::Record)]
pub struct ReaderPostEmailFrequencyParams {
    /// How often to email the posts. Send `Instantly`, `Daily` or `Weekly`: the API takes
    /// anything else as `Instantly`, without an error.
    pub delivery_frequency: ReaderSubscriptionEmailFrequency,
    /// Where the change was made, which the API records. It wasn't tried.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[uniffi(default = None)]
    pub track_source: Option<String>,
}

/// The answer to starting, stopping or changing the emails of a site's new posts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderPostEmailSubscriptionResponse {
    /// Whether the site's new posts are emailed to the user now.
    #[serde(deserialize_with = "deserialize_bool_or_int")]
    pub subscribed: bool,
    /// Go by [`Self::subscribed`] instead, as the apps do. One request to start the emails
    /// answered `success: false` with `subscribed: true` and a new subscription. It was sent for a
    /// site the user had just unfollowed. Every other answer seen had `true`, including a
    /// frequency change the API didn't apply as sent.
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    #[uniffi(default = None)]
    pub success: Option<bool>,
    /// The emails' settings. `None` once they're stopped, which the API sends as `null`.
    #[serde(default, deserialize_with = "deserialize_object_or_none")]
    #[uniffi(default = None)]
    pub subscription: Option<ReaderPostEmailSubscription>,
}

/// The emails of a site's new posts, in the answer to changing them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderPostEmailSubscription {
    /// The site. Sent as a string.
    #[serde(
        rename = "blog_ID",
        default,
        deserialize_with = "deserialize_optional_id"
    )]
    #[uniffi(default = None)]
    pub blog_id: Option<WpComSiteId>,
    /// How often the posts are emailed now. After a change, check it: a frequency the API doesn't
    /// know comes back as `Instantly`.
    #[serde(default, deserialize_with = "deserialize_optional_frequency")]
    #[uniffi(default = None)]
    pub delivery_frequency: Option<ReaderSubscriptionEmailFrequency>,
    /// `active` in every answer seen.
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    #[uniffi(default = None)]
    pub status: Option<String>,
    /// When the emails were started, which the API calls `ts`. A frequency change doesn't move it.
    #[serde(
        rename = "ts",
        default,
        deserialize_with = "deserialize_optional_wp_gmt_date_time"
    )]
    #[uniffi(default = None)]
    pub date_subscribed: Option<WpGmtDateTime>,
}

/// The answer to starting or stopping the emails of a site's new comments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderCommentEmailSubscriptionResponse {
    /// Whether the site's new comments are emailed to the user now.
    #[serde(deserialize_with = "deserialize_bool_or_int")]
    pub subscribed: bool,
    /// `true` in both answers seen. Go by [`Self::subscribed`], as the apps do.
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    #[uniffi(default = None)]
    pub success: Option<bool>,
    /// The emails' settings. `None` once they're stopped, which the API sends as `null`.
    #[serde(default, deserialize_with = "deserialize_object_or_none")]
    #[uniffi(default = None)]
    pub subscription: Option<ReaderCommentEmailSubscription>,
}

/// The emails of a site's new comments, in the answer to starting them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderCommentEmailSubscription {
    /// The site. Sent as a string.
    #[serde(
        rename = "blog_ID",
        default,
        deserialize_with = "deserialize_optional_id"
    )]
    #[uniffi(default = None)]
    pub blog_id: Option<WpComSiteId>,
    /// When the emails were started, which the API calls `ts`.
    #[serde(
        rename = "ts",
        default,
        deserialize_with = "deserialize_optional_wp_gmt_date_time"
    )]
    #[uniffi(default = None)]
    pub date_subscribed: Option<WpGmtDateTime>,
}

/// The answer to turning notifications of a site's new posts on or off.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderNotificationSubscriptionResponse {
    /// Whether the site's new posts send the user a notification now.
    #[serde(deserialize_with = "deserialize_bool_or_int")]
    pub subscribed: bool,
    /// `true` in both answers seen. Go by [`Self::subscribed`], as the apps do.
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    #[uniffi(default = None)]
    pub success: Option<bool>,
}

/// An ID the API sends as a number or a string, and as `0`, `false`, `""` or `null` when there
/// isn't one.
fn deserialize_optional_id<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: From<u64>,
{
    Ok(deserialize_optional_count(deserializer)?
        .filter(|id| *id != 0)
        .map(T::from))
}

/// A count the API sends as a number or a string. Anything that isn't one reads as `None` rather
/// than failing the subscription.
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

fn deserialize_optional_u32<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(deserialize_optional_count(deserializer)?.and_then(|count| u32::try_from(count).ok()))
}

/// `delivery_methods`, an object. PHP sends an empty one as `[]`.
fn deserialize_delivery_methods<'de, D>(
    deserializer: D,
) -> Result<ReaderSubscriptionDeliveryMethods, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(deserialize_object_or_none(deserializer)?.unwrap_or_default())
}

/// An object, or `None` for `null`, `false`, or an empty list, which PHP sends for an empty object.
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

fn deserialize_optional_frequency<'de, D>(
    deserializer: D,
) -> Result<Option<ReaderSubscriptionEmailFrequency>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(
        deserialize_optional_text(deserializer)?.map(|frequency| match frequency.as_str() {
            "instantly" => ReaderSubscriptionEmailFrequency::Instantly,
            "daily" => ReaderSubscriptionEmailFrequency::Daily,
            "weekly" => ReaderSubscriptionEmailFrequency::Weekly,
            _ => ReaderSubscriptionEmailFrequency::Other(frequency),
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use serde_json::json;

    fn page(json: &str) -> ReaderSubscriptionsResponse {
        serde_json::from_str(json).expect("the page parses")
    }

    /// A page recorded from WordPress.com, with one subscription of each kind seen, and the
    /// names, addresses and IDs replaced. It keeps the types the API sent: IDs are strings,
    /// and a missing `blog_ID` is `0` or `false`.
    #[test]
    fn test_deserialize_subscriptions() {
        let page = page(include_str!(
            "../../tests/wpcom/reader_subscriptions/subscriptions.json"
        ));

        assert_eq!(page.page, Some(1));
        assert_eq!(page.number, Some(4));
        assert_eq!(page.total_subscriptions, Some(39));
        assert!(page.unreadable_subscriptions.is_empty());
        let [emailed, team, feed, rss] = &page.subscriptions[..] else {
            panic!("Expected four subscriptions");
        };

        // A WordPress.com site, whose posts are emailed.
        assert_eq!(emailed.id, Some(ReaderSubscriptionId(900000004)));
        assert_eq!(emailed.blog_id, Some(WpComSiteId(100000001)));
        assert_eq!(emailed.feed_id, Some(ReaderFeedId(200000001)));
        assert_eq!(emailed.url.as_deref(), Some("https://example.blog"));
        assert_eq!(emailed.feed_url.as_deref(), Some("https://example.blog"));
        assert_eq!(emailed.name.as_deref(), Some("Example Blog"));
        assert_eq!(
            emailed.date_subscribed,
            Some("2026-11-03T23:17:59+00:00".parse().unwrap())
        );
        assert_eq!(emailed.unseen_count, Some(30));
        assert_eq!(emailed.organization_id, Some(0));
        assert_eq!(emailed.is_rss, Some(false));
        let email = emailed.delivery_methods.email.as_ref().expect("an email");
        assert_eq!(email.send_posts, Some(true));
        assert_eq!(email.send_comments, Some(false));
        assert_eq!(
            email.post_delivery_frequency,
            Some(ReaderSubscriptionEmailFrequency::Instantly)
        );
        // Sent without an offset, and read as UTC.
        assert_eq!(
            email.date_subscribed,
            Some("2026-11-04T08:00:00+00:00".parse().unwrap())
        );
        assert_eq!(
            emailed.delivery_methods.notification,
            Some(ReaderSubscriptionNotificationDelivery {
                send_posts: Some(true)
            })
        );

        // A P2 the user belongs to, with no email.
        assert_eq!(team.organization_id, Some(1));
        assert_eq!(team.is_wpforteams_site, Some(true));
        assert_eq!(team.is_owner, Some(true));
        assert_eq!(team.delivery_methods.email, None);

        // A site that isn't on WordPress.com: `blog_ID` is `0`, and it isn't RSS.
        assert_eq!(feed.blog_id, None);
        assert_eq!(feed.feed_id, Some(ReaderFeedId(200000003)));
        assert_eq!(feed.is_rss, Some(false));
        assert_eq!(feed.site_icon, None);
        assert_ne!(feed.url, feed.feed_url);

        // An RSS feed: `blog_ID` is `false`.
        assert_eq!(rss.blog_id, None);
        assert_eq!(rss.feed_id, Some(ReaderFeedId(200000004)));
        assert_eq!(rss.is_rss, Some(true));
    }

    /// What the API sends past the last page, and for a search that finds nothing.
    #[test]
    fn test_deserialize_an_empty_page() {
        let page =
            page(r#"{ "subscriptions": [], "page": 9, "number": 0, "total_subscriptions": 39 }"#);
        assert!(page.subscriptions.is_empty());
        assert_eq!(page.page, Some(9));
        assert_eq!(page.number, Some(0));
        assert!(!page.has_next_page(5));
    }

    #[rstest]
    // Pages of 5 over 39 subscriptions, of which the API lists 37. The seventh page is short,
    // with one more after it.
    #[case::first_page(json!({ "page": 1, "number": 5, "total": 39 }), 5, true)]
    #[case::short_page_with_more_to_come(json!({ "page": 7, "number": 4, "total": 39 }), 5, true)]
    #[case::last_page(json!({ "page": 8, "number": 3, "total": 39 }), 5, false)]
    #[case::past_the_end(json!({ "page": 9, "number": 0, "total": 39 }), 5, false)]
    // One page that holds the whole list, which then counts only what it lists.
    #[case::whole_list(json!({ "page": 1, "number": 37, "total": 37 }), 100, false)]
    // A page covers 100 at most, however many were asked for.
    #[case::asked_for_too_many(json!({ "page": 1, "number": 100, "total": 250 }), 500, true)]
    #[case::asked_for_too_many_second_page(json!({ "page": 2, "number": 100, "total": 250 }), 500, true)]
    #[case::asked_for_too_many_last_page(json!({ "page": 3, "number": 50, "total": 250 }), 500, false)]
    fn test_has_next_page(
        #[case] sent: serde_json::Value,
        #[case] number_asked_for: u32,
        #[case] expected: bool,
    ) {
        let subscriptions: Vec<_> = (0..sent["number"].as_u64().unwrap())
            .map(|id| json!({ "ID": id + 1 }))
            .collect();
        let page: ReaderSubscriptionsResponse = serde_json::from_value(json!({
            "subscriptions": subscriptions,
            "page": sent["page"],
            "number": sent["number"],
            "total_subscriptions": sent["total"],
        }))
        .expect("the page parses");
        assert_eq!(page.has_next_page(number_asked_for), expected);
    }

    /// Without a page or a total to go by, only an empty page ends the list.
    #[test]
    fn test_has_next_page_without_a_total() {
        let some: ReaderSubscriptionsResponse =
            serde_json::from_value(json!({ "subscriptions": [{ "ID": 1 }] })).unwrap();
        assert!(some.has_next_page(100));
        let none: ReaderSubscriptionsResponse =
            serde_json::from_value(json!({ "subscriptions": [] })).unwrap();
        assert!(!none.has_next_page(100));
    }

    #[rstest]
    #[case::string(json!("123"), Some(123))]
    #[case::number(json!(123), Some(123))]
    #[case::zero(json!(0), None)]
    #[case::zero_string(json!("0"), None)]
    #[case::false_(json!(false), None)]
    #[case::null(json!(null), None)]
    #[case::empty(json!(""), None)]
    #[case::negative(json!(-1), None)]
    fn test_ids_that_vary_in_shape(#[case] sent: serde_json::Value, #[case] expected: Option<u64>) {
        let subscription: ReaderSubscription =
            serde_json::from_value(json!({ "ID": sent, "blog_ID": sent, "feed_ID": sent }))
                .expect("the subscription parses");
        assert_eq!(subscription.id, expected.map(ReaderSubscriptionId));
        assert_eq!(subscription.blog_id, expected.map(WpComSiteId));
        assert_eq!(subscription.feed_id, expected.map(ReaderFeedId));
    }

    #[rstest]
    #[case::left_out(json!({ "ID": 1 }))]
    #[case::empty_list(json!({ "ID": 1, "delivery_methods": [] }))]
    #[case::empty_object(json!({ "ID": 1, "delivery_methods": {} }))]
    #[case::null(json!({ "ID": 1, "delivery_methods": null }))]
    fn test_a_subscription_without_delivery_methods(#[case] subscription: serde_json::Value) {
        let subscription: ReaderSubscription =
            serde_json::from_value(subscription).expect("the subscription parses");
        assert_eq!(
            subscription.delivery_methods,
            ReaderSubscriptionDeliveryMethods::default()
        );
    }

    /// A site the user doesn't follow has `null` and `[]` where a subscription has settings.
    #[rstest]
    #[case(json!({ "email": null, "notification": [] }))]
    #[case(json!({ "email": false, "notification": null }))]
    #[case(json!({ "email": [], "notification": {} }))]
    fn test_delivery_methods_that_are_not_set(#[case] delivery_methods: serde_json::Value) {
        let subscription: ReaderSubscription =
            serde_json::from_value(json!({ "ID": 1, "delivery_methods": delivery_methods }))
                .expect("the subscription parses");
        assert_eq!(subscription.delivery_methods.email, None);
        let notification = subscription.delivery_methods.notification;
        assert!(notification.is_none_or(|notification| notification.send_posts.is_none()));
    }

    #[rstest]
    #[case("instantly", ReaderSubscriptionEmailFrequency::Instantly)]
    #[case("daily", ReaderSubscriptionEmailFrequency::Daily)]
    #[case("weekly", ReaderSubscriptionEmailFrequency::Weekly)]
    #[case("monthly", ReaderSubscriptionEmailFrequency::Other("monthly".to_string()))]
    fn test_email_frequency(
        #[case] sent: &str,
        #[case] expected: ReaderSubscriptionEmailFrequency,
    ) {
        let email: ReaderSubscriptionEmailDelivery =
            serde_json::from_value(json!({ "post_delivery_frequency": sent })).unwrap();
        assert_eq!(email.post_delivery_frequency, Some(expected.clone()));
        // It's sent the way it's read, for the requests that change it.
        assert_eq!(serde_json::to_value(&expected).unwrap(), json!(sent));
    }

    #[test]
    fn test_a_page_keeps_its_subscriptions_when_one_cannot_be_read() {
        let page = page(
            r#"{
                "subscriptions": [
                    { "ID": "1", "blog_ID": "10", "feed_ID": "20" },
                    { "ID": "2", "date_subscribed": "2026-11-16T16:45:34+91:00" },
                    "not a subscription",
                    { "ID": "4", "blog_ID": false, "feed_ID": "40" }
                ],
                "page": 1,
                "number": 4,
                "total_subscriptions": 4
            }"#,
        );

        let ids: Vec<_> = page.subscriptions.iter().map(|s| s.id).collect();
        assert_eq!(
            ids,
            [Some(ReaderSubscriptionId(1)), Some(ReaderSubscriptionId(4))]
        );
        let unreadable: Vec<_> = page.unreadable_subscriptions.iter().map(|s| s.id).collect();
        assert_eq!(unreadable, [Some(ReaderSubscriptionId(2)), None]);
        assert!(
            page.unreadable_subscriptions[0]
                .error
                .starts_with("Invalid date format"),
            "{}",
            page.unreadable_subscriptions[0].error
        );
    }

    #[rstest]
    #[case(json!({ "subscriptions": null }))]
    #[case(json!({}))]
    fn test_a_page_without_subscriptions(#[case] page: serde_json::Value) {
        let page: ReaderSubscriptionsResponse =
            serde_json::from_value(page).expect("the page parses");
        assert!(page.subscriptions.is_empty());
        assert!(page.unreadable_subscriptions.is_empty());
    }

    #[test]
    fn test_a_page_that_is_not_one_is_an_error() {
        let result =
            serde_json::from_value::<ReaderSubscriptionsResponse>(json!({ "subscriptions": 7 }));
        assert!(result.is_err());
    }

    /// The answer to unfollowing a site by its ID, recorded from WordPress.com, with the site
    /// replaced.
    #[test]
    fn test_deserialize_unfollow_site() {
        let answer: ReaderFollowSiteResponse = serde_json::from_str(include_str!(
            "../../tests/wpcom/reader_subscriptions/unfollow_site.json"
        ))
        .expect("the answer parses");
        assert_eq!(
            answer,
            ReaderFollowSiteResponse {
                is_following: false,
                success: Some(true),
            }
        );
    }

    /// The answer to a follow by ID, recorded from WordPress.com, with the site's ID replaced.
    #[test]
    fn test_deserialize_follow_site() {
        let answer: ReaderFollowSiteResponse = serde_json::from_str(include_str!(
            "../../tests/wpcom/reader_subscriptions/follow_site.json"
        ))
        .expect("the answer parses");
        assert_eq!(
            answer,
            ReaderFollowSiteResponse {
                is_following: true,
                success: Some(true),
            }
        );
    }

    /// The same answer with numbers for its flags, and an answer with only `is_following` in it.
    #[rstest]
    #[case(json!({ "success": true, "is_following": true }), Some(true))]
    #[case(json!({ "success": 1, "is_following": 1 }), Some(true))]
    #[case(json!({ "is_following": true }), None)]
    fn test_a_follow_site_answer(#[case] answer: serde_json::Value, #[case] success: Option<bool>) {
        let answer: ReaderFollowSiteResponse =
            serde_json::from_value(answer).expect("the answer parses");
        assert!(answer.is_following);
        assert_eq!(answer.success, success);
    }

    #[test]
    fn test_a_follow_site_answer_without_is_following_is_an_error() {
        let result = serde_json::from_value::<ReaderFollowSiteResponse>(json!({ "success": true }));
        assert!(result.is_err());
    }

    /// The answer to following a WordPress.com site by its address, recorded from WordPress.com,
    /// with the site and the IDs replaced. It keeps the types the API sent: the subscription's
    /// `ID` and `blog_ID` are strings, and its `feed_ID` is a number, which the list sends as a
    /// string.
    #[test]
    fn test_deserialize_follow_url() {
        let answer: ReaderFollowUrlResponse = serde_json::from_str(include_str!(
            "../../tests/wpcom/reader_subscriptions/follow_url.json"
        ))
        .expect("the answer parses");

        assert!(answer.subscribed);
        assert_eq!(answer.info, None);
        let subscription = answer.subscription.expect("a subscription");
        assert_eq!(subscription.id, Some(ReaderSubscriptionId(900000005)));
        assert_eq!(subscription.blog_id, Some(WpComSiteId(100000001)));
        assert_eq!(subscription.feed_id, Some(ReaderFeedId(200000001)));
        assert_eq!(
            subscription.url.as_deref(),
            Some("http://example.wordpress.com")
        );
        assert_eq!(
            subscription.date_subscribed,
            Some("2026-11-19T07:14:56+00:00".parse().unwrap())
        );
        let email = subscription.delivery_methods.email.expect("an email");
        assert_eq!(email.send_posts, Some(true));
        assert_eq!(email.send_comments, Some(false));
        assert_eq!(
            email.post_delivery_frequency,
            Some(ReaderSubscriptionEmailFrequency::Instantly)
        );
        assert_eq!(
            subscription.delivery_methods.notification,
            Some(ReaderSubscriptionNotificationDelivery {
                send_posts: Some(false)
            })
        );
        // What the list sends and a follow doesn't.
        assert_eq!(subscription.name, None);
        assert_eq!(subscription.feed_url, None);
        assert_eq!(subscription.is_rss, None);
    }

    /// What the API sends, in a 200, for an address with no feed.
    #[test]
    fn test_deserialize_follow_url_without_a_feed() {
        let answer: ReaderFollowUrlResponse =
            serde_json::from_str(r#"{ "info": "invalid_feed", "subscribed": false }"#)
                .expect("the answer parses");
        assert_eq!(
            answer,
            ReaderFollowUrlResponse {
                subscribed: false,
                subscription: None,
                info: Some("invalid_feed".to_string()),
            }
        );
    }

    #[rstest]
    #[case(json!({ "subscribed": false, "subscription": null, "info": false }))]
    #[case(json!({ "subscribed": 0, "subscription": [], "info": "" }))]
    fn test_a_follow_url_answer_with_nothing_set(#[case] answer: serde_json::Value) {
        let answer: ReaderFollowUrlResponse =
            serde_json::from_value(answer).expect("the answer parses");
        assert!(!answer.subscribed);
        assert_eq!(answer.subscription, None);
        assert_eq!(answer.info, None);
    }

    #[test]
    fn test_deserialize_unfollow_url() {
        let answer: ReaderUnfollowUrlResponse =
            serde_json::from_str(r#"{ "subscribed": false }"#).expect("the answer parses");
        assert!(!answer.subscribed);
    }

    #[test]
    fn test_the_body_to_follow_a_url() {
        let with_source = ReaderFollowUrlParams {
            url: "https://example.wordpress.com".to_string(),
            source: Some("ios".to_string()),
        };
        assert_eq!(
            serde_json::to_value(&with_source).unwrap(),
            json!({ "url": "https://example.wordpress.com", "source": "ios" })
        );

        let without = ReaderFollowUrlParams {
            url: "https://example.wordpress.com/feed/".to_string(),
            source: None,
        };
        assert_eq!(
            serde_json::to_value(&without).unwrap(),
            json!({ "url": "https://example.wordpress.com/feed/" })
        );
    }

    #[test]
    fn test_the_body_to_unfollow() {
        let by_url = ReaderUnfollowUrlParams {
            url: Some("https://example.wordpress.com".to_string()),
            ..Default::default()
        };
        assert_eq!(
            serde_json::to_value(&by_url).unwrap(),
            json!({ "url": "https://example.wordpress.com" })
        );

        let by_subscription = ReaderUnfollowUrlParams {
            sub_id: Some(ReaderSubscriptionId(900000005)),
            ..Default::default()
        };
        assert_eq!(
            serde_json::to_value(&by_subscription).unwrap(),
            json!({ "sub_id": 900000005 })
        );

        let everything = ReaderUnfollowUrlParams {
            sub_id: Some(ReaderSubscriptionId(900000005)),
            url: Some("https://example.wordpress.com".to_string()),
            blog_id: Some(WpComSiteId(100000001)),
            email_id: Some("abc123".to_string()),
        };
        assert_eq!(
            serde_json::to_value(&everything).unwrap(),
            json!({
                "sub_id": 900000005,
                "url": "https://example.wordpress.com",
                "blog_id": 100000001,
                "email_id": "abc123",
            })
        );
    }

    /// The answer to changing how often a site's posts are emailed, recorded from WordPress.com,
    /// with the site replaced. Starting the emails answers in the same shape.
    #[test]
    fn test_deserialize_post_email_subscription() {
        let answer: ReaderPostEmailSubscriptionResponse = serde_json::from_str(include_str!(
            "../../tests/wpcom/reader_subscriptions/post_email_subscription.json"
        ))
        .expect("the answer parses");

        assert!(answer.subscribed);
        assert_eq!(answer.success, Some(true));
        assert_eq!(
            answer.subscription,
            Some(ReaderPostEmailSubscription {
                blog_id: Some(WpComSiteId(100000001)),
                delivery_frequency: Some(ReaderSubscriptionEmailFrequency::Daily),
                status: Some("active".to_string()),
                // Sent as `2026-11-19 07:14:48`, without an offset, and read as UTC.
                date_subscribed: Some("2026-11-19T07:14:48+00:00".parse().unwrap()),
            })
        );
    }

    /// What the API sent to start the emails for a site the user had just unfollowed: the emails
    /// were started, and `success` was `false`.
    #[test]
    fn test_post_emails_started_without_success() {
        let answer: ReaderPostEmailSubscriptionResponse = serde_json::from_value(json!({
            "success": false,
            "subscribed": true,
            "subscription": {
                "blog_ID": "100000001",
                "delivery_frequency": "instantly",
                "status": "active",
                "ts": "2026-11-19 07:14:48",
            },
        }))
        .expect("the answer parses");

        assert!(answer.subscribed);
        assert_eq!(answer.success, Some(false));
        let subscription = answer.subscription.expect("a subscription");
        assert_eq!(
            subscription.delivery_frequency,
            Some(ReaderSubscriptionEmailFrequency::Instantly)
        );
    }

    /// What the API sends once the emails of posts are stopped.
    #[test]
    fn test_deserialize_post_emails_stopped() {
        let answer: ReaderPostEmailSubscriptionResponse = serde_json::from_str(
            r#"{ "success": true, "subscribed": false, "subscription": null }"#,
        )
        .expect("the answer parses");
        assert_eq!(
            answer,
            ReaderPostEmailSubscriptionResponse {
                subscribed: false,
                success: Some(true),
                subscription: None,
            }
        );
    }

    /// The answer to starting the emails of a site's comments, recorded from WordPress.com, with
    /// the site replaced.
    #[test]
    fn test_deserialize_comment_email_subscription() {
        let answer: ReaderCommentEmailSubscriptionResponse = serde_json::from_str(include_str!(
            "../../tests/wpcom/reader_subscriptions/comment_email_subscription.json"
        ))
        .expect("the answer parses");

        assert!(answer.subscribed);
        assert_eq!(answer.success, Some(true));
        assert_eq!(
            answer.subscription,
            Some(ReaderCommentEmailSubscription {
                blog_id: Some(WpComSiteId(100000001)),
                date_subscribed: Some("2026-11-19T07:14:51+00:00".parse().unwrap()),
            })
        );
    }

    /// What the API sends once the emails of comments are stopped.
    #[test]
    fn test_deserialize_comment_emails_stopped() {
        let answer: ReaderCommentEmailSubscriptionResponse = serde_json::from_str(
            r#"{ "success": true, "subscribed": false, "subscription": null, "meta": { "links": {} } }"#,
        )
        .expect("the answer parses");
        assert!(!answer.subscribed);
        assert_eq!(answer.subscription, None);
    }

    #[rstest]
    #[case(r#"{ "success": true, "subscribed": true }"#, true)]
    #[case(r#"{ "success": true, "subscribed": false }"#, false)]
    fn test_deserialize_notification_subscription(#[case] json: &str, #[case] subscribed: bool) {
        let answer: ReaderNotificationSubscriptionResponse =
            serde_json::from_str(json).expect("the answer parses");
        assert_eq!(
            answer,
            ReaderNotificationSubscriptionResponse {
                subscribed,
                success: Some(true),
            }
        );
    }

    /// Only `subscribed` has to be in each answer.
    #[test]
    fn test_delivery_answers_with_only_subscribed() {
        let json = r#"{ "subscribed": 1 }"#;
        let posts: ReaderPostEmailSubscriptionResponse = serde_json::from_str(json).unwrap();
        let comments: ReaderCommentEmailSubscriptionResponse = serde_json::from_str(json).unwrap();
        let notifications: ReaderNotificationSubscriptionResponse =
            serde_json::from_str(json).unwrap();
        assert!(posts.subscribed && comments.subscribed && notifications.subscribed);
        assert_eq!(
            (posts.success, comments.success, notifications.success),
            (None, None, None)
        );

        let without = r#"{ "success": true }"#;
        assert!(serde_json::from_str::<ReaderPostEmailSubscriptionResponse>(without).is_err());
        assert!(serde_json::from_str::<ReaderCommentEmailSubscriptionResponse>(without).is_err());
        assert!(serde_json::from_str::<ReaderNotificationSubscriptionResponse>(without).is_err());
    }

    #[test]
    fn test_the_body_to_start_or_stop_post_emails() {
        // With nothing set it's an empty object, which is what the trials sent.
        assert_eq!(
            serde_json::to_value(ReaderPostEmailSubscriptionParams::default()).unwrap(),
            json!({})
        );

        let everything = ReaderPostEmailSubscriptionParams {
            delivery_frequency: Some(ReaderSubscriptionEmailFrequency::Weekly),
            track_source: Some("reader".to_string()),
            email_id: Some("abc123".to_string()),
        };
        assert_eq!(
            serde_json::to_value(&everything).unwrap(),
            json!({ "delivery_frequency": "weekly", "track_source": "reader", "email_id": "abc123" })
        );
    }

    #[test]
    fn test_the_body_to_change_the_post_email_frequency() {
        let daily = ReaderPostEmailFrequencyParams {
            delivery_frequency: ReaderSubscriptionEmailFrequency::Daily,
            track_source: None,
        };
        assert_eq!(
            serde_json::to_value(&daily).unwrap(),
            json!({ "delivery_frequency": "daily" })
        );

        let with_source = ReaderPostEmailFrequencyParams {
            delivery_frequency: ReaderSubscriptionEmailFrequency::Instantly,
            track_source: Some("reader".to_string()),
        };
        assert_eq!(
            serde_json::to_value(&with_source).unwrap(),
            json!({ "delivery_frequency": "instantly", "track_source": "reader" })
        );
    }

    #[rstest]
    #[case(r#"{"is_following":true,"meta":{"links":{"site":"https://public-api.wordpress.com/rest/v1.1/sites/3584907"}}}"#, true)]
    #[case(r#"{"is_following":false,"meta":{"links":{"site":"https://public-api.wordpress.com/rest/v1.1/sites/3584907"}}}"#, false)]
    fn test_deserialize_site_follow_status(#[case] json: &str, #[case] expected: bool) {
        let status: ReaderSiteFollowStatus = serde_json::from_str(json).expect("it parses");
        assert_eq!(status.is_following, expected);
    }
}

//! The Reader's sites and feeds: a site's info, a feed's info, and the search that finds them.
//!
//! The Reader knows a source of posts two ways. A WordPress.com or Jetpack **site** has a site ID,
//! and a **feed** has a feed ID. Most sites have a feed, and a feed can belong to a site, so the
//! same source can be asked about either way. The two answers link to each other:
//!
//! - [`ReaderSite::feed_id`] is the site's feed. It's `None` for a site with no feed, for which
//!   the API leaves `feed_ID` out.
//! - [`ReaderFeed::site_id`] is the feed's site. It's `None` for a feed that belongs to no site,
//!   such as an RSS feed from elsewhere, for which the API sends a `blog_ID` of `"0"`.
//!
//! A site can have more than one feed, one for each address it was followed by, so the feed a
//! search finds for a site's address isn't always the site's [`ReaderSite::feed_id`]. The two IDs
//! are also different kinds of number: a feed ID can equal the site ID of an unrelated site.

use crate::{
    date::{WpGmtDateTime, deserialize_optional_wp_gmt_date_time},
    html_entities::decode_html_entities,
    impl_as_query_value_from_to_string,
    url_query::{
        AppendUrlQueryPairs, FromUrlQueryPairs, QueryPairs, QueryPairsExtension, UrlQueryPairsMap,
    },
    wp_com::{
        WpComSiteId,
        me::WpComUserId,
        reader_saved::{ReaderFeedId, deserialize_optional_text},
        reader_subscriptions::{ReaderSubscriptionDeliveryMethods, ReaderSubscriptionId},
    },
};
use serde::{Deserialize, Deserializer, Serialize, de::DeserializeOwned};
use wp_derive::WpDeriveParamsField;
use wp_serde_helper::deserialize_optional_bool_or_int;

/// Query parameters for a site's Reader info.
#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(false)]
pub struct ReaderSiteParams {
    /// The fields of the site to return, comma-separated, such as `ID,name,feed_ID,is_following`.
    /// Without it the site arrives with every field, which for a site the user can manage is
    /// about 75 kB.
    ///
    /// `ID` has to be among them, or the request fails.
    #[uniffi(default = None)]
    pub fields: Option<String>,
    /// Which of the site's options to return, comma-separated, such as
    /// `is_mapped_domain,unmapped_url`.
    /// Only someone who can manage the site is sent any options, and [`ReaderSite`] keeps none of
    /// them, so all this does is make that response smaller.
    #[uniffi(default = None)]
    pub options: Option<String>,
}

/// A WordPress.com or Jetpack site, as the Reader shows it.
///
/// Signed out, the API still answers for a public site, with nothing about the user:
/// [`Self::is_following`] is `false`, [`Self::unseen_count`] is `0`, [`Self::delivery_methods`]
/// says nothing is sent, and [`Self::lang`], [`Self::post_count`] and [`Self::is_visible`] are
/// `None`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
#[serde(from = "RawReaderSite")]
pub struct ReaderSite {
    pub site_id: WpComSiteId,
    /// The site's feed, to ask for its [`ReaderFeed`] or its posts as a feed. `None` when the site
    /// has no feed.
    #[uniffi(default = None)]
    pub feed_id: Option<ReaderFeedId>,
    /// Plain text. `None` for a site with no name.
    #[uniffi(default = None)]
    pub name: Option<String>,
    /// Plain text.
    #[uniffi(default = None)]
    pub description: Option<String>,
    #[uniffi(default = None)]
    pub url: Option<String>,
    /// The address the site's feed was found at. `None` when the site has no feed.
    #[uniffi(default = None)]
    pub feed_url: Option<String>,
    /// The site's icon: `icon.img`, else `icon.ico`. About half of all sites have none.
    #[uniffi(default = None)]
    pub icon_url: Option<String>,
    /// The site's language, such as `en`. `None` signed out.
    #[uniffi(default = None)]
    pub lang: Option<String>,
    /// `None` signed out.
    #[uniffi(default = None)]
    pub post_count: Option<u64>,
    #[uniffi(default = None)]
    pub subscribers_count: Option<u64>,
    #[uniffi(default = None)]
    pub is_private: Option<bool>,
    #[uniffi(default = None)]
    pub is_coming_soon: Option<bool>,
    #[uniffi(default = None)]
    pub is_jetpack: Option<bool>,
    #[uniffi(default = None)]
    pub is_atomic: Option<bool>,
    #[uniffi(default = None)]
    pub is_multi_author: Option<bool>,
    /// Whether the site is shown in the user's list of sites. `None` signed out.
    #[uniffi(default = None)]
    pub is_visible: Option<bool>,
    /// `1` for an Automattic site and `2` for a P2, else `0`.
    #[uniffi(default = None)]
    pub organization_id: Option<u64>,
    /// Set on a few Jetpack sites. `None` on every other site.
    #[uniffi(default = None)]
    pub prefer_feed: Option<bool>,
    /// `None` for the few sites the API sends no owner for.
    #[uniffi(default = None)]
    pub owner: Option<ReaderSiteOwner>,
    /// Whether the user follows the site. `false` signed out.
    #[uniffi(default = None)]
    pub is_following: Option<bool>,
    /// Whether the user blocked the site.
    #[uniffi(default = None)]
    pub is_blocked: Option<bool>,
    /// How many of the site's posts the user hasn't seen. `0` signed out.
    #[uniffi(default = None)]
    pub unseen_count: Option<u64>,
    /// How the user is sent the site's new posts, from `subscription.delivery_methods`. The API
    /// sends it for a site the user doesn't follow too, with nothing turned on.
    #[uniffi(default = None)]
    pub delivery_methods: Option<ReaderSubscriptionDeliveryMethods>,
}

/// The owner of a [`ReaderSite`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderSiteOwner {
    #[serde(
        rename = "ID",
        default,
        deserialize_with = "deserialize_optional_nonzero_id"
    )]
    #[uniffi(default = None)]
    pub id: Option<WpComUserId>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    #[uniffi(default = None)]
    pub login: Option<String>,
    /// Can carry character references such as `&amp;`. Decode it with
    /// `decode_html_entities` to show it as text.
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
    #[serde(
        rename = "URL",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    #[uniffi(default = None)]
    pub url: Option<String>,
    #[serde(
        rename = "avatar_URL",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    #[uniffi(default = None)]
    pub avatar_url: Option<String>,
    #[serde(
        rename = "profile_URL",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    #[uniffi(default = None)]
    pub profile_url: Option<String>,
}

/// Every field a site can arrive with, before the nested ones are flattened.
#[derive(Deserialize)]
struct RawReaderSite {
    #[serde(
        rename = "ID",
        deserialize_with = "wp_serde_helper::deserialize_u64_or_string_as_t"
    )]
    site_id: WpComSiteId,
    #[serde(
        rename = "feed_ID",
        default,
        deserialize_with = "deserialize_optional_nonzero_id"
    )]
    feed_id: Option<ReaderFeedId>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    description: Option<String>,
    #[serde(
        rename = "URL",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    url: Option<String>,
    #[serde(
        rename = "feed_URL",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    feed_url: Option<String>,
    #[serde(default, deserialize_with = "deserialize_object_or_none")]
    icon: Option<RawIcon>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    lang: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_count")]
    post_count: Option<u64>,
    #[serde(default, deserialize_with = "deserialize_optional_count")]
    subscribers_count: Option<u64>,
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    is_private: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    is_coming_soon: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    jetpack: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    is_wpcom_atomic: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    is_multi_author: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    visible: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_optional_count")]
    organization_id: Option<u64>,
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    prefer_feed: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_object_or_none")]
    owner: Option<ReaderSiteOwner>,
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    is_following: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    is_blocked: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_optional_count")]
    unseen_count: Option<u64>,
    #[serde(default, deserialize_with = "deserialize_object_or_none")]
    subscription: Option<RawSubscription>,
}

#[derive(Deserialize)]
struct RawIcon {
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    img: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    ico: Option<String>,
}

#[derive(Deserialize)]
struct RawSubscription {
    #[serde(default, deserialize_with = "deserialize_object_or_none")]
    delivery_methods: Option<ReaderSubscriptionDeliveryMethods>,
}

impl From<RawReaderSite> for ReaderSite {
    fn from(raw: RawReaderSite) -> Self {
        Self {
            site_id: raw.site_id,
            feed_id: raw.feed_id,
            name: raw.name,
            description: raw.description,
            url: raw.url,
            feed_url: raw.feed_url,
            icon_url: raw.icon.and_then(|icon| icon.img.or(icon.ico)),
            lang: raw.lang,
            post_count: raw.post_count,
            subscribers_count: raw.subscribers_count,
            is_private: raw.is_private,
            is_coming_soon: raw.is_coming_soon,
            is_jetpack: raw.jetpack,
            is_atomic: raw.is_wpcom_atomic,
            is_multi_author: raw.is_multi_author,
            is_visible: raw.visible,
            organization_id: raw.organization_id,
            prefer_feed: raw.prefer_feed,
            owner: raw.owner,
            is_following: raw.is_following,
            is_blocked: raw.is_blocked,
            unseen_count: raw.unseen_count,
            delivery_methods: raw
                .subscription
                .and_then(|subscription| subscription.delivery_methods),
        }
    }
}

/// A feed, as the Reader shows it: an RSS feed from anywhere, or the feed of a WordPress.com or
/// Jetpack site.
///
/// Signed out, the API still answers for a public feed, with nothing about the user:
/// [`Self::is_following`] is `false`, [`Self::subscription_id`] is `None` and
/// [`Self::unseen_count`] is `0`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
#[serde(from = "RawReaderFeed")]
pub struct ReaderFeed {
    pub feed_id: ReaderFeedId,
    /// The WordPress.com or Jetpack site the feed belongs to, to ask for its [`ReaderSite`] or its
    /// posts as a site. `None` when the feed belongs to no site.
    #[uniffi(default = None)]
    pub site_id: Option<WpComSiteId>,
    /// Can carry character references such as `&amp;`. Decode it with `decode_html_entities` to
    /// show it as text. `None` for a feed the API has no name for.
    #[uniffi(default = None)]
    pub name: Option<String>,
    /// Can carry character references such as `&amp;`. Decode it with `decode_html_entities` to
    /// show it as text.
    #[uniffi(default = None)]
    pub description: Option<String>,
    /// The address of the site the feed is for.
    #[uniffi(default = None)]
    pub url: Option<String>,
    /// The address the feed was followed by.
    #[uniffi(default = None)]
    pub feed_url: Option<String>,
    /// The address the feed is fetched from, after any redirect. Sites often have none.
    #[uniffi(default = None)]
    pub resolved_feed_url: Option<String>,
    /// The feed's image, small. Many feeds have none.
    #[uniffi(default = None)]
    pub image_url: Option<String>,
    #[uniffi(default = None)]
    pub subscribers_count: Option<u64>,
    /// `1` for an Automattic site's feed and `2` for a P2's, else `0`.
    #[uniffi(default = None)]
    pub organization_id: Option<u64>,
    /// The owner of the feed's site. `None` when the feed belongs to no site.
    #[uniffi(default = None)]
    pub owner: Option<ReaderFeedOwner>,
    /// When the feed's newest post was published.
    #[uniffi(default = None)]
    pub last_update: Option<WpGmtDateTime>,
    /// When the feed was last fetched.
    #[uniffi(default = None)]
    pub last_checked: Option<WpGmtDateTime>,
    #[uniffi(default = None)]
    pub marked_for_refresh: Option<bool>,
    /// When the feed is next fetched. Most feeds have none.
    #[uniffi(default = None)]
    pub next_refresh_time: Option<WpGmtDateTime>,
    /// Whether the user follows the feed. `false` signed out.
    #[uniffi(default = None)]
    pub is_following: Option<bool>,
    /// The user's subscription to the feed. `None` when they don't follow it.
    #[uniffi(default = None)]
    pub subscription_id: Option<ReaderSubscriptionId>,
    /// How many of the feed's posts the user hasn't seen. Signed in, it counts for a feed the user
    /// doesn't follow too. `0` signed out.
    #[uniffi(default = None)]
    pub unseen_count: Option<u64>,
}

/// The owner of the site a [`ReaderFeed`] belongs to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderFeedOwner {
    #[serde(
        rename = "ID",
        default,
        deserialize_with = "deserialize_optional_nonzero_id"
    )]
    #[uniffi(default = None)]
    pub id: Option<WpComUserId>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    #[uniffi(default = None)]
    pub name: Option<String>,
}

/// Every field a feed can arrive with. The API sends `feed_ID` and `blog_ID` as strings here.
#[derive(Deserialize)]
struct RawReaderFeed {
    #[serde(
        rename = "feed_ID",
        deserialize_with = "wp_serde_helper::deserialize_u64_or_string_as_t"
    )]
    feed_id: ReaderFeedId,
    #[serde(
        rename = "blog_ID",
        default,
        deserialize_with = "deserialize_optional_nonzero_id"
    )]
    site_id: Option<WpComSiteId>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    description: Option<String>,
    #[serde(
        rename = "URL",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    url: Option<String>,
    #[serde(
        rename = "feed_URL",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    feed_url: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    resolved_feed_url: Option<String>,
    /// HTML-escaped: `&` arrives as `&amp;`.
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    image: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_count")]
    subscribers_count: Option<u64>,
    #[serde(default, deserialize_with = "deserialize_optional_count")]
    organization_id: Option<u64>,
    #[serde(default, deserialize_with = "deserialize_object_or_none")]
    blog_owner: Option<ReaderFeedOwner>,
    #[serde(default, deserialize_with = "deserialize_optional_wp_gmt_date_time")]
    last_update: Option<WpGmtDateTime>,
    #[serde(default, deserialize_with = "deserialize_optional_wp_gmt_date_time")]
    last_checked: Option<WpGmtDateTime>,
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    marked_for_refresh: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_optional_wp_gmt_date_time")]
    next_refresh_time: Option<WpGmtDateTime>,
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    is_following: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_optional_nonzero_id")]
    subscription_id: Option<ReaderSubscriptionId>,
    #[serde(default, deserialize_with = "deserialize_optional_count")]
    unseen_count: Option<u64>,
}

impl From<RawReaderFeed> for ReaderFeed {
    fn from(raw: RawReaderFeed) -> Self {
        Self {
            feed_id: raw.feed_id,
            site_id: raw.site_id,
            name: raw.name,
            description: raw.description,
            url: raw.url,
            feed_url: raw.feed_url,
            resolved_feed_url: raw.resolved_feed_url,
            image_url: raw.image.map(decode_html_entities),
            subscribers_count: raw.subscribers_count,
            organization_id: raw.organization_id,
            owner: raw.blog_owner,
            last_update: raw.last_update,
            last_checked: raw.last_checked,
            marked_for_refresh: raw.marked_for_refresh,
            next_refresh_time: raw.next_refresh_time,
            is_following: raw.is_following,
            subscription_id: raw.subscription_id,
            unseen_count: raw.unseen_count,
        }
    }
}

/// How a search's results are ordered.
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
pub enum ReaderFeedSearchSort {
    /// Best match first. The API's default. Page with [`ReaderFeedSearchParams::offset`].
    Relevance,
    /// Most recently updated first. Page with [`ReaderFeedSearchParams::page_handle`].
    LastUpdated,
}

impl_as_query_value_from_to_string!(ReaderFeedSearchSort);

/// Query parameters for searching sites and feeds, or finding the feeds at an address.
///
/// Send [`Self::q`] or [`Self::url`]: with neither, the request fails. With both, the API
/// searches for `q`.
///
/// A search pages two ways, depending on [`Self::sort`]:
///
/// - By relevance, with [`Self::offset`]: pass the page's
///   [`ReaderFeedSearchResponse::next_offset`]. The API ignores a `page_handle`.
/// - By last update, with [`Self::page_handle`]: pass the page's
///   [`ReaderFeedSearchResponse::next_page_handle`], and the same `sort`.
///
/// Either way, stop when the page has neither, or has no results. Only the first 200 results of a
/// search can be reached by offset.
#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(false)] // A search pages by offset or page handle, not by page number.
pub struct ReaderFeedSearchParams {
    /// The text to search sites and feeds for. Text that looks like an address, such as
    /// `example.com`, is looked up the way [`Self::url`] is.
    #[uniffi(default = None)]
    pub q: Option<String>,
    /// The address of a site or a feed, to find the feeds there, such as `example.com` or
    /// `https://example.com/feed/`. An address with no feed is a `no_such_feed` error.
    #[uniffi(default = None)]
    pub url: Option<String>,
    /// How many results to return: 10 by default and 20 at most, or the request fails. A lookup
    /// by address ignores it.
    #[uniffi(default = None)]
    pub number: Option<u32>,
    /// How many results to skip: 199 at most, or the request fails.
    #[uniffi(default = None)]
    pub offset: Option<u32>,
    /// Whether to leave out the sites the user follows. The API's default is to. It makes no
    /// difference signed out.
    #[uniffi(default = None)]
    pub exclude_followed: Option<bool>,
    /// The order of the results. The API's default is [`ReaderFeedSearchSort::Relevance`].
    #[uniffi(default = None)]
    pub sort: Option<ReaderFeedSearchSort>,
    /// The previous page's [`ReaderFeedSearchResponse::next_page_handle`], when sorting by
    /// [`ReaderFeedSearchSort::LastUpdated`].
    #[uniffi(default = None)]
    pub page_handle: Option<String>,
    /// The name of the algorithm to search with. The response names it back. A name the API
    /// doesn't know still returns results.
    #[uniffi(default = None)]
    pub algorithm: Option<String>,
    /// Extra data to include, comma-separated: `site` sends each result's
    /// [`ReaderFeedSearchResult::site`] and `feed` its [`ReaderFeedSearchResult::feed`].
    #[uniffi(default = None)]
    pub meta: Option<String>,
}

/// A page of a search for sites and feeds, or the feeds found at an address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
#[serde(from = "RawReaderFeedSearchResponse")]
pub struct ReaderFeedSearchResponse {
    /// The results. Empty for a search that finds nothing.
    pub feeds: Vec<ReaderFeedSearchResult>,
    /// How many results the search has. `None` for a lookup by address.
    #[uniffi(default = None)]
    pub total: Option<u64>,
    /// The algorithm that searched. `None` for a lookup by address.
    #[uniffi(default = None)]
    pub algorithm: Option<String>,
    /// Pass it as `offset` for the next page of a search sorted by relevance. `None` when there's
    /// no next page, and when the next page is asked for with [`Self::next_page_handle`].
    ///
    /// The API sends one after a page that used up the results, so the next page can be empty.
    #[uniffi(default = None)]
    pub next_offset: Option<u32>,
    /// Pass it as `page_handle` for the next page of a search sorted by last update. `None` when
    /// there's no next page, and when the next page is asked for with [`Self::next_offset`].
    #[uniffi(default = None)]
    pub next_page_handle: Option<String>,
}

#[derive(Deserialize)]
struct RawReaderFeedSearchResponse {
    #[serde(default, deserialize_with = "deserialize_list_or_none")]
    feeds: Vec<ReaderFeedSearchResult>,
    #[serde(default, deserialize_with = "deserialize_optional_count")]
    total: Option<u64>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    algorithm: Option<String>,
    /// A query string. Sorted by relevance it holds the next `offset`, as
    /// `offset=10&algorithm=...`. Sorted by last update it's a handle to send back whole, as
    /// `last=...&algorithm=...&blog=...&feed=...`.
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    next_page: Option<String>,
}

impl From<RawReaderFeedSearchResponse> for ReaderFeedSearchResponse {
    fn from(raw: RawReaderFeedSearchResponse) -> Self {
        let next_offset = raw.next_page.as_deref().and_then(|next_page| {
            url::form_urlencoded::parse(next_page.as_bytes())
                .find(|(name, _)| name == "offset")
                .and_then(|(_, offset)| offset.parse().ok())
        });
        Self {
            feeds: raw.feeds,
            total: raw.total,
            algorithm: raw.algorithm,
            next_offset,
            next_page_handle: raw.next_page.filter(|_| next_offset.is_none()),
        }
    }
}

/// A site or feed a search found.
///
/// A search by text and a lookup by address send different fields. A lookup sends only
/// [`Self::feed_id`] and [`Self::subscribe_url`]: ask for `meta=feed` to get the rest, including
/// the feed's site, from [`Self::feed`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
#[serde(from = "RawReaderFeedSearchResult")]
pub struct ReaderFeedSearchResult {
    /// `None` when the API leaves it out, which it does for some sites, more often in a search by
    /// last update. The site can still have a feed: `meta=site` sends it as the
    /// [`ReaderSite::feed_id`] of [`Self::site`].
    #[uniffi(default = None)]
    pub feed_id: Option<ReaderFeedId>,
    /// The WordPress.com or Jetpack site the result is, or the feed belongs to. `None` when it
    /// belongs to no site, and for a lookup by address, which leaves it out.
    #[uniffi(default = None)]
    pub site_id: Option<WpComSiteId>,
    #[uniffi(default = None)]
    pub title: Option<String>,
    #[uniffi(default = None)]
    pub url: Option<String>,
    /// The address to follow the result by.
    #[uniffi(default = None)]
    pub subscribe_url: Option<String>,
    #[uniffi(default = None)]
    pub subscribers_count: Option<u64>,
    /// The analytics context of where the result was shown, as raw JSON, to send back with
    /// events.
    #[uniffi(default = None)]
    pub railcar: Option<String>,
    /// The result's site, when asked for with `meta=site` and the result has one the user can
    /// read.
    #[uniffi(default = None)]
    pub site: Option<ReaderSite>,
    /// The result's feed, when asked for with `meta=feed` and the result has one.
    #[uniffi(default = None)]
    pub feed: Option<ReaderFeed>,
}

/// The API sends `feed_ID` as a number in a search and as a string in a lookup by address.
#[derive(Deserialize)]
struct RawReaderFeedSearchResult {
    #[serde(
        rename = "feed_ID",
        default,
        deserialize_with = "deserialize_optional_nonzero_id"
    )]
    feed_id: Option<ReaderFeedId>,
    #[serde(
        rename = "blog_ID",
        default,
        deserialize_with = "deserialize_optional_nonzero_id"
    )]
    site_id: Option<WpComSiteId>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    title: Option<String>,
    #[serde(
        rename = "URL",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    url: Option<String>,
    #[serde(
        rename = "subscribe_URL",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    subscribe_url: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_count")]
    subscribers_count: Option<u64>,
    #[serde(default)]
    railcar: Option<serde_json::Value>,
    #[serde(default, deserialize_with = "deserialize_object_or_none")]
    meta: Option<RawSearchResultMeta>,
}

#[derive(Deserialize)]
struct RawSearchResultMeta {
    #[serde(default, deserialize_with = "deserialize_object_or_none")]
    data: Option<RawSearchResultMetaData>,
}

/// `meta.data`, which `meta=site,feed` fills in. Where the API can't read a result's site it sends
/// an error in its place, which reads as no site.
#[derive(Deserialize)]
struct RawSearchResultMetaData {
    #[serde(default, deserialize_with = "deserialize_object_or_none")]
    site: Option<ReaderSite>,
    #[serde(default, deserialize_with = "deserialize_object_or_none")]
    feed: Option<ReaderFeed>,
}

impl From<RawReaderFeedSearchResult> for ReaderFeedSearchResult {
    fn from(raw: RawReaderFeedSearchResult) -> Self {
        let data = raw.meta.and_then(|meta| meta.data);
        let (site, feed) = data.map(|data| (data.site, data.feed)).unwrap_or_default();
        Self {
            feed_id: raw.feed_id,
            site_id: raw.site_id,
            title: raw.title,
            url: raw.url,
            subscribe_url: raw.subscribe_url,
            subscribers_count: raw.subscribers_count,
            railcar: raw
                .railcar
                .filter(|value| !value.is_null())
                .map(|value| value.to_string()),
            site,
            feed,
        }
    }
}

/// An ID the API can send as a number or a string. `0`, `"0"`, `""`, `false` and `null` all mean
/// there isn't one.
fn deserialize_optional_nonzero_id<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: From<u64>,
{
    let id = match Option::<serde_json::Value>::deserialize(deserializer)? {
        Some(serde_json::Value::Number(number)) => number.as_u64(),
        Some(serde_json::Value::String(text)) => text.parse().ok(),
        _ => None,
    };
    Ok(id.filter(|id| *id != 0).map(T::from))
}

/// A count the API can send as a number or a string. Anything that isn't a count, such as `null`
/// or a negative number, reads as `None`.
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

/// An object, or `None` for anything else: `null`, `false`, an empty array, or an object that
/// can't be read, such as the error the API sends in place of a site it can't read.
fn deserialize_object_or_none<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned,
{
    Ok(
        match Option::<serde_json::Value>::deserialize(deserializer)? {
            Some(value @ serde_json::Value::Object(_)) => serde_json::from_value(value).ok(),
            _ => None,
        },
    )
}

/// A list, or an empty one for `null`. An item that can't be read is left out, so one odd result
/// doesn't fail the page.
fn deserialize_list_or_none<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned,
{
    Ok(Option::<Vec<serde_json::Value>>::deserialize(deserializer)?
        .unwrap_or_default()
        .into_iter()
        .filter_map(|item| serde_json::from_value(item).ok())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wp_com::reader_subscriptions::{
        ReaderSubscriptionEmailDelivery, ReaderSubscriptionEmailFrequency,
        ReaderSubscriptionNotificationDelivery,
    };
    use rstest::rstest;
    use serde_json::json;

    /// A recording of a Jetpack site the user follows, with its post emails turned on, with the
    /// site's details replaced.
    #[test]
    fn test_deserialize_site_followed() {
        let json = include_str!("../../tests/wpcom/reader_sites/site_followed.json");
        let site: ReaderSite = serde_json::from_str(json).expect("the site parses");

        assert_eq!(site.site_id, WpComSiteId(2001));
        assert_eq!(site.feed_id, Some(ReaderFeedId(3001)));
        assert_eq!(site.name.as_deref(), Some("Example Site"));
        assert_eq!(site.url.as_deref(), Some("https://example.com"));
        assert_eq!(site.feed_url.as_deref(), Some("https://example.com/feed/"));
        assert_eq!(
            site.icon_url.as_deref(),
            Some("https://example.com/icon.png")
        );
        assert_eq!(site.lang.as_deref(), Some("en-US"));
        assert_eq!(site.post_count, Some(56));
        assert_eq!(site.subscribers_count, Some(1234));
        assert_eq!(site.is_jetpack, Some(true));
        assert_eq!(site.is_atomic, Some(false));
        assert_eq!(site.is_private, Some(false));
        assert_eq!(site.is_visible, Some(true));
        assert_eq!(site.organization_id, Some(0));
        assert_eq!(site.prefer_feed, None);
        assert_eq!(site.is_following, Some(true));
        assert_eq!(site.is_blocked, Some(false));
        assert_eq!(site.unseen_count, Some(7));
        let owner = site.owner.expect("the site has an owner");
        assert_eq!(owner.id, Some(WpComUserId(5001)));
        assert_eq!(owner.login.as_deref(), Some("example"));

        let methods = site
            .delivery_methods
            .expect("the site has delivery methods");
        assert_eq!(
            methods.email,
            Some(ReaderSubscriptionEmailDelivery {
                send_posts: Some(true),
                send_comments: Some(false),
                post_delivery_frequency: Some(ReaderSubscriptionEmailFrequency::Instantly),
                date_subscribed: Some("2026-01-02T03:04:05Z".parse().unwrap()),
            })
        );
        assert_eq!(
            methods.notification,
            Some(ReaderSubscriptionNotificationDelivery {
                send_posts: Some(false),
            })
        );
    }

    /// A recording of the same site asked for signed out: nothing about the user, `lang` is
    /// `false`, `visible` is `null` and there's no `post_count`.
    #[test]
    fn test_deserialize_site_signed_out() {
        let json = include_str!("../../tests/wpcom/reader_sites/site_signed_out.json");
        let site: ReaderSite = serde_json::from_str(json).expect("the site parses");

        assert_eq!(site.site_id, WpComSiteId(2001));
        assert_eq!(site.feed_id, Some(ReaderFeedId(3001)));
        assert_eq!(site.name.as_deref(), Some("Example Site"));
        assert_eq!(site.subscribers_count, Some(1234));
        assert_eq!(site.is_following, Some(false));
        assert_eq!(site.unseen_count, Some(0));
        assert_eq!(site.lang, None);
        assert_eq!(site.post_count, None);
        assert_eq!(site.is_visible, None);
        assert_eq!(
            site.delivery_methods,
            Some(ReaderSubscriptionDeliveryMethods {
                email: None,
                notification: Some(ReaderSubscriptionNotificationDelivery {
                    send_posts: Some(false),
                }),
            })
        );
    }

    /// A site with no feed arrives without `feed_ID` and `feed_URL`.
    #[rstest]
    #[case::left_out(json!({ "ID": 2001, "name": "No feed" }))]
    #[case::zero(json!({ "ID": 2001, "feed_ID": 0, "feed_URL": "" }))]
    #[case::false_(json!({ "ID": 2001, "feed_ID": false }))]
    fn test_a_site_without_a_feed(#[case] site: serde_json::Value) {
        let site: ReaderSite = serde_json::from_value(site).expect("the site parses");
        assert_eq!(site.feed_id, None);
        assert_eq!(site.feed_url, None);
    }

    /// What real sites send where another has a value: no `icon`, a `null` owner, an empty name.
    #[test]
    fn test_site_fields_that_vary_in_shape() {
        let site: ReaderSite = serde_json::from_value(json!({
            "ID": 2001,
            "name": "",
            "description": "",
            "feed_ID": "3001",
            "lang": false,
            "visible": null,
            "owner": null,
            "prefer_feed": true,
            "subscribers_count": "12",
            "subscription": { "delivery_methods": { "email": null, "notification": [] } },
        }))
        .expect("the site parses");

        assert_eq!(site.name, None);
        assert_eq!(site.description, None);
        assert_eq!(site.feed_id, Some(ReaderFeedId(3001)));
        assert_eq!(site.icon_url, None);
        assert_eq!(site.lang, None);
        assert_eq!(site.is_visible, None);
        assert_eq!(site.owner, None);
        assert_eq!(site.prefer_feed, Some(true));
        assert_eq!(site.subscribers_count, Some(12));
        assert_eq!(
            site.delivery_methods,
            Some(ReaderSubscriptionDeliveryMethods {
                email: None,
                notification: None,
            })
        );
    }

    /// Asked for `fields=ID,feed_ID`, the API sends only those.
    #[test]
    fn test_a_site_with_few_fields() {
        let site: ReaderSite =
            serde_json::from_value(json!({ "ID": 2001, "feed_ID": 3001 })).expect("it parses");
        assert_eq!(site.site_id, WpComSiteId(2001));
        assert_eq!(site.feed_id, Some(ReaderFeedId(3001)));
        assert_eq!(site.is_following, None);
        assert_eq!(site.delivery_methods, None);
    }

    #[test]
    fn test_a_site_without_an_id_is_an_error() {
        assert!(serde_json::from_value::<ReaderSite>(json!({ "name": "No ID" })).is_err());
    }

    #[test]
    fn test_site_icon_falls_back_to_the_favicon() {
        let site: ReaderSite = serde_json::from_value(json!({
            "ID": 2001,
            "icon": { "ico": "https://example.com/favicon.ico" },
        }))
        .expect("the site parses");
        assert_eq!(
            site.icon_url.as_deref(),
            Some("https://example.com/favicon.ico")
        );
    }

    #[rstest]
    #[case("instantly", ReaderSubscriptionEmailFrequency::Instantly)]
    #[case("daily", ReaderSubscriptionEmailFrequency::Daily)]
    #[case("weekly", ReaderSubscriptionEmailFrequency::Weekly)]
    #[case("monthly", ReaderSubscriptionEmailFrequency::Other("monthly".to_string()))]
    fn test_email_delivery_frequency(
        #[case] sent: &str,
        #[case] expected: ReaderSubscriptionEmailFrequency,
    ) {
        let email: ReaderSubscriptionEmailDelivery =
            serde_json::from_value(json!({ "post_delivery_frequency": sent })).unwrap();
        assert_eq!(email.post_delivery_frequency, Some(expected));
    }

    /// A recording of an RSS feed that belongs to no site, which the user doesn't follow, with
    /// its details replaced. `feed_ID` and `blog_ID` are strings, and `blog_ID` is `"0"`.
    #[test]
    fn test_deserialize_feed() {
        let json = include_str!("../../tests/wpcom/reader_sites/feed.json");
        let feed: ReaderFeed = serde_json::from_str(json).expect("the feed parses");

        assert_eq!(feed.feed_id, ReaderFeedId(3002));
        assert_eq!(feed.site_id, None);
        assert_eq!(feed.owner, None);
        assert_eq!(feed.name.as_deref(), Some("Example Feed"));
        assert_eq!(feed.url.as_deref(), Some("https://example.com/"));
        assert_eq!(feed.feed_url.as_deref(), Some("https://example.com/feed/"));
        assert_eq!(
            feed.resolved_feed_url.as_deref(),
            Some("https://example.com/feed/")
        );
        assert_eq!(feed.image_url, None);
        assert_eq!(feed.subscribers_count, Some(1234));
        assert_eq!(feed.is_following, Some(false));
        assert_eq!(feed.subscription_id, None);
        assert_eq!(feed.unseen_count, Some(7));
        assert_eq!(
            feed.last_update,
            Some("2026-11-18T00:00:00Z".parse().unwrap())
        );
        assert!(feed.last_checked.is_some());
        assert_eq!(feed.marked_for_refresh, Some(false));
        assert_eq!(feed.next_refresh_time, None);
    }

    /// A recording of a WordPress.com site's feed, which the user follows, with its details
    /// replaced. `subscription_id` is a string, and `image` and `description` are HTML-escaped.
    #[test]
    fn test_deserialize_feed_of_a_site() {
        let json = include_str!("../../tests/wpcom/reader_sites/feed_of_a_site.json");
        let feed: ReaderFeed = serde_json::from_str(json).expect("the feed parses");

        assert_eq!(feed.feed_id, ReaderFeedId(3001));
        assert_eq!(feed.site_id, Some(WpComSiteId(2001)));
        assert_eq!(
            feed.owner,
            Some(ReaderFeedOwner {
                id: Some(WpComUserId(5001)),
                name: Some("Example Name".to_string()),
            })
        );
        assert_eq!(feed.is_following, Some(true));
        assert_eq!(feed.subscription_id, Some(ReaderSubscriptionId(8001)));
        assert_eq!(
            feed.image_url.as_deref(),
            Some("https://example.com/icon.png?w=32&h=32"),
            "The address is unescaped"
        );
        assert_eq!(
            feed.description.as_deref(),
            Some("News &amp; examples"),
            "Text is left as the API sent it"
        );
        assert_eq!(feed.resolved_feed_url, None);
    }

    /// What real feeds send where another has a value.
    #[test]
    fn test_feed_fields_that_vary_in_shape() {
        let feed: ReaderFeed = serde_json::from_value(json!({
            "feed_ID": 3001,
            "blog_ID": 2001,
            "name": null,
            "URL": "",
            "last_update": null,
            "last_checked": null,
            "next_refresh_time": "2026-11-19T05:20:11+00:00",
            "blog_owner": null,
        }))
        .expect("the feed parses");

        assert_eq!(feed.feed_id, ReaderFeedId(3001));
        assert_eq!(feed.site_id, Some(WpComSiteId(2001)));
        assert_eq!(feed.name, None);
        assert_eq!(feed.url, None);
        assert_eq!(feed.last_update, None);
        assert_eq!(feed.last_checked, None);
        assert!(feed.next_refresh_time.is_some());
        assert_eq!(feed.owner, None);
    }

    /// The ways the API says a feed belongs to no site.
    #[rstest]
    #[case(json!("0"))]
    #[case(json!(0))]
    #[case(json!(false))]
    #[case(json!(null))]
    #[case(json!(""))]
    fn test_a_feed_that_belongs_to_no_site(#[case] blog_id: serde_json::Value) {
        let feed: ReaderFeed =
            serde_json::from_value(json!({ "feed_ID": "3001", "blog_ID": blog_id })).unwrap();
        assert_eq!(feed.site_id, None);
    }

    #[test]
    fn test_a_feed_without_an_id_is_an_error() {
        assert!(serde_json::from_value::<ReaderFeed>(json!({ "blog_ID": "2001" })).is_err());
    }

    /// A recording of a search by text, with the results' details replaced.
    #[test]
    fn test_deserialize_feed_search() {
        let json = include_str!("../../tests/wpcom/reader_sites/feed_search.json");
        let page: ReaderFeedSearchResponse = serde_json::from_str(json).expect("the page parses");

        assert_eq!(page.total, Some(12345));
        assert_eq!(page.algorithm.as_deref(), Some("reader/manage/search:0"));
        assert_eq!(page.next_offset, Some(2));
        assert_eq!(page.next_page_handle, None);
        assert_eq!(page.feeds.len(), 2);

        let result = &page.feeds[0];
        assert_eq!(result.feed_id, Some(ReaderFeedId(3001)));
        assert_eq!(result.site_id, Some(WpComSiteId(2001)));
        assert_eq!(result.title.as_deref(), Some("Example Site 1"));
        assert_eq!(
            result.url.as_deref(),
            Some("https://example1.wordpress.com")
        );
        assert_eq!(
            result.subscribe_url.as_deref(),
            Some("https://example1.wordpress.com")
        );
        assert_eq!(result.subscribers_count, Some(100));
        assert_eq!(result.site, None, "Not asked for with meta=site");
        assert_eq!(result.feed, None, "Not asked for with meta=feed");
        let railcar: serde_json::Value =
            serde_json::from_str(result.railcar.as_deref().unwrap()).unwrap();
        assert_eq!(railcar["fetch_algo"], "reader/manage/search:0");
    }

    /// A recording of a search sorted by last update, with `meta=site,feed`, with the results'
    /// details replaced: a result with no `feed_ID`, one whose site the API couldn't read, and
    /// one with its site and its feed.
    #[test]
    fn test_deserialize_feed_search_by_last_update() {
        let json = include_str!("../../tests/wpcom/reader_sites/feed_search_by_last_update.json");
        let page: ReaderFeedSearchResponse = serde_json::from_str(json).expect("the page parses");

        assert_eq!(page.next_offset, None);
        assert_eq!(
            page.next_page_handle.as_deref(),
            Some("last=1795060447&algorithm=reader/manage/search:0&blog=2003&feed=0")
        );
        assert_eq!(page.feeds.len(), 3);

        let without_feed_id = &page.feeds[0];
        assert_eq!(without_feed_id.feed_id, None);
        assert_eq!(without_feed_id.site_id, Some(WpComSiteId(2001)));
        assert_eq!(without_feed_id.subscribers_count, None);
        assert_eq!(without_feed_id.feed, None);
        let site = without_feed_id.site.as_ref().expect("its site was sent");
        assert_eq!(site.site_id, WpComSiteId(2001));
        assert_eq!(
            site.feed_id,
            Some(ReaderFeedId(3001)),
            "The site has a feed all the same"
        );

        let unreadable_site = &page.feeds[1];
        assert_eq!(unreadable_site.site_id, Some(WpComSiteId(2002)));
        assert_eq!(unreadable_site.site, None, "An error was sent in its place");

        let whole = &page.feeds[2];
        assert_eq!(whole.feed_id, Some(ReaderFeedId(3003)));
        let site = whole.site.as_ref().expect("its site was sent");
        let feed = whole.feed.as_ref().expect("its feed was sent");
        assert_eq!(site.feed_id, Some(feed.feed_id));
        assert_eq!(feed.site_id, Some(site.site_id));
    }

    /// A recording of the feeds found at an address, with their details replaced. The results
    /// have only a feed ID, sent as a string, and the address to follow.
    #[test]
    fn test_deserialize_feed_lookup_by_address() {
        let json = include_str!("../../tests/wpcom/reader_sites/feed_lookup_by_address.json");
        let page: ReaderFeedSearchResponse = serde_json::from_str(json).expect("the page parses");

        assert_eq!(page.total, None);
        assert_eq!(page.algorithm, None);
        assert_eq!(page.next_offset, None);
        assert_eq!(page.next_page_handle, None);
        let feed_ids: Vec<_> = page.feeds.iter().map(|result| result.feed_id).collect();
        assert_eq!(
            feed_ids,
            [Some(ReaderFeedId(3001)), Some(ReaderFeedId(3002))]
        );
        let first = &page.feeds[0];
        assert_eq!(
            first.subscribe_url.as_deref(),
            Some("https://example.com/atom.xml")
        );
        assert_eq!(first.site_id, None);
        assert_eq!(first.title, None);
        assert_eq!(first.railcar, None);
    }

    /// What the API sends for a search that finds nothing: no `next_page`.
    #[test]
    fn test_a_search_that_finds_nothing() {
        let page: ReaderFeedSearchResponse = serde_json::from_value(json!({
            "feeds": [],
            "total": 0,
            "algorithm": "reader/manage/search:0",
        }))
        .expect("the page parses");
        assert!(page.feeds.is_empty());
        assert_eq!(page.total, Some(0));
        assert_eq!(page.next_offset, None);
        assert_eq!(page.next_page_handle, None);
    }

    #[rstest]
    #[case::by_offset("offset=20&algorithm=reader/manage/search:0", Some(20), None)]
    #[case::by_handle(
        "last=1795062592&algorithm=reader/manage/search:0&blog=2001&feed=0",
        None,
        Some("last=1795062592&algorithm=reader/manage/search:0&blog=2001&feed=0")
    )]
    fn test_next_page(
        #[case] next_page: &str,
        #[case] next_offset: Option<u32>,
        #[case] next_page_handle: Option<&str>,
    ) {
        let page: ReaderFeedSearchResponse =
            serde_json::from_value(json!({ "feeds": [], "next_page": next_page })).unwrap();
        assert_eq!(page.next_offset, next_offset);
        assert_eq!(page.next_page_handle.as_deref(), next_page_handle);
    }

    /// One result that can't be read doesn't fail the page.
    #[test]
    fn test_a_search_keeps_its_results_when_one_cannot_be_read() {
        let page: ReaderFeedSearchResponse = serde_json::from_value(json!({
            "feeds": [{ "feed_ID": 3001 }, "not a result", { "feed_ID": "3002" }],
        }))
        .expect("the page parses");
        let feed_ids: Vec<_> = page.feeds.iter().map(|result| result.feed_id).collect();
        assert_eq!(
            feed_ids,
            [Some(ReaderFeedId(3001)), Some(ReaderFeedId(3002))]
        );
    }
}

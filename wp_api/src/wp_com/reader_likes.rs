//! The users who liked a post.
//!
//! The list answers without a token for a public site. For a private site it's then a 403
//! `unauthorized`.

use crate::{
    date::{WpGmtDateTime, deserialize_optional_wp_gmt_date_time},
    url_query::{AppendUrlQueryPairs, QueryPairs, QueryPairsExtension},
    wp_com::{WpComSiteId, me::WpComUserId, reader_saved::deserialize_optional_text},
};
use chrono::Duration;
use serde::{Deserialize, Deserializer, Serialize};
use wp_serde_helper::{
    LenientVec, deserialize_optional_bool_or_int, deserialize_u64_or_string_as_t,
    deserialize_u64_or_string_or_none,
};

/// Query parameters for the users who liked something. For the next page, use
/// [`ReaderLikesResponse::next_page_params`].
#[derive(Debug, Default, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ReaderLikesParams {
    /// The number of likes to return: 20 by default, and 90 at most, whatever is asked for.
    #[uniffi(default = None)]
    pub number: Option<u32>,
    /// Only return likes from before this time, to the second.
    #[uniffi(default = None)]
    pub before: Option<WpGmtDateTime>,
    /// Users whose likes to leave out. [`ReaderLikesResponse::found`] still counts them.
    #[uniffi(default = [])]
    pub exclude: Vec<WpComUserId>,
}

impl AppendUrlQueryPairs for ReaderLikesParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut
            .append_option_query_value_pair("number", self.number.as_ref())
            .append_option_query_value_pair("before", self.before.as_ref());
        // One `exclude[]` for each user: of a comma-separated list, the API only leaves out the
        // first.
        for user in &self.exclude {
            query_pairs_mut.append_query_value_pair("exclude[]", user);
        }
    }
}

/// A page of the users who liked something, newest like first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
#[serde(from = "RawReaderLikesResponse")]
pub struct ReaderLikesResponse {
    /// The page's likes, without any in [`Self::unreadable_likes`].
    pub likes: Vec<ReaderLike>,
    /// The likes on the page that couldn't be read. Almost always empty.
    #[uniffi(default = [])]
    pub unreadable_likes: Vec<ReaderUnreadableLike>,
    /// How many likes there are in all, on every page.
    #[uniffi(default = None)]
    pub found: Option<u64>,
    /// Whether the user is one of them. `false` signed out.
    #[uniffi(default = None)]
    pub i_like: Option<bool>,
}

impl ReaderLikesResponse {
    /// The parameters for the page after this one, of `number` likes. `None` when this is the
    /// last page: it has fewer likes than `number`, or its last like has no date to page from.
    ///
    /// The list pages by the date of the last like. `before` leaves out every like in the second
    /// it names, so the next page asks for likes before the second after the last one, and leaves
    /// out the users whose likes in that second are on this page. That way a like isn't skipped
    /// for sharing a second with the last like of a page.
    pub fn next_page_params(&self, number: u32) -> Option<ReaderLikesParams> {
        let count = self.likes.len() + self.unreadable_likes.len();
        if count == 0 || (count as u64) < u64::from(number.min(MAX_PAGE_SIZE)) {
            return None;
        }
        let last = self.likes.last()?.date_liked?;
        let exclude = self
            .likes
            .iter()
            .filter(|like| like.date_liked == Some(last))
            .map(|like| like.id)
            .collect();
        Some(ReaderLikesParams {
            number: Some(number),
            before: Some(WpGmtDateTime(last.0 + Duration::seconds(1))),
            exclude,
        })
    }
}

/// The most likes the API returns in a page.
const MAX_PAGE_SIZE: u32 = 90;

#[derive(Deserialize)]
struct RawReaderLikesResponse {
    #[serde(default)]
    likes: Option<LenientVec<ReaderLike>>,
    #[serde(default, deserialize_with = "deserialize_u64_or_string_or_none")]
    found: Option<u64>,
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    i_like: Option<bool>,
}

impl From<RawReaderLikesResponse> for ReaderLikesResponse {
    fn from(raw: RawReaderLikesResponse) -> Self {
        let likes = raw.likes.unwrap_or_default();
        let unreadable_likes = likes
            .invalid
            .into_iter()
            .map(|like| ReaderUnreadableLike {
                user_id: serde_json::from_str::<UserIdOnly>(like.json.get())
                    .ok()
                    .map(|like| like.id),
                error: like.error.to_string(),
            })
            .collect();
        Self {
            likes: likes.valid,
            unreadable_likes,
            found: raw.found,
            i_like: raw.i_like,
        }
    }
}

#[derive(Deserialize)]
struct UserIdOnly {
    #[serde(rename = "ID", deserialize_with = "deserialize_u64_or_string_as_t")]
    id: WpComUserId,
}

/// A like on a page that couldn't be read. It doesn't fail the page: the page keeps the likes it
/// can read, and lists the others as these.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, uniffi::Record)]
pub struct ReaderUnreadableLike {
    /// Whose like it is. `None` when the user's ID couldn't be read either.
    #[uniffi(default = None)]
    pub user_id: Option<WpComUserId>,
    /// Why it couldn't be read. A line and column in it count from the start of the like.
    pub error: String,
}

/// A like: the user who liked, and when.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
#[serde(from = "RawReaderLike")]
pub struct ReaderLike {
    /// The user's ID.
    pub id: WpComUserId,
    /// When the user liked it.
    #[uniffi(default = None)]
    pub date_liked: Option<WpGmtDateTime>,
    /// The user's username.
    #[uniffi(default = None)]
    pub login: Option<String>,
    /// The name to show for the user.
    #[uniffi(default = None)]
    pub name: Option<String>,
    #[uniffi(default = None)]
    pub first_name: Option<String>,
    #[uniffi(default = None)]
    pub last_name: Option<String>,
    #[uniffi(default = None)]
    pub nice_name: Option<String>,
    /// The user's website, as they gave it.
    #[uniffi(default = None)]
    pub url: Option<String>,
    #[uniffi(default = None)]
    pub avatar_url: Option<String>,
    /// Whether [`Self::avatar_url`] is the placeholder for a user who has set no picture.
    #[uniffi(default = None)]
    pub default_avatar: Option<bool>,
    /// The user's Gravatar profile.
    #[uniffi(default = None)]
    pub profile_url: Option<String>,
    /// What the user says about themselves.
    #[uniffi(default = None)]
    pub bio: Option<String>,
    /// The user's main site. Some users have none.
    #[uniffi(default = None)]
    pub site_id: Option<WpComSiteId>,
    /// Whether the user's main site can be shown. `None` for a user without one.
    #[uniffi(default = None)]
    pub site_visible: Option<bool>,
    /// The address of the user's main site. Only sent when [`Self::site_visible`] is true.
    #[uniffi(default = None)]
    pub primary_blog: Option<String>,
    /// The user's preferred site. Some users have none.
    #[uniffi(default = None)]
    pub preferred_blog: Option<ReaderLikePreferredBlog>,
}

/// A user's preferred site, as a like carries it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, uniffi::Record)]
pub struct ReaderLikePreferredBlog {
    /// Missing for about a fifth of the sites seen.
    #[uniffi(default = None)]
    pub id: Option<WpComSiteId>,
    #[uniffi(default = None)]
    pub name: Option<String>,
    #[uniffi(default = None)]
    pub url: Option<String>,
    /// The site's icon: `icon.img`, else `icon.ico`. Most have none.
    #[uniffi(default = None)]
    pub icon_url: Option<String>,
}

/// Every field a like can arrive with. `email` and `ip_address` are always `false`, and aren't
/// kept.
#[derive(Deserialize)]
struct RawReaderLike {
    #[serde(rename = "ID", deserialize_with = "deserialize_u64_or_string_as_t")]
    id: WpComUserId,
    /// Without an offset, in UTC.
    #[serde(default, deserialize_with = "deserialize_optional_wp_gmt_date_time")]
    date_liked: Option<WpGmtDateTime>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    login: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    first_name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    last_name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    nice_name: Option<String>,
    #[serde(
        rename = "URL",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    url: Option<String>,
    #[serde(
        rename = "avatar_URL",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    avatar_url: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    default_avatar: Option<bool>,
    #[serde(
        rename = "profile_URL",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    profile_url: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    bio: Option<String>,
    #[serde(
        rename = "site_ID",
        default,
        deserialize_with = "deserialize_optional_site_id"
    )]
    site_id: Option<WpComSiteId>,
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    site_visible: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    primary_blog: Option<String>,
    #[serde(default, deserialize_with = "deserialize_object_or_none")]
    preferred_blog: Option<RawPreferredBlog>,
}

#[derive(Deserialize)]
struct RawPreferredBlog {
    #[serde(default, deserialize_with = "deserialize_optional_site_id")]
    id: Option<WpComSiteId>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    url: Option<String>,
    #[serde(default, deserialize_with = "deserialize_object_or_none")]
    icon: Option<RawIcon>,
}

#[derive(Deserialize)]
struct RawIcon {
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    img: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    ico: Option<String>,
}

impl From<RawReaderLike> for ReaderLike {
    fn from(raw: RawReaderLike) -> Self {
        Self {
            id: raw.id,
            date_liked: raw.date_liked,
            login: raw.login,
            name: raw.name,
            first_name: raw.first_name,
            last_name: raw.last_name,
            nice_name: raw.nice_name,
            url: raw.url,
            avatar_url: raw.avatar_url,
            default_avatar: raw.default_avatar,
            profile_url: raw.profile_url,
            bio: raw.bio,
            site_id: raw.site_id,
            site_visible: raw.site_visible,
            primary_blog: raw.primary_blog,
            preferred_blog: raw.preferred_blog.map(|blog| ReaderLikePreferredBlog {
                id: blog.id,
                name: blog.name,
                url: blog.url,
                icon_url: blog.icon.and_then(|icon| icon.img.or(icon.ico)),
            }),
        }
    }
}

/// A site's ID, sent as a number or a string. Anything that isn't one, such as `0`, `-1`, `false`
/// or `null`, reads as `None` rather than failing the like.
fn deserialize_optional_site_id<'de, D>(deserializer: D) -> Result<Option<WpComSiteId>, D::Error>
where
    D: Deserializer<'de>,
{
    let id = match Option::<serde_json::Value>::deserialize(deserializer)? {
        Some(serde_json::Value::Number(number)) => number.as_u64(),
        Some(serde_json::Value::String(text)) => text.parse().ok(),
        _ => None,
    };
    Ok(id.filter(|id| *id != 0).map(WpComSiteId))
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

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use serde_json::json;

    fn page(json: &str) -> ReaderLikesResponse {
        serde_json::from_str(json).expect("the page parses")
    }

    fn like(id: u64, date_liked: &str) -> serde_json::Value {
        json!({ "ID": id, "date_liked": date_liked })
    }

    /// A page recorded from WordPress.com, with the users replaced. It keeps the types the API
    /// sent, and the three kinds of user seen: with a site, with a site that isn't shown, and
    /// with none.
    #[test]
    fn test_deserialize_post_likes() {
        let page = page(include_str!(
            "../../tests/wpcom/reader_likes/post_likes.json"
        ));

        assert_eq!(page.found, Some(1768));
        assert_eq!(page.i_like, Some(false));
        assert!(page.unreadable_likes.is_empty());
        let [with_site, hidden_site, no_site] = &page.likes[..] else {
            panic!("Expected three likes");
        };

        assert_eq!(with_site.id, WpComUserId(1001));
        // Sent as `2015-10-02 23:40:29`, without an offset, and read as UTC.
        assert_eq!(
            with_site.date_liked,
            Some("2015-10-02T23:40:29+00:00".parse().unwrap())
        );
        assert_eq!(with_site.login.as_deref(), Some("exampleuser1"));
        assert_eq!(with_site.name.as_deref(), Some("Example User One"));
        assert_eq!(with_site.default_avatar, Some(false));
        assert_eq!(with_site.site_id, Some(WpComSiteId(2001)));
        assert_eq!(with_site.site_visible, Some(true));
        assert_eq!(
            with_site.primary_blog.as_deref(),
            Some("http://exampleuser1.wordpress.com")
        );
        assert_eq!(
            with_site.preferred_blog,
            Some(ReaderLikePreferredBlog {
                id: Some(WpComSiteId(2001)),
                name: Some("Example Site One".to_string()),
                url: Some("http://exampleuser1.wordpress.com".to_string()),
                icon_url: Some("https://exampleuser1.wordpress.com/icon.png".to_string()),
            })
        );

        assert_eq!(hidden_site.site_id, Some(WpComSiteId(2002)));
        assert_eq!(hidden_site.site_visible, Some(false));
        assert_eq!(hidden_site.primary_blog, None);
        assert_eq!(hidden_site.first_name, None, "Sent as an empty string");
        let preferred = hidden_site.preferred_blog.as_ref().expect("a blog");
        assert_eq!((preferred.id, &preferred.name), (None, &None));
        assert_eq!(preferred.icon_url, None);

        assert_eq!(no_site.site_id, None);
        assert_eq!(no_site.site_visible, None);
        assert_eq!(no_site.preferred_blog, None);
        assert_eq!(no_site.default_avatar, Some(true));
    }

    /// What the API sends for something nobody has liked, and past the last page.
    #[test]
    fn test_deserialize_no_likes() {
        let page = page(r#"{ "found": 0, "i_like": false, "likes": [], "meta": { "links": {} } }"#);
        assert!(page.likes.is_empty());
        assert_eq!(page.found, Some(0));
        assert_eq!(page.next_page_params(20), None);
    }

    #[test]
    fn test_next_page_params() {
        let page: ReaderLikesResponse = serde_json::from_value(json!({
            "found": 5,
            "likes": [
                like(1, "2026-11-01 10:00:02"),
                like(2, "2026-11-01 10:00:01"),
                like(3, "2026-11-01 10:00:00"),
            ],
        }))
        .unwrap();

        assert_eq!(
            page.next_page_params(3),
            Some(ReaderLikesParams {
                number: Some(3),
                before: Some("2026-11-01T10:00:01+00:00".parse().unwrap()),
                exclude: vec![WpComUserId(3)],
            })
        );
    }

    /// Two likes in the last like's second: the next page asks for that second again, without
    /// the ones this page has, so a third like in it isn't skipped.
    #[test]
    fn test_next_page_params_when_likes_share_the_last_second() {
        let page: ReaderLikesResponse = serde_json::from_value(json!({
            "likes": [
                like(1, "2026-11-01 10:00:05"),
                like(2, "2026-11-01 10:00:00"),
                like(3, "2026-11-01 10:00:00"),
            ],
        }))
        .unwrap();

        let next = page.next_page_params(3).expect("a next page");
        assert_eq!(
            next.before,
            Some("2026-11-01T10:00:01+00:00".parse().unwrap())
        );
        assert_eq!(next.exclude, [WpComUserId(2), WpComUserId(3)]);
    }

    #[rstest]
    // A page with fewer likes than were asked for is the last.
    #[case::short_page(2, 3, false)]
    #[case::full_page(3, 3, true)]
    // The API returns 90 at most, so 90 of 200 asked for is a full page.
    #[case::full_page_of_the_most(90, 200, true)]
    #[case::short_page_of_the_most(89, 200, false)]
    #[case::empty_page(0, 20, false)]
    fn test_next_page_params_at_the_end(
        #[case] likes: u64,
        #[case] number: u32,
        #[case] has_next: bool,
    ) {
        let likes: Vec<_> = (0..likes)
            .map(|id| like(id + 1, "2026-11-01 10:00:00"))
            .collect();
        let page: ReaderLikesResponse = serde_json::from_value(json!({ "likes": likes })).unwrap();
        assert_eq!(page.next_page_params(number).is_some(), has_next);
    }

    #[test]
    fn test_no_next_page_when_the_last_like_has_no_date() {
        let page: ReaderLikesResponse =
            serde_json::from_value(json!({ "likes": [{ "ID": 1 }] })).unwrap();
        assert_eq!(page.next_page_params(1), None);
    }

    #[test]
    fn test_fields_that_vary_in_shape() {
        let like: ReaderLike = serde_json::from_value(json!({
            "ID": "7",
            "email": false,
            "ip_address": false,
            "URL": "",
            "bio": "",
            "site_ID": -1,
            "site_visible": 1,
            "date_liked": "",
            "preferred_blog": { "id": null, "name": "", "url": "https://example.com", "icon": null },
        }))
        .expect("the like parses");

        assert_eq!(like.id, WpComUserId(7));
        assert_eq!(like.url, None);
        assert_eq!(like.bio, None);
        assert_eq!(like.site_id, None);
        assert_eq!(like.site_visible, Some(true));
        assert_eq!(like.date_liked, None);
        let preferred = like.preferred_blog.expect("a blog");
        assert_eq!(preferred.url.as_deref(), Some("https://example.com"));
        assert_eq!(preferred.icon_url, None);
    }

    #[rstest]
    #[case::img_first(json!({ "img": "https://a/img.png", "ico": "https://a/favicon.ico" }), Some("https://a/img.png"))]
    #[case::ico(json!({ "ico": "https://a/favicon.ico" }), Some("https://a/favicon.ico"))]
    #[case::empty_list(json!([]), None)]
    #[case::false_(json!(false), None)]
    fn test_preferred_blog_icon(#[case] icon: serde_json::Value, #[case] expected: Option<&str>) {
        let like: ReaderLike =
            serde_json::from_value(json!({ "ID": 1, "preferred_blog": { "icon": icon } }))
                .expect("the like parses");
        assert_eq!(
            like.preferred_blog
                .and_then(|blog| blog.icon_url)
                .as_deref(),
            expected
        );
    }

    #[test]
    fn test_a_page_keeps_its_likes_when_one_cannot_be_read() {
        let page = page(
            r#"{
                "found": 4,
                "likes": [
                    { "ID": 1, "date_liked": "2026-11-01 10:00:03" },
                    { "ID": 2, "date_liked": "2026-11-16T16:45:34+91:00" },
                    { "login": "no-id" },
                    { "ID": 4, "date_liked": "2026-11-01 10:00:00" }
                ]
            }"#,
        );

        let ids: Vec<_> = page.likes.iter().map(|like| like.id).collect();
        assert_eq!(ids, [WpComUserId(1), WpComUserId(4)]);
        let unreadable: Vec<_> = page.unreadable_likes.iter().map(|l| l.user_id).collect();
        assert_eq!(unreadable, [Some(WpComUserId(2)), None]);
        // The page was full, so there's another, paged from the last like that was read.
        assert!(page.next_page_params(4).is_some());
    }

    #[rstest]
    #[case(json!({ "likes": null }))]
    #[case(json!({}))]
    fn test_a_page_without_likes(#[case] page: serde_json::Value) {
        let page: ReaderLikesResponse = serde_json::from_value(page).expect("the page parses");
        assert!(page.likes.is_empty());
        assert!(page.unreadable_likes.is_empty());
    }

    #[test]
    fn test_a_page_that_is_not_one_is_an_error() {
        assert!(serde_json::from_value::<ReaderLikesResponse>(json!({ "likes": "none" })).is_err());
    }
}

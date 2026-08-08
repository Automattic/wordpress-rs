use crate::{
    date::WpGmtDateTime, posts::PostId, users::UserId, wp_com::stats_visits::StatsVisitsDataValue,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt};
use wp_serde_helper::{
    deserialize_empty_array_or_hashmap, deserialize_false_as_none, deserialize_i64_or_string_as_t,
};

// The column names the API uses for the daily view history.
const PERIOD_COLUMN: &str = "period";
const VIEWS_COLUMN: &str = "views";

/// What a per-post stats request is about.
///
/// The API addresses the site's home page as post `0`, which is not a valid
/// [`PostId`] anywhere else in the crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum StatsPostTarget {
    /// A specific post or page.
    Post { id: PostId },
    /// The site's home page. See [`StatsPostResponse`] for what the API counts
    /// for it, which depends on how the site's front page is configured.
    HomePage,
}

impl fmt::Display for StatsPostTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Post { id } => write!(f, "{id}"),
            Self::HomePage => write!(f, "0"),
        }
    }
}

/// Response from the per-post stats endpoint.
///
/// The endpoint returns the target's complete view history, so
/// [`Self::daily_views`] can hold thousands of entries for a long-lived post.
/// Callers that only need a trailing window (such as the "Latest Post Summary"
/// card) should slice the tail of it — `dailyViews.suffix(7)` in Swift,
/// `dailyViews.takeLast(7)` in Kotlin.
///
/// # The site's home page
///
/// [`StatsPostTarget::HomePage`] requests post `0`, which `/stats/top-posts`
/// also reports as a pseudo-entry. What the view figures cover then depends on
/// how the site's front page is configured, and the two cases are
/// indistinguishable in the response:
///
/// - a "latest posts" front page — the views recorded against the home page
/// - a static front page — the whole site's view history
///
/// The home page is not a post either way, so [`Self::post`],
/// [`Self::discussion`] and [`Self::like_count`] are `None` for both.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
#[serde(from = "RawStatsPostResponse", into = "RawStatsPostResponse")]
pub struct StatsPostResponse {
    /// The date the stats were generated for (format: YYYY-MM-DD).
    pub date: String,
    /// The target's all-time view count.
    pub views: u64,
    /// Yearly view totals, keyed by year (e.g. `"2026"`).
    ///
    /// A target with no recorded views gets an entry for every year from 1970
    /// to the present, each with an empty `months` map, rather than a map
    /// covering only the years the target existed for.
    pub years: HashMap<String, StatsPostYear>,
    /// Yearly view averages, keyed by year (e.g. `"2026"`). Populated on the
    /// same basis as [`Self::years`].
    pub averages: HashMap<String, StatsPostAverage>,
    /// The most recent weeks of daily views, oldest first.
    pub weeks: Vec<StatsPostWeek>,
    /// The target's complete daily view history, oldest first.
    ///
    /// The API sends this as a `fields`/`data` column table; it is flattened
    /// while deserializing so callers never handle the column indirection.
    ///
    /// The history is padded rather than sparse: a target with no views still
    /// gets one zero-count entry per day since it was published.
    ///
    /// Empty if the response doesn't name both the `period` and `views`
    /// columns.
    pub daily_views: Vec<StatsPostDailyView>,
    /// The highest view count the target reached in a single month.
    pub highest_month: u64,
    /// The highest monthly average of daily views the target reached.
    pub highest_day_average: u64,
    /// The highest single-day view count across the last few weeks. Despite the
    /// name, this is neither a weekly figure nor an average.
    pub highest_week_average: u64,
    /// The post's like count. `None` for the site's home page.
    pub like_count: Option<u64>,
    /// The post's comment counts. `None` for the site's home page.
    pub discussion: Option<StatsPostDiscussion>,
    /// The post the stats belong to. `None` for the site's home page.
    pub post: Option<StatsPostDetails>,
}

/// The response as the API sends it, before the `fields`/`data` column table is
/// flattened into [`StatsPostResponse::daily_views`].
#[derive(Serialize, Deserialize)]
struct RawStatsPostResponse {
    date: String,
    views: u64,
    #[serde(deserialize_with = "deserialize_empty_array_or_hashmap")]
    years: HashMap<String, StatsPostYear>,
    #[serde(deserialize_with = "deserialize_empty_array_or_hashmap")]
    averages: HashMap<String, StatsPostAverage>,
    weeks: Vec<StatsPostWeek>,
    /// Column names for the `data` rows. Always `["period", "views"]` in every
    /// response observed, but read rather than assumed.
    fields: Vec<String>,
    data: Vec<Vec<StatsVisitsDataValue>>,
    highest_month: u64,
    highest_day_average: u64,
    highest_week_average: u64,
    // The home page has no post behind it, so the API sends these as `null` —
    // and `post` as boolean `false` rather than `null`.
    like_count: Option<u64>,
    discussion: Option<StatsPostDiscussion>,
    #[serde(deserialize_with = "deserialize_false_as_none")]
    post: Option<StatsPostDetails>,
}

impl From<RawStatsPostResponse> for StatsPostResponse {
    fn from(raw: RawStatsPostResponse) -> Self {
        Self {
            date: raw.date,
            views: raw.views,
            years: raw.years,
            averages: raw.averages,
            weeks: raw.weeks,
            daily_views: daily_views(&raw.fields, raw.data),
            highest_month: raw.highest_month,
            highest_day_average: raw.highest_day_average,
            highest_week_average: raw.highest_week_average,
            like_count: raw.like_count,
            discussion: raw.discussion,
            post: raw.post,
        }
    }
}

impl From<StatsPostResponse> for RawStatsPostResponse {
    fn from(response: StatsPostResponse) -> Self {
        Self {
            date: response.date,
            views: response.views,
            years: response.years,
            averages: response.averages,
            weeks: response.weeks,
            fields: vec![PERIOD_COLUMN.to_string(), VIEWS_COLUMN.to_string()],
            data: response
                .daily_views
                .into_iter()
                .map(|daily_view| {
                    vec![
                        StatsVisitsDataValue::String(daily_view.period),
                        StatsVisitsDataValue::Number(daily_view.views),
                    ]
                })
                .collect(),
            highest_month: response.highest_month,
            highest_day_average: response.highest_day_average,
            highest_week_average: response.highest_week_average,
            like_count: response.like_count,
            discussion: response.discussion,
            post: response.post,
        }
    }
}

/// Flattens the `fields`/`data` column table into data points, skipping rows the
/// columns can't be read from.
///
/// Takes `data` by value so each row's period string is moved rather than
/// copied — the history runs to thousands of rows on a long-lived post.
fn daily_views(fields: &[String], data: Vec<Vec<StatsVisitsDataValue>>) -> Vec<StatsPostDailyView> {
    let (Some(period_index), Some(views_index)) = (
        fields.iter().position(|field| field == PERIOD_COLUMN),
        fields.iter().position(|field| field == VIEWS_COLUMN),
    ) else {
        return vec![];
    };

    let mut daily_views = Vec::with_capacity(data.len());
    for mut row in data {
        let Some(views) = row
            .get(views_index)
            .and_then(StatsVisitsDataValue::as_number)
        else {
            continue;
        };
        if period_index >= row.len() {
            continue;
        }
        let StatsVisitsDataValue::String(period) = row.swap_remove(period_index) else {
            continue;
        };
        daily_views.push(StatsPostDailyView { period, views });
    }
    daily_views
}

/// A single day's view count from the daily view history.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct StatsPostDailyView {
    /// The day the views were recorded on (format: YYYY-MM-DD).
    pub period: String,
    /// The number of views on that day.
    pub views: u64,
}

/// A year's view totals.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsPostYear {
    /// View totals keyed by month number (`"1"` through `"12"`).
    #[serde(deserialize_with = "deserialize_empty_array_or_hashmap")]
    pub months: HashMap<String, u64>,
    /// The total views for the year.
    pub total: u64,
}

/// A year's view averages.
///
/// The API truncates every average to a whole number before sending it.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsPostAverage {
    /// View averages keyed by month number (`"1"` through `"12"`).
    #[serde(deserialize_with = "deserialize_empty_array_or_hashmap")]
    pub months: HashMap<String, u64>,
    /// The average views across the whole year.
    pub overall: u64,
}

/// A week of daily views.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsPostWeek {
    /// The days in the week, oldest first. The final week may be partial.
    pub days: Vec<StatsPostDay>,
    /// The total views for the week.
    pub total: u64,
    /// The average daily views for the week, truncated to a whole number by the
    /// API.
    pub average: u64,
    /// The change from the previous week, or `None` for the first week.
    pub change: Option<StatsPostChange>,
}

/// The change in views from one week to the next.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, uniffi::Enum)]
#[serde(from = "RawStatsPostChange", into = "RawStatsPostChange")]
pub enum StatsPostChange {
    /// The percentage change from the previous week. Negative when views fell.
    Percentage { value: f64 },
    /// The previous week had no views, so the change is unbounded. The API
    /// sends this as `{"isInfinity": true}` because the underlying value is
    /// infinite and cannot be represented in JSON.
    Infinite,
}

/// The wire representations the API uses for a week's `change`.
///
/// These three shapes — a number, `{"isInfinity": true}`, and `null` (handled by
/// the surrounding `Option`) — are the only ones the endpoint produces. A week
/// following a zero-view week reports an integer `0` rather than a not-a-number
/// marker, so there is no `isNan` counterpart to model.
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum RawStatsPostChange {
    Percentage(f64),
    Infinite {
        // Only ever `true` on the wire; the presence of the key is what
        // identifies the shape.
        #[serde(rename = "isInfinity")]
        is_infinity: bool,
    },
}

impl From<RawStatsPostChange> for StatsPostChange {
    fn from(raw: RawStatsPostChange) -> Self {
        match raw {
            RawStatsPostChange::Percentage(value) => Self::Percentage { value },
            RawStatsPostChange::Infinite { .. } => Self::Infinite,
        }
    }
}

impl From<StatsPostChange> for RawStatsPostChange {
    fn from(change: StatsPostChange) -> Self {
        match change {
            StatsPostChange::Percentage { value } => Self::Percentage(value),
            StatsPostChange::Infinite => Self::Infinite { is_infinity: true },
        }
    }
}

/// A single day within a [`StatsPostWeek`].
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsPostDay {
    /// The day (format: YYYY-MM-DD).
    pub day: String,
    /// The number of views on that day.
    pub count: u64,
}

/// A post's comment counts.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsPostDiscussion {
    /// The number of comments on the post.
    pub comment_count: u64,
}

/// The post the stats belong to.
///
/// This mirrors WordPress' raw post row, so it carries the post's editorial
/// metadata. Fields the API sends that aren't modelled here (post content,
/// ping status, and similar) are ignored.
///
/// The row's `comment_count` is deliberately omitted: the API sends it as a
/// string here, and [`StatsPostDiscussion::comment_count`] carries the same
/// value as a number.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsPostDetails {
    /// The post's ID.
    #[serde(rename = "ID")]
    pub id: PostId,
    /// The post's title.
    ///
    /// When the stats service can't resolve the stored title, the API
    /// substitutes a generated placeholder of the form `#<id> (not found)`.
    #[serde(rename = "post_title")]
    pub title: String,
    /// The post's excerpt. Empty when the post has none.
    #[serde(rename = "post_excerpt")]
    pub excerpt: String,
    /// The post's publication date in the site's timezone (format: YYYY-MM-DD HH:MM:SS).
    #[serde(rename = "post_date")]
    pub date: String,
    /// The post's publication date in GMT.
    #[serde(rename = "post_date_gmt")]
    pub date_gmt: WpGmtDateTime,
    /// The date the post was last modified, in the site's timezone (format: YYYY-MM-DD HH:MM:SS).
    #[serde(rename = "post_modified")]
    pub modified: String,
    /// The date the post was last modified, in GMT.
    #[serde(rename = "post_modified_gmt")]
    pub modified_gmt: WpGmtDateTime,
    /// The post's slug.
    #[serde(rename = "post_name")]
    pub slug: String,
    /// The post's status, e.g. `"publish"` or `"draft"`.
    #[serde(rename = "post_status")]
    pub status: String,
    /// The post's type, e.g. `"post"` or `"page"`.
    pub post_type: String,
    /// The ID of the post's author.
    #[serde(
        rename = "post_author",
        deserialize_with = "deserialize_i64_or_string_as_t"
    )]
    pub author_id: UserId,
    /// The post's public URL.
    pub permalink: String,
    /// The post's globally unique identifier. Use [`Self::permalink`] for a
    /// URL that resolves.
    pub guid: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    const WITH_VIEWS: &str = "tests/wpcom/stats_post/post-with-views.json";
    const NO_VIEWS: &str = "tests/wpcom/stats_post/post-no-views.json";
    const HOMEPAGE: &str = "tests/wpcom/stats_post/homepage.json";

    fn parse(json_file_path: &str) -> StatsPostResponse {
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        serde_json::from_reader(file).expect("Unable to parse JSON")
    }

    /// Flattens a `fields`/`data` column table given as JSON, for exercising the
    /// column handling without a full response envelope.
    fn flatten(fields: &[&str], data: &str) -> Vec<StatsPostDailyView> {
        let fields: Vec<String> = fields.iter().map(|f| f.to_string()).collect();
        let data = serde_json::from_str(data).expect("Unable to parse JSON");
        daily_views(&fields, data)
    }

    #[test]
    fn test_stats_post_response_details() {
        let response = parse(WITH_VIEWS);

        assert_eq!(response.date, "2026-08-06");
        assert_eq!(response.views, 19096);
        assert_eq!(response.highest_month, 3224);
        assert_eq!(response.highest_day_average, 293);
        assert_eq!(response.highest_week_average, 3);
        assert_eq!(response.like_count, Some(9));
        assert_eq!(response.discussion.expect("present").comment_count, 48);

        let year = response.years.get("2013").expect("2013 should exist");
        assert_eq!(year.total, 6146);
        assert_eq!(year.months.get("6"), Some(&3224));

        let average = response.averages.get("2013").expect("2013 should exist");
        assert_eq!(average.overall, 31);
        assert_eq!(average.months.get("6"), Some(&293));
    }

    #[test]
    fn test_stats_post_details() {
        let post = parse(WITH_VIEWS).post.expect("a real post has details");

        assert_eq!(post.id, PostId(2729));
        assert_eq!(
            post.title,
            "The Last Version of FeedDemon is Here, and it's Free"
        );
        assert_eq!(post.excerpt, "The wait is over.");
        assert_eq!(post.date, "2013-06-20 09:15:49");
        assert_eq!(post.date_gmt.0.to_rfc3339(), "2013-06-20T13:15:49+00:00");
        assert_eq!(post.modified, "2013-06-23 21:57:23");
        assert_eq!(
            post.modified_gmt.0.to_rfc3339(),
            "2013-06-24T01:57:23+00:00"
        );
        assert_eq!(
            post.slug,
            "the-last-version-of-feeddemon-is-here-and-its-free"
        );
        assert_eq!(post.status, "publish");
        assert_eq!(post.post_type, "post");
        assert_eq!(post.guid, "https://example.com/?p=2729");
        assert_eq!(
            post.permalink,
            "https://example.com/2013/06/20/the-last-version-of-feeddemon-is-here-and-its-free/"
        );
        // The API sends `post_author` as a string.
        assert_eq!(post.author_id, UserId(5399133));
    }

    #[test]
    fn test_stats_post_weeks() {
        let weeks = parse(WITH_VIEWS).weeks;

        assert_eq!(weeks.len(), 3);

        let first = &weeks[0];
        assert_eq!(first.days.len(), 7);
        assert_eq!(first.days[0].day, "2026-06-29");
        assert_eq!(first.days[0].count, 2);
        assert_eq!(first.total, 7);
        assert_eq!(first.average, 1);
        assert!(first.change.is_none(), "the first week has no prior week");

        let second = &weeks[1];
        assert_eq!(second.days.len(), 4, "a week may be partial");
        assert_eq!(second.total, 6);
        assert_eq!(
            second.change,
            Some(StatsPostChange::Percentage {
                value: 133.33333333333334
            })
        );

        // The API sends `{"isInfinity": true}` when the previous week had no views.
        let last = &weeks[2];
        assert_eq!(last.change, Some(StatsPostChange::Infinite));
    }

    #[test]
    fn test_stats_post_change_round_trips() {
        let weeks = parse(WITH_VIEWS).weeks;

        assert_eq!(serde_json::to_string(&weeks[0].change).unwrap(), "null");
        assert_eq!(
            serde_json::to_string(&weeks[1].change).unwrap(),
            "133.33333333333334"
        );
        assert_eq!(
            serde_json::to_string(&weeks[2].change).unwrap(),
            r#"{"isInfinity":true}"#
        );
    }

    #[rstest]
    #[case::with_views(WITH_VIEWS)]
    #[case::no_views(NO_VIEWS)]
    #[case::homepage(HOMEPAGE)]
    fn test_stats_post_response_round_trips(#[case] json_file_path: &str) {
        let serialized =
            serde_json::to_value(parse(json_file_path)).expect("Unable to serialize response");
        let reparsed: StatsPostResponse =
            serde_json::from_value(serialized.clone()).expect("Unable to parse JSON");

        assert_eq!(
            serde_json::to_value(reparsed).expect("Unable to serialize response"),
            serialized
        );
    }

    #[test]
    fn test_stats_post_daily_views() {
        let daily_views = parse(WITH_VIEWS).daily_views;

        assert_eq!(daily_views.len(), 5);
        assert_eq!(
            daily_views[0],
            StatsPostDailyView {
                period: "2013-06-20".to_string(),
                views: 1194,
            }
        );
        assert_eq!(
            daily_views[4],
            StatsPostDailyView {
                period: "2026-08-06".to_string(),
                views: 0,
            }
        );
    }

    #[rstest]
    // The column positions are read rather than assumed, so swapping them still
    // yields the same data points.
    #[case::reordered_columns(
        &["views", "period"],
        r#"[[5, "2026-08-04"], [7, "2026-08-06"]]"#,
        vec![("2026-08-04", 5), ("2026-08-06", 7)]
    )]
    // An unreadable row is dropped rather than derailing the rest.
    #[case::unreadable_row(
        &["period", "views"],
        r#"[["2026-08-04", 5], [null, null], ["2026-08-06", 7]]"#,
        vec![("2026-08-04", 5), ("2026-08-06", 7)]
    )]
    #[case::missing_views_column(&["period"], r#"[["2026-08-04"]]"#, vec![])]
    fn test_stats_post_daily_views_column_handling(
        #[case] fields: &[&str],
        #[case] data: &str,
        #[case] expected: Vec<(&str, u64)>,
    ) {
        let expected: Vec<StatsPostDailyView> = expected
            .into_iter()
            .map(|(period, views)| StatsPostDailyView {
                period: period.to_string(),
                views,
            })
            .collect();

        assert_eq!(flatten(fields, data), expected);
    }

    #[test]
    fn test_stats_post_homepage() {
        // Post 0 is the site's home page. It isn't a post, so the API sends
        // `like_count` and `discussion` as null and `post` as boolean `false`,
        // while every view field is populated as usual.
        let response = parse(HOMEPAGE);

        assert!(response.post.is_none());
        assert!(response.discussion.is_none());
        assert!(response.like_count.is_none());

        assert_eq!(response.views, 74286);
        assert_eq!(response.highest_month, 3822);
        assert_eq!(response.daily_views.len(), 3);
        assert_eq!(response.daily_views[0].period, "2013-05-19");
        assert_eq!(response.daily_views[0].views, 73);
        assert_eq!(
            response.years.get("2013").expect("2013 should exist").total,
            14276
        );
        assert!(!response.weeks.is_empty());
    }

    #[test]
    fn test_stats_post_without_views() {
        let response = parse(NO_VIEWS);

        assert_eq!(response.views, 0);
        assert_eq!(response.highest_month, 0);
        assert_eq!(response.like_count, Some(0));
        assert_eq!(response.discussion.expect("present").comment_count, 0);

        // With no view to anchor on, the API reports every year from 1970.
        assert!(response.years.contains_key("1970"));
        assert!(response.averages.contains_key("1970"));

        // The API sends `months` as an empty array rather than an empty object.
        let year = response.years.get("2026").expect("2026 should exist");
        assert_eq!(year.total, 0);
        assert!(year.months.is_empty());

        let average = response.averages.get("2026").expect("2026 should exist");
        assert_eq!(average.overall, 0);
        assert!(average.months.is_empty());

        assert_eq!(response.daily_views.len(), 3);
        assert!(response.daily_views.iter().all(|d| d.views == 0));
    }
}

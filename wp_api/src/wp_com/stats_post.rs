use crate::{posts::PostId, wp_com::stats_visits::StatsVisitsDataValue};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wp_serde_helper::{
    deserialize_empty_array_or_hashmap, deserialize_false_as_none, deserialize_u64_or_string,
};

/// Response from the per-post stats endpoint.
///
/// The endpoint returns the post's complete view history, so
/// [`Self::daily_views`] can hold thousands of entries for a long-lived post.
/// Callers that only need a trailing window (such as the "Latest Post Summary"
/// card) should slice the tail of it — `daily_views.suffix(7)` in Swift,
/// `dailyViews.takeLast(7)` in Kotlin.
///
/// # The site's home page
///
/// Requesting `PostId(0)` returns view stats for the site's home page, which
/// `/stats/top-posts` reports as a pseudo-entry with that id. The home page
/// isn't a post, so [`Self::post`], [`Self::discussion`] and [`Self::like_count`]
/// are all `None` for it; every view field is populated as usual.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
#[serde(from = "RawStatsPostResponse")]
pub struct StatsPostResponse {
    /// The date the stats were generated for (format: YYYY-MM-DD).
    pub date: String,
    /// The post's all-time view count.
    pub views: u64,
    /// Yearly view totals, keyed by year (e.g. `"2026"`).
    pub years: HashMap<String, StatsPostYear>,
    /// Yearly view averages, keyed by year (e.g. `"2026"`).
    pub averages: HashMap<String, StatsPostAverage>,
    /// The most recent weeks of daily views, oldest first.
    pub weeks: Vec<StatsPostWeek>,
    /// The post's complete daily view history, oldest first.
    ///
    /// The API sends this as a `fields`/`data` column table; it is flattened
    /// while deserializing so callers never handle the column indirection.
    /// Empty if the response doesn't name both the `period` and `views` columns.
    pub daily_views: Vec<StatsPostDailyView>,
    /// The highest view count the post reached in a single month.
    pub highest_month: u64,
    /// The highest daily view average the post reached.
    pub highest_day_average: u64,
    /// The highest weekly view average the post reached.
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
#[derive(Deserialize)]
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
    // The home page (`PostId(0)`) has no post behind it, so the API sends these
    // as `null` — and `post` as boolean `false` rather than `null`.
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

/// Flattens the `fields`/`data` column table into data points, skipping rows the
/// columns can't be read from.
///
/// Takes `data` by value so each row's period string is moved rather than
/// copied — the history runs to thousands of rows on a long-lived post.
fn daily_views(fields: &[String], data: Vec<Vec<StatsVisitsDataValue>>) -> Vec<StatsPostDailyView> {
    let (Some(period_index), Some(views_index)) = (
        fields.iter().position(|field| field == "period"),
        fields.iter().position(|field| field == "views"),
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
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsPostAverage {
    /// View averages keyed by month number (`"1"` through `"12"`).
    #[serde(deserialize_with = "deserialize_empty_array_or_hashmap")]
    pub months: HashMap<String, f64>,
    /// The average views across the whole year.
    pub overall: f64,
}

/// A week of daily views.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsPostWeek {
    /// The days in the week, oldest first. The final week may be partial.
    pub days: Vec<StatsPostDay>,
    /// The total views for the week.
    pub total: u64,
    /// The average daily views for the week.
    pub average: f64,
    /// The change from the previous week, or `None` for the first week.
    pub change: Option<StatsPostChange>,
}

/// The change in views from one week to the next.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, uniffi::Enum)]
#[serde(from = "RawStatsPostChange", into = "RawStatsPostChange")]
pub enum StatsPostChange {
    /// The percentage change from the previous week.
    Percentage { value: f64 },
    /// The previous week had no views, so the change is unbounded. The API
    /// sends this as `{"isInfinity": true}` because the underlying value is
    /// infinite and cannot be represented in JSON.
    Infinite,
}

/// The wire representations the API uses for a week's `change`.
///
/// These three shapes — a number, `{"isInfinity": true}`, and `null` (handled by
/// the surrounding `Option`) — are the only ones observed across 60 real
/// responses spanning 15 sites. A week following a zero-view week always reports
/// an integer `0` rather than a not-a-number marker, so there is no `isNan`
/// counterpart to model.
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
/// metadata but not a permalink. Fields the API sends that aren't modelled here
/// (post content, ping status, and similar) are ignored.
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
    #[serde(rename = "post_title")]
    pub title: String,
    /// The post's publication date in the site's timezone (format: YYYY-MM-DD HH:MM:SS).
    #[serde(rename = "post_date")]
    pub date: String,
    /// The post's publication date in GMT (format: YYYY-MM-DD HH:MM:SS).
    #[serde(rename = "post_date_gmt")]
    pub date_gmt: String,
    /// The date the post was last modified (format: YYYY-MM-DD HH:MM:SS).
    #[serde(rename = "post_modified")]
    pub modified: String,
    /// The post's slug.
    #[serde(rename = "post_name")]
    pub slug: String,
    /// The post's status, e.g. `"publish"` or `"draft"`.
    #[serde(rename = "post_status")]
    pub status: String,
    /// The post's type, e.g. `"post"` or `"page"`.
    pub post_type: String,
    /// The ID of the post's author.
    #[serde(rename = "post_author", deserialize_with = "deserialize_u64_or_string")]
    pub author_id: u64,
    /// The post's globally unique identifier. Not a permalink.
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
        assert_eq!(average.overall, 31.0);
        assert_eq!(average.months.get("6"), Some(&293.0));
    }

    #[test]
    fn test_stats_post_details() {
        let post = parse(WITH_VIEWS).post.expect("a real post has details");

        assert_eq!(post.id, PostId(2729));
        assert_eq!(
            post.title,
            "The Last Version of FeedDemon is Here, and it's Free"
        );
        assert_eq!(post.date, "2013-06-20 09:15:49");
        assert_eq!(post.date_gmt, "2013-06-20 13:15:49");
        assert_eq!(post.modified, "2013-06-23 21:57:23");
        assert_eq!(
            post.slug,
            "the-last-version-of-feeddemon-is-here-and-its-free"
        );
        assert_eq!(post.status, "publish");
        assert_eq!(post.post_type, "post");
        assert_eq!(post.guid, "https://example.com/?p=2729");
        // The API sends `post_author` as a string.
        assert_eq!(post.author_id, 5399133);
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
        assert_eq!(first.average, 1.0);
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
        // `PostId(0)` is the site's home page. It isn't a post, so the API sends
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

        // The API sends `months` as an empty array rather than an empty object.
        let year = response.years.get("2026").expect("2026 should exist");
        assert_eq!(year.total, 0);
        assert!(year.months.is_empty());

        let average = response.averages.get("2026").expect("2026 should exist");
        assert_eq!(average.overall, 0.0);
        assert!(average.months.is_empty());

        assert_eq!(response.daily_views.len(), 3);
        assert!(response.daily_views.iter().all(|d| d.views == 0));
    }
}

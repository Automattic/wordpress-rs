use crate::posts::PostId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wp_serde_helper::{deserialize_empty_array_or_hashmap, deserialize_u64_or_string};

/// Response from the per-post stats endpoint.
///
/// The endpoint returns the post's complete view history, so
/// [`Self::daily_views`] can hold thousands of entries for a long-lived post.
/// Callers that only need a trailing window (such as the "Latest Post Summary"
/// card) should slice the tail of it — `daily_views.suffix(7)` in Swift,
/// `dailyViews.takeLast(7)` in Kotlin.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
#[serde(from = "RawStatsPostViewsResponse")]
pub struct StatsPostViewsResponse {
    /// The date the stats were generated for (format: YYYY-MM-DD).
    pub date: String,
    /// The post's all-time view count.
    pub views: u64,
    /// Yearly view totals, keyed by year (e.g. `"2026"`).
    pub years: HashMap<String, StatsPostViewsYear>,
    /// Yearly view averages, keyed by year (e.g. `"2026"`).
    pub averages: HashMap<String, StatsPostViewsAverage>,
    /// The most recent weeks of daily views, oldest first.
    pub weeks: Vec<StatsPostViewsWeek>,
    /// The post's complete daily view history, oldest first.
    ///
    /// The API sends this as a `fields`/`data` column table; it is flattened
    /// while deserializing so callers never handle the column indirection.
    /// Empty if the response doesn't name both the `period` and `views` columns.
    pub daily_views: Vec<StatsPostViewsDataPoint>,
    /// The highest view count the post reached in a single month.
    pub highest_month: u64,
    /// The highest daily view average the post reached.
    pub highest_day_average: u64,
    /// The highest weekly view average the post reached.
    pub highest_week_average: u64,
    /// The post's like count.
    pub like_count: u64,
    /// The post's comment counts.
    pub discussion: StatsPostViewsDiscussion,
    /// The post the stats belong to.
    pub post: StatsPostViewsPost,
}

/// The response as the API sends it, before the `fields`/`data` column table is
/// flattened into [`StatsPostViewsResponse::daily_views`].
#[derive(Deserialize)]
struct RawStatsPostViewsResponse {
    date: String,
    views: u64,
    #[serde(deserialize_with = "deserialize_empty_array_or_hashmap")]
    years: HashMap<String, StatsPostViewsYear>,
    #[serde(deserialize_with = "deserialize_empty_array_or_hashmap")]
    averages: HashMap<String, StatsPostViewsAverage>,
    weeks: Vec<StatsPostViewsWeek>,
    /// Column names for the `data` rows. Always `["period", "views"]` in every
    /// response observed, but read rather than assumed.
    fields: Vec<String>,
    data: Vec<Vec<RawStatsPostViewsDataValue>>,
    highest_month: u64,
    highest_day_average: u64,
    highest_week_average: u64,
    like_count: u64,
    discussion: StatsPostViewsDiscussion,
    post: StatsPostViewsPost,
}

/// A value in a raw `data` row (a string, a number, or null).
#[derive(Deserialize)]
#[serde(untagged)]
enum RawStatsPostViewsDataValue {
    String(String),
    Number(u64),
    Null,
}

impl RawStatsPostViewsDataValue {
    fn as_string(&self) -> Option<&String> {
        match self {
            Self::String(string) => Some(string),
            _ => None,
        }
    }

    fn as_number(&self) -> Option<u64> {
        match self {
            Self::Number(number) => Some(*number),
            _ => None,
        }
    }
}

impl From<RawStatsPostViewsResponse> for StatsPostViewsResponse {
    fn from(raw: RawStatsPostViewsResponse) -> Self {
        Self {
            date: raw.date,
            views: raw.views,
            years: raw.years,
            averages: raw.averages,
            weeks: raw.weeks,
            daily_views: daily_views(&raw.fields, &raw.data),
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
fn daily_views(
    fields: &[String],
    data: &[Vec<RawStatsPostViewsDataValue>],
) -> Vec<StatsPostViewsDataPoint> {
    let (Some(period_index), Some(views_index)) = (
        fields.iter().position(|field| field == "period"),
        fields.iter().position(|field| field == "views"),
    ) else {
        return vec![];
    };

    data.iter()
        .filter_map(|row| {
            Some(StatsPostViewsDataPoint {
                period: row.get(period_index)?.as_string()?.clone(),
                views: row.get(views_index)?.as_number()?,
            })
        })
        .collect()
}

/// A single day's view count from the daily view history.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct StatsPostViewsDataPoint {
    /// The day the views were recorded on (format: YYYY-MM-DD).
    pub period: String,
    /// The number of views on that day.
    pub views: u64,
}

/// A year's view totals.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsPostViewsYear {
    /// View totals keyed by month number (`"1"` through `"12"`).
    #[serde(deserialize_with = "deserialize_empty_array_or_hashmap")]
    pub months: HashMap<String, u64>,
    /// The total views for the year.
    pub total: u64,
}

/// A year's view averages.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsPostViewsAverage {
    /// View averages keyed by month number (`"1"` through `"12"`).
    #[serde(deserialize_with = "deserialize_empty_array_or_hashmap")]
    pub months: HashMap<String, f64>,
    /// The average views across the whole year.
    pub overall: f64,
}

/// A week of daily views.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsPostViewsWeek {
    /// The days in the week, oldest first. The final week may be partial.
    pub days: Vec<StatsPostViewsDay>,
    /// The total views for the week.
    pub total: u64,
    /// The average daily views for the week.
    pub average: f64,
    /// The change from the previous week, or `None` for the first week.
    pub change: Option<StatsPostViewsChange>,
}

/// The change in views from one week to the next.
#[derive(Debug, Clone, Copy, PartialEq, uniffi::Enum)]
pub enum StatsPostViewsChange {
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
#[derive(Deserialize)]
#[serde(untagged)]
enum RawStatsPostViewsChange {
    Percentage(f64),
    Infinite {
        // The value is ignored: the API only ever sends `true`, and the presence
        // of the key is what identifies the shape.
        #[allow(dead_code)]
        #[serde(rename = "isInfinity")]
        is_infinity: bool,
    },
}

impl<'de> Deserialize<'de> for StatsPostViewsChange {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(match RawStatsPostViewsChange::deserialize(deserializer)? {
            RawStatsPostViewsChange::Percentage(value) => Self::Percentage { value },
            RawStatsPostViewsChange::Infinite { .. } => Self::Infinite,
        })
    }
}

impl Serialize for StatsPostViewsChange {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        match self {
            Self::Percentage { value } => serializer.serialize_f64(*value),
            Self::Infinite => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("isInfinity", &true)?;
                map.end()
            }
        }
    }
}

/// A single day within a [`StatsPostViewsWeek`].
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsPostViewsDay {
    /// The day (format: YYYY-MM-DD).
    pub day: String,
    /// The number of views on that day.
    pub count: u64,
}

/// A post's comment counts.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsPostViewsDiscussion {
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
/// string here, and [`StatsPostViewsDiscussion::comment_count`] carries the same
/// value as a number.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsPostViewsPost {
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

    const WITH_VIEWS: &str = "tests/wpcom/stats_post_views/post-with-views.json";
    const NO_VIEWS: &str = "tests/wpcom/stats_post_views/post-no-views.json";

    fn parse(json_file_path: &str) -> StatsPostViewsResponse {
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        serde_json::from_reader(file).expect("Unable to parse JSON")
    }

    /// A minimal response carrying the given `fields`/`data` column table, for
    /// exercising the flattening in isolation.
    fn parse_with_columns(fields: &str, data: &str) -> StatsPostViewsResponse {
        let json = format!(
            r#"{{
                "date": "2026-08-06",
                "views": 0,
                "years": {{}},
                "averages": {{}},
                "weeks": [],
                "fields": {fields},
                "data": {data},
                "highest_month": 0,
                "highest_day_average": 0,
                "highest_week_average": 0,
                "like_count": 0,
                "discussion": {{ "comment_count": 0 }},
                "post": {{
                    "ID": 1,
                    "post_title": "A Post",
                    "post_date": "2026-01-01 00:00:00",
                    "post_date_gmt": "2026-01-01 00:00:00",
                    "post_modified": "2026-01-01 00:00:00",
                    "post_name": "a-post",
                    "post_status": "publish",
                    "post_type": "post",
                    "post_author": "1",
                    "guid": "https://example.com/?p=1"
                }}
            }}"#
        );
        serde_json::from_str(&json).expect("Unable to parse JSON")
    }

    #[rstest]
    #[case(WITH_VIEWS)]
    #[case(NO_VIEWS)]
    fn test_stats_post_views_response_deserialization(#[case] json_file_path: &str) {
        let response = parse(json_file_path);

        assert!(!response.date.is_empty());
        assert!(!response.daily_views.is_empty());
        assert!(!response.weeks.is_empty());
    }

    #[test]
    fn test_stats_post_views_response_details() {
        let response = parse(WITH_VIEWS);

        assert_eq!(response.date, "2026-08-06");
        assert_eq!(response.views, 19096);
        assert_eq!(response.highest_month, 3224);
        assert_eq!(response.highest_day_average, 293);
        assert_eq!(response.highest_week_average, 3);
        assert_eq!(response.like_count, 9);
        assert_eq!(response.discussion.comment_count, 48);

        let year = response.years.get("2013").expect("2013 should exist");
        assert_eq!(year.total, 6146);
        assert_eq!(year.months.get("6"), Some(&3224));

        let average = response.averages.get("2013").expect("2013 should exist");
        assert_eq!(average.overall, 31.0);
        assert_eq!(average.months.get("6"), Some(&293.0));
    }

    #[test]
    fn test_stats_post_views_post() {
        let post = parse(WITH_VIEWS).post;

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
    fn test_stats_post_views_weeks() {
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
            Some(StatsPostViewsChange::Percentage {
                value: 133.33333333333334
            })
        );

        // The API sends `{"isInfinity": true}` when the previous week had no views.
        let last = &weeks[2];
        assert_eq!(last.change, Some(StatsPostViewsChange::Infinite));
    }

    #[test]
    fn test_stats_post_views_change_round_trips() {
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
    fn test_stats_post_views_daily_views() {
        let daily_views = parse(WITH_VIEWS).daily_views;

        assert_eq!(daily_views.len(), 5);
        assert_eq!(
            daily_views[0],
            StatsPostViewsDataPoint {
                period: "2013-06-20".to_string(),
                views: 1194,
            }
        );
        assert_eq!(
            daily_views[4],
            StatsPostViewsDataPoint {
                period: "2026-08-06".to_string(),
                views: 0,
            }
        );
    }

    #[test]
    fn test_stats_post_views_daily_views_reads_the_column_table() {
        // The column positions are read rather than assumed, so swapping them
        // still yields the same data points.
        let response = parse_with_columns(
            r#"["views", "period"]"#,
            r#"[[5, "2026-08-04"], [7, "2026-08-06"]]"#,
        );

        assert_eq!(response.daily_views.len(), 2);
        assert_eq!(response.daily_views[0].period, "2026-08-04");
        assert_eq!(response.daily_views[0].views, 5);
    }

    #[test]
    fn test_stats_post_views_daily_views_skips_unreadable_rows() {
        let response = parse_with_columns(
            r#"["period", "views"]"#,
            r#"[["2026-08-04", 5], [null, null], ["2026-08-06", 7]]"#,
        );

        assert_eq!(
            response.daily_views.len(),
            2,
            "the unreadable row should be dropped, not derail the rest"
        );
        assert_eq!(response.daily_views[0].period, "2026-08-04");
        assert_eq!(response.daily_views[1].period, "2026-08-06");
    }

    #[test]
    fn test_stats_post_views_daily_views_without_known_columns() {
        let response = parse_with_columns(r#"["period"]"#, r#"[["2026-08-04"]]"#);

        assert!(response.daily_views.is_empty());
    }

    #[test]
    fn test_stats_post_views_post_without_views() {
        let response = parse(NO_VIEWS);

        assert_eq!(response.views, 0);
        assert_eq!(response.highest_month, 0);
        assert_eq!(response.like_count, 0);
        assert_eq!(response.discussion.comment_count, 0);

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

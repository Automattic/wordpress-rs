use crate::{
    date::WpDateString,
    impl_as_query_value_from_to_string,
    url_query::{AppendUrlQueryPairs, QueryPairs, QueryPairsExtension},
};
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{self, IgnoredAny, MapAccess, SeqAccess, Unexpected, Visitor},
};
use std::{collections::HashMap, fmt};
use wp_serde_helper::deserialize_option_empty_array_or_hashmap;

/// The time period for grouping archive views.
#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    uniffi::Enum,
    strum_macros::EnumString,
    strum_macros::Display,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum StatsArchivesPeriod {
    #[default]
    Day,
    Week,
    Month,
    Year,
}

impl_as_query_value_from_to_string!(StatsArchivesPeriod);

/// Parameters for the stats archives endpoint.
#[derive(Debug, PartialEq, Eq, uniffi::Record)]
pub struct StatsArchivesParams {
    /// The time period for grouping stats.
    #[uniffi(default = None)]
    pub period: Option<StatsArchivesPeriod>,
    /// The date to query stats for (format: YYYY-MM-DD).
    #[uniffi(default = None)]
    pub date: Option<WpDateString>,
    /// The start date to query stats for (format: YYYY-MM-DD).
    #[uniffi(default = None)]
    pub start_date: Option<WpDateString>,
    /// The maximum number of entries to return for each archive type.
    #[uniffi(default = None)]
    pub max: Option<u32>,
    /// The number of periods to include in the response.
    #[uniffi(default = None)]
    pub num: Option<u32>,
    /// Whether to return a summary of the data.
    ///
    /// - `true` (default): Response contains `summary` field with aggregated data
    /// - `false`: Response contains `days` field with per-day breakdown
    #[uniffi(default = true)]
    pub summarize: bool,
    /// Whether to skip archive pages (date-based archives, category archives, etc.) when
    /// attributing views to posts.
    ///
    /// - `true` (default): Archive pages are excluded from results
    /// - `false`: Archive pages are included in results
    #[uniffi(default = true)]
    pub skip_archives: bool,
}

impl Default for StatsArchivesParams {
    fn default() -> Self {
        Self {
            period: None,
            date: None,
            start_date: None,
            max: None,
            num: None,
            summarize: true,
            skip_archives: true,
        }
    }
}

impl AppendUrlQueryPairs for StatsArchivesParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut
            .append_option_query_value_pair("period", self.period.as_ref())
            .append_option_query_value_pair("date", self.date.as_ref())
            .append_option_query_value_pair("start_date", self.start_date.as_ref())
            .append_option_query_value_pair("max", self.max.as_ref())
            .append_option_query_value_pair("num", self.num.as_ref())
            .append_query_value_pair("summarize", &(self.summarize as u32))
            .append_query_value_pair("skip_archives", &(self.skip_archives as u32));
    }
}

/// Response from the stats archives endpoint.
///
/// The response structure varies based on the `summarize` parameter:
/// - When `summarize=1`: Contains `summary` field with aggregated data
/// - When `summarize=0`: Contains `days` field with per-day data
#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct StatsArchivesResponse {
    /// The date for the stats query.
    pub date: WpDateString,
    /// The time period used for grouping.
    pub period: Option<String>,
    /// Archive views aggregated over the queried period, keyed by archive type
    /// (present when summarize=1).
    ///
    /// The API decides which groups it sends, so a response only carries the kinds of archive
    /// page the site actually has views for. Observed keys are `search` (the site's search
    /// results pages) and `cat` (category archives); others — such as tag, author and date
    /// archives — follow the same shape. Each list is sorted by descending views.
    #[serde(default, deserialize_with = "deserialize_optional_archive_groups")]
    pub summary: Option<ArchiveGroupMap>,
    /// Per-day archive views, keyed by date string and then by archive type (present when
    /// summarize=0).
    ///
    /// Each day carries the same groups as [`StatsArchivesResponse::summary`]; see its
    /// documentation for which keys to expect.
    #[serde(default, deserialize_with = "deserialize_optional_days")]
    pub days: Option<HashMap<String, ArchiveGroupMap>>,
}

/// Archive entries keyed by archive type, each sorted by descending views.
pub type ArchiveGroupMap = HashMap<String, Vec<StatsArchivesEntry>>;

/// A single archive page entry in the stats archives response.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsArchivesEntry {
    /// The name of the archive, e.g. the search term or the category slug.
    pub value: Option<String>,
    /// The URL of the archive page.
    pub href: Option<String>,
    /// The number of views of this archive page.
    pub views: Option<u64>,
}

/// A group map that tolerates the shapes the API actually sends.
///
/// Two of them need handling beyond a plain `HashMap`:
///
/// - PHP's `json_encode` turns an empty associative array into `[]`, so a site with no archive
///   views sends `[]` rather than `{}`. This is the norm across `wp_com`'s stats endpoints — see
///   [`wp_serde_helper::deserialize_empty_array_or_hashmap`] and its call sites.
/// - Every sibling stats endpoint sends scalar totals (`other_*`, `total_*`) alongside its
///   entry lists. Archive groups are keyed by the server, so a total would otherwise have to
///   parse as a list of entries and fail the entire response.
#[derive(Debug, Clone, Default)]
struct ArchiveGroups(ArchiveGroupMap);

impl<'de> Deserialize<'de> for ArchiveGroups {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ArchiveGroupsVisitor)
    }
}

struct ArchiveGroupsVisitor;

impl<'de> Visitor<'de> for ArchiveGroupsVisitor {
    type Value = ArchiveGroups;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a map of archive types to entry lists, or an empty array")
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(ArchiveGroups::default())
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        if seq.next_element::<IgnoredAny>()?.is_some() {
            return Err(de::Error::invalid_type(Unexpected::Seq, &self));
        }
        Ok(ArchiveGroups::default())
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut groups = ArchiveGroupMap::new();
        while let Some(key) = map.next_key::<String>()? {
            if let MaybeEntries::Entries(entries) = map.next_value()? {
                groups.insert(key, entries);
            }
        }
        Ok(ArchiveGroups(groups))
    }
}

/// An archive group's value: a list of entries, or anything else the API sends next to them.
#[derive(Deserialize)]
#[serde(untagged)]
enum MaybeEntries {
    Entries(Vec<StatsArchivesEntry>),
    #[allow(dead_code)]
    Other(IgnoredAny),
}

fn deserialize_optional_archive_groups<'de, D>(
    deserializer: D,
) -> Result<Option<ArchiveGroupMap>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<ArchiveGroups>::deserialize(deserializer)?.map(|groups| groups.0))
}

fn deserialize_optional_days<'de, D>(
    deserializer: D,
) -> Result<Option<HashMap<String, ArchiveGroupMap>>, D::Error>
where
    D: Deserializer<'de>,
{
    let days: Option<HashMap<String, ArchiveGroups>> =
        deserialize_option_empty_array_or_hashmap(deserializer)?;
    Ok(days.map(|days| {
        days.into_iter()
            .map(|(date, groups)| (date, groups.0))
            .collect()
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[test]
    fn test_stats_archives_params_serialization() {
        let mut url =
            url::Url::parse("https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/archives")
                .expect("Failed to parse url");

        let params = StatsArchivesParams {
            period: Some(StatsArchivesPeriod::Day),
            date: Some(WpDateString::new("2026-10-08".to_string())),
            start_date: Some(WpDateString::new("2026-10-01".to_string())),
            max: Some(10),
            num: Some(30),
            summarize: true,
            skip_archives: true,
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/archives?period=day&date=2026-10-08&start_date=2026-10-01&max=10&num=30&summarize=1&skip_archives=1"
        );
    }

    #[test]
    fn test_stats_archives_params_serialization_partial() {
        let mut url =
            url::Url::parse("https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/archives")
                .expect("Failed to parse url");

        let params = StatsArchivesParams {
            period: Some(StatsArchivesPeriod::Week),
            date: Some(WpDateString::new("2026-10-08".to_string())),
            ..Default::default()
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/archives?period=week&date=2026-10-08&summarize=1&skip_archives=1"
        );
    }

    #[test]
    fn test_stats_archives_params_with_false_values() {
        let mut url =
            url::Url::parse("https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/archives")
                .expect("Failed to parse url");

        let params = StatsArchivesParams {
            period: Some(StatsArchivesPeriod::Day),
            summarize: false,
            skip_archives: false,
            ..Default::default()
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/archives?period=day&summarize=0&skip_archives=0"
        );
    }

    #[rstest]
    #[case("tests/wpcom/stats_archives/summarized-01-day.json", true)]
    #[case("tests/wpcom/stats_archives/summarized-02-day-with-nulls.json", true)]
    #[case(
        "tests/wpcom/stats_archives/summarized-03-day-empty-response.json",
        true
    )]
    #[case("tests/wpcom/stats_archives/summarized-04-day-empty-array.json", true)]
    #[case("tests/wpcom/stats_archives/no-summary-01.json", false)]
    #[case(
        "tests/wpcom/stats_archives/no-summary-02-empty-array-days.json",
        false
    )]
    fn test_stats_archives_response_deserialization(
        #[case] json_file_path: &str,
        #[case] expect_summary: bool,
    ) {
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsArchivesResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        // Common assertion: date is always present
        assert!(!response.date.value.is_empty());

        if expect_summary {
            assert!(
                response.period.is_some(),
                "Expected period for summarized response"
            );
            assert!(!response.period.as_ref().unwrap().is_empty());
            assert!(
                response.days.is_none(),
                "Summarized response should not have days"
            );

            response
                .summary
                .as_ref()
                .expect("Summary should be present for summarized response");
        } else {
            assert!(
                response.summary.is_none(),
                "Days response should not have summary"
            );

            let days = response
                .days
                .as_ref()
                .expect("Days should be present for non-summarized response");
            assert!(!days.is_empty());
        }
    }

    #[test]
    fn test_stats_archives_response_deserialization_summary() {
        let json_file_path = "tests/wpcom/stats_archives/summarized-01-day.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsArchivesResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.date.value, "2026-10-08");
        assert_eq!(response.period, Some("day".to_string()));

        let summary = response
            .summary
            .as_ref()
            .expect("Summary should be present");
        assert_eq!(summary.len(), 2);

        let search = summary
            .get("search")
            .expect("The search archive type should be present");
        assert_eq!(search.len(), 3);
        assert_eq!(search[0].value, Some("sabbatical".to_string()));
        assert_eq!(
            search[0].href,
            Some("https://example.com/?s=sabbatical".to_string())
        );
        assert_eq!(search[0].views, Some(5));

        let categories = summary
            .get("cat")
            .expect("The cat archive type should be present");
        assert_eq!(categories.len(), 2);
        assert_eq!(categories[0].value, Some("benefits".to_string()));
        assert_eq!(
            categories[0].href,
            Some("https://example.com/category/benefits/".to_string())
        );
        assert_eq!(categories[0].views, Some(27));
    }

    #[test]
    fn test_stats_archives_response_deserialization_days() {
        let json_file_path = "tests/wpcom/stats_archives/no-summary-01.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsArchivesResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.date.value, "2026-10-08");
        assert!(response.summary.is_none());

        let days = response.days.as_ref().expect("Days should be present");
        assert_eq!(days.len(), 2);

        // Verify day with archive views
        let day = days.get("2026-10-07").expect("2026-10-07 should exist");
        assert_eq!(day.len(), 2);
        let search = day
            .get("search")
            .expect("The search archive type should be present");
        assert_eq!(search.len(), 1);
        assert_eq!(search[0].value, Some("swag".to_string()));
        assert_eq!(search[0].views, Some(4));
        let categories = day
            .get("cat")
            .expect("The cat archive type should be present");
        assert_eq!(categories.len(), 1);
        assert_eq!(categories[0].value, Some("benefits".to_string()));
        assert_eq!(categories[0].views, Some(27));

        // A day without any archive views sends no groups at all
        let empty_day = days.get("2026-10-08").expect("2026-10-08 should exist");
        assert!(empty_day.is_empty());
    }

    #[test]
    fn test_stats_archives_with_null_values() {
        let json_file_path = "tests/wpcom/stats_archives/summarized-02-day-with-nulls.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsArchivesResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON with null values");

        assert_eq!(response.date.value, "2026-10-08");
        assert_eq!(response.period, Some("day".to_string()));

        let summary = response
            .summary
            .as_ref()
            .expect("Summary should be present");
        let search = summary
            .get("search")
            .expect("The search archive type should be present");
        assert_eq!(search.len(), 2);

        // First entry: all nullable fields are null
        let all_nulls = &search[0];
        assert!(all_nulls.value.is_none());
        assert!(all_nulls.href.is_none());
        assert!(all_nulls.views.is_none());

        // Second entry: all fields have values
        let with_values = &search[1];
        assert_eq!(with_values.value, Some("swag".to_string()));
        assert_eq!(
            with_values.href,
            Some("https://example.com/?s=swag".to_string())
        );
        assert_eq!(with_values.views, Some(4));
    }

    /// A site with no archive views at all sends `summary` as `[]`, because PHP's `json_encode`
    /// turns an empty associative array into an array rather than an object.
    #[test]
    fn test_stats_archives_empty_summary_as_array() {
        let json_file_path = "tests/wpcom/stats_archives/summarized-04-day-empty-array.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsArchivesResponse =
            serde_json::from_reader(file).expect("Unable to parse an empty `summary` array");

        assert_eq!(response.date.value, "2026-10-08");
        let summary = response
            .summary
            .as_ref()
            .expect("An empty summary should still be present");
        assert!(summary.is_empty());
        assert!(response.days.is_none());
    }

    /// A day with no archive views sends `[]` for the same reason, and days with views carry
    /// scalar totals alongside the entry lists, as every sibling stats endpoint does.
    #[test]
    fn test_stats_archives_days_with_empty_array_and_totals() {
        let json_file_path = "tests/wpcom/stats_archives/no-summary-02-empty-array-days.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsArchivesResponse =
            serde_json::from_reader(file).expect("Unable to parse `days` with `[]` and totals");

        let days = response.days.as_ref().expect("Days should be present");
        assert_eq!(days.len(), 2);

        let empty_day = days.get("2026-10-08").expect("2026-10-08 should exist");
        assert!(empty_day.is_empty());

        // The scalar totals are skipped, leaving only the entry lists.
        let day = days.get("2026-10-07").expect("2026-10-07 should exist");
        assert_eq!(day.len(), 1);
        let search = day
            .get("search")
            .expect("The search archive type should be present");
        assert_eq!(search.len(), 1);
        assert_eq!(search[0].value, Some("swag".to_string()));
        assert_eq!(search[0].views, Some(4));
    }

    /// The exact bodies that failed in CI before the group maps tolerated `[]`.
    #[rstest]
    #[case(r#"{"date":"2026-10-08","utc_offset":"+03:00","period":"day","summary":[]}"#)]
    #[case(
        r#"{"date":"2026-10-08","utc_offset":"+03:00","period":"day","days":{"2026-10-08":[]}}"#
    )]
    #[case(r#"{"date":"2026-10-08","utc_offset":"+03:00","period":"day","days":[]}"#)]
    fn test_stats_archives_sites_without_archive_views(#[case] body: &str) {
        let response: StatsArchivesResponse =
            serde_json::from_str(body).expect("A site with no archive views should parse");

        assert_eq!(response.date.value, "2026-10-08");
        assert!(response.summary.as_ref().is_none_or(|s| s.is_empty()));
        assert!(
            response
                .days
                .as_ref()
                .is_none_or(|d| d.values().all(|groups| groups.is_empty()))
        );
    }

    /// `null` is absent, not empty.
    #[test]
    fn test_stats_archives_null_summary_and_days() {
        let response: StatsArchivesResponse =
            serde_json::from_str(r#"{"date":"2026-10-08","summary":null,"days":null}"#)
                .expect("Unable to parse null summary and days");

        assert!(response.summary.is_none());
        assert!(response.days.is_none());
    }

    /// A scalar alongside the groups in `summary` is skipped rather than failing the response.
    #[test]
    fn test_stats_archives_summary_with_totals() {
        let response: StatsArchivesResponse = serde_json::from_str(
            r#"{"date":"2026-10-08","summary":{"search":[{"value":"swag","views":4}],"total_archives":4}}"#,
        )
        .expect("Unable to parse a summary carrying totals");

        let summary = response
            .summary
            .as_ref()
            .expect("Summary should be present");
        assert_eq!(summary.len(), 1);
        assert_eq!(summary["search"][0].value, Some("swag".to_string()));
    }

    #[test]
    fn test_stats_archives_empty_summary() {
        let json_file_path = "tests/wpcom/stats_archives/summarized-03-day-empty-response.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsArchivesResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        let summary = response
            .summary
            .as_ref()
            .expect("Summary should be present");
        assert!(summary.is_empty());
    }
}

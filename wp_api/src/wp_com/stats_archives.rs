use crate::{
    date::WpDateString,
    impl_as_query_value_from_to_string,
    url_query::{AppendUrlQueryPairs, QueryPairs, QueryPairsExtension},
};
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{self, IgnoredAny, MapAccess, SeqAccess, Unexpected, Visitor},
    ser::SerializeMap,
};
use std::{collections::HashMap, fmt};
use wp_serde_helper::{
    deserialize_null_or_empty_array_as_hashmap, deserialize_option_empty_array_or_hashmap,
    deserialize_string_or_integer_or_null,
};

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
    /// Archive views aggregated over the queried period (present when summarize=1).
    ///
    /// See [`StatsArchivesSummaryData`] for how the API groups them.
    #[serde(default)]
    pub summary: Option<StatsArchivesSummaryData>,
    /// Per-day archive views, keyed by date string (present when summarize=0).
    ///
    /// Each day carries the same groups as [`StatsArchivesResponse::summary`], built by the same
    /// code path on the server.
    #[serde(
        default,
        deserialize_with = "deserialize_option_empty_array_or_hashmap"
    )]
    pub days: Option<HashMap<String, StatsArchivesDayData>>,
}

/// Archive views aggregated over the queried period.
#[derive(Debug, Clone, Default, uniffi::Record)]
pub struct StatsArchivesSummaryData {
    /// Entry lists keyed by archive type, each sorted by descending views.
    ///
    /// The API only sends the kinds of archive page the site actually has views for, so a
    /// missing key means no views rather than no such archive. Observed keys are `home`,
    /// `search`, `cat`, `tag`, `author` and `date`.
    ///
    /// Never contains `tax`; those entries live in [`Self::tax`].
    pub groups: ArchiveGroupMap,
    /// Entry lists for custom taxonomy archives, keyed by taxonomy name (e.g. `topics`), each
    /// sorted by descending views.
    ///
    /// The API collapses every `tax_<taxonomy>` archive type under a single `tax` key whose
    /// value is an object of entry lists rather than a list, so these cannot sit alongside the
    /// other archive types in [`Self::groups`].
    pub tax: ArchiveGroupMap,
}

/// Archive views for a single day.
#[derive(Debug, Clone, Default, uniffi::Record)]
pub struct StatsArchivesDayData {
    /// Entry lists keyed by archive type for this day, each sorted by descending views.
    ///
    /// Populated on the same basis as [`StatsArchivesSummaryData::groups`], and likewise never
    /// contains `tax`.
    pub groups: ArchiveGroupMap,
    /// Entry lists for custom taxonomy archives for this day, keyed by taxonomy name.
    ///
    /// Populated on the same basis as [`StatsArchivesSummaryData::tax`].
    pub tax: ArchiveGroupMap,
}

/// Archive entries keyed by archive type or taxonomy name, each sorted by descending views.
pub type ArchiveGroupMap = HashMap<String, Vec<StatsArchivesEntry>>;

/// A single archive page entry in the stats archives response.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsArchivesEntry {
    /// The name of the archive, e.g. the search term or the category slug.
    ///
    /// The API sends these as PHP array keys, so a numeric name arrives as a JSON number — `1`
    /// for the home page, `2024` for a search for that year — and is read as the equivalent
    /// string. Serializing writes it back as a string, so a number does not survive a
    /// round-trip verbatim.
    #[serde(default, deserialize_with = "deserialize_string_or_integer_or_null")]
    pub value: Option<String>,
    /// The URL of the archive page.
    pub href: Option<String>,
    /// The number of views of this archive page.
    pub views: Option<u64>,
}

/// The key the API collapses every `tax_<taxonomy>` archive type under.
const TAX_GROUP_KEY: &str = "tax";

/// A group map split into its flat archive types and its taxonomy entries.
///
/// Shared by [`StatsArchivesSummaryData`] and [`StatsArchivesDayData`], which the endpoint
/// builds from the same `format_response` code path.
#[derive(Debug, Clone, Default)]
struct ArchiveGroups {
    groups: ArchiveGroupMap,
    tax: ArchiveGroupMap,
}

/// Reads the shapes the endpoint actually sends, and nothing else.
///
/// Two need handling beyond a plain `HashMap`:
///
/// - PHP's `json_encode` turns an empty associative array into `[]`, so a site with no archive
///   views sends `[]` rather than `{}`, and a day with none does the same. This is the norm
///   across `wp_com`'s stats endpoints — see
///   [`wp_serde_helper::deserialize_empty_array_or_hashmap`] and its call sites.
/// - `tax` holds an object of entry lists rather than a list, because the server collapses every
///   `tax_<taxonomy>` group under that one key.
///
/// Every other shape is an error. `format_response()` emits an entry list per archive type and
/// an object of entry lists for `tax`; anything else means the endpoint changed, and failing is
/// how that becomes visible rather than becoming missing data.
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
        let mut archive_groups = ArchiveGroups::default();
        while let Some(key) = map.next_key::<String>()? {
            // Naming the key is the point of the strictness: a bare serde type error says a
            // group was the wrong shape, not which one.
            if key == TAX_GROUP_KEY {
                archive_groups.tax = map
                    .next_value::<TaxGroups>()
                    .map_err(|error| de::Error::custom(format!("archive group `{key}`: {error}")))?
                    .0;
            } else {
                let entries = map
                    .next_value::<Vec<StatsArchivesEntry>>()
                    .map_err(|error| {
                        de::Error::custom(format!("archive group `{key}`: {error}"))
                    })?;
                archive_groups.groups.insert(key, entries);
            }
        }
        Ok(archive_groups)
    }
}

/// The `tax` group's value: taxonomy-keyed entry lists.
///
/// Carries the same `null` and `[]` tolerance as the group map around it, for the same reason —
/// PHP renders an empty associative array as an array — and the same strictness about what the
/// lists contain.
#[derive(Deserialize)]
struct TaxGroups(
    #[serde(deserialize_with = "deserialize_null_or_empty_array_as_hashmap")] ArchiveGroupMap,
);

/// Writes the wire shape back out, so a response round-trips through its own `Deserialize`.
///
/// The taxonomy entries go back under a top-level `tax` key, and the key is omitted when there
/// are none, as the API omits it. A derived implementation would emit `{"groups":…,"tax":…}`,
/// which the `Deserialize` above rejects.
fn serialize_archive_groups<S>(
    groups: &ArchiveGroupMap,
    tax: &ArchiveGroupMap,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    debug_assert!(
        !groups.contains_key(TAX_GROUP_KEY),
        "`tax` is reserved for the taxonomy entries"
    );
    let len = groups.len() + usize::from(!tax.is_empty());
    let mut map = serializer.serialize_map(Some(len))?;
    for (archive_type, entries) in groups {
        map.serialize_entry(archive_type, entries)?;
    }
    if !tax.is_empty() {
        map.serialize_entry(TAX_GROUP_KEY, tax)?;
    }
    map.end()
}

macro_rules! impl_archive_group_serde {
    ($type:ty) => {
        impl<'de> Deserialize<'de> for $type {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let archive_groups = ArchiveGroups::deserialize(deserializer)?;
                Ok(Self {
                    groups: archive_groups.groups,
                    tax: archive_groups.tax,
                })
            }
        }

        impl Serialize for $type {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serialize_archive_groups(&self.groups, &self.tax, serializer)
            }
        }
    };
}

impl_archive_group_serde!(StatsArchivesSummaryData);
impl_archive_group_serde!(StatsArchivesDayData);

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
    #[case("tests/wpcom/stats_archives/summarized-05-day-taxonomies.json", true)]
    #[case(
        "tests/wpcom/stats_archives/summarized-06-day-numeric-values.json",
        true
    )]
    #[case("tests/wpcom/stats_archives/no-summary-01.json", false)]
    #[case("tests/wpcom/stats_archives/no-summary-03-day-taxonomies.json", false)]
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
        assert_eq!(summary.groups.len(), 2);
        assert!(summary.tax.is_empty());

        let search = summary
            .groups
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
            .groups
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
        assert_eq!(day.groups.len(), 2);
        let search = day
            .groups
            .get("search")
            .expect("The search archive type should be present");
        assert_eq!(search.len(), 1);
        assert_eq!(search[0].value, Some("swag".to_string()));
        assert_eq!(search[0].views, Some(4));
        let categories = day
            .groups
            .get("cat")
            .expect("The cat archive type should be present");
        assert_eq!(categories.len(), 1);
        assert_eq!(categories[0].value, Some("benefits".to_string()));
        assert_eq!(categories[0].views, Some(27));

        // A day without any archive views sends no groups at all
        let empty_day = days.get("2026-10-08").expect("2026-10-08 should exist");
        assert!(empty_day.groups.is_empty() && empty_day.tax.is_empty());
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
            .groups
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
        assert!(summary.groups.is_empty() && summary.tax.is_empty());
        assert!(response.days.is_none());
    }

    /// A day with no archive views sends `[]` for the same reason.
    #[test]
    fn test_stats_archives_days_with_empty_array() {
        let json_file_path = "tests/wpcom/stats_archives/no-summary-02-empty-array-days.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsArchivesResponse =
            serde_json::from_reader(file).expect("Unable to parse `days` with `[]`");

        let days = response.days.as_ref().expect("Days should be present");
        assert_eq!(days.len(), 2);

        let empty_day = days.get("2026-10-08").expect("2026-10-08 should exist");
        assert!(empty_day.groups.is_empty() && empty_day.tax.is_empty());

        let day = days.get("2026-10-07").expect("2026-10-07 should exist");
        assert_eq!(day.groups.len(), 1);
        let search = day
            .groups
            .get("search")
            .expect("The search archive type should be present");
        assert_eq!(search.len(), 1);
        assert_eq!(search[0].value, Some("swag".to_string()));
        assert_eq!(search[0].views, Some(4));
    }

    /// A site with no archive views sends `[]` in place of an empty object, in each of the three
    /// positions a group map can appear: `summary`, a single day inside `days`, and `days` itself.
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
        assert!(
            response
                .summary
                .as_ref()
                .is_none_or(|s| s.groups.is_empty() && s.tax.is_empty())
        );
        assert!(response.days.as_ref().is_none_or(|d| {
            d.values()
                .all(|day| day.groups.is_empty() && day.tax.is_empty())
        }));
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

    /// `format_taxonomy_data()` collapses every `tax_<taxonomy>` archive type under a single
    /// `tax` key whose value is an object of entry lists, so taxonomy views cannot be read as
    /// just another archive type.
    #[test]
    fn test_stats_archives_taxonomy_groups() {
        let json_file_path = "tests/wpcom/stats_archives/summarized-05-day-taxonomies.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsArchivesResponse =
            serde_json::from_reader(file).expect("Unable to parse a `tax` block");

        let summary = response
            .summary
            .as_ref()
            .expect("Summary should be present");

        // The taxonomy entries do not masquerade as an archive type.
        assert_eq!(summary.groups.keys().collect::<Vec<_>>(), vec!["cat"]);
        assert!(!summary.groups.contains_key("tax"));

        assert_eq!(summary.tax.len(), 2);
        let topics = summary
            .tax
            .get("topics")
            .expect("The topics taxonomy should be present");
        assert_eq!(topics.len(), 1);
        assert_eq!(topics[0].value, Some("jetpack".to_string()));
        assert_eq!(
            topics[0].href,
            Some("https://example.com/topics/jetpack/".to_string())
        );
        assert_eq!(topics[0].views, Some(4));
        assert_eq!(
            summary.tax["series"][0].value,
            Some("field-notes".to_string())
        );
    }

    /// A day carries its taxonomy entries the same way the summary does.
    #[test]
    fn test_stats_archives_taxonomy_groups_per_day() {
        let json_file_path = "tests/wpcom/stats_archives/no-summary-03-day-taxonomies.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsArchivesResponse =
            serde_json::from_reader(file).expect("Unable to parse a `tax` block in a day");

        let days = response.days.as_ref().expect("Days should be present");
        let day = days.get("2026-10-07").expect("2026-10-07 should exist");
        assert_eq!(day.groups.keys().collect::<Vec<_>>(), vec!["cat"]);
        assert_eq!(day.tax["topics"][0].value, Some("jetpack".to_string()));
    }

    /// PHP turns a numeric-string array key into an integer, so a numeric archive name arrives
    /// as a JSON number. The whole group — including any string-named sibling — used to be
    /// dropped when that happened.
    #[test]
    fn test_stats_archives_numeric_values() {
        let json_file_path = "tests/wpcom/stats_archives/summarized-06-day-numeric-values.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsArchivesResponse =
            serde_json::from_reader(file).expect("Unable to parse numeric archive names");

        let summary = response
            .summary
            .as_ref()
            .expect("Summary should be present");
        assert_eq!(summary.groups.len(), 2);

        // `home`'s value is always `1`.
        let home = summary
            .groups
            .get("home")
            .expect("The home archive type should be present");
        assert_eq!(home.len(), 1);
        assert_eq!(home[0].value, Some("1".to_string()));
        assert_eq!(home[0].views, Some(31));

        let search = summary
            .groups
            .get("search")
            .expect("The search archive type should be present");
        assert_eq!(search.len(), 2);
        assert_eq!(search[0].value, Some("2024".to_string()));
        assert_eq!(search[1].value, Some("swag".to_string()));
    }

    /// A group value that is neither an entry list nor, for `tax`, an object of entry lists
    /// fails the response, and the error names the group. Skipping it would mean losing data
    /// without saying so, which is what this endpoint used to do.
    #[rstest]
    #[case::scalar_total_in_summary(
        r#"{"date":"2026-10-08","summary":{"search":[{"value":"swag","views":4}],"total_archives":4}}"#,
        "total_archives"
    )]
    #[case::scalar_total_in_a_day(
        r#"{"date":"2026-10-08","days":{"2026-10-07":{"other_archives":0}}}"#,
        "other_archives"
    )]
    #[case::object_under_a_non_tax_key(
        r#"{"date":"2026-10-08","summary":{"cat":{"topics":[]}}}"#,
        "cat"
    )]
    #[case::tax_as_a_list(
        r#"{"date":"2026-10-08","summary":{"tax":[{"value":"jetpack","views":4}]}}"#,
        "tax"
    )]
    #[case::unreadable_entry_field(
        r#"{"date":"2026-10-08","summary":{"search":[{"value":1.5}]}}"#,
        "search"
    )]
    fn test_stats_archives_rejects_unexpected_group_values(
        #[case] body: &str,
        #[case] expected_group_name: &str,
    ) {
        let error = serde_json::from_str::<StatsArchivesResponse>(body)
            .expect_err("An unexpected group value should fail the response");

        let message = error.to_string();
        assert!(
            message.contains(&format!("archive group `{expected_group_name}`")),
            "The error should name the offending group, got: {message}"
        );
    }

    /// `tax` carries the same `null` and `[]` tolerance as the group map around it, because PHP
    /// would render an empty taxonomy map the same way.
    #[rstest]
    #[case(r#"{"date":"2026-10-08","summary":{"cat":[],"tax":[]}}"#)]
    #[case(r#"{"date":"2026-10-08","summary":{"cat":[],"tax":null}}"#)]
    fn test_stats_archives_empty_tax(#[case] body: &str) {
        let response: StatsArchivesResponse =
            serde_json::from_str(body).expect("An empty `tax` should parse");

        let summary = response
            .summary
            .as_ref()
            .expect("Summary should be present");
        assert!(summary.tax.is_empty());
        assert!(summary.groups.contains_key("cat"));
    }

    /// A `null` day is absent views, not a broken response.
    #[test]
    fn test_stats_archives_null_day() {
        let response: StatsArchivesResponse =
            serde_json::from_str(r#"{"date":"2026-10-08","days":{"2026-10-08":null}}"#)
                .expect("A null day should parse");

        let days = response.days.as_ref().expect("Days should be present");
        let day = days.get("2026-10-08").expect("2026-10-08 should exist");
        assert!(day.groups.is_empty() && day.tax.is_empty());
    }

    /// Serializing puts the taxonomy entries back under a top-level `tax` key, so a response
    /// survives a round-trip through its own `Deserialize`. A derived `Serialize` would emit
    /// `{"groups":…,"tax":…}`, which the `Deserialize` rejects.
    ///
    /// The comparison is between two serializations rather than against the fixture, because
    /// `HashMap` iteration order is arbitrary, a group map sent as `[]` normalises to `{}`, and
    /// a numeric archive name is written back as a string.
    #[rstest]
    #[case("tests/wpcom/stats_archives/summarized-01-day.json")]
    #[case("tests/wpcom/stats_archives/summarized-02-day-with-nulls.json")]
    #[case("tests/wpcom/stats_archives/summarized-03-day-empty-response.json")]
    #[case("tests/wpcom/stats_archives/summarized-04-day-empty-array.json")]
    #[case("tests/wpcom/stats_archives/summarized-05-day-taxonomies.json")]
    #[case("tests/wpcom/stats_archives/summarized-06-day-numeric-values.json")]
    #[case("tests/wpcom/stats_archives/no-summary-01.json")]
    #[case("tests/wpcom/stats_archives/no-summary-02-empty-array-days.json")]
    #[case("tests/wpcom/stats_archives/no-summary-03-day-taxonomies.json")]
    fn test_stats_archives_response_round_trips(#[case] json_file_path: &str) {
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsArchivesResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        let serialized = serde_json::to_value(&response).expect("Unable to serialize response");
        let reparsed: StatsArchivesResponse =
            serde_json::from_value(serialized.clone()).expect("Unable to reparse the response");

        assert_eq!(
            serde_json::to_value(reparsed).expect("Unable to serialize response"),
            serialized
        );
    }

    /// The taxonomy entries serialize back to the wire's top-level `tax` key rather than to the
    /// record's field layout.
    #[test]
    fn test_stats_archives_serializes_tax_at_the_top_level() {
        let json_file_path = "tests/wpcom/stats_archives/summarized-05-day-taxonomies.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsArchivesResponse =
            serde_json::from_reader(file).expect("Unable to parse a `tax` block");

        let serialized = serde_json::to_value(&response).expect("Unable to serialize response");
        let summary = &serialized["summary"];

        assert!(summary["tax"]["topics"].is_array());
        assert!(summary["cat"].is_array());
        assert!(
            summary.get("groups").is_none(),
            "The record's field layout should not reach the wire"
        );
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
        assert!(summary.groups.is_empty() && summary.tax.is_empty());
    }
}

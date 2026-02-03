use crate::{
    impl_as_query_value_from_to_string,
    url_query::{AppendUrlQueryPairs, QueryPairs, QueryPairsExtension},
};
use serde::{Deserialize, Serialize};

/// The time unit for grouping visits.
#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    uniffi::Enum,
    strum_macros::EnumString,
    strum_macros::Display,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum StatsVisitsUnit {
    #[default]
    Day,
    Hour,
    Week,
    Month,
    Year,
}

impl_as_query_value_from_to_string!(StatsVisitsUnit);

/// Parameters for the stats visits endpoint.
#[derive(Debug, Default, PartialEq, Eq, uniffi::Record)]
pub struct StatsVisitsParams {
    /// The time unit for grouping visits.
    #[uniffi(default = None)]
    pub unit: Option<StatsVisitsUnit>,
    /// The number of time units to return.
    #[uniffi(default = None)]
    pub quantity: Option<u32>,
    /// The end date to query stats for (format: YYYY-MM-DD).
    #[uniffi(default = None)]
    pub end_date: Option<String>,
    /// The locale for the response.
    #[uniffi(default = None)]
    pub locale: Option<String>,
}

impl AppendUrlQueryPairs for StatsVisitsParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut
            .append_option_query_value_pair("unit", self.unit.as_ref())
            .append_option_query_value_pair("quantity", self.quantity.as_ref())
            .append_option_query_value_pair("date", self.end_date.as_ref())
            .append_option_query_value_pair("locale", self.locale.as_ref());
    }
}

/// Response from the stats visits endpoint.
#[derive(Debug, Hash, Serialize, Deserialize, uniffi::Record)]
pub struct StatsVisitsResponse {
    /// The date for the stats query.
    pub date: String,
    /// The time unit used for grouping.
    pub unit: String,
    /// Field names for the data arrays.
    pub fields: Vec<String>,
    /// The stats data as arrays of values corresponding to fields.
    pub data: Vec<Vec<StatsVisitsDataValue>>,
}

#[uniffi::export]
impl StatsVisitsResponse {
    pub fn visits_data(&self) -> Vec<StatsVisitsDataPoint> {
        get_stats_data("views", self)
            .into_iter()
            .map(|(period, visits)| StatsVisitsDataPoint { period, visits })
            .collect()
    }

    pub fn visitors_data(&self) -> Vec<StatsVisitorsDataPoint> {
        get_stats_data("visitors", self)
            .into_iter()
            .map(|(period, visitors)| StatsVisitorsDataPoint { period, visitors })
            .collect()
    }

    pub fn likes_data(&self) -> Vec<StatsLikesDataPoint> {
        get_stats_data("likes", self)
            .into_iter()
            .map(|(period, likes)| StatsLikesDataPoint { period, likes })
            .collect()
    }

    pub fn reblogs_data(&self) -> Vec<StatsReblogsDataPoint> {
        get_stats_data("reblogs", self)
            .into_iter()
            .map(|(period, reblogs)| StatsReblogsDataPoint { period, reblogs })
            .collect()
    }

    pub fn comments_data(&self) -> Vec<StatsCommentsDataPoint> {
        get_stats_data("comments", self)
            .into_iter()
            .map(|(period, comments)| StatsCommentsDataPoint { period, comments })
            .collect()
    }

    pub fn posts_data(&self) -> Vec<StatsPostsDataPoint> {
        get_stats_data("posts", self)
            .into_iter()
            .map(|(period, posts)| StatsPostsDataPoint { period, posts })
            .collect()
    }
}

fn get_stats_data(handle: &str, response: &StatsVisitsResponse) -> Vec<(String, u64)> {
    let field_index = response
        .fields
        .iter()
        .position(|field| field == handle)
        .unwrap();

    response
        .data
        .iter()
        .filter_map(|row| {
            if let Some(period) = row[0].as_string()
                && let Some(value) = row[field_index].as_number()
            {
                return Some((period.clone(), value));
            }

            None
        })
        .collect()
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, uniffi::Record)]
pub struct StatsVisitsDataPoint {
    pub period: String,
    pub visits: u64,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, uniffi::Record)]
pub struct StatsVisitorsDataPoint {
    pub period: String,
    pub visitors: u64,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, uniffi::Record)]
pub struct StatsLikesDataPoint {
    pub period: String,
    pub likes: u64,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, uniffi::Record)]
pub struct StatsReblogsDataPoint {
    pub period: String,
    pub reblogs: u64,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, uniffi::Record)]
pub struct StatsCommentsDataPoint {
    pub period: String,
    pub comments: u64,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, uniffi::Record)]
pub struct StatsPostsDataPoint {
    pub period: String,
    pub posts: u64,
}

/// A value in the stats visits data array (can be string, number, or null).
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, uniffi::Enum)]
#[serde(untagged)]
pub enum StatsVisitsDataValue {
    String(String),
    Number(u64),
    Null,
}

impl StatsVisitsDataValue {
    pub fn as_string(&self) -> Option<&String> {
        match self {
            StatsVisitsDataValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<u64> {
        match self {
            StatsVisitsDataValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, StatsVisitsDataValue::Null)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[test]
    fn test_stats_visits_params_serialization() {
        let mut url =
            url::Url::parse("https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/visits")
                .expect("Failed to parse url");

        let params = StatsVisitsParams {
            unit: Some(StatsVisitsUnit::Hour),
            quantity: Some(24),
            end_date: Some("2025-01-15".to_string()),
            locale: Some("en".to_string()),
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/visits?unit=hour&quantity=24&date=2025-01-15&locale=en"
        );
    }

    #[test]
    fn test_stats_visits_params_serialization_partial() {
        let mut url =
            url::Url::parse("https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/visits")
                .expect("Failed to parse url");

        let params = StatsVisitsParams {
            unit: Some(StatsVisitsUnit::Day),
            quantity: Some(7),
            end_date: None,
            locale: None,
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/visits?unit=day&quantity=7"
        );
    }

    #[rstest]
    #[case("tests/wpcom/stats_visits/visits-01-hour-with-nulls.json")]
    #[case("tests/wpcom/stats_visits/visits-02-day.json")]
    fn test_stats_visits_response_deserialization(#[case] json_file_path: &str) {
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsVisitsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert!(!response.date.is_empty());
        assert!(!response.unit.is_empty());
        assert!(!response.fields.is_empty());
    }

    #[test]
    fn test_stats_visits_response_deserialization_hourly_with_nulls() {
        let json_file_path = "tests/wpcom/stats_visits/visits-01-hour-with-nulls.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsVisitsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.date, "2026-01-18 00:00:00");
        assert_eq!(response.unit, "hour");
        assert_eq!(
            response.fields,
            vec![
                "period", "views", "visitors", "likes", "reblogs", "comments", "posts"
            ]
        );
        assert_eq!(response.data.len(), 24);

        // Verify first data row
        let first_row = &response.data[0];
        assert_eq!(first_row.len(), 7);
        assert_eq!(
            first_row[0].as_string(),
            Some(&"2026-01-17 01:00:00".to_string())
        );
        assert_eq!(first_row[1].as_number(), Some(9));
        assert!(first_row[2].is_null());
    }

    #[test]
    fn test_get_stats_visits_data_hourly() {
        let json_file_path = "tests/wpcom/stats_visits/visits-01-hour-with-nulls.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsVisitsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        let data_points = response.visits_data();

        assert_eq!(data_points.len(), 24);
        assert_eq!(
            data_points[0],
            StatsVisitsDataPoint {
                period: "2026-01-17 01:00:00".to_string(),
                visits: 9,
            }
        );
        assert_eq!(
            data_points[21],
            StatsVisitsDataPoint {
                period: "2026-01-17 22:00:00".to_string(),
                visits: 27,
            }
        );
        assert_eq!(
            data_points[23],
            StatsVisitsDataPoint {
                period: "2026-01-18 00:00:00".to_string(),
                visits: 4,
            }
        );
    }

    #[test]
    fn test_get_stats_visits_data_daily() {
        let json_file_path = "tests/wpcom/stats_visits/visits-02-day.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsVisitsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        let data_points = response.visits_data();

        assert_eq!(data_points.len(), 30);
        assert_eq!(
            data_points[0],
            StatsVisitsDataPoint {
                period: "2025-12-21".to_string(),
                visits: 67,
            }
        );
        assert_eq!(
            data_points[20],
            StatsVisitsDataPoint {
                period: "2026-01-10".to_string(),
                visits: 57,
            }
        );
        assert_eq!(
            data_points[29],
            StatsVisitsDataPoint {
                period: "2026-01-19".to_string(),
                visits: 50,
            }
        );
    }

    #[test]
    fn test_get_stats_visits_data_empty_response() {
        let response = StatsVisitsResponse {
            date: "2026-01-19".to_string(),
            unit: "day".to_string(),
            fields: vec![
                "period".to_string(),
                "views".to_string(),
                "visitors".to_string(),
            ],
            data: vec![],
        };

        let data_points = response.visits_data();

        assert!(data_points.is_empty());
    }

    #[test]
    fn test_get_stats_visitors_data_with_nulls() {
        // visits-hourly-with-nulls.json has null visitors values
        let json_file_path = "tests/wpcom/stats_visits/visits-01-hour-with-nulls.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsVisitsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        let data_points = response.visitors_data();

        // All visitors are null, so no data points should be returned
        assert!(data_points.is_empty());
    }

    #[test]
    fn test_get_stats_visitors_data_daily() {
        let json_file_path = "tests/wpcom/stats_visits/visits-02-day.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsVisitsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        let data_points = response.visitors_data();

        assert_eq!(data_points.len(), 30);
        assert_eq!(
            data_points[0],
            StatsVisitorsDataPoint {
                period: "2025-12-21".to_string(),
                visitors: 60,
            }
        );
        assert_eq!(
            data_points[20],
            StatsVisitorsDataPoint {
                period: "2026-01-10".to_string(),
                visitors: 50,
            }
        );
        assert_eq!(
            data_points[29],
            StatsVisitorsDataPoint {
                period: "2026-01-19".to_string(),
                visitors: 47,
            }
        );
    }

    #[test]
    fn test_get_stats_likes_data_daily() {
        let json_file_path = "tests/wpcom/stats_visits/visits-02-day.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsVisitsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        let data_points = response.likes_data();

        assert_eq!(data_points.len(), 30);
        // Most likes are 0, but 2026-01-05 has 1 like
        assert_eq!(
            data_points[0],
            StatsLikesDataPoint {
                period: "2025-12-21".to_string(),
                likes: 0,
            }
        );
        assert_eq!(
            data_points[15],
            StatsLikesDataPoint {
                period: "2026-01-05".to_string(),
                likes: 1,
            }
        );
    }

    #[test]
    fn test_get_stats_reblogs_data_daily() {
        let json_file_path = "tests/wpcom/stats_visits/visits-02-day.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsVisitsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        let data_points = response.reblogs_data();

        assert_eq!(data_points.len(), 30);
        assert_eq!(
            data_points[0],
            StatsReblogsDataPoint {
                period: "2025-12-21".to_string(),
                reblogs: 0,
            }
        );
    }

    #[test]
    fn test_get_stats_comments_data_daily() {
        let json_file_path = "tests/wpcom/stats_visits/visits-02-day.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsVisitsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        let data_points = response.comments_data();

        assert_eq!(data_points.len(), 30);
        assert_eq!(
            data_points[0],
            StatsCommentsDataPoint {
                period: "2025-12-21".to_string(),
                comments: 0,
            }
        );
    }

    #[test]
    fn test_get_stats_posts_data_daily() {
        let json_file_path = "tests/wpcom/stats_visits/visits-02-day.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsVisitsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        let data_points = response.posts_data();

        assert_eq!(data_points.len(), 30);
        assert_eq!(
            data_points[0],
            StatsPostsDataPoint {
                period: "2025-12-21".to_string(),
                posts: 0,
            }
        );
    }
}

use crate::{
    date::WpDateString,
    impl_as_query_value_from_to_string,
    url_query::{AppendUrlQueryPairs, QueryPairs, QueryPairsExtension},
    wp_com::stats_visits::StatsVisitsDataValue,
};
use serde::{Deserialize, Serialize};

/// The time unit for grouping subscriber stats.
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
pub enum StatsSubscribersUnit {
    #[default]
    Day,
    Week,
    Month,
    Year,
}

impl_as_query_value_from_to_string!(StatsSubscribersUnit);

/// The stat fields to include in the subscriber stats response.
#[derive(
    Debug,
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
pub enum StatsSubscribersStatField {
    Subscribers,
    #[serde(rename = "subscribers_paid")]
    #[strum(serialize = "subscribers_paid")]
    SubscribersPaid,
}

impl_as_query_value_from_to_string!(StatsSubscribersStatField);

/// Parameters for the stats subscribers endpoint.
#[derive(Debug, Default, PartialEq, Eq, uniffi::Record)]
pub struct StatsSubscribersParams {
    /// The time unit for grouping stats.
    #[uniffi(default = None)]
    pub unit: Option<StatsSubscribersUnit>,
    /// The number of time units to return.
    #[uniffi(default = None)]
    pub quantity: Option<u32>,
    /// The date to query stats for (format: YYYY-MM-DD).
    #[uniffi(default = None)]
    pub date: Option<WpDateString>,
    /// The stat fields to include in the response (comma-separated in the URL).
    #[uniffi(default = [])]
    pub stat_fields: Vec<StatsSubscribersStatField>,
}

impl AppendUrlQueryPairs for StatsSubscribersParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut
            .append_option_query_value_pair("unit", self.unit.as_ref())
            .append_option_query_value_pair("quantity", self.quantity.as_ref())
            .append_option_query_value_pair("date", self.date.as_ref());

        if !self.stat_fields.is_empty() {
            let stat_fields_str = self
                .stat_fields
                .iter()
                .map(|f| f.to_string())
                .collect::<Vec<_>>()
                .join(",");
            query_pairs_mut.append_query_value_pair("stat_fields", &stat_fields_str);
        }
    }
}

/// Response from the stats subscribers endpoint.
#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct StatsSubscribersResponse {
    /// The date for the stats query.
    pub date: WpDateString,
    /// The time unit used for grouping.
    pub unit: String,
    /// Field names for the data arrays.
    pub fields: Vec<String>,
    /// The stats data as arrays of values corresponding to fields.
    #[serde(default)]
    pub data: Vec<Vec<StatsVisitsDataValue>>,
}

#[uniffi::export]
impl StatsSubscribersResponse {
    pub fn subscribers_data(&self) -> Vec<StatsSubscribersDataPoint> {
        get_stats_subscribers_data("subscribers", self)
            .into_iter()
            .map(|(period, subscribers)| StatsSubscribersDataPoint {
                period,
                subscribers,
            })
            .collect()
    }

    pub fn subscribers_paid_data(&self) -> Vec<StatsSubscribersPaidDataPoint> {
        get_stats_subscribers_data("subscribers_paid", self)
            .into_iter()
            .map(|(period, subscribers_paid)| StatsSubscribersPaidDataPoint {
                period,
                subscribers_paid,
            })
            .collect()
    }
}

fn get_stats_subscribers_data(
    handle: &str,
    response: &StatsSubscribersResponse,
) -> Vec<(String, u64)> {
    let period_index = match response.fields.iter().position(|f| f == "period") {
        Some(i) => i,
        None => return vec![],
    };

    let field_index = match response.fields.iter().position(|field| field == handle) {
        Some(index) => index,
        None => return vec![],
    };

    response
        .data
        .iter()
        .filter_map(|row| {
            if let Some(period) = row.get(period_index).and_then(|v| v.as_string())
                && let Some(value) = row.get(field_index).and_then(|v| v.as_number())
            {
                return Some((period.clone(), value));
            }

            None
        })
        .collect()
}

/// A subscriber count data point.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, uniffi::Record)]
pub struct StatsSubscribersDataPoint {
    /// The span this point covers, labelled to match the requested
    /// [`StatsSubscribersUnit`]: `"2026-01-27"` for a day, `"2026W02W23"` for
    /// a week, `"2025-11-01"` for a month, `"2024"` for a year.
    ///
    /// This is a label rather than a date — the week form isn't one at all —
    /// so it stays a `String` where a date would be a `WpDateString`.
    pub period: String,
    pub subscribers: u64,
}

/// A paid subscriber count data point.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, uniffi::Record)]
pub struct StatsSubscribersPaidDataPoint {
    /// Labelled as [`StatsSubscribersDataPoint::period`].
    pub period: String,
    pub subscribers_paid: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[test]
    fn test_stats_subscribers_params_serialization() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/subscribers",
        )
        .expect("Failed to parse url");

        let params = StatsSubscribersParams {
            unit: Some(StatsSubscribersUnit::Day),
            quantity: Some(30),
            date: Some(WpDateString("2026-02-26".to_string())),
            stat_fields: vec![
                StatsSubscribersStatField::Subscribers,
                StatsSubscribersStatField::SubscribersPaid,
            ],
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/subscribers?unit=day&quantity=30&date=2026-02-26&stat_fields=subscribers%2Csubscribers_paid"
        );
    }

    #[test]
    fn test_stats_subscribers_params_serialization_partial() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/subscribers",
        )
        .expect("Failed to parse url");

        let params = StatsSubscribersParams {
            unit: Some(StatsSubscribersUnit::Week),
            quantity: Some(12),
            date: None,
            stat_fields: vec![],
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/subscribers?unit=week&quantity=12"
        );
    }

    #[test]
    fn test_stats_subscribers_params_default() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/subscribers",
        )
        .expect("Failed to parse url");

        let params = StatsSubscribersParams::default();

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/subscribers?"
        );
    }

    #[rstest]
    #[case("tests/wpcom/stats_subscribers/response-01-day.json")]
    #[case("tests/wpcom/stats_subscribers/response-02-week.json")]
    #[case("tests/wpcom/stats_subscribers/response-03-month.json")]
    #[case("tests/wpcom/stats_subscribers/response-04-year.json")]
    #[case("tests/wpcom/stats_subscribers/response-05-empty.json")]
    fn test_stats_subscribers_response_deserialization(#[case] json_file_path: &str) {
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let _response: StatsSubscribersResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");
    }

    #[test]
    fn test_stats_subscribers_response_day() {
        let json_file_path = "tests/wpcom/stats_subscribers/response-01-day.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsSubscribersResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.date.0, "2026-01-27");
        assert_eq!(response.unit, "day");
        assert_eq!(
            response.fields,
            vec!["period", "subscribers", "subscribers_paid"]
        );
        assert_eq!(response.data.len(), 1);

        let subscribers = response.subscribers_data();
        assert_eq!(subscribers.len(), 1);
        assert_eq!(
            subscribers[0],
            StatsSubscribersDataPoint {
                period: "2026-01-27".to_string(),
                subscribers: 89,
            }
        );

        let paid = response.subscribers_paid_data();
        assert_eq!(paid.len(), 1);
        assert_eq!(
            paid[0],
            StatsSubscribersPaidDataPoint {
                period: "2026-01-27".to_string(),
                subscribers_paid: 0,
            }
        );
    }

    #[test]
    fn test_stats_subscribers_response_week() {
        let json_file_path = "tests/wpcom/stats_subscribers/response-02-week.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsSubscribersResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.date.0, "2026-02-26");
        assert_eq!(response.unit, "week");
        assert_eq!(response.data.len(), 12);

        let subscribers = response.subscribers_data();
        assert_eq!(subscribers.len(), 12);
        assert_eq!(
            subscribers[0],
            StatsSubscribersDataPoint {
                period: "2026W02W23".to_string(),
                subscribers: 89,
            }
        );
    }

    #[test]
    fn test_stats_subscribers_response_month() {
        let json_file_path = "tests/wpcom/stats_subscribers/response-03-month.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsSubscribersResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.date.0, "2026-02-26");
        assert_eq!(response.unit, "month");
        assert_eq!(response.data.len(), 6);

        let subscribers = response.subscribers_data();
        assert_eq!(subscribers.len(), 6);
        assert_eq!(
            subscribers[3],
            StatsSubscribersDataPoint {
                period: "2025-11-01".to_string(),
                subscribers: 90,
            }
        );
    }

    #[test]
    fn test_stats_subscribers_response_year() {
        let json_file_path = "tests/wpcom/stats_subscribers/response-04-year.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsSubscribersResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.date.0, "2026-02-26");
        assert_eq!(response.unit, "year");
        assert_eq!(response.data.len(), 3);

        let subscribers = response.subscribers_data();
        assert_eq!(subscribers.len(), 3);
        assert_eq!(
            subscribers[2],
            StatsSubscribersDataPoint {
                period: "2024".to_string(),
                subscribers: 114,
            }
        );
    }

    #[test]
    fn test_stats_subscribers_empty_response() {
        let json_file_path = "tests/wpcom/stats_subscribers/response-05-empty.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsSubscribersResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert!(response.data.is_empty());
        assert!(response.subscribers_data().is_empty());
        assert!(response.subscribers_paid_data().is_empty());
    }
}

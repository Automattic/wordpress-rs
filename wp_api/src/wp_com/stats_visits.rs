use crate::url_query::{AppendUrlQueryPairs, QueryPairs, QueryPairsExtension};
use serde::{Deserialize, Serialize};

/// Parameters for the stats visits endpoint.
#[derive(Debug, Default, PartialEq, Eq, uniffi::Record)]
pub struct StatsVisitsParams {
    /// The time unit for grouping visits (e.g., "hour", "day", "week", "month", "year").
    #[uniffi(default = None)]
    pub unit: Option<String>,
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
#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
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

/// A value in the stats visits data array (can be string, number, or null).
#[derive(Debug, Serialize, Deserialize, uniffi::Enum)]
#[serde(untagged)]
pub enum StatsVisitsDataValue {
    String(String),
    Number(i64),
    Null,
}

impl StatsVisitsDataValue {
    pub fn as_string(&self) -> Option<&String> {
        match self {
            StatsVisitsDataValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<i64> {
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

    #[test]
    fn test_stats_visits_params_serialization() {
        let mut url =
            url::Url::parse("https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/visits")
                .expect("Failed to parse url");

        let params = StatsVisitsParams {
            unit: Some("hour".to_string()),
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
            unit: Some("day".to_string()),
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

    #[test]
    fn test_stats_visits_response_deserialization() {
        let json_file_path = "tests/wpcom/stats_visits/visits-01.json";
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
}

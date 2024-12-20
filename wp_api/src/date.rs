use std::fmt::Display;
use std::str::FromStr;
use std::sync::Arc;

use chrono::DateTime;
use chrono::NaiveDateTime;
use chrono::TimeZone;
use chrono::Utc;
use serde::Deserializer;
use serde::Serializer;
use serde::{Deserialize, Serialize};

pub const REST_API_DATE_FORMAT: &str = "%Y-%m-%dT%H:%M:%S";

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct WpDateTime {
    gmt: Arc<GMTDateTime>,
}

impl WpDateTime {
    pub fn date_time(&self) -> &DateTime<Utc> {
        &self.gmt.date_time
    }
}

// This is an intermediate struct to allow the public `WpDateTime` to be exported as a
// uniffi Record type. This struct itself is exported as a uniffi Object type.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Object)]
struct GMTDateTime {
    date_time: DateTime<Utc>,
}

impl<'de> Deserialize<'de> for WpDateTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let str = String::deserialize(deserializer)?;
        str.parse::<WpDateTime>().map_err(serde::de::Error::custom)
    }
}

impl Serialize for WpDateTime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

impl From<DateTime<Utc>> for WpDateTime {
    fn from(value: DateTime<Utc>) -> Self {
        Self {
            gmt: GMTDateTime { date_time: value }.into(),
        }
    }
}

impl From<WpDateTime> for DateTime<Utc> {
    fn from(value: WpDateTime) -> Self {
        value.gmt.date_time
    }
}

impl FromStr for WpDateTime {
    type Err = chrono::ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        NaiveDateTime::parse_from_str(s, REST_API_DATE_FORMAT)
            .map(|v| Utc.from_utc_datetime(&v).into())
    }
}

impl Display for WpDateTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.gmt.date_time.format(REST_API_DATE_FORMAT))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize, Deserialize, Debug)]
    struct Dummy {
        value: WpDateTime,
    }

    #[test]
    fn test_string_round_trip() {
        let original = "2025-01-02T03:04:05";
        let value = original.parse::<WpDateTime>().unwrap();
        let str = value.to_string();
        assert_eq!(&str, original);
    }

    #[test]
    fn test_serialization_round_trip() {
        let original = r#""2025-01-02T03:04:05""#;
        let value = serde_json::from_str::<WpDateTime>(original).unwrap();
        let str = serde_json::to_string(&value).unwrap();
        assert_eq!(&str, original);
    }

    #[test]
    fn test_parse_error() {
        let result = "not-a-date".parse::<WpDateTime>();
        assert!(result.is_err());
    }
}

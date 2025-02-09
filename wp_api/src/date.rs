use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use wp_serde_helper::{wp_naive_date_format, wp_utc_date_format};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, uniffi::Object)]
#[serde(transparent)]
pub struct WpGmtDateTime {
    #[serde(with = "wp_utc_date_format")]
    pub inner: DateTime<Utc>,
}

impl WpGmtDateTime {
    pub fn to_rfc3339(&self) -> String {
        self.inner.to_rfc3339()
    }
}

impl From<DateTime<Utc>> for WpGmtDateTime {
    fn from(value: DateTime<Utc>) -> Self {
        Self { inner: value }
    }
}

impl From<std::time::SystemTime> for WpGmtDateTime {
    fn from(value: std::time::SystemTime) -> Self {
        Self {
            inner: value.into(),
        }
    }
}

impl FromStr for WpGmtDateTime {
    type Err = chrono::format::ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<DateTime<Utc>>().map(|inner| Self { inner })
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, uniffi::Object)]
#[serde(transparent)]
pub struct WpNaiveDateTime {
    #[serde(with = "wp_naive_date_format")]
    pub inner: NaiveDateTime,
}

impl FromStr for WpNaiveDateTime {
    type Err = chrono::format::ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<NaiveDateTime>().map(|inner| Self { inner })
    }
}

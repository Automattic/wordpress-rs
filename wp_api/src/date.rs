use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use wp_serde_helper::wp_naive_date_format;
use wp_serde_helper::wp_utc_date_format;

#[derive(Debug, Serialize, Deserialize, uniffi::Object)]
#[serde(transparent)]
pub struct WpDateTimeISO8601 {
    pub inner: DateTime<Utc>,
}

impl WpDateTimeISO8601 {
    pub fn to_rfc3339(&self) -> String {
        self.inner.to_rfc3339()
    }
}

impl From<DateTime<Utc>> for WpDateTimeISO8601 {
    fn from(value: DateTime<Utc>) -> Self {
        Self { inner: value }
    }
}

impl From<std::time::SystemTime> for WpDateTimeISO8601 {
    fn from(value: std::time::SystemTime) -> Self {
        Self {
            inner: value.into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, uniffi::Object)]
#[serde(transparent)]
pub struct WpGmtDateTime {
    #[serde(with = "wp_utc_date_format")]
    pub inner: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Object)]
#[serde(transparent)]
pub struct WpNaiveDateTime {
    #[serde(with = "wp_naive_date_format")]
    pub inner: NaiveDateTime,
}

use chrono::{format::ParseErrorKind, DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use wp_serde_helper::{wp_naive_date_format, wp_utc_date_format};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, uniffi::Object)]
#[serde(transparent)]
pub struct WpGmtDateTime {
    #[serde(with = "wp_utc_date_format")]
    pub inner: DateTime<Utc>,
}

#[uniffi::export]
impl WpGmtDateTime {
    #[uniffi::constructor]
    pub fn parse(date: &str) -> Result<Self, WpGmtDateTimeParseError> {
        date.parse::<WpGmtDateTime>()
    }

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
    type Err = WpGmtDateTimeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<DateTime<Utc>>()
            .map(|inner| Self { inner })
            .map_err(Into::<Self::Err>::into)
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

/// This enum is a direct copy of [`chrono::format::ParseErrorKind`](https://docs.rs/chrono/latest/chrono/format/enum.ParseErrorKind.html)
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, thiserror::Error, uniffi::Error)]
#[non_exhaustive]
pub enum WpGmtDateTimeParseError {
    /// Given field is out of permitted range
    #[error("Given field is out of permitted range")]
    OutOfRange,

    /// There is no possible date and time value with given set of fields.
    ///
    /// This does not include the out-of-range conditions, which are trivially invalid.
    /// It includes the case that there are one or more fields that are inconsistent to each other.
    #[error("There is no possible date and time value with given set of fields")]
    Impossible,

    /// Given set of fields is not enough to make a requested date and time value.
    ///
    /// Note that there *may* be a case that given fields constrain the possible values so much
    /// that there is a unique possible value. Chrono only tries to be correct for
    /// most useful sets of fields however, as such constraint solving can be expensive.
    #[error("Given set of fields is not enough to make a requested date and time value")]
    NotEnough,

    /// The input string has some invalid character sequence for given formatting items.
    #[error("The input string has some invalid character sequence for given formatting items")]
    Invalid,

    /// The input string has been prematurely ended.
    #[error("The input string has been prematurely ended")]
    TooShort,

    /// All formatting items have been read but there is a remaining input.
    #[error("All formatting items have been read but there is a remaining input")]
    TooLong,

    /// There was an error on the formatting string, or there were non-supported formatting items.
    #[error(
        "There was an error on the formatting string, or there were non-supported formatting items"
    )]
    BadFormat,
}

impl From<chrono::format::ParseError> for WpGmtDateTimeParseError {
    fn from(value: chrono::format::ParseError) -> Self {
        match value.kind() {
            ParseErrorKind::OutOfRange => Self::OutOfRange,
            ParseErrorKind::Impossible => Self::Impossible,
            ParseErrorKind::NotEnough => Self::NotEnough,
            ParseErrorKind::Invalid => Self::Invalid,
            ParseErrorKind::TooShort => Self::TooShort,
            ParseErrorKind::TooLong => Self::TooLong,
            ParseErrorKind::BadFormat => Self::BadFormat,
            _ => panic!(
                "A new error kind is added to `chrono::format::ParseErrorKind`, we need to handle that by adding it to `WpGmtDateTimeParseError`"
            ),
        }
    }
}

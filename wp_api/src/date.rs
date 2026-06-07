use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};
use wp_serde_helper::wp_utc_date_format;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct WpGmtDateTime(#[serde(with = "wp_utc_date_format")] pub DateTime<Utc>);

impl WpGmtDateTime {
    pub fn from_timestamp(seconds: i64) -> Self {
        let date_time =
            DateTime::<Utc>::from_timestamp(seconds, 0).unwrap_or(DateTime::<Utc>::UNIX_EPOCH);
        Self(date_time)
    }
}

impl FromStr for WpGmtDateTime {
    type Err = chrono::ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<DateTime<Utc>>().map(Self)
    }
}

impl Display for WpGmtDateTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.to_rfc3339())
    }
}

uniffi::custom_type!(WpGmtDateTime, i64, {
    lower: |date_time| date_time.0.timestamp(),
    try_lift: |seconds| Ok(WpGmtDateTime::from_timestamp(seconds)),
});

uniffi::custom_newtype!(WpDateString, String);
/// A date string in `"YYYY-MM-DD"` format as returned by some WordPress.com
/// API fields (e.g. domain expiry, registration date).
///
/// Some PHP endpoints return `false` instead of `null` when a date is not
/// applicable. Use [`deserialize_optional_date_string`] on fields that
/// exhibit this pattern.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WpDateString(pub String);

impl Display for WpDateString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Deserialize an `Option<WpDateString>` that may be a string, `null`, or
/// boolean `false` (a common PHP pattern for "not applicable").
pub fn deserialize_optional_date_string<'de, D>(
    deserializer: D,
) -> Result<Option<WpDateString>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum NullFalseOrString {
        Null,
        Bool(bool),
        String(String),
    }

    match NullFalseOrString::deserialize(deserializer)? {
        NullFalseOrString::Null | NullFalseOrString::Bool(false) => Ok(None),
        NullFalseOrString::Bool(true) => Err(serde::de::Error::custom(
            "expected a date string, `null`, or `false`, got `true`",
        )),
        NullFalseOrString::String(s) if s.to_lowercase().trim() == "false" => Ok(None),
        NullFalseOrString::String(s) => Ok(Some(WpDateString(s))),
    }
}

// Assertion functions that should only be used by the native test suite
// These are hidden from the Rust public API, but will be visible/usable in the generated bindings
mod native_test_helper {
    use super::WpGmtDateTime;
    use chrono::offset::FixedOffset;

    const OFFSET_FOR_EXAMPLE_DATE: i32 = 60 * 60 * 2;
    const EXAMPLE_DATE: &str = "2020-08-14T15:00:00+02:00";

    #[uniffi::export]
    fn assertion_example_date_that_can_be_used_to_verify_conversion_between_rust_and_native()
    -> WpGmtDateTime {
        EXAMPLE_DATE
            .parse::<WpGmtDateTime>()
            .expect("Example date is parseable")
    }

    #[uniffi::export]
    fn assert_date_is_converted_from_native_to_rust_correctly(date: WpGmtDateTime) {
        assert_eq!(
            date.0
                .with_timezone(
                    &FixedOffset::east_opt(OFFSET_FOR_EXAMPLE_DATE)
                        .expect("Example offset is valid")
                )
                .to_rfc3339(),
            EXAMPLE_DATE
        );
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_that_helper_functions_work_correctly() {
            let date = assertion_example_date_that_can_be_used_to_verify_conversion_between_rust_and_native();
            assert_date_is_converted_from_native_to_rust_correctly(date);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case::unix_epoch_plus_one_day(60 * 60 * 24, "1970-01-02T00:00:00+00:00")]
    #[case::unix_epoch_plus_one_month(60 * 60 * 24 * 31, "1970-02-01T00:00:00+00:00")]
    #[case::unix_epoch_plus_one_year(60 * 60 * 24 * 365, "1971-01-01T00:00:00+00:00")]
    #[case::year_3000(32503680000, "3000-01-01T00:00:00+00:00")]
    fn test_gmt_date_time_from_time_stamp(#[case] seconds: i64, #[case] expected_date_str: &str) {
        assert_eq!(
            WpGmtDateTime::from_timestamp(seconds).0.to_rfc3339(),
            expected_date_str
        );
    }
}

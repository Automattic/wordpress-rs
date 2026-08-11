use crate::impl_as_query_value_from_to_string;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};
use wp_serde_helper::{parse_wp_date_time, wp_utc_date_format};

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
        parse_wp_date_time(s).map(Self)
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
/// A date the API sends as a string that can't be resolved to an instant —
/// either because it carries no time (`"2026-08-06"`, e.g. domain expiry) or
/// because its time is in the site's timezone rather than GMT
/// (`"2026-08-06 09:15:49"`).
///
/// Use [`WpGmtDateTime`] instead wherever the API gives a GMT or offset-bearing
/// value; every date field should be one or the other rather than a bare
/// `String`.
///
/// Some PHP endpoints return `false` instead of `null` when a date is not
/// applicable. Use [`deserialize_optional_date_string`] on fields that
/// exhibit this pattern.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WpDateString(pub String);

impl Display for WpDateString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl_as_query_value_from_to_string!(WpDateString);

/// Deserialize an `Option<WpDateString>` that may be a string, `null`, or
/// boolean `false` (a common PHP pattern for "not applicable").
pub fn deserialize_optional_date_string<'de, D>(
    deserializer: D,
) -> Result<Option<WpDateString>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    wp_serde_helper::deserialize_false_or_string_or_null(deserializer)
        .map(|opt| opt.map(WpDateString))
}

/// Deserialize an `Option<WpGmtDateTime>` for a field that may be unset.
///
/// `null`, an absent field, and an empty string read as `None`. A populated
/// value accepts every form [`WpGmtDateTime`] does; anything else is an error.
pub fn deserialize_optional_wp_gmt_date_time<'de, D>(
    deserializer: D,
) -> Result<Option<WpGmtDateTime>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    match Option::<String>::deserialize(deserializer)? {
        Some(s) if !s.trim().is_empty() => WpGmtDateTime::from_str(&s)
            .map(Some)
            .map_err(serde::de::Error::custom),
        _ => Ok(None),
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

    #[rstest]
    #[case::offset("2026-08-06T09:15:49+00:00")]
    #[case::offsetless("2026-08-06T09:15:49")]
    #[case::mysql("2026-08-06 09:15:49")]
    #[case::sub_second("2026-08-06T09:15:49.000000")]
    #[case::unix_timestamp("1786007749")]
    fn test_gmt_date_time_from_str_accepts_every_form_serde_does(#[case] value: &str) {
        let parsed = value
            .parse::<WpGmtDateTime>()
            .expect("Every form the serde path accepts should parse");
        assert_eq!(parsed.0.to_rfc3339(), "2026-08-06T09:15:49+00:00");

        let via_serde: WpGmtDateTime = serde_json::from_value(serde_json::json!(value))
            .expect("The serde path should accept it too");
        assert_eq!(via_serde, parsed);
    }

    #[derive(serde::Deserialize)]
    struct OptionalGmtDateTime {
        #[serde(default, deserialize_with = "deserialize_optional_wp_gmt_date_time")]
        value: Option<WpGmtDateTime>,
    }

    #[rstest]
    #[case::offset(r#"{"value": "2026-08-06T09:15:49+00:00"}"#)]
    #[case::offsetless(r#"{"value": "2026-08-06T09:15:49"}"#)]
    #[case::mysql(r#"{"value": "2026-08-06 09:15:49"}"#)]
    fn test_deserialize_optional_wp_gmt_date_time(#[case] json: &str) {
        let parsed: OptionalGmtDateTime =
            serde_json::from_str(json).expect("Test case should be a valid JSON");
        assert_eq!(
            parsed.value.expect("present").0.to_rfc3339(),
            "2026-08-06T09:15:49+00:00"
        );
    }

    #[rstest]
    #[case::empty_string(r#"{"value": ""}"#)]
    #[case::null(r#"{"value": null}"#)]
    #[case::absent(r#"{}"#)]
    fn test_deserialize_optional_wp_gmt_date_time_absent(#[case] json: &str) {
        let parsed: OptionalGmtDateTime =
            serde_json::from_str(json).expect("Test case should be a valid JSON");
        assert!(parsed.value.is_none());
    }
}

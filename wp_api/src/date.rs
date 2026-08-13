use crate::impl_as_query_value_from_to_string;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};
use wp_serde_helper::{WpDateTimeParseError, parse_wp_date_time, wp_utc_date_format};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct WpGmtDateTime(#[serde(with = "wp_utc_date_format")] pub DateTime<Utc>);

impl WpGmtDateTime {
    /// Build an instant from a timestamp that did not come from WordPress, and
    /// so is not held to WordPress's rules — an X.509 certificate's validity
    /// bounds, for instance. Falls back to the unix epoch if the value is out
    /// of range.
    ///
    /// A timestamp arriving *from* WordPress or across the bindings goes
    /// through [`wp_serde_helper::wp_date_time_from_timestamp`], which rejects
    /// the never-set date rather than resolving it.
    pub(crate) fn from_unchecked_timestamp(seconds: i64) -> Self {
        let date_time =
            DateTime::<Utc>::from_timestamp(seconds, 0).unwrap_or(DateTime::<Utc>::UNIX_EPOCH);
        Self(date_time)
    }
}

impl FromStr for WpGmtDateTime {
    type Err = WpDateTimeParseError;

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
    try_lift: |seconds| Ok(wp_serde_helper::wp_date_time_from_timestamp(seconds).map(WpGmtDateTime)?),
});

/// A date the API sends as a string that can't be resolved to an instant —
/// either because it carries no time (`"2026-08-06"`, e.g. domain expiry) or
/// because its time is in the site's timezone rather than GMT
/// (`"2026-08-06 09:15:49"`).
///
/// Use [`WpGmtDateTime`] instead wherever the API gives a GMT or
/// offset-bearing value.
///
/// Some PHP endpoints return `false` instead of `null` when a date is not
/// applicable. Use [`deserialize_optional_date_string`] on fields that
/// exhibit this pattern.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
#[serde(transparent)]
pub struct WpDateString {
    pub value: String,
}

impl WpDateString {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
        }
    }
}

impl Display for WpDateString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
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
        .map(|opt| opt.map(WpDateString::new))
}

/// Deserialize an `Option<WpGmtDateTime>` for a field that may be unset.
///
/// `null`, an empty string, and WordPress's never-set date read as `None`, as
/// does an absent field on a member carrying `#[serde(default)]`. A populated
/// value accepts every form [`WpGmtDateTime`] does, including a bare timestamp
/// sent as a JSON number; anything else is an error.
pub fn deserialize_optional_wp_gmt_date_time<'de, D>(
    deserializer: D,
) -> Result<Option<WpGmtDateTime>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    /// The two shapes a populated value arrives in, matching what the
    /// non-optional path accepts.
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Populated {
        String(String),
        Timestamp(i64),
    }

    let parsed = match Option::<Populated>::deserialize(deserializer)? {
        Some(Populated::String(s)) if !s.trim().is_empty() => {
            WpGmtDateTime::from_str(&s).map_err(|e| (e, s))
        }
        Some(Populated::Timestamp(seconds)) => {
            wp_serde_helper::wp_date_time_from_timestamp(seconds)
                .map(WpGmtDateTime)
                .map_err(|e| (e, seconds.to_string()))
        }
        _ => return Ok(None),
    };

    match parsed {
        Ok(date_time) => Ok(Some(date_time)),
        Err((WpDateTimeParseError::NotSet, _)) => Ok(None),
        Err((e, value)) => Err(serde::de::Error::custom(format!("{e}: {value}"))),
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
            WpGmtDateTime::from_unchecked_timestamp(seconds)
                .0
                .to_rfc3339(),
            expected_date_str
        );
    }

    /// The bindings lift a `WpGmtDateTime` from a timestamp, so that lift is a
    /// parse boundary and holds to the same rules as every other read.
    #[rstest]
    #[case::never_set(-62169984000)]
    #[case::before_year_one(-62200000000)]
    fn test_lifting_an_impossible_timestamp_is_rejected(#[case] seconds: i64) {
        assert!(wp_serde_helper::wp_date_time_from_timestamp(seconds).is_err());
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

    /// The optional path accepts the same set as the non-optional one. The
    /// timestamp cases are the ones that diverged: it read the value as a
    /// string, so a bare JSON number errored where `WpGmtDateTime`'s own
    /// `Deserialize` accepted it.
    #[rstest]
    #[case::timestamp_as_number(r#"{"value": 1786007749}"#)]
    #[case::timestamp_as_string(r#"{"value": "1786007749"}"#)]
    #[case::sub_second(r#"{"value": "2026-08-06T09:15:49.000000"}"#)]
    fn test_deserialize_optional_wp_gmt_date_time_matches_the_required_path(#[case] json: &str) {
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

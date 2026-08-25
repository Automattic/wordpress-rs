use chrono::{DateTime, Datelike, NaiveDateTime, Utc};
use std::fmt::Display;

// https://core.trac.wordpress.org/ticket/41032
const WP_DATE_FORMAT: &str = "%Y-%m-%dT%H:%M:%S";
const MYSQL_DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

/// The spellings in which the zero date reaches a client without parsing.
///
/// A spelling that does parse is caught by its instant instead, against
/// [`ZERO_DATE_TIMESTAMP`]. These two never will: `0000-00-00 00:00:00` has a
/// month and day out of range, and the three-digit-year form isn't valid
/// RFC 3339.
const ZERO_DATE_SPELLINGS: [&str; 3] = ["0000-00-00", "-001-11-30", "-0001-11-30"];

/// The instant PHP derives from the zero date, as a unix timestamp.
const ZERO_DATE_TIMESTAMP: i64 = -62_169_984_000;

/// How far a timezone conversion can move that instant. Offsets run from UTC-12
/// to UTC+14, so anything landing within a day of it is the zero date rather
/// than a date somebody meant.
const ZERO_DATE_TOLERANCE_SECONDS: i64 = 24 * 60 * 60;

/// Why a value could not be read as a WordPress datetime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WpDateTimeParseError {
    /// The value is MySQL's zero date, `0000-00-00 00:00:00`, which WordPress
    /// stores to mean no datetime was ever written here. It is the schema
    /// default for every datetime column, and core writes it deliberately —
    /// a draft whose publish date should float until the post is published
    /// gets exactly this — so it is a legal value rather than corruption.
    ///
    /// It seldom arrives as that literal. An endpoint that doesn't guard the
    /// column hands it to PHP's formatter, whose lenient parser rolls the zero
    /// month and day back into 30 November of 1 BCE, written with three or
    /// four year digits depending on the format used.
    ///
    /// A field that can legitimately be unset is an `Option` whose
    /// deserializer reads this as `None`. Every other read path treats it as
    /// an error, because the only alternative is that 1 BCE instant, which is
    /// indistinguishable from real data once parsed.
    ///
    /// The trade-off that choice carries: on a field that is *not* optional,
    /// serde aborts the whole document, so one row with a zero date fails an
    /// entire list response rather than losing one field. Whether a given
    /// field needs to be optional depends on the endpoint — some guard the
    /// column and send `null`, some format it unguarded and send this. If a
    /// response starts failing to parse and the message names this error, the
    /// fix is to make that field optional, not to loosen the parser.
    NotSet,
    /// The value matches none of the forms WordPress sends, or resolves to an
    /// instant before year 1, which no WordPress datetime legitimately has.
    Invalid,
}

impl Display for WpDateTimeParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotSet => write!(f, "Date is WordPress's zero date"),
            Self::Invalid => write!(f, "Invalid date format"),
        }
    }
}

impl std::error::Error for WpDateTimeParseError {}

/// Parse a datetime in any of the forms WordPress and WordPress.com send it:
/// with a timezone offset (`2026-08-06T09:15:49+00:00`), the offsetless
/// WordPress form (`2026-08-06T09:15:49`), that form with sub-second
/// precision, MySQL's (`2026-08-06 09:15:49`), and a unix timestamp.
///
/// The offsetless forms are read as UTC, so only pass values already known to
/// be GMT.
///
/// # Errors
///
/// Returns [`WpDateTimeParseError::NotSet`] for WordPress's zero date,
/// and [`WpDateTimeParseError::Invalid`] if the value matches none of the
/// forms above.
pub fn parse_wp_date_time(s: &str) -> Result<DateTime<Utc>, WpDateTimeParseError> {
    if ZERO_DATE_SPELLINGS
        .iter()
        .any(|spelling| s.starts_with(spelling))
    {
        return Err(WpDateTimeParseError::NotSet);
    }

    parse_known_format(s)
        .ok_or(WpDateTimeParseError::Invalid)
        .and_then(reject_instant_no_wp_date_has)
}

/// Read a unix timestamp as a WordPress datetime, holding it to the same rules
/// as a string value.
///
/// # Errors
///
/// As [`parse_wp_date_time`].
pub fn wp_date_time_from_timestamp(seconds: i64) -> Result<DateTime<Utc>, WpDateTimeParseError> {
    DateTime::<Utc>::from_timestamp(seconds, 0)
        .ok_or(WpDateTimeParseError::Invalid)
        .and_then(reject_instant_no_wp_date_has)
}

fn parse_known_format(s: &str) -> Option<DateTime<Utc>> {
    let from_naive = |dt| Some(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc));

    // WP.org REST API Format
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, WP_DATE_FORMAT) {
        return from_naive(dt);
    }

    // ISO-8601
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }

    // Unix Timestamp (wrapped in a string)
    if let Ok(timestamp) = s.parse::<i64>() {
        return DateTime::<Utc>::from_timestamp(timestamp, 0);
    }

    // MySQL format
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, MYSQL_DATE_FORMAT) {
        return from_naive(dt);
    }

    // WP format with sub-second precision
    NaiveDateTime::parse_from_str(s, &format!("{WP_DATE_FORMAT}.%f"))
        .ok()
        .and_then(from_naive)
}

/// Reject an instant that parsed cleanly but that no WordPress datetime can
/// hold, so it never reaches a caller looking like real data.
///
/// The zero date is matched on a window rather than the exact instant, because
/// an endpoint that converts it out of the site's timezone before formatting
/// shifts it by that offset — far enough to land on a neighbouring day.
fn reject_instant_no_wp_date_has(
    date_time: DateTime<Utc>,
) -> Result<DateTime<Utc>, WpDateTimeParseError> {
    if (date_time.timestamp() - ZERO_DATE_TIMESTAMP).abs() <= ZERO_DATE_TOLERANCE_SECONDS {
        return Err(WpDateTimeParseError::NotSet);
    }

    if date_time.year() < 1 {
        return Err(WpDateTimeParseError::Invalid);
    }

    Ok(date_time)
}

pub mod wp_utc_date_format {
    use super::{WP_DATE_FORMAT, parse_wp_date_time, wp_date_time_from_timestamp};
    use chrono::{DateTime, Utc};
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{}", date.format(WP_DATE_FORMAT)))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        match DateRepresentation::deserialize(deserializer)? {
            DateRepresentation::Int(timestamp) => wp_date_time_from_timestamp(timestamp)
                .map_err(|e| serde::de::Error::custom(format!("{e}: {timestamp}"))),
            DateRepresentation::String(s) => {
                parse_wp_date_time(&s).map_err(|e| serde::de::Error::custom(format!("{e}: {s}")))
            }
        }
    }

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum DateRepresentation {
        String(String),
        Int(i64),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Datelike, Timelike, Utc};
    use rstest::rstest;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug)]
    struct Foo {
        #[serde(with = "wp_utc_date_format")]
        pub wp_utc_date_time: DateTime<Utc>,
    }

    #[rstest]
    #[case(r#""2023-05-12T14:30:45""#)]
    #[case(r#""2023-05-12T14:30:45+00:00""#)]
    #[case(r#""2023-05-12T14:30:45.126769""#)]
    #[case(r#""1683901845""#)]
    #[case("1683901845")]
    fn test_deserialize_date(#[case] date_string: &str) {
        let json_str = format!("{{\"wp_utc_date_time\": {date_string}}}");
        let foo: Foo = serde_json::from_str(&json_str).expect("Failed to deserialize JSON");

        assert_eq!(foo.wp_utc_date_time.year_ce(), (true, 2023));
        assert_eq!(foo.wp_utc_date_time.month(), 5);
        assert_eq!(foo.wp_utc_date_time.day(), 12);
        assert_eq!(foo.wp_utc_date_time.hour(), 14);
        assert_eq!(foo.wp_utc_date_time.minute(), 30);
        assert_eq!(foo.wp_utc_date_time.second(), 45);

        if foo.wp_utc_date_time.nanosecond() != 0 {
            assert_eq!(foo.wp_utc_date_time.nanosecond(), 126769);
        }
    }

    #[rstest]
    #[case(r#""invalid""#)] // invalid date string
    #[case("42.78")] // invalid timestamp
    fn test_invalid_date(#[case] date_string: &str) {
        let json_str = format!("{{\"wp_utc_date_time\": {date_string}}}");
        assert!(
            serde_json::from_str::<Foo>(&json_str).is_err(),
            "Expected error for invalid date"
        );
    }

    /// WordPress's zero date is recognised as such in each spelling it
    /// arrives in, rather than read as a datetime.
    ///
    /// The offsetless and MySQL-shaped spellings used to parse cleanly and
    /// return 30 November of 1 BCE as a real instant; the offset-bearing ones
    /// were rejected. One value, two behaviours, decided by which format the
    /// endpoint used.
    #[rstest]
    #[case::zero_date_column("0000-00-00 00:00:00")]
    #[case::zero_date_column_iso("0000-00-00T00:00:00")]
    #[case::zero_date_column_with_offset("0000-00-00T00:00:00+00:00")]
    #[case::three_digit_year_with_offset("-001-11-30T00:00:00+00:00")]
    #[case::four_digit_year_with_offset("-0001-11-30T00:00:00+00:00")]
    #[case::three_digit_year("-001-11-30T00:00:00")]
    #[case::four_digit_year("-0001-11-30T00:00:00")]
    #[case::three_digit_year_mysql("-001-11-30 00:00:00")]
    #[case::four_digit_year_mysql("-0001-11-30 00:00:00")]
    #[case::unix_timestamp("-62169984000")]
    // An endpoint that converts the zero date out of the site's timezone before
    // formatting shifts the instant, far enough to land on a neighbouring day.
    // `Europe/Berlin` to UTC gives the first of these.
    #[case::shifted_west("-0001-11-29 23:06:32")]
    #[case::shifted_furthest_west("-0001-11-29 10:00:00")]
    #[case::shifted_furthest_east("-0001-11-30 14:00:00")]
    fn test_zero_date_is_recognised(#[case] value: &str) {
        assert_eq!(
            parse_wp_date_time(value),
            Err(WpDateTimeParseError::NotSet),
            "{value} is WordPress's zero date"
        );
    }

    /// An instant before year 1 that isn't the zero date is malformed,
    /// not absent — the two are different failures and stay distinguishable.
    #[rstest]
    #[case::bce("-0500-01-01T00:00:00")]
    #[case::year_zero("0000-01-01T00:00:00")]
    fn test_instant_before_year_one_is_invalid(#[case] value: &str) {
        assert_eq!(
            parse_wp_date_time(value),
            Err(WpDateTimeParseError::Invalid),
            "no WordPress date is before year 1"
        );
    }

    /// A field that isn't an `Option` has no way to say "absent", so the
    /// zero date has to fail rather than resolve to an instant.
    #[rstest]
    #[case::string(r#""0000-00-00 00:00:00""#)]
    #[case::offsetless_string(r#""-0001-11-30T00:00:00""#)]
    #[case::timestamp("-62169984000")]
    fn test_never_set_date_fails_a_required_field(#[case] date_string: &str) {
        let json_str = format!("{{\"wp_utc_date_time\": {date_string}}}");
        assert!(
            serde_json::from_str::<Foo>(&json_str).is_err(),
            "Expected error for WordPress's zero date"
        );
    }
}

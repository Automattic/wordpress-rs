// https://core.trac.wordpress.org/ticket/41032
const WP_DATE_FORMAT: &str = "%Y-%m-%dT%H:%M:%S";
const MYSQL_DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

pub mod wp_utc_date_format {
    use super::{MYSQL_DATE_FORMAT, WP_DATE_FORMAT};
    use chrono::{DateTime, NaiveDateTime, Utc};
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
            DateRepresentation::Int(timestamp) => {
                if let Some(dt) = DateTime::<Utc>::from_timestamp(timestamp, 0) {
                    return Ok(dt);
                }

                Err(serde::de::Error::custom(format!(
                    "Invalid date : {timestamp}"
                )))
            }
            DateRepresentation::String(s) => {
                // WP.org REST API Format
                if let Ok(dt) = NaiveDateTime::parse_from_str(&s, WP_DATE_FORMAT) {
                    return Ok(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc));
                }

                // ISO-8601
                if let Ok(dt) = DateTime::parse_from_rfc3339(&s) {
                    return Ok(dt.with_timezone(&Utc));
                }

                // Unix Timestamp (wrapped in a string)
                if let Ok(timestamp) = s.parse::<i64>()
                    && let Some(dt) = DateTime::<Utc>::from_timestamp(timestamp, 0) {
                        return Ok(dt);
                    }

                // MySQL format
                if let Ok(dt) = NaiveDateTime::parse_from_str(&s, MYSQL_DATE_FORMAT) {
                    return Ok(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc));
                }

                // WP format with sub-second precision
                if let Ok(dt) = NaiveDateTime::parse_from_str(&s, &format!("{WP_DATE_FORMAT}.%f")) {
                    return Ok(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc));
                }

                Err(serde::de::Error::custom(format!(
                    "Invalid date format: {s}"
                )))
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
}

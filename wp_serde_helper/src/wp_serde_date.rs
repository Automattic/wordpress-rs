// https://core.trac.wordpress.org/ticket/41032
const WP_DATE_FORMAT: &str = "%Y-%m-%dT%H:%M:%S";

pub mod wp_utc_date_format {
    use super::WP_DATE_FORMAT;
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
        let s = String::deserialize(deserializer)?;
        NaiveDateTime::parse_from_str(&s, WP_DATE_FORMAT)
            .map_err(serde::de::Error::custom)
            .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Datelike, Timelike, Utc};
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug)]
    struct Foo {
        #[serde(with = "wp_utc_date_format")]
        pub wp_utc_date_time: DateTime<Utc>,
    }

    #[test]
    fn test_deserialize_date() {
        let json_str = r#"
          {
            "wp_utc_date_time": "2024-09-18T22:37:19"
          }
        "#;

        let foo: Foo = serde_json::from_str(json_str).unwrap();

        assert_eq!(foo.wp_utc_date_time.year_ce(), (true, 2024));
        assert_eq!(foo.wp_utc_date_time.month(), 9);
        assert_eq!(foo.wp_utc_date_time.day(), 18);
        assert_eq!(foo.wp_utc_date_time.minute(), 37);
        assert_eq!(foo.wp_utc_date_time.second(), 19);
        assert_eq!(foo.wp_utc_date_time.hour(), 22);
    }
}

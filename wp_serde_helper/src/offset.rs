use serde::{Deserializer, de};
use std::fmt;

/// Deserialize a timezone offset as an optional `f64`.
///
/// Accepts:
/// - Float values (e.g., `5.5` for UTC+5:30)
/// - Integer values (converted to float)
/// - String representations of numbers
///
/// Returns `None` if the field is missing (requires `#[serde(default)]`).
///
/// # Errors
///
/// Returns an error for non-numeric strings, booleans, null, arrays, or objects.
pub fn deserialize_offset<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeOffsetVisitor)
}

struct DeserializeOffsetVisitor;

impl de::Visitor<'_> for DeserializeOffsetVisitor {
    type Value = Option<f64>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("f64 or i64 or a string")
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Some(v))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Some(v as f64))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Some(v as f64))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        v.parse::<f64>().map_err(E::custom).map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    pub struct Offset {
        #[serde(default, deserialize_with = "deserialize_offset")]
        pub offset: Option<f64>,
    }

    #[rstest]
    #[case(r#"{"offset": "1"}"#, Some(1.0))]
    #[case(r#"{"offset": 1}"#, Some(1.0))]
    #[case(r#"{"offset": -1}"#, Some(-1.0))]
    #[case(r#"{"offset": 5.5}"#, Some(5.5))]
    #[case(r#"{"offset": "5.5"}"#, Some(5.5))]
    #[case("{}", None)]
    fn test_deserialize_offset(#[case] test_case: &str, #[case] expected_result: Option<f64>) {
        let offset: Offset =
            serde_json::from_str(test_case).expect("Test case should be a valid JSON");
        assert_eq!(expected_result, offset.offset);
    }
}

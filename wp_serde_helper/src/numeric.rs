use serde::{
    Deserializer,
    de::{self, Unexpected},
};
use std::fmt;

struct DeserializeI64OrStringVisitor;

impl de::Visitor<'_> for DeserializeI64OrStringVisitor {
    type Value = i64;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("i64 or a string")
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        i64::try_from(v).map_err(|_| E::invalid_value(Unexpected::Unsigned(v), &self))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(v)
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        v.parse::<i64>()
            .map_err(|_| E::invalid_value(Unexpected::Str(v), &self))
    }
}

pub fn deserialize_i64_or_string<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeI64OrStringVisitor)
}

pub fn deserialize_u64_or_string<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeU64OrStringVisitor)
}

pub fn deserialize_i64_or_string_as_t<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: From<i64>,
{
    deserialize_i64_or_string(deserializer).map(Into::into)
}

struct DeserializeU64OrStringVisitor;

impl de::Visitor<'_> for DeserializeU64OrStringVisitor {
    type Value = u64;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("u64 or a string")
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(v)
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        v.parse::<u64>()
            .map_err(|_| E::invalid_value(Unexpected::Str(v), &self))
    }
}

pub struct DeserializeOffsetVisitor;

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

pub fn deserialize_offset<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeOffsetVisitor)
}

pub fn deserialize_u64_or_none<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeU64OrNoneVisitor)
}

struct DeserializeU64OrNoneVisitor;

impl de::Visitor<'_> for DeserializeU64OrNoneVisitor {
    type Value = Option<u64>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("u64, false, or null")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if !v {
            Ok(None)
        } else {
            Err(E::invalid_value(Unexpected::Bool(v), &self))
        }
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Some(v))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(None)
    }
}

pub fn deserialize_u64_or_none_with_negative_as_none<'de, D>(
    deserializer: D,
) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeU64OrNoneWithNegativeAsNoneVisitor)
}

struct DeserializeU64OrNoneWithNegativeAsNoneVisitor;

impl de::Visitor<'_> for DeserializeU64OrNoneWithNegativeAsNoneVisitor {
    type Value = Option<u64>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("u64, false, null, or negative number")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if !v {
            Ok(None)
        } else {
            Err(E::invalid_value(Unexpected::Bool(v), &self))
        }
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Some(v))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v < 0 { Ok(None) } else { Ok(Some(v as u64)) }
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(None)
    }
}

pub fn deserialize_u64_or_none_with_zero_as_none<'de, D>(
    deserializer: D,
) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeU64OrNoneWithZeroAsNoneVisitor)
}

struct DeserializeU64OrNoneWithZeroAsNoneVisitor;

impl de::Visitor<'_> for DeserializeU64OrNoneWithZeroAsNoneVisitor {
    type Value = Option<u64>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("u64, false, or null")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if !v {
            Ok(None)
        } else {
            Err(E::invalid_value(Unexpected::Bool(v), &self))
        }
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v == 0 { Ok(None) } else { Ok(Some(v)) }
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(None)
    }
}

pub fn deserialize_bool_or_int<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeBoolOrIntVisitor)
}

struct DeserializeBoolOrIntVisitor;

impl de::Visitor<'_> for DeserializeBoolOrIntVisitor {
    type Value = bool;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("bool or integer (0/1)")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(v)
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match v {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(E::invalid_value(Unexpected::Signed(v), &self)),
        }
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match v {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(E::invalid_value(Unexpected::Unsigned(v), &self)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    pub struct Foo {
        #[serde(deserialize_with = "deserialize_i64_or_string")]
        pub bar: i64,
    }

    #[rstest]
    #[case(r#"{"bar": "1"}"#, 1)]
    #[case(r#"{"bar": 1}"#, 1)]
    #[case(r#"{"bar": -1}"#, -1)]
    fn test_deserialize_i64_or_string(#[case] test_case: &str, #[case] expected_result: i64) {
        let foo: Foo = serde_json::from_str(test_case).expect("Test case should be a valid JSON");
        assert_eq!(expected_result, foo.bar);
    }

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

    #[derive(Debug, Deserialize)]
    pub struct U64OrNone {
        #[serde(deserialize_with = "deserialize_u64_or_none")]
        pub u64: Option<u64>,
    }

    #[rstest]
    #[case(r#"{"u64": 1}"#, Some(1))]
    #[case(r#"{"u64": false}"#, None)]
    #[case(r#"{"u64": null}"#, None)]
    fn test_deserialize_u64_or_none(#[case] test_case: &str, #[case] expected_result: Option<u64>) {
        let u64_or_none: U64OrNone =
            serde_json::from_str(test_case).expect("Test case should be a valid JSON");
        assert_eq!(expected_result, u64_or_none.u64);
    }

    #[rstest]
    #[case(
        r#"{"u64": -1}"#,
        r#"invalid type: integer `-1`, expected u64, false, or null at line 1 column 10"#
    )]
    #[case(
        r#"{"u64": "1"}"#,
        r#"invalid type: string "1", expected u64, false, or null at line 1 column 11"#
    )]
    #[case(
        r#"{"u64": true}"#,
        r#"invalid value: boolean `true`, expected u64, false, or null at line 1 column 12"#
    )]
    fn test_deserialize_u64_or_none_errors(
        #[case] test_case: &str,
        #[case] expected_error_message: &str,
    ) {
        let u64_or_none: Result<U64OrNone, serde_json::Error> = serde_json::from_str(test_case);
        assert!(u64_or_none.is_err(), "The serializer should emit an error");
        assert_eq!(
            u64_or_none.err().unwrap().to_string(),
            expected_error_message
        );
    }

    #[derive(Debug, Deserialize)]
    pub struct U64OrNoneWithZeroAsNone {
        #[serde(deserialize_with = "deserialize_u64_or_none_with_zero_as_none")]
        pub u64: Option<u64>,
    }

    #[rstest]
    #[case(r#"{"u64": 1}"#, Some(1))]
    #[case(r#"{"u64": false}"#, None)]
    #[case(r#"{"u64": null}"#, None)]
    #[case(r#"{"u64": 0}"#, None)]
    fn test_deserialize_u64_or_none_with_zero_as_none(
        #[case] test_case: &str,
        #[case] expected_result: Option<u64>,
    ) {
        let u64_or_none: U64OrNoneWithZeroAsNone =
            serde_json::from_str(test_case).expect("Test case should be a valid JSON");
        assert_eq!(expected_result, u64_or_none.u64);
    }

    #[rstest]
    #[case(
        r#"{"u64": "1"}"#,
        r#"invalid type: string "1", expected u64, false, or null at line 1 column 11"#
    )]
    #[case(
        r#"{"u64": true}"#,
        r#"invalid value: boolean `true`, expected u64, false, or null at line 1 column 12"#
    )]
    fn test_deserialize_u64_or_none_with_zero_as_none_errors(
        #[case] test_case: &str,
        #[case] expected_error_message: &str,
    ) {
        let u64_or_none: Result<U64OrNoneWithZeroAsNone, serde_json::Error> =
            serde_json::from_str(test_case);
        assert!(u64_or_none.is_err(), "The serializer should emit an error");
        assert_eq!(
            u64_or_none.err().unwrap().to_string(),
            expected_error_message
        );
    }

    #[derive(Debug, Deserialize)]
    pub struct U64OrNoneWithNegativeAsNone {
        #[serde(deserialize_with = "deserialize_u64_or_none_with_negative_as_none")]
        pub u64: Option<u64>,
    }

    #[rstest]
    #[case(r#"{"u64": 1}"#, Some(1))]
    #[case(r#"{"u64": false}"#, None)]
    #[case(r#"{"u64": null}"#, None)]
    #[case(r#"{"u64": -1}"#, None)]
    fn test_deserialize_u64_or_none_with_negative_as_none(
        #[case] test_case: &str,
        #[case] expected_result: Option<u64>,
    ) {
        let u64_or_none: U64OrNoneWithNegativeAsNone =
            serde_json::from_str(test_case).expect("Test case should be a valid JSON");
        assert_eq!(expected_result, u64_or_none.u64);
    }

    #[rstest]
    #[case(r#"{"u64": "1"}"#, r#"invalid type: string "1", expected u64, false, null, or negative number at line 1 column 11"#)]
    #[case(r#"{"u64": true}"#, r#"invalid value: boolean `true`, expected u64, false, null, or negative number at line 1 column 12"#)]
    fn test_deserialize_u64_or_none_with_negative_as_none_errors(
        #[case] test_case: &str,
        #[case] expected_error_message: &str,
    ) {
        let u64_or_none: Result<U64OrNoneWithNegativeAsNone, serde_json::Error> =
            serde_json::from_str(test_case);
        assert!(u64_or_none.is_err(), "The serializer should emit an error");
        assert_eq!(
            u64_or_none.err().unwrap().to_string(),
            expected_error_message
        );
    }

    #[derive(Debug, Deserialize)]
    pub struct BoolOrInt {
        #[serde(deserialize_with = "deserialize_bool_or_int")]
        pub value: bool,
    }

    #[rstest]
    #[case(r#"{"value": true}"#, true)]
    #[case(r#"{"value": false}"#, false)]
    #[case(r#"{"value": 1}"#, true)]
    #[case(r#"{"value": 0}"#, false)]
    fn test_deserialize_bool_or_int(#[case] test_case: &str, #[case] expected_result: bool) {
        let bool_or_int: BoolOrInt =
            serde_json::from_str(test_case).expect("Test case should be a valid JSON");
        assert_eq!(expected_result, bool_or_int.value);
    }
}

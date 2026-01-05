use serde::{
    Deserialize, Deserializer,
    de::{self, Unexpected},
};
use std::fmt;

/// Deserialize an `i64` from either a number or a string representation.
///
/// Accepts:
/// - Integer values (positive or negative)
/// - String representations of integers (e.g., `"42"`, `"-1"`)
///
/// # Errors
///
/// Returns an error for non-numeric strings, booleans, null, arrays, or objects.
pub fn deserialize_i64_or_string<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeI64OrStringVisitor)
}

/// Deserialize a `u64` from either a number or a string representation.
///
/// Accepts:
/// - Unsigned integer values
/// - String representations of unsigned integers (e.g., `"42"`, `"0"`)
///
/// # Errors
///
/// Returns an error for negative numbers, non-numeric strings, booleans, null, arrays, or objects.
pub fn deserialize_u64_or_string<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeU64OrStringVisitor)
}

/// Deserialize an `i64` and convert it to a type that implements `From<i64>`.
///
/// This is useful for deserializing into newtype wrappers around `i64`.
///
/// # Errors
///
/// Returns an error for non-numeric strings, booleans, null, arrays, or objects.
pub fn deserialize_i64_or_string_as_t<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: From<i64>,
{
    deserialize_i64_or_string(deserializer).map(Into::into)
}

/// Deserialize an optional `u64`, treating `false` and `null` as `None`.
///
/// Accepts:
/// - Unsigned integer values → `Some(value)`
/// - Boolean `false` → `None`
/// - `null` → `None`
///
/// # Errors
///
/// Returns an error for `true`, negative numbers, strings, arrays, or objects.
pub fn deserialize_u64_or_none<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeU64OrNoneVisitor)
}

/// Deserialize an optional `u64`, treating `false`, `null`, and `0` as `None`.
///
/// Accepts:
/// - Unsigned integer values > 0 → `Some(value)`
/// - `0` → `None`
/// - Boolean `false` → `None`
/// - `null` → `None`
///
/// # Errors
///
/// Returns an error for `true`, negative numbers, strings, arrays, or objects.
pub fn deserialize_u64_or_none_with_zero_as_none<'de, D>(
    deserializer: D,
) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_u64_or_none(deserializer).map(|opt| opt.filter(|&v| v != 0))
}

/// Deserialize an optional `u64`, treating `false`, `null`, and negative numbers as `None`.
///
/// Accepts:
/// - Unsigned integer values → `Some(value)`
/// - Negative integer values → `None`
/// - Boolean `false` → `None`
/// - `null` → `None`
///
/// # Errors
///
/// Returns an error for `true`, strings, arrays, or objects.
pub fn deserialize_u64_or_none_with_negative_as_none<'de, D>(
    deserializer: D,
) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum U64OrNoneWithNegative {
        Signed(i64),
        Bool(bool),
        Null,
        Other(serde_json::Value),
    }

    match U64OrNoneWithNegative::deserialize(deserializer)? {
        U64OrNoneWithNegative::Signed(v) if v >= 0 => Ok(Some(v as u64)),
        U64OrNoneWithNegative::Signed(_) => Ok(None), // negative
        U64OrNoneWithNegative::Bool(false) => Ok(None),
        U64OrNoneWithNegative::Bool(true) => Err(de::Error::custom(
            "expected u64, false, null, or negative number, got `true`",
        )),
        U64OrNoneWithNegative::Null => Ok(None),
        U64OrNoneWithNegative::Other(v) => Err(de::Error::custom(format!(
            "expected u64, false, null, or negative number, got `{v}`"
        ))),
    }
}

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

/// Deserialize a boolean from either a bool or an integer (0/1).
///
/// Accepts:
/// - Boolean `true` or `false`
/// - Integer `1` → `true`
/// - Integer `0` → `false`
///
/// # Errors
///
/// Returns an error for integers other than 0 or 1, strings, null, arrays, or objects.
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
    #[case(
        r#"{"u64": "1"}"#,
        r#"expected u64, false, null, or negative number, got `"1"` at line 1 column 12"#
    )]
    #[case(
        r#"{"u64": true}"#,
        r#"expected u64, false, null, or negative number, got `true` at line 1 column 13"#
    )]
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

    #[rstest]
    #[case(
        r#"{"value": 2}"#,
        r#"invalid value: integer `2`, expected bool or integer (0/1) at line 1 column 11"#
    )]
    #[case(
        r#"{"value": -1}"#,
        r#"invalid value: integer `-1`, expected bool or integer (0/1) at line 1 column 12"#
    )]
    #[case(
        r#"{"value": "true"}"#,
        r#"invalid type: string "true", expected bool or integer (0/1) at line 1 column 16"#
    )]
    fn test_deserialize_bool_or_int_errors(
        #[case] test_case: &str,
        #[case] expected_error_message: &str,
    ) {
        let bool_or_int: Result<BoolOrInt, serde_json::Error> = serde_json::from_str(test_case);
        assert!(
            bool_or_int.is_err(),
            "The deserializer should emit an error"
        );
        assert_eq!(
            bool_or_int.err().unwrap().to_string(),
            expected_error_message
        );
    }

    #[derive(Debug, Deserialize)]
    pub struct U64OrString {
        #[serde(deserialize_with = "deserialize_u64_or_string")]
        pub value: u64,
    }

    #[rstest]
    #[case(r#"{"value": "1"}"#, 1)]
    #[case(r#"{"value": 1}"#, 1)]
    #[case(r#"{"value": "0"}"#, 0)]
    #[case(r#"{"value": 0}"#, 0)]
    #[case(r#"{"value": "18446744073709551615"}"#, u64::MAX)]
    fn test_deserialize_u64_or_string(#[case] test_case: &str, #[case] expected_result: u64) {
        let u64_or_string: U64OrString =
            serde_json::from_str(test_case).expect("Test case should be a valid JSON");
        assert_eq!(expected_result, u64_or_string.value);
    }

    #[rstest]
    #[case(
        r#"{"value": "abc"}"#,
        r#"invalid value: string "abc", expected u64 or a string at line 1 column 15"#
    )]
    #[case(
        r#"{"value": "-1"}"#,
        r#"invalid value: string "-1", expected u64 or a string at line 1 column 14"#
    )]
    fn test_deserialize_u64_or_string_errors(
        #[case] test_case: &str,
        #[case] expected_error_message: &str,
    ) {
        let u64_or_string: Result<U64OrString, serde_json::Error> = serde_json::from_str(test_case);
        assert!(
            u64_or_string.is_err(),
            "The deserializer should emit an error"
        );
        assert_eq!(
            u64_or_string.err().unwrap().to_string(),
            expected_error_message
        );
    }

    #[rstest]
    #[case(
        r#"{"bar": "abc"}"#,
        r#"invalid value: string "abc", expected i64 or a string at line 1 column 13"#
    )]
    #[case(
        r#"{"bar": true}"#,
        r#"invalid type: boolean `true`, expected i64 or a string at line 1 column 12"#
    )]
    fn test_deserialize_i64_or_string_errors(
        #[case] test_case: &str,
        #[case] expected_error_message: &str,
    ) {
        let foo: Result<Foo, serde_json::Error> = serde_json::from_str(test_case);
        assert!(foo.is_err(), "The deserializer should emit an error");
        assert_eq!(foo.err().unwrap().to_string(), expected_error_message);
    }
}

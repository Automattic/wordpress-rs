use serde::{Deserialize, Deserializer, de};
use std::fmt;

/// Deserialize a value that can be either a boolean `false` or a string.
///
/// Returns `None` if the value is:
/// - Boolean `false`
/// - The string `"false"` (case-insensitive, whitespace-trimmed)
///
/// Returns `Some(String)` for any other string value.
pub fn deserialize_false_or_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum FalseOrString {
        Bool(bool),
        String(String),
    }

    match FalseOrString::deserialize(deserializer)? {
        FalseOrString::Bool(false) => Ok(None),
        FalseOrString::Bool(true) => Err(de::Error::custom(
            "expected boolean `false` or a string, got `true`",
        )),
        FalseOrString::String(s) if s.to_lowercase().trim() == "false" => Ok(None),
        FalseOrString::String(s) => Ok(Some(s)),
    }
}

/// Deserialize a string, treating empty or whitespace-only strings as `None`.
///
/// Returns `None` if the string is empty or contains only whitespace characters.
/// Returns `Some(String)` for any non-empty string (preserving the original value).
pub fn deserialize_empty_string_as_none<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if s.trim().is_empty() {
        Ok(None)
    } else {
        Ok(Some(s))
    }
}

/// Deserialize a string, treating empty/whitespace-only strings and `"N/A"` as `None`.
///
/// Returns `None` if the string is empty, contains only whitespace, or equals `"N/A"`.
/// Returns `Some(String)` for any other non-empty string (preserving the original value).
pub fn deserialize_placeholder_string_as_none<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if s.trim().is_empty() || s.trim() == "N/A" {
        Ok(None)
    } else {
        Ok(Some(s))
    }
}

/// Deserialize a value that can be `null`, boolean `false`, or a string.
///
/// Returns `None` if the value is:
/// - `null`
/// - Boolean `false`
/// - The string `"false"` (case-insensitive, whitespace-trimmed)
///
/// Returns `Some(String)` for any other string value.
///
/// This is the null-aware counterpart of [`deserialize_false_or_string`],
/// intended for `Option<T>` fields where the API may return `null`, `false`,
/// or a string.
pub fn deserialize_false_or_string_or_null<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum NullFalseOrString {
        Null,
        Bool(bool),
        String(String),
    }

    match NullFalseOrString::deserialize(deserializer)? {
        NullFalseOrString::Null | NullFalseOrString::Bool(false) => Ok(None),
        NullFalseOrString::Bool(true) => Err(de::Error::custom(
            "expected a string, `null`, or `false`, got `true`",
        )),
        NullFalseOrString::String(s) if s.to_lowercase().trim() == "false" => Ok(None),
        NullFalseOrString::String(s) => Ok(Some(s)),
    }
}

/// Deserialize a value that can be either a single string or an array of strings.
///
/// - A single string `"foo"` becomes `vec!["foo"]`
/// - An array `["foo", "bar"]` becomes `vec!["foo", "bar"]`
/// - An empty array `[]` becomes `vec![]`
///
/// # Errors
///
/// Returns an error if the value is `null` or any other non-string/non-array type.
/// Use [`deserialize_string_vec_or_string_as_option`] if `null` should be accepted.
pub fn deserialize_string_vec_or_string<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_string_vec_or_string_as_option(deserializer)?
        .ok_or_else(|| de::Error::custom("expected a string or vector of strings"))
}

/// Deserialize an optional value that can be a single string, an array of strings, or null.
///
/// - A single string `"foo"` becomes `Some(vec!["foo"])`
/// - An array `["foo", "bar"]` becomes `Some(vec!["foo", "bar"])`
/// - An empty array `[]` becomes `Some(vec![])`
/// - `null` becomes `None`
///
/// # Errors
///
/// Returns an error for non-string/non-array/non-null types (boolean, number, object).
pub fn deserialize_string_vec_or_string_as_option<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrVec {
        Null,
        String(String),
        Vec(Vec<String>),
    }

    match StringOrVec::deserialize(deserializer)? {
        StringOrVec::Null => Ok(None),
        StringOrVec::String(s) => Ok(Some(vec![s])),
        StringOrVec::Vec(v) => Ok(Some(v)),
    }
}

/// Deserialize an optional string from a string, an integer, or `null`.
///
/// PHP turns an array key that looks like an integer into one, so a WordPress.com field whose
/// values are array keys arrives as a JSON number whenever the value is numeric. The stats
/// archives endpoint's archive names are such keys: the home page is always `1`, and a search
/// for `2024` or a category slug of `2024` comes back as a number rather than a string.
///
/// Accepts:
/// - `"swag"` → `Some("swag")`
/// - `2024` → `Some("2024")`
/// - `-1` → `Some("-1")`
/// - `""` → `Some("")`
/// - `null` → `None`
///
/// Numbers are rendered with `to_string`, so a number and the equivalent string are
/// indistinguishable afterwards. That is the point where this is used — a numeric archive name
/// and a numeric-looking one are the same page. Use an untagged enum where the distinction
/// matters.
///
/// # Errors
///
/// Returns an error for floating-point numbers, booleans, arrays, and objects. PHP array keys
/// are only ever integers or strings, so a fraction is drift rather than a value to coerce.
/// Unlike [`deserialize_false_or_string_or_null`], boolean `false` is not an absence marker
/// here.
pub fn deserialize_string_or_integer_or_null<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeStringOrIntegerOrNullVisitor)
}

struct DeserializeStringOrIntegerOrNullVisitor;

impl de::Visitor<'_> for DeserializeStringOrIntegerOrNullVisitor {
    type Value = Option<String>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string, an integer, or null")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Some(value.to_string()))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Some(value.to_string()))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Some(value.to_string()))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(None)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    pub struct StringOrBool {
        #[serde(deserialize_with = "deserialize_false_or_string")]
        pub value: Option<String>,
    }

    #[rstest]
    #[case(r#"{"value": "foo"}"#, Some("foo".to_string()))]
    #[case(r#"{"value": "false"}"#, None)]
    #[case(r#"{"value": false}"#, None)]
    fn test_deserialize_false_or_string(
        #[case] test_case: &str,
        #[case] expected_result: Option<String>,
    ) {
        let string_or_bool: StringOrBool =
            serde_json::from_str(test_case).expect("Test case should be a valid JSON");
        assert_eq!(expected_result, string_or_bool.value);
    }

    #[derive(Debug, Deserialize)]
    pub struct StringVecOrString {
        #[serde(deserialize_with = "deserialize_string_vec_or_string")]
        pub string: Vec<String>,
    }

    #[rstest]
    #[case(r#"{"string": "string"}"#, vec!["string".to_string()])]
    #[case(r#"{"string": ["string", "string2"]}"#, vec!["string".to_string(), "string2".to_string()])]
    #[case(r#"{"string": []}"#, vec![])]
    fn test_deserialize_string_vec_or_string(
        #[case] test_case: &str,
        #[case] expected_result: Vec<String>,
    ) {
        let string_vec_or_string: StringVecOrString =
            serde_json::from_str(test_case).expect("Test case should be a valid JSON");
        assert_eq!(expected_result, string_vec_or_string.string);
    }

    #[derive(Debug, Deserialize)]
    pub struct OptionStringVecOrString {
        #[serde(deserialize_with = "deserialize_string_vec_or_string_as_option")]
        #[serde(default)]
        pub string: Option<Vec<String>>,
    }

    #[rstest]
    #[case(r#"{"string": "string"}"#, Some(vec!["string".to_string()]))]
    #[case(r#"{"string": ["string", "string2"]}"#, Some(vec!["string".to_string(), "string2".to_string()]))]
    #[case(r#"{"string": []}"#, Some(vec![]))]
    #[case(r#"{"string": null}"#, None)]
    #[case(r#"{}"#, None)]
    fn test_deserialize_string_vec_or_string_as_option(
        #[case] test_case: &str,
        #[case] expected_result: Option<Vec<String>>,
    ) {
        let option_string_vec_or_string: OptionStringVecOrString =
            serde_json::from_str(test_case).expect("Test case should be a valid JSON");
        assert_eq!(expected_result, option_string_vec_or_string.string);
    }

    #[derive(Debug, Deserialize)]
    pub struct NullStringOrBool {
        #[serde(default, deserialize_with = "deserialize_false_or_string_or_null")]
        pub value: Option<String>,
    }

    #[rstest]
    #[case(r#"{"value": "foo"}"#, Some("foo".to_string()))]
    #[case(r#"{"value": "false"}"#, None)]
    #[case(r#"{"value": false}"#, None)]
    #[case(r#"{"value": null}"#, None)]
    #[case(r#"{}"#, None)]
    fn test_deserialize_false_or_string_or_null(
        #[case] test_case: &str,
        #[case] expected_result: Option<String>,
    ) {
        let result: NullStringOrBool =
            serde_json::from_str(test_case).expect("Test case should be a valid JSON");
        assert_eq!(expected_result, result.value);
    }

    #[rstest]
    #[case(
        r#"{"value": true}"#,
        r#"expected a string, `null`, or `false`, got `true` at line 1 column 15"#
    )]
    fn test_deserialize_false_or_string_or_null_errors(
        #[case] test_case: &str,
        #[case] expected_error_message: &str,
    ) {
        let result: Result<NullStringOrBool, serde_json::Error> = serde_json::from_str(test_case);
        assert!(result.is_err(), "The deserializer should emit an error");
        assert_eq!(result.err().unwrap().to_string(), expected_error_message);
    }

    #[rstest]
    #[case(
        r#"{"value": true}"#,
        r#"expected boolean `false` or a string, got `true` at line 1 column 15"#
    )]
    fn test_deserialize_false_or_string_errors(
        #[case] test_case: &str,
        #[case] expected_error_message: &str,
    ) {
        let string_or_bool: Result<StringOrBool, serde_json::Error> =
            serde_json::from_str(test_case);
        assert!(
            string_or_bool.is_err(),
            "The deserializer should emit an error"
        );
        assert_eq!(
            string_or_bool.err().unwrap().to_string(),
            expected_error_message
        );
    }

    #[derive(Debug, Deserialize)]
    pub struct EmptyStringAsNone {
        #[serde(deserialize_with = "deserialize_empty_string_as_none")]
        pub value: Option<String>,
    }

    #[rstest]
    #[case(r#"{"value": "foo"}"#, Some("foo".to_string()))]
    #[case(r#"{"value": "hello world"}"#, Some("hello world".to_string()))]
    #[case(r#"{"value": ""}"#, None)]
    #[case(r#"{"value": "   "}"#, None)]
    #[case(r#"{"value": "\t\n"}"#, None)]
    fn test_deserialize_empty_string_as_none(
        #[case] test_case: &str,
        #[case] expected_result: Option<String>,
    ) {
        let empty_string_as_none: EmptyStringAsNone =
            serde_json::from_str(test_case).expect("Test case should be a valid JSON");
        assert_eq!(expected_result, empty_string_as_none.value);
    }

    #[derive(Debug, Deserialize)]
    pub struct PlaceholderStringAsNone {
        #[serde(deserialize_with = "deserialize_placeholder_string_as_none")]
        pub value: Option<String>,
    }

    #[rstest]
    #[case(r#"{"value": "N/A"}"#, None)]
    #[case(r#"{"value": ""}"#, None)]
    #[case(r#"{"value": " "}"#, None)]
    #[case(r#"{"value": "some value"}"#, Some("some value".to_string()))]
    fn test_deserialize_placeholder_string_as_none(
        #[case] test_case: &str,
        #[case] expected_result: Option<String>,
    ) {
        let wrapper: PlaceholderStringAsNone =
            serde_json::from_str(test_case).expect("Test case should be a valid JSON");
        assert_eq!(expected_result, wrapper.value);
    }

    #[derive(Debug, Deserialize)]
    pub struct StringOrInteger {
        #[serde(default, deserialize_with = "deserialize_string_or_integer_or_null")]
        pub value: Option<String>,
    }

    #[rstest]
    #[case(r#"{"value": "swag"}"#, Some("swag".to_string()))]
    #[case(r#"{"value": "2024"}"#, Some("2024".to_string()))]
    #[case(r#"{"value": ""}"#, Some(String::new()))]
    #[case(r#"{"value": 2024}"#, Some("2024".to_string()))]
    #[case(r#"{"value": 1}"#, Some("1".to_string()))]
    #[case(r#"{"value": 0}"#, Some("0".to_string()))]
    #[case(r#"{"value": -1}"#, Some("-1".to_string()))]
    // PHP's integer keys span the whole signed range, so both halves matter.
    #[case(
        r#"{"value": 9223372036854775807}"#,
        Some("9223372036854775807".to_string())
    )]
    #[case(
        r#"{"value": -9223372036854775808}"#,
        Some("-9223372036854775808".to_string())
    )]
    #[case(r#"{"value": null}"#, None)]
    #[case(r#"{}"#, None)]
    fn test_deserialize_string_or_integer_or_null(
        #[case] test_case: &str,
        #[case] expected_result: Option<String>,
    ) {
        let wrapper: StringOrInteger =
            serde_json::from_str(test_case).expect("Test case should be a valid JSON");
        assert_eq!(expected_result, wrapper.value);
    }

    /// A PHP array key is an integer or a string, never a fraction or a boolean, so anything
    /// else is drift rather than a value to coerce.
    #[rstest]
    #[case(
        r#"{"value": 1.5}"#,
        "invalid type: floating point `1.5`, expected a string, an integer, or null at line 1 column 13"
    )]
    #[case(
        r#"{"value": true}"#,
        "invalid type: boolean `true`, expected a string, an integer, or null at line 1 column 14"
    )]
    #[case(
        r#"{"value": []}"#,
        "invalid type: sequence, expected a string, an integer, or null at line 1 column 12"
    )]
    #[case(
        r#"{"value": {}}"#,
        "invalid type: map, expected a string, an integer, or null at line 1 column 12"
    )]
    fn test_deserialize_string_or_integer_or_null_errors(
        #[case] test_case: &str,
        #[case] expected_error_message: &str,
    ) {
        let result: Result<StringOrInteger, serde_json::Error> = serde_json::from_str(test_case);
        assert_eq!(
            result
                .expect_err("The deserializer should emit an error")
                .to_string(),
            expected_error_message
        );
    }
}

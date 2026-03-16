use serde::{Deserialize, Deserializer, de};

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
}

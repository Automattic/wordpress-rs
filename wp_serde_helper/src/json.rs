use serde::{
    Serialize, Serializer,
    de::{self, DeserializeOwned},
    ser,
};
use std::{fmt, marker::PhantomData};

/// Serialize a value as a JSON string embedded within the parent JSON structure.
///
/// This function serializes the value to a JSON string, then embeds that string
/// as a string field in the parent object. Useful for APIs that expect nested
/// JSON to be passed as an escaped string rather than as a nested object.
///
/// # Errors
///
/// Returns an error if the value cannot be serialized to JSON.
pub fn serialize_as_json_string<T, S, E>(value: &T, s: S) -> Result<S::Ok, E>
where
    T: Serialize,
    S: Serializer<Error = E>,
    E: serde::ser::Error,
{
    serde_json::to_string(value)
        .map_err(|e| ser::Error::custom(e.to_string()))?
        .serialize(s)
}

/// Deserialize a JSON array that is encoded as a string.
///
/// Some APIs return arrays as escaped JSON strings rather than native JSON arrays.
/// This function parses the string and deserializes it as a `Vec<T>`.
///
/// Accepts:
/// - Non-empty string containing a JSON array → `Some(Vec<T>)`
/// - Empty string `""` → `None`
///
/// # Errors
///
/// Returns an error if the string contains invalid JSON or is not an array.
pub fn deserialize_from_string_of_json_array<'de, T, D>(
    deserializer: D,
) -> Result<Option<Vec<T>>, D::Error>
where
    T: DeserializeOwned,
    D: de::Deserializer<'de>,
{
    deserializer.deserialize_any(StringOfJsonArrayVisitor::<T>(PhantomData))
}

// Use `PhantomData` to avoid "unused generic `T` error"
struct StringOfJsonArrayVisitor<T>(PhantomData<T>);

impl<T: DeserializeOwned> de::Visitor<'_> for StringOfJsonArrayVisitor<T> {
    type Value = Option<Vec<T>>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string containing json array")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v.is_empty() {
            Ok(None)
        } else {
            serde_json::from_str(v).map_err(E::custom)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    pub struct Inner {
        pub name: String,
        pub value: i32,
    }

    #[derive(Debug, Serialize)]
    pub struct SerializeAsJsonString {
        #[serde(serialize_with = "serialize_as_json_string")]
        pub data: Inner,
    }

    #[test]
    fn test_serialize_as_json_string() {
        let obj = SerializeAsJsonString {
            data: Inner {
                name: "test".to_string(),
                value: 42,
            },
        };
        let json = serde_json::to_string(&obj).expect("Should serialize");
        assert_eq!(json, r#"{"data":"{\"name\":\"test\",\"value\":42}"}"#);
    }

    #[test]
    fn test_serialize_as_json_string_with_vec() {
        #[derive(Debug, Serialize)]
        pub struct SerializeVecAsJsonString {
            #[serde(serialize_with = "serialize_as_json_string")]
            pub items: Vec<i32>,
        }

        let obj = SerializeVecAsJsonString {
            items: vec![1, 2, 3],
        };
        let json = serde_json::to_string(&obj).expect("Should serialize");
        assert_eq!(json, r#"{"items":"[1,2,3]"}"#);
    }

    #[derive(Debug, Deserialize)]
    pub struct StringOfJsonArray {
        #[serde(deserialize_with = "deserialize_from_string_of_json_array")]
        pub items: Option<Vec<i32>>,
    }

    #[rstest]
    #[case(r#"{"items": "[1, 2, 3]"}"#, Some(vec![1, 2, 3]))]
    #[case(r#"{"items": "[]"}"#, Some(vec![]))]
    #[case(r#"{"items": ""}"#, None)]
    fn test_deserialize_from_string_of_json_array(
        #[case] test_case: &str,
        #[case] expected_result: Option<Vec<i32>>,
    ) {
        let string_of_json_array: StringOfJsonArray =
            serde_json::from_str(test_case).expect("Test case should be a valid JSON");
        assert_eq!(expected_result, string_of_json_array.items);
    }

    #[derive(Debug, Deserialize)]
    pub struct StringOfJsonArrayObjects {
        #[serde(deserialize_with = "deserialize_from_string_of_json_array")]
        pub items: Option<Vec<Inner>>,
    }

    #[test]
    fn test_deserialize_from_string_of_json_array_with_objects() {
        let json = r#"{"items": "[{\"name\":\"a\",\"value\":1},{\"name\":\"b\",\"value\":2}]"}"#;
        let result: StringOfJsonArrayObjects =
            serde_json::from_str(json).expect("Test case should be a valid JSON");
        assert_eq!(
            result.items,
            Some(vec![
                Inner {
                    name: "a".to_string(),
                    value: 1
                },
                Inner {
                    name: "b".to_string(),
                    value: 2
                }
            ])
        );
    }

    #[rstest]
    #[case(r#"{"items": "invalid json"}"#)]
    #[case(r#"{"items": "[1, 2, invalid]"}"#)]
    fn test_deserialize_from_string_of_json_array_errors(#[case] test_case: &str) {
        let result: Result<StringOfJsonArray, serde_json::Error> = serde_json::from_str(test_case);
        assert!(result.is_err(), "The deserializer should emit an error");
    }
}

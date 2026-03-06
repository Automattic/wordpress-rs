use serde::{
    Deserialize,
    de::{self, DeserializeOwned},
};
use std::collections::HashMap;

/// Deserialize a `HashMap` that may be represented as an empty JSON array
/// or any other non-map placeholder value.
///
/// Some APIs return `[]`, `0`, `null`, `false`, or other non-map values
/// instead of `{}` when a map is empty. This function handles all such
/// representations by first consuming the JSON value, then converting
/// only JSON objects into a `HashMap`.
///
/// Accepts:
/// - JSON object `{...}` → `HashMap` with deserialized key-value pairs
/// - Any other JSON value (`[]`, `0`, `null`, `false`, etc.) → empty `HashMap`
///
/// # Errors
///
/// Returns an error only if a JSON object's keys or values can't be
/// deserialized into the target types.
pub fn deserialize_empty_array_or_hashmap<'de, D, K, V>(
    deserializer: D,
) -> Result<HashMap<K, V>, D::Error>
where
    D: de::Deserializer<'de>,
    K: DeserializeOwned + std::hash::Hash + Eq,
    V: DeserializeOwned,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    if let serde_json::Value::Object(map) = value {
        serde_json::from_value(serde_json::Value::Object(map)).map_err(de::Error::custom)
    } else {
        Ok(HashMap::new())
    }
}

/// Deserialize an `Option<HashMap>` that may be represented as `null`, an empty JSON array,
/// or any other non-map placeholder value.
///
/// Some APIs return `[]`, `0`, `null`, `false`, or other non-map values
/// instead of `{}` when a map is empty. This function handles all such
/// representations by first consuming the JSON value, then converting
/// only JSON objects into a `HashMap`.
///
/// Accepts:
/// - JSON object `{...}` → `Some(HashMap)` with deserialized key-value pairs
/// - Any other JSON value (`null`, `[]`, `0`, `false`, etc.) → `None`
///
/// # Errors
///
/// Returns an error only if a JSON object's keys or values can't be
/// deserialized into the target types.
pub fn deserialize_option_empty_array_or_hashmap<'de, D, K, V>(
    deserializer: D,
) -> Result<Option<HashMap<K, V>>, D::Error>
where
    D: de::Deserializer<'de>,
    K: DeserializeOwned + std::hash::Hash + Eq,
    V: DeserializeOwned,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    if let serde_json::Value::Object(map) = value {
        serde_json::from_value(serde_json::Value::Object(map))
            .map(Some)
            .map_err(de::Error::custom)
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    pub struct HashMapWrapper {
        #[serde(deserialize_with = "deserialize_empty_array_or_hashmap")]
        pub map: HashMap<String, String>,
    }

    #[rstest]
    #[case(r#"{"map": []}"#, HashMap::new())]
    #[case(r#"{"map": {"key": "value"}}"#, {
        let mut map = HashMap::new();
        map.insert("key".to_string(), "value".to_string());
        map
    })]
    #[case(r#"{"map": {"foo": "bar", "hello": "world"}}"#, {
        let mut map = HashMap::new();
        map.insert("foo".to_string(), "bar".to_string());
        map.insert("hello".to_string(), "world".to_string());
        map
    })]
    fn test_deserialize_empty_array_or_hashmap(
        #[case] test_case: &str,
        #[case] expected_result: HashMap<String, String>,
    ) {
        let wrapper: HashMapWrapper =
            serde_json::from_str(test_case).expect("Test case should be a valid JSON");
        assert_eq!(expected_result, wrapper.map);
    }

    #[rstest]
    #[case(r#"{"map": 0}"#, HashMap::new())]
    #[case(r#"{"map": 42}"#, HashMap::new())]
    #[case(r#"{"map": null}"#, HashMap::new())]
    #[case(r#"{"map": false}"#, HashMap::new())]
    #[case(r#"{"map": true}"#, HashMap::new())]
    #[case(r#"{"map": 0.0}"#, HashMap::new())]
    fn test_deserialize_non_map_as_empty_hashmap(
        #[case] test_case: &str,
        #[case] expected_result: HashMap<String, String>,
    ) {
        let wrapper: HashMapWrapper =
            serde_json::from_str(test_case).expect("Test case should be a valid JSON");
        assert_eq!(expected_result, wrapper.map);
    }

    #[rstest]
    #[case(r#"{"map": ["not", "empty"]}"#, HashMap::new())]
    #[case(r#"{"map": "some string"}"#, HashMap::new())]
    fn test_deserialize_non_object_as_empty_hashmap(
        #[case] test_case: &str,
        #[case] expected_result: HashMap<String, String>,
    ) {
        let wrapper: HashMapWrapper =
            serde_json::from_str(test_case).expect("Test case should be a valid JSON");
        assert_eq!(expected_result, wrapper.map);
    }

    #[derive(Debug, Deserialize)]
    pub struct HashMapWithIntValues {
        #[serde(deserialize_with = "deserialize_empty_array_or_hashmap")]
        pub map: HashMap<String, i64>,
    }

    #[rstest]
    #[case(r#"{"map": []}"#, HashMap::new())]
    #[case(r#"{"map": {"count": 42}}"#, {
        let mut map = HashMap::new();
        map.insert("count".to_string(), 42);
        map
    })]
    #[case(r#"{"map": {"a": 1, "b": -5, "c": 0}}"#, {
        let mut map = HashMap::new();
        map.insert("a".to_string(), 1);
        map.insert("b".to_string(), -5);
        map.insert("c".to_string(), 0);
        map
    })]
    fn test_deserialize_empty_array_or_hashmap_with_int_values(
        #[case] test_case: &str,
        #[case] expected_result: HashMap<String, i64>,
    ) {
        let wrapper: HashMapWithIntValues =
            serde_json::from_str(test_case).expect("Test case should be a valid JSON");
        assert_eq!(expected_result, wrapper.map);
    }

    #[derive(Debug, Deserialize)]
    pub struct OptionHashMapWrapper {
        #[serde(deserialize_with = "deserialize_option_empty_array_or_hashmap")]
        pub map: Option<HashMap<String, String>>,
    }

    #[rstest]
    #[case(r#"{"map": null}"#, None)]
    #[case(r#"{"map": []}"#, None)]
    #[case(r#"{"map": {"key": "value"}}"#, Some({
        let mut map = HashMap::new();
        map.insert("key".to_string(), "value".to_string());
        map
    }))]
    #[case(r#"{"map": {"foo": "bar", "hello": "world"}}"#, Some({
        let mut map = HashMap::new();
        map.insert("foo".to_string(), "bar".to_string());
        map.insert("hello".to_string(), "world".to_string());
        map
    }))]
    fn test_deserialize_option_empty_array_or_hashmap(
        #[case] test_case: &str,
        #[case] expected_result: Option<HashMap<String, String>>,
    ) {
        let wrapper: OptionHashMapWrapper =
            serde_json::from_str(test_case).expect("Test case should be a valid JSON");
        assert_eq!(expected_result, wrapper.map);
    }

    #[rstest]
    #[case(r#"{"map": 0}"#, None)]
    #[case(r#"{"map": false}"#, None)]
    #[case(r#"{"map": ["not", "empty"]}"#, None)]
    #[case(r#"{"map": "string"}"#, None)]
    fn test_deserialize_option_non_object_as_none(
        #[case] test_case: &str,
        #[case] expected_result: Option<HashMap<String, String>>,
    ) {
        let wrapper: OptionHashMapWrapper =
            serde_json::from_str(test_case).expect("Test case should be a valid JSON");
        assert_eq!(expected_result, wrapper.map);
    }
}

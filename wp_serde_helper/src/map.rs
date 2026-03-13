use serde::{
    Deserialize,
    de::{self, DeserializeOwned, Unexpected},
};
use std::{collections::HashMap, fmt, marker::PhantomData};

/// Deserialize a `HashMap` that may be represented as an empty JSON array.
///
/// Some APIs return `[]` instead of `{}` when a map is empty. This function
/// handles both representations, returning an empty `HashMap` for `[]` and
/// deserializing normally for JSON objects.
///
/// Accepts:
/// - Empty JSON array `[]` → empty `HashMap`
/// - JSON object `{...}` → `HashMap` with deserialized key-value pairs
///
/// # Errors
///
/// Returns an error for non-empty arrays, `null`, strings, numbers, or booleans.
pub fn deserialize_empty_array_or_hashmap<'de, D, K, V>(
    deserializer: D,
) -> Result<HashMap<K, V>, D::Error>
where
    D: de::Deserializer<'de>,
    K: DeserializeOwned + std::hash::Hash + Eq,
    V: DeserializeOwned,
{
    deserializer.deserialize_any(DeserializeEmptyArrayOrHashMapVisitor::<K, V>(PhantomData))
}

struct DeserializeEmptyArrayOrHashMapVisitor<K, V>(PhantomData<(K, V)>);

impl<'de, K, V> de::Visitor<'de> for DeserializeEmptyArrayOrHashMapVisitor<K, V>
where
    K: DeserializeOwned + std::hash::Hash + Eq,
    V: DeserializeOwned,
{
    type Value = HashMap<K, V>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("empty array or a HashMap")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        if seq.next_element::<Self::Value>()?.is_none() {
            // It's an empty array
            Ok(HashMap::new())
        } else {
            // not an empty array
            Err(serde::de::Error::invalid_type(Unexpected::Seq, &self))
        }
    }

    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
    where
        A: de::MapAccess<'de>,
    {
        HashMap::deserialize(de::value::MapAccessDeserializer::new(map))
    }
}

/// Deserialize a `HashMap` that may be represented as a non-map placeholder value.
///
/// Some APIs return `0`, `[]`, `null`, `false`, or other non-map values
/// instead of `{}` when a map has no data. This function treats any non-object
/// JSON value as an empty `HashMap`.
///
/// Accepts:
/// - JSON object `{...}` → `HashMap` with deserialized key-value pairs
/// - Any other JSON value (`[]`, `0`, `null`, `false`, etc.) → empty `HashMap`
///
/// # Errors
///
/// Returns an error only if a JSON object's keys or values can't be
/// deserialized into the target types.
pub fn deserialize_hashmap_or_placeholder_as_empty<'de, D, K, V>(
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

/// Deserialize an `Option<HashMap>` that may be represented as a non-map placeholder value.
///
/// Some APIs return `[]`, `0`, `null`, `false`, or other non-map values
/// instead of `{}` when a map has no data. This function treats any non-object
/// JSON value as `None`.
///
/// Accepts:
/// - JSON object `{...}` → `Some(HashMap)` with deserialized key-value pairs
/// - Any other JSON value (`null`, `[]`, `0`, `false`, etc.) → `None`
///
/// # Errors
///
/// Returns an error only if a JSON object's keys or values can't be
/// deserialized into the target types.
pub fn deserialize_option_hashmap_or_placeholder_as_none<'de, D, K, V>(
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
    #[case(r#"{"map": ["not", "empty"]}"#)]
    #[case(r#"{"map": null}"#)]
    fn test_deserialize_empty_array_or_hashmap_errors(#[case] test_case: &str) {
        let result: Result<HashMapWrapper, serde_json::Error> = serde_json::from_str(test_case);
        assert!(result.is_err(), "The deserializer should emit an error");
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

    // Tests for deserialize_hashmap_or_placeholder_as_empty

    #[derive(Debug, Deserialize)]
    pub struct PlaceholderHashMapWrapper {
        #[serde(deserialize_with = "deserialize_hashmap_or_placeholder_as_empty")]
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
    fn test_deserialize_hashmap_or_placeholder_as_empty(
        #[case] test_case: &str,
        #[case] expected_result: HashMap<String, String>,
    ) {
        let wrapper: PlaceholderHashMapWrapper =
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
    #[case(r#"{"map": ["not", "empty"]}"#, HashMap::new())]
    #[case(r#"{"map": "some string"}"#, HashMap::new())]
    fn test_deserialize_placeholder_as_empty_hashmap(
        #[case] test_case: &str,
        #[case] expected_result: HashMap<String, String>,
    ) {
        let wrapper: PlaceholderHashMapWrapper =
            serde_json::from_str(test_case).expect("Test case should be a valid JSON");
        assert_eq!(expected_result, wrapper.map);
    }

    // Tests for deserialize_option_hashmap_or_placeholder_as_none

    #[derive(Debug, Deserialize)]
    pub struct OptionPlaceholderHashMapWrapper {
        #[serde(deserialize_with = "deserialize_option_hashmap_or_placeholder_as_none")]
        pub map: Option<HashMap<String, String>>,
    }

    #[rstest]
    #[case(r#"{"map": null}"#, None)]
    #[case(r#"{"map": []}"#, None)]
    #[case(r#"{"map": 0}"#, None)]
    #[case(r#"{"map": false}"#, None)]
    #[case(r#"{"map": ["not", "empty"]}"#, None)]
    #[case(r#"{"map": "string"}"#, None)]
    fn test_deserialize_option_placeholder_as_none(
        #[case] test_case: &str,
        #[case] expected_result: Option<HashMap<String, String>>,
    ) {
        let wrapper: OptionPlaceholderHashMapWrapper =
            serde_json::from_str(test_case).expect("Test case should be a valid JSON");
        assert_eq!(expected_result, wrapper.map);
    }

    #[rstest]
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
    fn test_deserialize_option_placeholder_as_some(
        #[case] test_case: &str,
        #[case] expected_result: Option<HashMap<String, String>>,
    ) {
        let wrapper: OptionPlaceholderHashMapWrapper =
            serde_json::from_str(test_case).expect("Test case should be a valid JSON");
        assert_eq!(expected_result, wrapper.map);
    }
}

use serde::{Deserialize, de::{self, DeserializeOwned, Unexpected}};
use std::{collections::HashMap, fmt, marker::PhantomData};

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
}

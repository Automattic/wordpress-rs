use serde::{
    Deserialize, Deserializer,
    de::{self, IgnoredAny},
};

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum AlternativeValues<T> {
    Expected(T),
    Bool(bool),
    List(Vec<IgnoredAny>),
}

/// The plugin directory API may return different types of values for the same
/// property.
///
/// Here are a couple of examples:
/// - The "requires" field is supposed to be the WordPress version string, but
///   it can also be `false`.
/// - The "contributors" field is supposed to be a map of contributor usernames
///   to their details, but it can also be `[]`.
///
/// This function deserializes the value as usual, but if it's any "default JSON
/// values"(`false`, an empty list), it returns the default value for the type.
pub(crate) fn deserialize_default_values<'de, D, V>(deserializer: D) -> Result<V, D::Error>
where
    D: Deserializer<'de>,
    V: Deserialize<'de> + Default,
{
    AlternativeValues::<V>::deserialize(deserializer).and_then(|result| match result {
        AlternativeValues::Expected(v) => Ok(v),
        AlternativeValues::Bool(false) => Ok(V::default()),
        AlternativeValues::Bool(true) => Err(de::Error::invalid_value(
            de::Unexpected::Bool(true),
            &"a boolean false",
        )),
        AlternativeValues::List(list) => {
            if list.is_empty() {
                Ok(V::default())
            } else {
                Err(de::Error::invalid_value(
                    de::Unexpected::Seq,
                    &"an empty list",
                ))
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[derive(Deserialize, Debug, Eq, PartialEq)]
    struct Payload {
        #[serde(deserialize_with = "deserialize_default_values")]
        string: String,
        #[serde(deserialize_with = "deserialize_default_values")]
        object: HashMap<String, String>,
    }

    #[test]
    fn test_parsing_correct_types() {
        let json = r#"{
            "string": "string",
            "object": {
                "key": "value"
            }
        }"#;
        let result = serde_json::from_str::<Payload>(json);
        assert!(result.is_ok(), "Parsing failed: {:?}", result);

        let payload = result.unwrap();

        assert_eq!(payload.string.as_str(), "string");

        assert_eq!(payload.object.len(), 1);
        assert_eq!(payload.object["key"], "value");
    }

    #[test]
    fn test_parsing_false() {
        let json = r#"{"string": false, "object": false}"#;
        let result = serde_json::from_str::<Payload>(json);
        assert!(result.is_ok(), "Parsing failed: {:?}", result);

        let payload = result.unwrap();
        assert_eq!(payload.string, "".to_string());
        assert_eq!(payload.object, HashMap::new());
    }

    #[test]
    fn test_parsing_empty_list() {
        let json = r#"{"string": "string", "object": []}"#;
        let result = serde_json::from_str::<Payload>(json);
        assert!(result.is_ok(), "Parsing failed: {:?}", result);

        let payload = result.unwrap();
        assert_eq!(payload.string, "string".to_string());
        assert_eq!(payload.object, HashMap::new());
    }

    #[test]
    fn test_parsing_nonempty_list() {
        let json = r#"{"string": "string", "object": ["list"]}"#;
        let result = serde_json::from_str::<Payload>(json);
        assert!(
            result.is_err(),
            "Parsing 'object' should fail. Expected an error, got {:?}",
            result
        );
    }
}

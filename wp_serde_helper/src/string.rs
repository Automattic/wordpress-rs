use serde::{
    Deserialize, Deserializer,
    de::{self, Unexpected},
};
use std::fmt;

struct DeserializeFalseOrStringVisitor;

impl de::Visitor<'_> for DeserializeFalseOrStringVisitor {
    type Value = Option<String>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("Boolean `false` or a string")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v {
            Err(E::invalid_value(Unexpected::Bool(v), &self))
        } else {
            Ok(None)
        }
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v.to_lowercase().trim() == "false" {
            return Ok(None);
        }

        Ok(Some(v.to_string()))
    }
}

pub fn deserialize_false_or_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeFalseOrStringVisitor)
}

pub fn deserialize_empty_string_as_none<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeEmptyStringAsNoneVisitor)
}

struct DeserializeEmptyStringAsNoneVisitor;

impl<'de> de::Visitor<'de> for DeserializeEmptyStringAsNoneVisitor {
    type Value = Option<String>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("String")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v.trim().is_empty() {
            Ok(None)
        } else {
            Ok(Some(v.to_string()))
        }
    }
}

pub fn deserialize_string_vec_or_string<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    if let Some(vec) = deserialize_string_vec_or_string_as_option(deserializer)? {
        Ok(vec)
    } else {
        Err(serde::de::Error::custom(
            "Expected a string or vector of strings",
        ))
    }
}

pub fn deserialize_string_vec_or_string_as_option<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeStringVecOrStringAsOptionVisitor)
}

struct DeserializeStringVecOrStringAsOptionVisitor;

impl<'de> de::Visitor<'de> for DeserializeStringVecOrStringAsOptionVisitor {
    type Value = Option<Vec<String>>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("string or a vector of strings")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Some(vec![v.to_string()]))
    }

    fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        Ok(Some(Deserialize::deserialize(
            de::value::SeqAccessDeserializer::new(seq),
        )?))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(None)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
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
}

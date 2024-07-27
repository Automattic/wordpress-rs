use serde::{
    de::{self, Unexpected},
    Deserializer,
};
use std::fmt;

struct DeserializeI64OrStringVisitor;

impl<'de> de::Visitor<'de> for DeserializeI64OrStringVisitor {
    type Value = i64;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("i64 or a string")
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        i64::try_from(v).map_err(|e| E::invalid_value(Unexpected::Unsigned(v), &self))
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
            .map_err(|e| E::invalid_value(Unexpected::Str(v), &self))
    }
}

pub fn deserialize_i64_or_string<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeI64OrStringVisitor)
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
    fn test_deserialize_i64_or_string_as_option(
        #[case] test_case: &str,
        #[case] expected_result: i64,
    ) {
        let foo: Foo = serde_json::from_str(test_case).expect("Test case should be a valid JSON");
        assert_eq!(expected_result, foo.bar);
    }
}

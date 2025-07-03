use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{self, DeserializeOwned, Unexpected},
    ser,
};
use std::{fmt, marker::PhantomData};

pub use wp_serde_date::wp_utc_date_format;

mod wp_serde_date;

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

// Use `PhantomData` to avoid "unused generic `T` error"
struct StringOfJsonArrayVisitor<T>(PhantomData<T>);

impl<T: DeserializeOwned> de::Visitor<'_> for StringOfJsonArrayVisitor<T> {
    type Value = Vec<T>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string containing json array")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v.is_empty() {
            Ok(vec![])
        } else {
            serde_json::from_str(v).map_err(E::custom)
        }
    }
}

pub fn deserialize_from_string_of_json_array<'de, T, D>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    T: DeserializeOwned,
    D: de::Deserializer<'de>,
{
    deserializer.deserialize_any(StringOfJsonArrayVisitor::<T>(PhantomData))
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

pub fn deserialize_i64_or_string<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeI64OrStringVisitor)
}

pub fn deserialize_u64_or_string<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeU64OrStringVisitor)
}

pub fn deserialize_i64_or_string_as_t<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: From<i64>,
{
    deserialize_i64_or_string(deserializer).map(Into::into)
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
pub struct DeserializeOffsetVisitor;

impl de::Visitor<'_> for DeserializeOffsetVisitor {
    type Value = Option<f64>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("f64 or i64 or a string")
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Some(v))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Some(v as f64))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Some(v as f64))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        v.parse::<f64>().map_err(E::custom).map(Some)
    }
}

pub fn deserialize_offset<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeOffsetVisitor)
}

pub struct DeserializeEmptyVecOrT<T> {
    fallback: Box<dyn Fn() -> T>,
}

impl<T> DeserializeEmptyVecOrT<T> {
    pub fn new(fallback: Box<dyn Fn() -> T>) -> Self {
        Self { fallback }
    }
}

impl<'de, T> de::Visitor<'de> for DeserializeEmptyVecOrT<T>
where
    T: Deserialize<'de>,
{
    type Value = T;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("empty Vec or T")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        if seq.next_element::<Self::Value>()?.is_none() {
            // It's an empty vec
            Ok((self.fallback)())
        } else {
            // not an empty vec
            Err(serde::de::Error::invalid_type(Unexpected::Seq, &self))
        }
    }

    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
    where
        A: de::MapAccess<'de>,
    {
        Deserialize::deserialize(de::value::MapAccessDeserializer::new(map))
    }
}

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

    #[derive(Debug, Deserialize)]
    pub struct Offset {
        #[serde(default, deserialize_with = "deserialize_offset")]
        pub offset: Option<f64>,
    }

    #[rstest]
    #[case(r#"{"offset": "1"}"#, Some(1.0))]
    #[case(r#"{"offset": 1}"#, Some(1.0))]
    #[case(r#"{"offset": -1}"#, Some(-1.0))]
    #[case(r#"{"offset": 5.5}"#, Some(5.5))]
    #[case(r#"{"offset": "5.5"}"#, Some(5.5))]
    #[case("{}", None)]
    fn test_deserialize_offset(#[case] test_case: &str, #[case] expected_result: Option<f64>) {
        let offset: Offset =
            serde_json::from_str(test_case).expect("Test case should be a valid JSON");
        assert_eq!(expected_result, offset.offset);
    }

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
}

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

pub fn deserialize_u64_or_none<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeU64OrNoneVisitor)
}

struct DeserializeU64OrNoneVisitor;

impl de::Visitor<'_> for DeserializeU64OrNoneVisitor {
    type Value = Option<u64>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("u64, false, or null")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v == false {
            Ok(None)
        } else {
            Err(E::invalid_value(Unexpected::Bool(v), &self))
        }
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Some(v))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(None)
    }
}

pub fn deserialize_u64_or_none_with_negative_as_none<'de, D>(
    deserializer: D,
) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeU64OrNoneWithNegativeAsNoneVisitor)
}

struct DeserializeU64OrNoneWithNegativeAsNoneVisitor;

impl de::Visitor<'_> for DeserializeU64OrNoneWithNegativeAsNoneVisitor {
    type Value = Option<u64>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("u64, false, null, or negative number")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v == false {
            Ok(None)
        } else {
            Err(E::invalid_value(Unexpected::Bool(v), &self))
        }
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Some(v))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v < 0 { Ok(None) } else { Ok(Some(v as u64)) }
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(None)
    }
}

pub fn deserialize_u64_or_none_with_zero_as_none<'de, D>(
    deserializer: D,
) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeU64OrNoneWithZeroAsNoneVisitor)
}

struct DeserializeU64OrNoneWithZeroAsNoneVisitor;

impl de::Visitor<'_> for DeserializeU64OrNoneWithZeroAsNoneVisitor {
    type Value = Option<u64>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("u64, false, or null")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v == false {
            Ok(None)
        } else {
            Err(E::invalid_value(Unexpected::Bool(v), &self))
        }
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v == 0 { Ok(None) } else { Ok(Some(v)) }
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(None)
    }
}

struct DeserializeEmptyArrayOrHashMapVisitor<K, V>(PhantomData<(K, V)>);

impl<'de, K, V> de::Visitor<'de> for DeserializeEmptyArrayOrHashMapVisitor<K, V>
where
    K: DeserializeOwned + std::hash::Hash + Eq,
    V: DeserializeOwned,
{
    type Value = std::collections::HashMap<K, V>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("empty array or a HashMap")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        if seq.next_element::<Self::Value>()?.is_none() {
            // It's an empty array
            Ok(std::collections::HashMap::new())
        } else {
            // not an empty array
            Err(serde::de::Error::invalid_type(Unexpected::Seq, &self))
        }
    }

    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
    where
        A: de::MapAccess<'de>,
    {
        std::collections::HashMap::deserialize(de::value::MapAccessDeserializer::new(map))
    }
}

pub fn deserialize_empty_array_or_hashmap<'de, D, K, V>(
    deserializer: D,
) -> Result<std::collections::HashMap<K, V>, D::Error>
where
    D: Deserializer<'de>,
    K: DeserializeOwned + std::hash::Hash + Eq,
    V: DeserializeOwned,
{
    deserializer.deserialize_any(DeserializeEmptyArrayOrHashMapVisitor::<K, V>(PhantomData))
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

struct DeserializeEmptyVecOrNone<T>(PhantomData<T>);

impl<'de, T> de::Visitor<'de> for DeserializeEmptyVecOrNone<T>
where
    T: Deserialize<'de>,
{
    type Value = Option<T>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("empty Vec or T")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        if seq.next_element::<T>()?.is_none() {
            // It's an empty vec
            Ok(None)
        } else {
            // not an empty vec
            Err(serde::de::Error::invalid_type(Unexpected::Seq, &self))
        }
    }

    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
    where
        A: de::MapAccess<'de>,
    {
        T::deserialize(de::value::MapAccessDeserializer::new(map)).map(Some)
    }
}

pub fn deserialize_empty_vec_or_none<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    deserializer.deserialize_any(DeserializeEmptyVecOrNone::<T>(PhantomData))
}

pub fn deserialize_null_as_empty_vec<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    deserializer.deserialize_any(DeserializeNullAsEmptyVec::<T>(PhantomData))
}

struct DeserializeNullAsEmptyVec<T>(PhantomData<T>);

impl<'de, T> de::Visitor<'de> for DeserializeNullAsEmptyVec<T>
where
    T: Deserialize<'de>,
{
    type Value = Vec<T>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("empty Vec or T")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        let mut vec = Vec::new();
        while let Some(elem) = seq.next_element::<T>()? {
            vec.push(elem);
        }
        Ok(vec)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Vec::new())
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use serde::Deserialize;
    use std::collections::HashMap;

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

    #[derive(Debug, Deserialize)]
    pub struct U64OrNone {
        #[serde(deserialize_with = "deserialize_u64_or_none")]
        pub u64: Option<u64>,
    }

    #[rstest]
    #[case(r#"{"u64": 1}"#, Some(1))]
    #[case(r#"{"u64": false}"#, None)]
    #[case(r#"{"u64": null}"#, None)]
    fn test_deserialize_u64_or_none(#[case] test_case: &str, #[case] expected_result: Option<u64>) {
        let u64_or_none: U64OrNone =
            serde_json::from_str(test_case).expect("Test case should be a valid JSON");
        assert_eq!(expected_result, u64_or_none.u64);
    }

    #[rstest]
    #[case(
        r#"{"u64": -1}"#,
        r#"invalid type: integer `-1`, expected u64, false, or null at line 1 column 10"#
    )]
    #[case(
        r#"{"u64": "1"}"#,
        r#"invalid type: string "1", expected u64, false, or null at line 1 column 11"#
    )]
    #[case(
        r#"{"u64": true}"#,
        r#"invalid value: boolean `true`, expected u64, false, or null at line 1 column 12"#
    )]
    fn test_deserialize_u64_or_none_errors(
        #[case] test_case: &str,
        #[case] expected_error_message: &str,
    ) {
        let u64_or_none: Result<U64OrNone, serde_json::Error> = serde_json::from_str(test_case);
        assert!(u64_or_none.is_err(), "The serializer should emit an error");
        assert_eq!(
            u64_or_none.err().unwrap().to_string(),
            expected_error_message
        );
    }

    #[derive(Debug, Deserialize)]
    pub struct U64OrNoneWithZeroAsNone {
        #[serde(deserialize_with = "deserialize_u64_or_none_with_zero_as_none")]
        pub u64: Option<u64>,
    }

    #[rstest]
    #[case(r#"{"u64": 1}"#, Some(1))]
    #[case(r#"{"u64": false}"#, None)]
    #[case(r#"{"u64": null}"#, None)]
    #[case(r#"{"u64": 0}"#, None)]
    fn test_deserialize_u64_or_none_with_zero_as_none(
        #[case] test_case: &str,
        #[case] expected_result: Option<u64>,
    ) {
        let u64_or_none: U64OrNoneWithZeroAsNone =
            serde_json::from_str(test_case).expect("Test case should be a valid JSON");
        assert_eq!(expected_result, u64_or_none.u64);
    }

    #[rstest]
    #[case(
        r#"{"u64": "1"}"#,
        r#"invalid type: string "1", expected u64, false, or null at line 1 column 11"#
    )]
    #[case(
        r#"{"u64": true}"#,
        r#"invalid value: boolean `true`, expected u64, false, or null at line 1 column 12"#
    )]
    fn test_deserialize_u64_or_none_with_zero_as_none_errors(
        #[case] test_case: &str,
        #[case] expected_error_message: &str,
    ) {
        let u64_or_none: Result<U64OrNoneWithZeroAsNone, serde_json::Error> =
            serde_json::from_str(test_case);
        assert!(u64_or_none.is_err(), "The serializer should emit an error");
        assert_eq!(
            u64_or_none.err().unwrap().to_string(),
            expected_error_message
        );
    }

    #[derive(Debug, Deserialize)]
    pub struct U64OrNoneWithNegativeAsNone {
        #[serde(deserialize_with = "deserialize_u64_or_none_with_negative_as_none")]
        pub u64: Option<u64>,
    }

    #[rstest]
    #[case(r#"{"u64": 1}"#, Some(1))]
    #[case(r#"{"u64": false}"#, None)]
    #[case(r#"{"u64": null}"#, None)]
    #[case(r#"{"u64": -1}"#, None)]
    fn test_deserialize_u64_or_none_with_negative_as_none(
        #[case] test_case: &str,
        #[case] expected_result: Option<u64>,
    ) {
        let u64_or_none: U64OrNoneWithNegativeAsNone =
            serde_json::from_str(test_case).expect("Test case should be a valid JSON");
        assert_eq!(expected_result, u64_or_none.u64);
    }

    #[rstest]
    #[case(r#"{"u64": "1"}"#, r#"invalid type: string "1", expected u64, false, null, or negative number at line 1 column 11"#)]
    #[case(r#"{"u64": true}"#, r#"invalid value: boolean `true`, expected u64, false, null, or negative number at line 1 column 12"#)]
    fn test_deserialize_u64_or_none_with_negative_as_none_errors(
        #[case] test_case: &str,
        #[case] expected_error_message: &str,
    ) {
        let u64_or_none: Result<U64OrNoneWithNegativeAsNone, serde_json::Error> =
            serde_json::from_str(test_case);
        assert!(u64_or_none.is_err(), "The serializer should emit an error");
        assert_eq!(
            u64_or_none.err().unwrap().to_string(),
            expected_error_message
        );
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

    #[derive(Debug, Deserialize, PartialEq, Eq)]
    pub struct OptionStructOrEmptyArray {
        #[serde(deserialize_with = "deserialize_empty_vec_or_none")]
        pub value: Option<OptionStructOrEmptyArrayInner>,
    }

    #[derive(Debug, Deserialize, PartialEq, Eq)]
    pub struct OptionStructOrEmptyArrayInner {
        foo: String,
    }

    #[rstest]
    #[case(r#"{"value": {"foo": "bar"}}"#, Some(OptionStructOrEmptyArrayInner { foo: "bar".to_string() }))]
    #[case(r#"{"value": []}"#, None)]
    fn test_deserialize_empty_vec_or_none(
        #[case] test_case: &str,
        #[case] expected_result: Option<OptionStructOrEmptyArrayInner>,
    ) {
        let option_struct: OptionStructOrEmptyArray =
            serde_json::from_str(test_case).expect("Test case should be a valid JSON");
        assert_eq!(expected_result, option_struct.value);
    }

    #[derive(Debug, Deserialize)]
    pub struct NullAsEmptyVec {
        #[serde(deserialize_with = "deserialize_null_as_empty_vec")]
        pub value: Vec<String>,
    }

    #[rstest]
    #[case(r#"{"value": ["string", "string2"]}"#, vec!["string".to_string(), "string2".to_string()])]
    #[case(r#"{"value": []}"#, vec![])]
    #[case(r#"{"value": null}"#, vec![])]
    fn test_deserialize_null_as_empty_vec(
        #[case] test_case: &str,
        #[case] expected_result: Vec<String>,
    ) {
        let null_as_empty_vec: NullAsEmptyVec =
            serde_json::from_str(test_case).expect("Test case should be a valid JSON");
        assert_eq!(expected_result, null_as_empty_vec.value);
    }
}

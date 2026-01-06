use serde::{
    Deserialize,
    de::{self, Unexpected},
};
use std::{fmt, marker::PhantomData};

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
    D: de::Deserializer<'de>,
    T: Deserialize<'de>,
{
    deserializer.deserialize_any(DeserializeEmptyVecOrNone::<T>(PhantomData))
}

pub fn deserialize_null_as_empty_vec<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: de::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<Vec<T>>::deserialize(deserializer)
        .map(|opt| opt.unwrap_or_default())
        .map_err(|err| {
            serde::de::Error::custom(
                err.to_string()
                    .replace("expected a sequence", "expected null or a sequence"),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use serde::Deserialize;

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

    #[rstest]
    #[case(
        r#"{"value": false}"#,
        r#"invalid type: boolean `false`, expected null or a sequence at line 1 column 15"#
    )]
    fn test_deserialize_null_as_empty_vec_errors(
        #[case] test_case: &str,
        #[case] expected_error_message: &str,
    ) {
        let null_as_empty_vec: Result<NullAsEmptyVec, serde_json::Error> =
            serde_json::from_str(test_case);
        assert!(
            null_as_empty_vec.is_err(),
            "The serializer should emit an error"
        );
        assert_eq!(
            null_as_empty_vec.err().unwrap().to_string(),
            expected_error_message
        );
    }
}

use serde::{Serialize, Serializer, de::{self, DeserializeOwned}, ser};
use std::{fmt, marker::PhantomData};

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

pub fn deserialize_from_string_of_json_array<'de, T, D>(
    deserializer: D,
) -> Result<Option<Vec<T>>, D::Error>
where
    T: DeserializeOwned,
    D: de::Deserializer<'de>,
{
    deserializer.deserialize_any(StringOfJsonArrayVisitor::<T>(PhantomData))
}

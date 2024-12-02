use core::fmt;

use serde::{
    de::{self, MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer,
};

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
    struct DefaultValuesVisitor<V> {
        _marker: std::marker::PhantomData<V>,
    }

    impl<'de, V> Visitor<'de> for DefaultValuesVisitor<V>
    where
        V: Deserialize<'de> + Default,
    {
        type Value = V;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a boolean false or any other value")
        }

        fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if !v {
                Ok(V::default())
            } else {
                Deserialize::deserialize(de::value::BoolDeserializer::new(v))
            }
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if v.is_empty() {
                Ok(V::default())
            } else {
                Deserialize::deserialize(de::value::StrDeserializer::new(v))
            }
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if v.is_empty() {
                Ok(V::default())
            } else {
                Deserialize::deserialize(de::value::StringDeserializer::new(v))
            }
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            if seq.next_element::<serde::de::IgnoredAny>()?.is_some() {
                return Err(de::Error::invalid_type(de::Unexpected::Seq, &self));
            }
            Ok(V::default())
        }

        fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            Deserialize::deserialize(de::value::MapAccessDeserializer::new(map))
        }
    }

    deserializer.deserialize_any(DefaultValuesVisitor::<V> {
        _marker: std::marker::PhantomData,
    })
}

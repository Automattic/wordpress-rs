#![allow(dead_code)]

use serde::{
    de::{self, MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer,
};

use std::{
    collections::HashMap,
    fmt::{self, Debug},
};

#[derive(Deserialize, Debug)]
pub struct PluginInformation {
    pub name: String,
    pub slug: String,
    pub version: String,
    pub author: String,
    #[serde(deserialize_with = "deserialize_default_values")]
    pub author_profile: String,
    #[serde(deserialize_with = "deserialize_default_values")]
    pub contributors: HashMap<String, ContributorDetails>,
    #[serde(deserialize_with = "deserialize_default_values")]
    pub requires: String,
    #[serde(deserialize_with = "deserialize_default_values")]
    pub tested: String,
    #[serde(deserialize_with = "deserialize_default_values")]
    pub requires_php: String,
    pub requires_plugins: Vec<String>,
    pub rating: u32,
    pub ratings: Ratings,
    pub num_ratings: u32,
    pub support_url: String,
    pub support_threads: u32,
    pub support_threads_resolved: u32,
    pub active_installs: u64,
    pub last_updated: String,
    pub added: String,
    pub homepage: String,
    pub sections: HashMap<String, String>,
    pub download_link: String,
    #[serde(deserialize_with = "deserialize_default_values")]
    pub upgrade_notice: HashMap<String, String>,
    pub screenshots: Screenshots,
    #[serde(deserialize_with = "deserialize_default_values")]
    pub tags: HashMap<String, String>,
    #[serde(deserialize_with = "deserialize_default_values")]
    pub versions: HashMap<String, String>,
    #[serde(deserialize_with = "deserialize_default_values")]
    pub business_model: String,
    pub repository_url: String,
    pub commercial_support_url: String,
    pub donate_link: String,
    #[serde(deserialize_with = "deserialize_default_values")]
    pub banners: Banners,
    pub icons: Option<Icons>,
    pub preview_link: String,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
pub struct ContributorDetails {
    pub profile: String,
    pub avatar: String,
    pub display_name: String,
}

#[derive(Deserialize, Debug)]
pub struct Ratings {
    #[serde(rename = "5")]
    pub five_star: u32,
    #[serde(rename = "4")]
    pub four_star: u32,
    #[serde(rename = "3")]
    pub three_star: u32,
    #[serde(rename = "2")]
    pub two_star: u32,
    #[serde(rename = "1")]
    pub one_star: u32,
}

/// https://developer.wordpress.org/plugins/wordpress-org/plugin-assets/#screenshots
#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum Screenshots {
    Named(HashMap<String, Screenshot>),
    List(Vec<Screenshot>),
}

#[derive(Deserialize, Debug)]
pub struct Screenshot {
    pub src: String,
    pub caption: String,
}

#[derive(Deserialize, Debug, Eq, PartialEq, Default)]
pub struct Banners {
    #[serde(deserialize_with = "deserialize_default_values")]
    pub low: String,
    #[serde(deserialize_with = "deserialize_default_values")]
    pub high: String,
}

#[derive(Deserialize, Debug)]
pub struct Icons {
    #[serde(rename = "1x")]
    pub low: Option<String>,
    #[serde(rename = "2x")]
    pub high: Option<String>,
    pub svg: Option<String>,
    pub default: Option<String>,
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
fn deserialize_default_values<'de, D, V>(deserializer: D) -> Result<V, D::Error>
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

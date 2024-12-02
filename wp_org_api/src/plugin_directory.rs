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
struct PluginInformation {
    name: String,
    slug: String,
    version: String,
    author: String,
    #[serde(deserialize_with = "deserialize_default_values")]
    author_profile: String,
    #[serde(deserialize_with = "deserialize_default_values")]
    contributors: HashMap<String, ContributorDetails>,
    #[serde(deserialize_with = "deserialize_default_values")]
    requires: String,
    #[serde(deserialize_with = "deserialize_default_values")]
    tested: String,
    #[serde(deserialize_with = "deserialize_default_values")]
    requires_php: String,
    requires_plugins: Vec<String>,
    rating: u32,
    ratings: Ratings,
    num_ratings: u32,
    support_url: String,
    support_threads: u32,
    support_threads_resolved: u32,
    active_installs: u64,
    last_updated: String,
    added: String,
    homepage: String,
    sections: HashMap<String, String>,
    download_link: String,
    #[serde(deserialize_with = "deserialize_default_values")]
    upgrade_notice: HashMap<String, String>,
    screenshots: Screenshots,
    #[serde(deserialize_with = "deserialize_default_values")]
    tags: HashMap<String, String>,
    #[serde(deserialize_with = "deserialize_default_values")]
    versions: HashMap<String, String>,
    #[serde(deserialize_with = "deserialize_default_values")]
    business_model: String,
    repository_url: String,
    commercial_support_url: String,
    donate_link: String,
    #[serde(deserialize_with = "deserialize_default_values")]
    banners: Banners,
    icons: Option<Icons>,
    preview_link: String,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct ContributorDetails {
    profile: String,
    avatar: String,
    display_name: String,
}

#[derive(Deserialize, Debug)]
struct Ratings {
    #[serde(rename = "5")]
    five_star: u32,
    #[serde(rename = "4")]
    four_star: u32,
    #[serde(rename = "3")]
    three_star: u32,
    #[serde(rename = "2")]
    two_star: u32,
    #[serde(rename = "1")]
    one_star: u32,
}

/// https://developer.wordpress.org/plugins/wordpress-org/plugin-assets/#screenshots
#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum Screenshots {
    Named(HashMap<String, Screenshot>),
    List(Vec<Screenshot>),
}

#[derive(Deserialize, Debug)]
struct Screenshot {
    src: String,
    caption: String,
}

#[derive(Deserialize, Debug, Eq, PartialEq, Default)]
struct Banners {
    #[serde(deserialize_with = "deserialize_default_values")]
    low: String,
    #[serde(deserialize_with = "deserialize_default_values")]
    high: String,
}

#[derive(Deserialize, Debug)]
struct Icons {
    #[serde(rename = "1x")]
    low: Option<String>,
    #[serde(rename = "2x")]
    high: Option<String>,
    svg: Option<String>,
    default: Option<String>,
}

/// The plugin directory API may return different types of values for the same
/// property.
///
/// Here are a couple of examples:
/// - The "requires" field is supposed to be the WordPress version string, but
///   it can also be `fasle`.
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
                return Ok(V::default());
            } else {
                Deserialize::deserialize(de::value::BoolDeserializer::new(v))
            }
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if v.len() == 0 {
                Ok(V::default())
            } else {
                Deserialize::deserialize(de::value::StrDeserializer::new(v))
            }
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if v.len() == 0 {
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

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use std::path::PathBuf;

    #[fixture]
    fn plugins_dir() -> PathBuf {
        std::fs::canonicalize("../target/wordpress-org-plugin-directory").unwrap()
    }

    #[fixture]
    fn plugin_info_files(plugins_dir: PathBuf) -> Vec<PathBuf> {
        println!(
            "Reading plugin information files from {:?}...",
            &plugins_dir
        );

        let mut files = vec![];
        for entry in std::fs::read_dir(plugins_dir).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_file()
                && entry.path().extension().and_then(|f| f.to_str()) == Some("json")
            {
                files.push(entry.path());
            }
        }
        files
    }

    fn parse_plugin(slug: &str) -> Result<PluginInformation, serde_json::Error> {
        let file = plugins_dir().join(format!("{}.json", slug));
        let content = std::fs::read_to_string(file).unwrap();
        serde_json::from_str::<PluginInformation>(&content)
    }

    #[rstest]
    fn parse_plugin_info(plugin_info_files: Vec<PathBuf>) {
        let results: HashMap<&PathBuf, _> =
            plugin_info_files
                .iter()
                .fold(HashMap::new(), |mut results, file| {
                    let info = std::fs::read_to_string(&file).unwrap();
                    let result = serde_json::from_str::<PluginInformation>(&info);
                    results.insert(&file, result);
                    results
                });

        let success: Vec<_> = results
            .iter()
            .filter(|(_, result)| result.is_ok())
            .collect();

        let failed: Vec<_> = results
            .iter()
            .filter(|(_, result)| result.is_err())
            .map(|(file, _)| file)
            .collect();
        if !failed.is_empty() {
            println!("Failed to parse the following files:");
            for file in &failed {
                println!("- {:?}", file.file_name().unwrap());
            }
        }

        assert_eq!(
            success.len(),
            results.len(),
            "{} out of {} files parsed successfully",
            success.len(),
            results.len()
        );
        assert!(
            failed.is_empty(),
            "Failed to parse {} out of {} files",
            failed.len(),
            results.len()
        );
    }

    #[rstest]
    #[case("jetpack", "https://profiles.wordpress.org/automattic/")]
    #[case("appmysite", "https://profiles.wordpress.org/appmysite/")]
    #[case("superlinks", "")]
    fn test_property_author_profile(#[case] slug: &str, #[case] value: &str) {
        let result = parse_plugin(slug);
        assert!(
            result.is_ok(),
            "Failed to parse plugin {:?}: {:?}",
            slug,
            result.err()
        );

        assert_eq!(result.unwrap().author_profile, value);
    }

    #[rstest]
    #[case("jetpack", "automattic", "https://profiles.wordpress.org/automattic/")]
    #[case("appmysite", "appmysite", "https://profiles.wordpress.org/appmysite/")]
    fn test_property_contributors_profile(
        #[case] slug: &str,
        #[case] username: &str,
        #[case] profile: &str,
    ) {
        let result = parse_plugin(slug);
        assert!(
            result.is_ok(),
            "Failed to parse plugin {:?}: {:?}",
            slug,
            result.err()
        );

        let contributors = result.unwrap().contributors;
        assert_eq!(
            contributors.get(username).map(|f| &f.profile),
            Some(&profile.to_string())
        );
    }

    #[rstest]
    #[case("superlinks")]
    fn test_property_no_contributors(#[case] slug: &str) {
        let result = parse_plugin(slug);
        assert!(
            result.is_ok(),
            "Failed to parse plugin {:?}: {:?}",
            slug,
            result.err()
        );

        assert!(result.unwrap().contributors.is_empty());
    }

    #[rstest]
    #[case("timeline-express-no-icons-add-on", "")]
    #[case("appmysite", "6.4")]
    #[case("superlinks", "2.5")]
    fn test_property_requires(#[case] slug: &str, #[case] value: &str) {
        let result = parse_plugin(slug);
        assert!(
            result.is_ok(),
            "Failed to parse plugin {:?}: {:?}",
            slug,
            result.err()
        );

        assert_eq!(result.unwrap().requires, value);
    }

    #[rstest]
    #[case("jetpack", "6.7.1")]
    #[case("add-rss", "")]
    fn test_property_tested(#[case] slug: &str, #[case] value: &str) {
        let result = parse_plugin(slug);
        assert!(
            result.is_ok(),
            "Failed to parse plugin {:?}: {:?}",
            slug,
            result.err()
        );

        assert_eq!(result.unwrap().tested, value);
    }

    #[rstest]
    #[case("about-author", "")]
    #[case("accessibility-toolbar", "7.4")]
    fn test_property_requires_php(#[case] slug: &str, #[case] value: &str) {
        let result = parse_plugin(slug);
        assert!(
            result.is_ok(),
            "Failed to parse plugin {:?}: {:?}",
            slug,
            result.err()
        );

        assert_eq!(result.unwrap().requires_php, value);
    }

    #[rstest]
    #[case("accordion-archive-widget", "")]
    #[case("abc-pricing-table", "commercial")]
    fn test_property_business_model(#[case] slug: &str, #[case] value: &str) {
        let result = parse_plugin(slug);
        assert!(
            result.is_ok(),
            "Failed to parse plugin {:?}: {:?}",
            slug,
            result.err()
        );

        assert_eq!(result.unwrap().business_model, value);
    }

    #[rstest]
    fn test_property_empty_upgrade_notice(#[values("1-click-close-store", "2em")] slug: &str) {
        let result = parse_plugin(slug);
        assert!(
            result.is_ok(),
            "Failed to parse plugin {:?}: {:?}",
            slug,
            result.err()
        );

        assert!(result.unwrap().upgrade_notice.is_empty());
    }

    #[rstest]
    fn test_property_nonempty_upgrade_notice(
        #[values("ab-wp-security", "absolute-addons")] slug: &str,
    ) {
        let result = parse_plugin(slug);
        assert!(
            result.is_ok(),
            "Failed to parse plugin {:?}: {:?}",
            slug,
            result.err()
        );

        assert!(!result.unwrap().upgrade_notice.is_empty());
    }

    #[rstest]
    fn test_property_empty_tags(#[values("acf-rest", "add-rss")] slug: &str) {
        let result = parse_plugin(slug);
        assert!(
            result.is_ok(),
            "Failed to parse plugin {:?}: {:?}",
            slug,
            result.err()
        );

        assert!(result.unwrap().tags.is_empty());
    }

    #[rstest]
    fn test_property_nonempty_tags(#[values("appbanners", "seo-assistant")] slug: &str) {
        let result = parse_plugin(slug);
        assert!(
            result.is_ok(),
            "Failed to parse plugin {:?}: {:?}",
            slug,
            result.err()
        );

        assert!(!result.unwrap().tags.is_empty());
    }

    #[rstest]
    fn test_property_empty_versions(#[values("mos-faqs", "adjustly-collapse")] slug: &str) {
        let result = parse_plugin(slug);
        assert!(
            result.is_ok(),
            "Failed to parse plugin {:?}: {:?}",
            slug,
            result.err()
        );

        assert!(result.unwrap().versions.is_empty());
    }

    #[rstest]
    fn test_property_nonempty_versions(#[values("abcsubmit", "acf-views")] slug: &str) {
        let result = parse_plugin(slug);
        assert!(
            result.is_ok(),
            "Failed to parse plugin {:?}: {:?}",
            slug,
            result.err()
        );

        assert!(!result.unwrap().versions.is_empty());
    }

    #[rstest]
    #[case(
        "appmysite",
        "https://ps.w.org/appmysite/assets/banner-772x250.png?rev=2829272",
        "https://ps.w.org/appmysite/assets/banner-1544x500.png?rev=2829272"
    )]
    #[case(
        "jetpack",
        "https://ps.w.org/jetpack/assets/banner-772x250.png?rev=2653649",
        "https://ps.w.org/jetpack/assets/banner-1544x500.png?rev=2653649"
    )]
    #[case(
        "1-click-migration",
        "https://ps.w.org/1-click-migration/assets/banner-772x250.png?rev=2333853",
        ""
    )]
    fn test_property_banners(#[case] slug: &str, #[case] low: String, #[case] high: String) {
        let result = parse_plugin(slug);
        assert!(
            result.is_ok(),
            "Failed to parse plugin {:?}: {:?}",
            slug,
            result.err()
        );

        let expected = Banners { low, high };
        let banners = result.unwrap().banners;
        assert_eq!(banners, expected);
    }

    #[rstest]
    #[case(
        "contact-form-7",
        Some("https://ps.w.org/contact-form-7/assets/icon.svg?rev=2339255"),
        None,
        Some("https://ps.w.org/contact-form-7/assets/icon.svg?rev=2339255"),
        None
    )]
    #[case(
        "adminimize",
        None,
        None,
        None,
        Some("https://s.w.org/plugins/geopattern-icon/adminimize_000000.svg")
    )]
    fn test_property_icons(
        #[case] slug: &str,
        #[case] low: Option<&str>,
        #[case] high: Option<&str>,
        #[case] svg: Option<&str>,
        #[case] default: Option<&str>,
    ) {
        let result = parse_plugin(slug);
        assert!(
            result.is_ok(),
            "Failed to parse plugin {:?}: {:?}",
            slug,
            result.err()
        );

        let icons = result.unwrap().icons;
        assert!(icons.is_some());

        let icons = icons.unwrap();
        assert_eq!(icons.low.as_deref(), low);
        assert_eq!(icons.high.as_deref(), high);
        assert_eq!(icons.svg.as_deref(), svg);
        assert_eq!(icons.default.as_deref(), default);
    }
}

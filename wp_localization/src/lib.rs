use fluent_bundle::FluentValue;
use fluent_langneg::{convert_vec_str_to_langids_lossy, negotiate_languages, NegotiationStrategy};
use fluent_templates::Loader;
use std::{collections::HashMap, fmt::Debug, fmt::Display};
use unic_langid::{langid, LanguageIdentifier};

fluent_templates::static_loader! {
    static LOCALES = {
        locales: "./localization",
        fallback_language: "en-US"
    };
}

#[wp_localization_macro::wp_messages]
pub struct WpMessages {}

#[derive(Debug)]
pub struct MessageBundle {
    // Keep the fields private to avoid clients from looking up messages that do not exist.
    key: &'static str,
    args: Option<HashMap<&'static str, String>>,
}

impl MessageBundle {
    // Keep this private to avoid clients from looking up messages that do not exist.
    fn new(key: &'static str, args: Option<HashMap<&'static str, String>>) -> Self {
        Self { key, args }
    }

    pub fn key(&self) -> &'static str {
        self.key
    }

    pub fn args(&self) -> Option<&HashMap<&'static str, String>> {
        self.args.as_ref()
    }

    pub fn localize(&self, locale: Option<WpLocale>) -> String {
        LOCALES.lookup_complete(
            locale.unwrap_or_default().as_language_id(),
            self.key,
            self.args
                .as_ref()
                .map(|h| {
                    h.iter()
                        .map(|(k, v)| ((*k).into(), FluentValue::from(v)))
                        .collect()
                })
                .as_ref(),
        )
    }
}

impl Display for MessageBundle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.localize(None))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WpLocale {
    lang_id: LanguageIdentifier,
}

// Export `WpLocale` as a string (i.e. "en-US"). Native platforms can pass any string
// to Rust code. Rust code will convert it to a `WpLocale` object, which would be one
// of the supported languages.
uniffi::custom_type!(WpLocale, String, {
    lower: |locale| locale.into(),
    try_lift: |str| Ok(str.as_str().into()),
});

include!(concat!(env!("OUT_DIR"), "/generated_wp_locale.rs"));

impl WpLocale {
    pub fn as_language_id(&self) -> &LanguageIdentifier {
        &self.lang_id
    }
}

impl Default for WpLocale {
    fn default() -> Self {
        WpLocale {
            lang_id: langid!("en-US"),
        }
    }
}

impl From<&str> for WpLocale {
    fn from(lang_id: &str) -> Self {
        vec![lang_id].into()
    }
}

impl<'a> From<Vec<&'a str>> for WpLocale {
    fn from(lang_ids: Vec<&'a str>) -> Self {
        let requested = convert_vec_str_to_langids_lossy(&lang_ids);
        let supported = negotiate_languages(
            &requested,
            AVAILABLE_LANGUAGES,
            None,
            NegotiationStrategy::Filtering,
        );

        if supported.is_empty() {
            return WpLocale::default();
        }

        WpLocale {
            lang_id: supported[0].clone(),
        }
    }
}

impl From<WpLocale> for String {
    fn from(locale: WpLocale) -> Self {
        locale.lang_id.to_string()
    }
}

#[uniffi::export]
fn wp_locale_resolve(lang_ids: Vec<String>) -> WpLocale {
    lang_ids
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<&str>>()
        .into()
}

pub trait WpSupportsLocalization: Send + Sync + Debug {
    fn message_bundle(&self) -> MessageBundle;
}

#[uniffi::export(with_foreign)]
pub trait WpLocalizable: Send + Sync + Debug {
    fn localize(&self, locale: Option<WpLocale>) -> String;
}

impl<T: WpSupportsLocalization> WpLocalizable for T {
    fn localize(&self, locale: Option<WpLocale>) -> String {
        self.message_bundle().localize(locale)
    }
}

uniffi::setup_scaffolding!();

#[cfg(test)]
mod language_identifier_tests {
    use super::*;

    #[test]
    fn test_ensure_all_localization_message_files_exist() {
        let localization_dir = std::path::Path::new("localization");
        assert!(
            localization_dir.exists(),
            "Localization directory should exist"
        );

        assert!(
            !AVAILABLE_LANGUAGES.is_empty(),
            "At least one language should be available"
        );
        for lang_id in AVAILABLE_LANGUAGES {
            let lang_str = lang_id.to_string();
            let dir_path = localization_dir.join(&lang_str);
            assert!(
                dir_path.exists(),
                "Language directory '{}' should exist in localization directory",
                lang_str
            );
        }

        let default_lang = WpLocale::default().lang_id.to_string();
        let dir_path = localization_dir.join(&default_lang);
        assert!(
            dir_path.exists(),
            "The default language directory '{}' should exist in localization directory",
            default_lang
        );
    }
}

#[cfg(test)]
mod localization_tests {
    use super::*;
    use rstest::*;
    use wp_localization_macro::WpDeriveLocalizable;

    #[derive(Debug, Clone, thiserror::Error, uniffi::Error, WpDeriveLocalizable)]
    pub enum ParseApiRootUrlError {
        ApiRootLinkHeaderNotFound {
            status_code: u16,
            header_map: String,
        },
    }

    impl WpSupportsLocalization for ParseApiRootUrlError {
        fn message_bundle(&self) -> crate::MessageBundle {
            match self {
                ParseApiRootUrlError::ApiRootLinkHeaderNotFound { .. } => {
                    WpMessages::api_root_link_header_not_found()
                }
            }
        }
    }

    #[test]
    fn test_example_localizable_error() {
        let expected_en_message = "Api root link header not found!\nStatus Code: '\u{2068}404\u{2069}'\nHeader Map: '\u{2068}foo\u{2069}'";
        let expected_tr_message = "Api kök bağlantı başlığı bulunamadı!\nDurum kodu: '\u{2068}404\u{2069}'\nBaşlık Haritası: '\u{2068}foo\u{2069}'";
        {
            let map = {
                let mut map = HashMap::new();
                map.insert("status_code".into(), "404".into());
                map.insert("header_map".into(), "foo".into());
                map
            };
            assert_eq!(
                LOCALES.lookup_with_args(
                    WpLocale::en_us().as_language_id(),
                    "api_root_link_header_not_found",
                    &map
                ),
                expected_en_message
            );
            assert_eq!(
                LOCALES.lookup_with_args(
                    WpLocale::tr_tr().as_language_id(),
                    "api_root_link_header_not_found",
                    &map
                ),
                expected_tr_message
            );
        }
        let api_root_link_header_not_found = ParseApiRootUrlError::ApiRootLinkHeaderNotFound {
            status_code: 404,
            header_map: "foo".to_string(),
        };

        let message_bundle = api_root_link_header_not_found.message_bundle();
        assert_eq!(message_bundle.key, "api_root_link_header_not_found");
        let message_args = message_bundle.args.unwrap();
        assert_eq!(message_args["status_code"], "404");
        assert_eq!(message_args["header_map"], "foo");
        assert_eq!(
            api_root_link_header_not_found.to_string(),
            expected_en_message
        );
        assert_eq!(
            api_root_link_header_not_found.localize(Some(WpLocale::tr_tr())),
            expected_tr_message
        );
    }

    #[rstest]
    fn test_parse_unknown_language() {
        let locale = WpLocale::from(vec!["unknown-language"]);
        assert_eq!(locale.lang_id.to_string(), "en-US");
    }

    #[rstest]
    #[case(vec!["en-US"], "en-US")]
    #[case(vec!["tr-TR"], "tr-TR")]
    #[case(vec!["en-US", "tr-TR"], "en-US")]
    #[case(vec!["unknown-lang", "en-US", "tr-TR"], "en-US")]
    #[case(vec!["unknown-lang", "tr-TR", "en-US"], "tr-TR")]
    fn test_parse_language(#[case] lang_ids: Vec<&str>, #[case] expected: &str) {
        let locale = WpLocale::from(lang_ids);
        assert_eq!(locale.lang_id.to_string(), expected);
    }
}

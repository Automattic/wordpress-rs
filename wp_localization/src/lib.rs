use fluent_bundle::FluentValue;
use fluent_langneg::{NegotiationStrategy, convert_vec_str_to_langids_lossy, negotiate_languages};
use fluent_templates::Loader;
use std::{
    borrow::Cow,
    collections::HashMap,
    fmt::{Debug, Display},
    marker::PhantomData,
};
use unic_langid::{LanguageIdentifier, langid};

fluent_templates::static_loader! {
    static LOCALES = {
        locales: "./localization",
        fallback_language: "en-US"
    };
}

#[wp_localization_macro::wp_messages]
pub struct WpMessages<'a> {
    _phantom: PhantomData<&'a ()>,
}

#[derive(Debug)]
pub struct MessageBundle<'a> {
    // Keep the fields private to avoid clients from looking up messages that do not exist.
    key: &'static str,
    args: Option<HashMap<Cow<'static, str>, FluentValue<'a>>>,
}

impl<'a> MessageBundle<'a> {
    // Keep this private to avoid clients from looking up messages that do not exist.
    fn new(key: &'static str, args: Option<HashMap<Cow<'static, str>, FluentValue<'a>>>) -> Self {
        Self { key, args }
    }

    pub fn key(&self) -> &'static str {
        self.key
    }

    pub fn args(&self) -> Option<&HashMap<Cow<'static, str>, FluentValue<'a>>> {
        self.args.as_ref()
    }

    pub fn localize(&self, locale: Option<WpLocale>) -> String {
        LOCALES.lookup_complete(
            locale.unwrap_or_default().as_language_id(),
            self.key,
            self.args(),
        )
    }
}

impl Display for MessageBundle<'_> {
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
    fn message_bundle(&self) -> MessageBundle<'_>;
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
    fn test_ensure_locales_are_loaded() {
        assert!(
            !AVAILABLE_LANGUAGES.is_empty(),
            "At least one language should be available"
        );
        assert_eq!(
            AVAILABLE_LANGUAGES.len(),
            LOCALES.locales().count(),
            "The number of available languages should match the number of loaded locales"
        );

        for lang_id in LOCALES.locales() {
            assert!(
                AVAILABLE_LANGUAGES.contains(lang_id),
                "Language identifier '{}' should be available",
                lang_id
            );
        }
    }

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
        Error { message: String },
    }

    impl WpSupportsLocalization for ParseApiRootUrlError {
        fn message_bundle(&self) -> crate::MessageBundle<'_> {
            match self {
                ParseApiRootUrlError::Error { message } => WpMessages::site_error_message(message),
            }
        }
    }

    #[test]
    fn test_example_localizable_error() {
        let expected_en_message = "Your site sent an error message: \u{2068}foo\u{2069}.";
        let expected_tr_message = "Siteniz bir hata mesajı gönderdi: \u{2068}foo\u{2069}.";
        {
            let map = {
                let mut map = HashMap::new();
                map.insert("error_message".into(), "foo".into());
                map
            };
            assert_eq!(
                LOCALES.lookup_with_args(
                    WpLocale::from("en-US").as_language_id(),
                    "site_error_message",
                    &map
                ),
                expected_en_message
            );
            assert_eq!(
                LOCALES.lookup_with_args(
                    WpLocale::from("tr-TR").as_language_id(),
                    "site_error_message",
                    &map
                ),
                expected_tr_message
            );
        }
        let error = ParseApiRootUrlError::Error {
            message: "foo".to_string(),
        };

        let message_bundle = error.message_bundle();
        assert_eq!(message_bundle.key, "site_error_message");
        let message_args = message_bundle.args.unwrap();
        assert_eq!(message_args["error_message"], "foo".into());
        assert_eq!(error.to_string(), expected_en_message);
        assert_eq!(error.localize(Some("tr-TR".into())), expected_tr_message);
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

    #[rstest]
    #[case("en", "en-US")]
    #[case("tr", "tr-TR")]
    // TODO: Add the following cases when we add the translations.
    // #[case("zh-Hans", "zh-CN")]
    // #[case("fr-FR", "fr")]
    fn test_fallback_locale(#[case] lang_ids: &str, #[case] expected: &str) {
        let locale = WpLocale::from(lang_ids);
        assert_eq!(locale.lang_id.to_string(), expected);
    }
}

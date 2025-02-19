use crate::LOCALES;
use fluent_bundle::FluentValue;
use fluent_langneg::{convert_vec_str_to_langids_lossy, negotiate_languages, NegotiationStrategy};
use fluent_templates::Loader;
use std::{collections::HashMap, fmt::Display};

const DEFAULT_LOCALE: &str = "en-US";

#[wp_derive::wp_messages]
pub struct Messages {}

#[derive(Debug, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum FooError {
    #[error("{}", Messages::foo_error_bar())]
    Bar,
    #[error("{}", Messages::foo_error_baz(value))]
    Baz { value: String },
    //#[error("{}", Messages::foo_error_bazzz(value1, value2))]
    //Bazzz { value1: String, value2: String },
}
//
//impl WpLocalizedError for FooError {
//    fn localized_error_message(&self, locale_id: String) -> String {
//        let messages = Messages::get(&locale_id).unwrap_or_default();
//        match self {
//            Self::Bar => messages.foo_bar().to_string(),
//            Self::Baz { value } => messages.foo_error_baz(value).to_string(),
//            Self::Bazzz { value1, value2 } => messages.foo_error_bazzz(value1, value2).to_string(),
//        }
//    }
//}

fn locale_language_id(lang_id: &str) -> unic_langid::LanguageIdentifier {
    // Look up the translated message for `message_key` in `lang_id`.
    let requested = convert_vec_str_to_langids_lossy([lang_id]);
    let default: icu_locid::LanguageIdentifier = icu_locid::langid!("en-US");
    let available: Vec<icu_locid::LanguageIdentifier> = LOCALES
        .locales()
        .filter_map(|f| f.to_string().parse().ok())
        .collect();
    let supported = negotiate_languages(
        &requested,
        &available,
        Some(&default),
        NegotiationStrategy::Filtering,
    );
    supported
        .first()
        .unwrap_or(&&default)
        .to_string()
        .parse()
        .unwrap_or(unic_langid::langid!("en-US"))
}

#[derive(Debug)]
pub struct MessageBundle {
    key: &'static str,
    args: Option<HashMap<&'static str, String>>,
}

impl MessageBundle {
    pub fn new(key: &'static str, args: Option<HashMap<&'static str, String>>) -> Self {
        Self { key, args }
    }

    pub fn with_default_locale(&self) -> String {
        self.localize(DEFAULT_LOCALE)
    }

    pub fn localize(&self, locale: &'static str) -> String {
        LOCALES.lookup_complete(
            &locale_language_id(locale),
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
        write!(f, "{}", self.with_default_locale())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_messages() {
        assert_eq!(localized_message(DEFAULT_LOCALE, "foo_bar"), "Foo is bar");
        assert_eq!(Messages::foo_bar().to_string(), "Foo is bar");
        assert_eq!(
            Messages::foo_bar_with_arg("baz").to_string(),
            "Foo is \u{2068}baz\u{2069}"
        );
    }

    #[test]
    fn test_foo_error() {
        assert_eq!(FooError::Bar.to_string(), "Foo is bar");
        assert_eq!(
            FooError::Baz {
                value: "baz!!".to_string()
            }
            .to_string(),
            "Foo is \u{2068}baz!!\u{2069}"
        );
    }

    fn localized_message(lang_id: &str, message_key: &str) -> String {
        LOCALES.lookup(&locale_language_id(lang_id), message_key)
    }
}

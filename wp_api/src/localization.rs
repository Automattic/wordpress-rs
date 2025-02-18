//#[allow(clippy::all)]
//use fluent_static::MessageBundle;
//
//#[fluent_static::message_bundle(
//    resources = [
//        ("localization/en-US/main.ftl", "en-US"),
//    ],
//    default_language = "en-US"
//)]
//pub struct Messages;
//#[uniffi::export(with_foreign)]
//pub trait WpLocalizedError: Send + Sync {
//    fn localized_error_message(&self, locale_id: String) -> String;
//}
//
//#[derive(Debug, PartialEq, Eq, thiserror::Error, uniffi::Error)]
//pub enum FooError {
//    #[error("{}", Messages::default().foo_error_bar())]
//    Bar,
//    #[error("{}", Messages::default().foo_error_baz(value))]
//    Baz { value: String },
//    #[error("{}", Messages::default().foo_error_bazzz(value1, value2))]
//    Bazzz { value1: String, value2: String },
//}
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
//
//#[cfg(test)]
//mod tests {
//    use super::*;
//
//    #[test]
//    fn test_foo_error() {
//        assert_eq!(FooError::Bar.to_string(), "Foo is bar");
//        assert_eq!(
//            FooError::Baz {
//                value: "custom_baz".to_string()
//            }
//            .to_string(),
//            "Foo is \u{2068}custom_baz\u{2069}"
//        );
//        assert_eq!(
//            FooError::Bazzz {
//                value1: "custom_bazzz1".to_string(),
//                value2: "custom_bazzz2".to_string()
//            }
//            .to_string(),
//            "Foo is \u{2068}custom_bazzz1\u{2069} & \u{2068}custom_bazzz2\u{2069}"
//        );
//    }
//}
//

use crate::LOCALES;
use fluent_bundle::FluentValue;
use fluent_langneg::{convert_vec_str_to_langids_lossy, negotiate_languages, NegotiationStrategy};
use fluent_templates::Loader;
use std::{borrow::Cow, collections::HashMap};

const DEFAULT_LOCALE: &str = "en-US";

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

pub fn localized_message(lang_id: &str, message_key: &str) -> String {
    LOCALES.lookup(&locale_language_id(lang_id), message_key)
}

pub fn localized_message_using_default_locale(message_key: &str) -> String {
    LOCALES.lookup(&locale_language_id(DEFAULT_LOCALE), message_key)
}

pub fn localized_message_with_args(
    lang_id: &str,
    message_key: &str,
    args: &HashMap<Cow<'static, str>, FluentValue>,
) -> String {
    LOCALES.lookup_with_args(&locale_language_id(lang_id), message_key, args)
}

pub fn localized_message_using_default_locale_with_args(
    message_key: &str,
    args: &HashMap<Cow<'static, str>, FluentValue>,
) -> String {
    LOCALES.lookup_with_args(&locale_language_id(DEFAULT_LOCALE), message_key, args)
}

#[wp_derive::wp_translations]
pub struct Translations {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translations() {
        assert_eq!(localized_message(DEFAULT_LOCALE, "foo_bar"), "Foo is bar");
        assert_eq!(Translations::foo_bar(), "Foo is bar");
    }
}

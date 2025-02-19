use crate::LOCALES;
use fluent_bundle::FluentValue;
use fluent_templates::Loader;
use std::{collections::HashMap, fmt::Display};
use strum_macros::IntoStaticStr;

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, IntoStaticStr, uniffi::Enum,
)]
pub enum WpLocale {
    #[default]
    #[strum(serialize = "en-US")]
    EnUS,
}

impl WpLocale {
    pub fn as_language_id(&self) -> unic_langid::LanguageIdentifier {
        Into::<&str>::into(self).parse().expect(
            // TODO: Add the unit tests
            "All locales are unit tested to ensure they can be converted to LanguageIdentifier",
        )
    }
}

#[uniffi::export(with_foreign)]
#[async_trait::async_trait]
pub trait Localizable: Send + Sync {
    async fn localize(&self, locale: WpLocale) -> String;

    async fn with_default_locale(&self) -> String;
}

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
        self.localize(WpLocale::default())
    }

    pub fn localize(&self, locale: WpLocale) -> String {
        LOCALES.lookup_complete(
            &locale.as_language_id(),
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
        assert_eq!(localized_message("foo_bar"), "Foo is bar");
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

    fn localized_message(message_key: &str) -> String {
        LOCALES.lookup(&WpLocale::default().as_language_id(), message_key)
    }
}

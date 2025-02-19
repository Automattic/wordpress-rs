use crate::LOCALES;
use fluent_bundle::FluentValue;
use fluent_templates::Loader;
use std::{collections::HashMap, fmt::Display};
use strum_macros::IntoStaticStr;

mod example {
    use super::*;

    #[derive(Debug, PartialEq, Eq, thiserror::Error, uniffi::Error)]
    pub enum FooError {
        Bar,
        Baz { value: String },
    }

    impl SupportsLocalization for FooError {
        fn message_bundle(&self) -> MessageBundle {
            match self {
                FooError::Bar => Messages::foo_error_bar(),
                FooError::Baz { value } => Messages::foo_error_baz(value),
            }
        }
    }

    impl Display for FooError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.with_default_locale())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

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
    }
}

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

pub trait SupportsLocalization: Send + Sync {
    fn message_bundle(&self) -> MessageBundle;
}

#[uniffi::export(with_foreign)]
pub trait Localizable: Send + Sync {
    fn localize(&self, locale: WpLocale) -> String;
    fn with_default_locale(&self) -> String;
}

impl<T: SupportsLocalization> Localizable for T {
    fn localize(&self, locale: WpLocale) -> String {
        self.message_bundle().localize(locale)
    }

    fn with_default_locale(&self) -> String {
        self.message_bundle().with_default_locale()
    }
}

#[wp_derive::wp_messages]
pub struct Messages {}

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

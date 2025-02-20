use fluent_bundle::FluentValue;
use fluent_templates::Loader;
use std::{collections::HashMap, fmt::Debug, fmt::Display, sync::Arc};
use strum_macros::IntoStaticStr;

fluent_templates::static_loader! {
    static LOCALES = {
        locales: "./localization",
        fallback_language: "en-US"
    };
}

#[derive(Debug, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum ExampleLocalizableError {
    Hello { value: String },
}

impl SupportsLocalization for ExampleLocalizableError {
    fn message_bundle(&self) -> MessageBundle {
        match self {
            Self::Hello { value } => Messages::example_localizable_error_hello(value),
        }
    }
}

impl Display for ExampleLocalizableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message_bundle())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example_localizable_error() {
        assert_eq!(
            ExampleLocalizableError::Hello {
                value: "world".to_string()
            }
            .to_string(),
            "Hello \u{2068}world\u{2069}!"
        );
        assert_eq!(
            ExampleLocalizableError::Hello {
                value: "world".to_string()
            }
            .localize(Some(WpLocale::TrTR)),
            "Merhaba \u{2068}world\u{2069}!"
        );
    }
}

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, IntoStaticStr, uniffi::Enum,
)]
pub enum WpLocale {
    #[default]
    #[strum(serialize = "en-US")]
    EnUS,
    #[strum(serialize = "tr-TR")]
    TrTR,
}

impl WpLocale {
    pub fn as_language_id(&self) -> unic_langid::LanguageIdentifier {
        Into::<&str>::into(self).parse().expect(
            // TODO: Add the unit tests
            "All locales are unit tested to ensure they can be converted to LanguageIdentifier",
        )
    }
}

pub trait SupportsLocalization: Send + Sync + Debug {
    fn message_bundle(&self) -> MessageBundle;
}

#[uniffi::export(with_foreign)]
pub trait Localizable: Send + Sync + Debug {
    fn localize(&self, locale: Option<WpLocale>) -> String;
}

impl<T: SupportsLocalization> Localizable for T {
    fn localize(&self, locale: Option<WpLocale>) -> String {
        self.message_bundle().localize(locale)
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

    pub fn localize(&self, locale: Option<WpLocale>) -> String {
        if let Some(l) = locale {
            println!("lang_id: {:#?}", l.as_language_id());
        }
        LOCALES.lookup_complete(
            &locale.unwrap_or_default().as_language_id(),
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

#[derive(Debug, uniffi::Object)]
struct UniffiLocalizable(Arc<dyn Localizable>);

#[uniffi::export]
impl UniffiLocalizable {
    #[uniffi::constructor]
    fn example_localizable_error(value: ExampleLocalizableError) -> Self {
        Self(Arc::new(value))
    }

    fn localize(&self, locale: Option<WpLocale>) -> String {
        self.0.localize(locale)
    }
}

use fluent_bundle::FluentValue;
use fluent_templates::Loader;
use std::{collections::HashMap, fmt::Debug, fmt::Display};
use strum_macros::{EnumIter, IntoStaticStr};

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
    key: &'static str,
    args: Option<HashMap<&'static str, String>>,
}

impl MessageBundle {
    pub fn new(key: &'static str, args: Option<HashMap<&'static str, String>>) -> Self {
        Self { key, args }
    }

    pub fn localize(&self, locale: Option<WpLocale>) -> String {
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

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    EnumIter,
    IntoStaticStr,
    uniffi::Enum,
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
            "All locales are unit tested to ensure they can be converted to LanguageIdentifier",
        )
    }
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

mod macro_helper {
    #[macro_export]
    macro_rules! display_from_supports_localization {
        ($ident:ident) => {
            paste::paste! {
                impl std::fmt::Display for $ident {
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(f, "{}", self.message_bundle())
                    }
                }
            }
        };
    }
}

uniffi::setup_scaffolding!();

#[cfg(test)]
mod language_identifier_tests {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn test_ensure_all_locales_can_be_parsed_into_language_identifiers() {
        // Note that this _only_ validates that `WpLocale` values can be converted to
        // `unic_langid::LanguageIdentifier`
        // https://docs.rs/unic-langid/latest/unic_langid/struct.LanguageIdentifier.html
        //
        // Since we unwrap the parsing result in `WpLocale::as_language_id`, we use this test to
        // make sure it won't panic.
        WpLocale::iter().for_each(|l| {
            let language_identifier = l.as_language_id();
            assert_eq!(language_identifier.to_string(), Into::<&str>::into(l));
        });
    }
}

#[cfg(test)]
mod localization_tests {
    use super::*;

    #[derive(Debug, Clone, thiserror::Error, uniffi::Error)]
    pub enum ParseApiRootUrlError {
        ApiRootLinkHeaderNotFound {
            status_code: u16,
            header_map: String,
        },
    }

    impl WpSupportsLocalization for ParseApiRootUrlError {
        fn message_bundle(&self) -> crate::MessageBundle {
            match self {
                ParseApiRootUrlError::ApiRootLinkHeaderNotFound {
                    status_code,
                    header_map,
                } => {
                    WpMessages::api_root_link_header_not_found(status_code.to_string(), header_map)
                }
            }
        }
    }

    crate::display_from_supports_localization!(ParseApiRootUrlError);

    #[test]
    fn test_example_localizable_error() {
        let api_root_link_header_not_found = ParseApiRootUrlError::ApiRootLinkHeaderNotFound {
            status_code: 404,
            header_map: "foo".to_string(),
        };
        assert_eq!(
            api_root_link_header_not_found.to_string(),
            "Hello \u{2068}world\u{2069}!"
        );
        assert_eq!(
            api_root_link_header_not_found.localize(Some(WpLocale::TrTR)),
            "Merhaba \u{2068}world\u{2069}!"
        );
    }
}

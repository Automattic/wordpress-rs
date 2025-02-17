use crate::LOCALES;
use fluent_bundle::FluentValue;
use fluent_langneg::{convert_vec_str_to_langids_lossy, negotiate_languages, NegotiationStrategy};
use fluent_templates::Loader;
use std::{borrow::Cow, collections::HashMap};

#[uniffi::export(with_foreign)]
pub trait WpLocalizedError: Send + Sync {
    fn localized_error_message(&self, locale_id: String) -> Option<String>;
}

#[derive(Debug, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum FooError {
    #[error("Foo is bar")]
    Bar,
}

impl WpLocalizedError for FooError {
    fn localized_error_message(&self, lang_id: String) -> Option<String> {
        match self {
            Self::Bar => Some(localized_message(
                &lang_id,
                "auto_discovery_attempt_failure_parse_site_url",
            )),
        }
    }
}

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

pub fn localized_message_with_args(
    lang_id: &str,
    message_key: &str,
    args: &HashMap<Cow<'static, str>, FluentValue>,
) -> String {
    LOCALES.lookup_with_args(&locale_language_id(lang_id), message_key, args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_localized_message() {
        assert_eq!(localized_message("en-US", "foo_bar"), "Foo is bar");
        let args = {
            let mut map = HashMap::new();
            map.insert("bar_arg".into(), "quox".into());
            map
        };
        assert_eq!(
            localized_message_with_args("en-US", "foo_bar_with_arg", &args),
            "Foo is \u{2068}quox\u{2069}"
        );
    }
}

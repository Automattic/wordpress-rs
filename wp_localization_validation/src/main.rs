use clap::Parser;
use ftl_file::TranslationKey;
use ftl_file::TranslationLanguage;
use std::collections::{HashMap, HashSet};
use wp_localization_parser::TranslationEntry;

use ftl_file::FtlSetupError;

mod ftl_file;

fn main() -> Result<(), FtlSetupError> {
    let args = Args::parse();
    let config = Config {
        default_lang: TranslationLanguage(args.default_lang.clone()),
        strict_mode: args.strict,
    };

    let mut entries_by_language = ftl_file::parse_localization_files(&args.localization_folder)?;
    let default_lang = entries_by_language
        .remove(&config.default_lang)
        .ok_or_else(|| FtlSetupError::DefaultLanguageNotFound(config.default_lang.clone()))?;

    // Handle validation errors
    let errors = find_translation_issues(&default_lang, &entries_by_language);
    print_and_check_errors(&errors, &config);
    Ok(())
}

/// Errors that can occur during validation of translations.
#[derive(Debug, PartialEq, thiserror::Error)]
enum ValidationError {
    /// Found when a translation key exists in a non-default language but not in the default language.
    #[error("Key '{key}' exists in language '{lang_id}' but not in the default language")]
    KeyNotInDefault {
        key: TranslationKey,
        lang_id: TranslationLanguage,
    },

    /// Found when a translation key exists in the default language but is missing in another language.
    #[error("Key '{key}' is missing in language '{lang_id}'")]
    MissingTranslation {
        key: TranslationKey,
        lang_id: TranslationLanguage,
    },

    /// Found when a translation has different placeables than the default language.
    #[error(
        "Key '{key}' in language '{lang_id}' has mismatched placeables. Expected: {expected}, Found: {found}"
    )]
    MismatchedPlaceables {
        key: TranslationKey,
        lang_id: TranslationLanguage,
        expected: String,
        found: String,
    },
}

/// Configuration for the validation process.
#[derive(Debug, Clone)]
struct Config {
    /// The default language code used for validation.
    /// This language is considered the source of truth for translation keys.
    default_lang: TranslationLanguage,
    /// Whether to treat missing translations as errors (true) or warnings (false).
    strict_mode: bool,
}

/// Command line arguments for the validation tool.
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the localization folder containing FTL files
    #[arg(short, long)]
    localization_folder: String,

    /// Enable strict mode (treat missing translations as errors)
    #[arg(short, long, default_value_t = false)]
    strict: bool,

    /// Default language code (e.g., "en-US")
    #[arg(short, long, default_value = "en-US")]
    default_lang: String,
}

impl ValidationError {
    /// Returns true if this error represents a critical issue that must be fixed.
    /// Critical issues are:
    /// 1. Keys that exist in non-default languages but not in the default language
    /// 2. Mismatched placeables between languages
    /// 3. Missing translations (if strict mode is enabled)
    fn is_critical(&self, config: &Config) -> bool {
        match self {
            ValidationError::KeyNotInDefault { .. }
            | ValidationError::MismatchedPlaceables { .. } => true,
            ValidationError::MissingTranslation { .. } => config.strict_mode,
        }
    }
}

/// Finds all translation issues by comparing each language's translations
/// against the default language.
///
/// This function performs a single pass through all languages to check for:
/// 1. Keys that exist in non-default languages but not in the default language
/// 2. Keys that exist in the default language but are missing in other languages
/// 3. Mismatched placeables
#[must_use]
fn find_translation_issues(
    default_lang: &HashMap<TranslationKey, TranslationEntry>,
    map: &HashMap<TranslationLanguage, HashMap<TranslationKey, TranslationEntry>>,
) -> Vec<ValidationError> {
    let default_keys: HashSet<_> = default_lang.keys().collect();
    let mut errors = Vec::new();

    // Check for missing translations and keys not in default language
    errors.extend(map.iter().flat_map(|(lang_id, translations)| {
        let translation_keys: HashSet<_> = translations.keys().collect();

        // Find keys that exist in the default language but are missing in this language
        let missing_keys = default_keys
            .difference(&translation_keys)
            .map(|key| ValidationError::MissingTranslation {
                key: (*key).clone(),
                lang_id: lang_id.clone(),
            })
            .collect::<Vec<_>>();

        // Find keys that exist in this language but not in the default language
        let extra_keys = translation_keys
            .difference(&default_keys)
            .map(|key| ValidationError::KeyNotInDefault {
                key: (*key).clone(),
                lang_id: lang_id.clone(),
            })
            .collect::<Vec<_>>();

        missing_keys.into_iter().chain(extra_keys)
    }));

    // Check for mismatched placeables
    errors.extend(map.iter().flat_map(|(lang_id, translations)| {
        translations.iter().filter_map(|(key, entry)| {
            default_lang.get(key).and_then(|default_entry| {
                let default_placeables: HashSet<_> = default_entry.placeables.iter().collect();
                let current_placeables: HashSet<_> = entry.placeables.iter().collect();

                if default_placeables != current_placeables {
                    let mut expected: Vec<_> =
                        default_placeables.iter().map(|s| s.0.clone()).collect();
                    expected.sort();
                    let mut found: Vec<_> =
                        current_placeables.iter().map(|s| s.0.clone()).collect();
                    found.sort();
                    Some(ValidationError::MismatchedPlaceables {
                        key: key.clone(),
                        lang_id: lang_id.clone(),
                        expected: expected.join(", "),
                        found: found.join(", "),
                    })
                } else {
                    None
                }
            })
        })
    }));

    errors
}

/// Prints validation errors and panics if critical issues were found.
/// See [ValidationError::is_critical] for a list of what constitutes a critical issue.
fn print_and_check_errors(errors: &[ValidationError], config: &Config) {
    let mut has_critical_issues = false;
    for error in errors {
        if error.is_critical(config) {
            has_critical_issues = true;
            eprintln!("Error: {}", error);
        } else {
            eprintln!("Warning: {}", error);
        }
    }
    if has_critical_issues {
        panic!("Critical issues found");
    }
}

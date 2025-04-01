use clap::Parser;
use ftl_file::FtlSetupError;
use itertools::Itertools;
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use wp_localization_parser::TranslationEntry;

mod ftl_file;

fn main() -> Result<(), FtlSetupError> {
    let args = Args::parse();
    let config = Config {
        default_lang: TranslationLanguage(args.default_lang.clone()),
        strict_mode: args.strict,
    };

    let entries_by_language = ftl_file::parse_localization_files(&args.localization_folder)?;
    let (mut entries_by_language, mut validation_errors) =
        find_duplicates_and_convert_to_map(entries_by_language);
    let default_lang = entries_by_language
        .remove(&config.default_lang)
        .ok_or_else(|| FtlSetupError::DefaultLanguageNotFound(config.default_lang.clone()))?;

    validation_errors.extend(ValidationError::find_all_issues(
        &default_lang,
        &entries_by_language,
    ));
    print_and_check_errors(&validation_errors, &config);
    // We only print translation percentages if there are no critical errors since we rely on the
    // number of entries to calculate the percentages
    print_translation_percentages(default_lang.len(), &entries_by_language);
    Ok(())
}

/// Errors that can occur during validation of translations.
#[derive(Debug, PartialEq, thiserror::Error)]
enum ValidationError {
    /// Found when a translation key exists in a non-default language but not in the default language.
    #[error("Following keys exists in language '{lang_id}' but not in the default language: '{}'", keys.iter().join(", "))]
    KeyNotInDefault {
        keys: Vec<TranslationKey>,
        lang_id: TranslationLanguage,
    },

    // Found when a language has duplicate translation keys
    #[error("Duplicate keys found in language '{lang_id}': '{}'", keys.iter().join(", "))]
    DuplicateKeys {
        keys: Vec<TranslationKey>,
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

    /// Found when a translation key exists in the default language but is missing in another language.
    #[error("Key '{}' is missing in language '{lang_id}'", keys.iter().join(", "))]
    MissingTranslations {
        keys: Vec<TranslationKey>,
        lang_id: TranslationLanguage,
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
            | ValidationError::DuplicateKeys { .. }
            | ValidationError::MismatchedPlaceables { .. } => true,
            ValidationError::MissingTranslations { .. } => config.strict_mode,
        }
    }

    fn find_all_issues(
        default_lang: &HashMap<TranslationKey, TranslationEntry>,
        entries_by_language: &HashMap<
            TranslationLanguage,
            HashMap<TranslationKey, TranslationEntry>,
        >,
    ) -> Vec<Self> {
        let default_lang_keys: HashSet<_> = default_lang.keys().collect();
        Self::find_keys_not_in_default_language_issues(&default_lang_keys, entries_by_language)
            .chain(Self::find_mismatched_placeables(
                default_lang,
                entries_by_language,
            ))
            .chain(Self::find_missing_translation_issues(
                &default_lang_keys,
                entries_by_language,
            ))
            .collect()
    }

    fn find_keys_not_in_default_language_issues(
        default_lang_keys: &HashSet<&TranslationKey>,
        entries_by_language: &HashMap<
            TranslationLanguage,
            HashMap<TranslationKey, TranslationEntry>,
        >,
    ) -> impl Iterator<Item = Self> {
        entries_by_language
            .iter()
            .filter_map(|(lang_id, translations)| {
                let translation_keys: HashSet<_> = translations.keys().collect();

                // Find keys that exist in this language but not in the default language
                let keys_not_in_default: Vec<TranslationKey> = translation_keys
                    .difference(default_lang_keys)
                    .cloned()
                    .cloned()
                    .collect();

                if keys_not_in_default.is_empty() {
                    None
                } else {
                    Some(ValidationError::KeyNotInDefault {
                        keys: keys_not_in_default,
                        lang_id: lang_id.clone(),
                    })
                }
            })
    }

    fn find_mismatched_placeables(
        default_lang: &HashMap<TranslationKey, TranslationEntry>,
        entries_by_languages: &HashMap<
            TranslationLanguage,
            HashMap<TranslationKey, TranslationEntry>,
        >,
    ) -> impl Iterator<Item = Self> {
        entries_by_languages
            .iter()
            .flat_map(|(lang_id, translations)| {
                translations.iter().filter_map(|(key, entry)| {
                    default_lang.get(key).and_then(|default_entry| {
                        let default_placeables: HashSet<_> =
                            default_entry.placeables.iter().collect();
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
            })
    }

    fn find_missing_translation_issues(
        default_lang_keys: &HashSet<&TranslationKey>,
        entries_by_languages: &HashMap<
            TranslationLanguage,
            HashMap<TranslationKey, TranslationEntry>,
        >,
    ) -> impl Iterator<Item = Self> {
        entries_by_languages
            .iter()
            .filter_map(|(lang_id, translations)| {
                let translation_keys: HashSet<_> = translations.keys().collect();

                // Find keys that exist in the default language but are missing in this language
                let missing_keys: Vec<TranslationKey> = default_lang_keys
                    .difference(&translation_keys)
                    .cloned()
                    .cloned()
                    .collect();

                if missing_keys.is_empty() {
                    None
                } else {
                    Some(ValidationError::MissingTranslations {
                        keys: missing_keys,
                        lang_id: lang_id.clone(),
                    })
                }
            })
    }
}

#[must_use]
fn find_duplicates_and_convert_to_map(
    entries_by_language: HashMap<TranslationLanguage, Vec<TranslationEntry>>,
) -> (
    HashMap<TranslationLanguage, HashMap<TranslationKey, TranslationEntry>>,
    Vec<ValidationError>,
) {
    let mut errors = Vec::new();
    let map = {
        let mut lang_entries = HashMap::new();
        for (lang_id, ftl_entries) in entries_by_language {
            let mut entries = HashMap::new();
            let mut duplicate_keys = vec![];
            for entry in ftl_entries {
                let key = TranslationKey(entry.key.clone());
                if entries.insert(key.clone(), entry).is_some() {
                    duplicate_keys.push(key);
                }
            }
            if !duplicate_keys.is_empty() {
                errors.push(ValidationError::DuplicateKeys {
                    keys: duplicate_keys,
                    lang_id: lang_id.clone(),
                });
            }
            lang_entries.insert(lang_id, entries);
        }
        lang_entries
    };
    (map, errors)
}

fn print_translation_percentages(
    number_of_entries_in_default_language: usize,
    entries_by_language: &HashMap<TranslationLanguage, HashMap<TranslationKey, TranslationEntry>>,
) {
    entries_by_language
        .iter()
        .for_each(|(lang_id, translations)| {
            let number_of_translation_entries = translations.len();
            println!(
                "Translation percentage for '{lang_id}' is %{:.2}",
                (number_of_translation_entries as f64
                    / number_of_entries_in_default_language as f64)
                    * 100.
            );
        });
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TranslationLanguage(pub String);

impl Display for TranslationLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for TranslationLanguage {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TranslationKey(pub String);

impl Display for TranslationKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for TranslationKey {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use wp_localization_parser::EntryPlaceable;

    use super::*;
    use std::collections::{HashMap, HashSet};

    #[test]
    fn test_find_keys_not_in_default_language_issues() {
        let mut translations = HashMap::new();
        translations.insert(
            "key".into(),
            TranslationEntry {
                placeables: vec![],
                documentation: "".to_string(),
                key: "".to_string(),
            },
        );

        let mut entries_by_language = HashMap::new();
        entries_by_language.insert("tr-TR".into(), translations);

        let issues: Vec<_> = ValidationError::find_keys_not_in_default_language_issues(
            &HashSet::new(),
            &entries_by_language,
        )
        .collect();

        assert_eq!(issues.len(), 1);
        assert!(matches!(issues[0], ValidationError::KeyNotInDefault { .. }));
    }

    #[test]
    fn test_find_mismatched_placeables() {
        let mut default_lang_translations = HashMap::new();
        default_lang_translations.insert(
            "key".into(),
            TranslationEntry {
                documentation: "".to_string(),
                key: "foo".to_string(),
                placeables: vec![EntryPlaceable("bar".to_string())],
            },
        );

        let mut tr_translations = HashMap::new();
        tr_translations.insert(
            "key".into(),
            TranslationEntry {
                documentation: "".to_string(),
                key: "foo".to_string(),
                placeables: vec![EntryPlaceable("baz".to_string())],
            },
        );

        let mut entries_by_language = HashMap::new();
        entries_by_language.insert("tr-TR".into(), tr_translations);

        let issues: Vec<_> = ValidationError::find_mismatched_placeables(
            &default_lang_translations,
            &entries_by_language,
        )
        .collect();

        assert_eq!(issues.len(), 1);
        assert!(matches!(
            issues[0],
            ValidationError::MismatchedPlaceables { .. }
        ));
    }

    #[test]
    fn test_find_missing_translation_issues() {
        let mock_key = "key1".into();
        let mut default_lang_keys = HashSet::new();
        default_lang_keys.insert(&mock_key);

        let mut entries_by_language = HashMap::new();
        entries_by_language.insert("tr-TR".into(), HashMap::new());

        let issues: Vec<_> = ValidationError::find_missing_translation_issues(
            &default_lang_keys,
            &entries_by_language,
        )
        .collect();

        assert_eq!(issues.len(), 1);
        assert!(matches!(
            issues[0],
            ValidationError::MissingTranslations { .. }
        ));
    }

    #[test]
    fn test_find_duplicates_and_convert_to_map() {
        let mut entries_by_language = HashMap::new();
        let entries = vec![
            TranslationEntry {
                documentation: "First entry".to_string(),
                key: "key1".to_string(),
                placeables: vec![],
            },
            TranslationEntry {
                documentation: "Duplicate entry".to_string(),
                key: "key1".to_string(), // Duplicate key
                placeables: vec![],
            },
            TranslationEntry {
                documentation: "Unique entry".to_string(),
                key: "key2".to_string(),
                placeables: vec![],
            },
        ];
        entries_by_language.insert("en-US".into(), entries);

        let (result_map, errors) = find_duplicates_and_convert_to_map(entries_by_language);

        // Check the result map
        assert_eq!(result_map.len(), 1);
        let map = result_map.get(&"en-US".into()).expect(
            "'find_duplicates_and_convert_to_map' function didn't correctly convert to a map",
        );
        assert_eq!(
            map.get(&"key1".into()).map(|t| t.key.as_str()),
            Some("key1")
        );
        assert_eq!(
            map.get(&"key2".into()).map(|t| t.key.as_str()),
            Some("key2")
        );

        // Check that the errors contain the duplicate key
        assert_eq!(errors.len(), 1);
        if let ValidationError::DuplicateKeys { keys, lang_id } = &errors[0] {
            assert_eq!(lang_id.0, "en-US");
            assert_eq!(keys.len(), 1);
            assert_eq!(keys[0].0, "key1"); // Check the duplicate key
        } else {
            panic!("Expected a DuplicateKeys error");
        }
    }
}

use crate::TranslationLanguage;
use std::{collections::HashMap, fmt::Debug, fs};
use wp_localization_parser::TranslationEntry;

/// The file extension for Fluent translation files.
const FTL_EXTENSION: &str = "ftl";

/// Parses all FTL files in the localization directory.
///
/// # Directory Structure
///
/// ```
/// localization/
///   en-US/
///     main.ftl
///   tr-TR/
///     main.ftl
/// ```
///
/// # Returns
///
/// A map of language codes to their translation entries.
pub fn parse_localization_files(
    localization_dir: &str,
) -> Result<HashMap<TranslationLanguage, Vec<TranslationEntry>>, FtlSetupError> {
    let files = fs::read_dir(localization_dir)
        .map_err(FtlSetupError::FileReadError)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir());

    files
        .map(|entry| {
            let lang_id = TranslationLanguage(entry.file_name().to_string_lossy().to_string());
            let ftl_files: Vec<_> = fs::read_dir(entry.path())
                .map_err(FtlSetupError::FileReadError)?
                .filter_map(|entry| entry.ok())
                .filter(|entry| {
                    entry.path().extension().and_then(|ext| ext.to_str()) == Some(FTL_EXTENSION)
                })
                .collect();

            match ftl_files.len() {
                0 => Err(FtlSetupError::NoFtlFiles { lang_id }),
                1 => {
                    let entries = translation_entries(&ftl_files[0].path())?;
                    Ok((lang_id, entries))
                }
                _ => Err(FtlSetupError::MultipleFilesForLanguage { lang_id }),
            }
        })
        .collect()
}

/// Parses a single FTL file.
fn translation_entries(path: &std::path::Path) -> Result<Vec<TranslationEntry>, FtlSetupError> {
    let contents = fs::read_to_string(path).map_err(FtlSetupError::FileReadError)?;
    Ok(wp_localization_parser::parse(contents)
        .map_err(FtlSetupError::FtlParseError)?
        .collect())
}

/// Errors that can occur during setup and initialization of the validation process.
#[derive(thiserror::Error)]
pub enum FtlSetupError {
    #[error("Failed to read file: {0}")]
    FileReadError(#[from] std::io::Error),

    #[error("Failed to parse FTL file: {0}")]
    FtlParseError(#[from] wp_localization_parser::LocalizationFileContentsParsingError),

    #[error("Multiple files found for language '{lang_id}'")]
    MultipleFilesForLanguage { lang_id: TranslationLanguage },

    #[error("Default language '{0}' not found")]
    DefaultLanguageNotFound(TranslationLanguage),

    #[error("No FTL files found in language directory '{lang_id}'")]
    NoFtlFiles { lang_id: TranslationLanguage },
}

// Forward the `Display` trait implementation
impl Debug for FtlSetupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

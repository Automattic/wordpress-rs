use fluent_syntax::ast::{self, Entry, InlineExpression, PatternElement};
use proc_macro2::TokenStream;
use quote::quote;
use serde::Deserialize;
use std::{env, fs};
use syn::{parse_macro_input, DeriveInput, Ident};

pub fn wp_translations(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let original_input = input.clone();
    let translations_ident = parse_macro_input!(input as DeriveInput).ident;
    let config = TranslationsConfig::read_config();
    let entries = parse_translations_file(&config.path);

    proc_macro::TokenStream::from_iter([
        // Since attribute macros completely replace the input, we need to manually preserve it
        original_input,
        bindings::generate_bindings(translations_ident, entries).into(),
    ])
}

#[derive(Debug, Deserialize)]
pub struct TranslationsConfig {
    path: String,
}

impl TranslationsConfig {
    fn read_config() -> TranslationsConfig {
        let file_path = normalize_file_path("wp_translations.toml");
        let contents = match fs::read_to_string(&file_path) {
            Ok(c) => toml::from_str(c.as_str()).unwrap_or_else(|e| {
                panic!(
                    "#[wp_translations] configuration file ('{file_path}'):\n{:#?}",
                    e
                )
            }),
            Err(_) => {
                panic!("#[wp_translations] configuration file is missing: '{file_path}'");
            }
        };
        contents
    }
}

fn parse_translations_file(file_path: &str) -> Vec<TranslationEntry> {
    let file_path = normalize_file_path(file_path);
    let contents =
        fs::read_to_string(file_path).expect("Couldn't read translations file in '{file_path}'");
    let resource = fluent_syntax::parser::parse(contents).unwrap();
    resource
        .body
        .into_iter()
        .flat_map(|e| {
            if let Entry::Message(message) = e {
                let key = message.id.name.to_string();
                let placeables = if let Some(pattern) = message.value {
                    pattern
                        .elements
                        .into_iter()
                        .filter_map(|pattern_element| {
                            if let PatternElement::Placeable { expression } = pattern_element {
                                match expression {
                                    ast::Expression::Select { .. } => {
                                        // Select expressions are not supported yet
                                        None
                                    }
                                    ast::Expression::Inline(inline) => {
                                        if let InlineExpression::VariableReference { id } = inline {
                                            Some(id.name)
                                        } else {
                                            // Only `fluent_syntax::ast::InlineExpression::VariableReference` supported
                                            None
                                        }
                                    }
                                }
                            } else {
                                None
                            }
                        })
                        .collect()
                } else {
                    vec![]
                };
                Some(TranslationEntry { key, placeables })
            } else {
                None
            }
        })
        .collect()
}

fn normalize_file_path(file_path: &str) -> String {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .expect("Crate config can't be found without the `CARGO_MANIFEST_DIR` environment varible");
    format!("{manifest_dir}/{file_path}")
}

#[derive(Debug)]
struct TranslationEntry {
    key: String,
    placeables: Vec<String>,
}

mod bindings {
    use quote::format_ident;

    use super::*;

    pub(super) fn generate_bindings(
        translations_ident: Ident,
        entries: Vec<TranslationEntry>,
    ) -> TokenStream {
        let functions = entries.iter().map(generate_entry_function);
        quote! {
            impl #translations_ident {
                #(#functions)*
            }
        }
    }

    fn generate_entry_function(entry: &TranslationEntry) -> TokenStream {
        let function_name = format_ident!("{}", entry.key);
        let entry_key = format_ident!("{}", entry.key).to_string();
        quote! {
            fn #function_name() -> String {
                crate::localization::localized_message_using_default_locale(#entry_key)
            }
        }
    }
}

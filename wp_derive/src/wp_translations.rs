use convert_case::{Case, Casing};
use fluent_syntax::ast::{self, Entry, InlineExpression, PatternElement};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
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

impl TranslationEntry {
    fn key_as_function_name(&self) -> Ident {
        format_ident!("{}", self.key.to_case(Case::Snake))
    }

    fn key_as_token_string(&self) -> String {
        format_ident!("{}", self.key).to_string()
    }
}

mod bindings {
    use super::*;

    pub(super) fn generate_bindings(
        translations_ident: Ident,
        entries: Vec<TranslationEntry>,
    ) -> TokenStream {
        let functions = entries.iter().map(|e| match e.placeables.len() {
            0 => generate_no_arg_entry_function(e),
            1 => generate_single_arg_entry_function(e),
            _ => generate_multi_arg_entry_function(e),
        });
        quote! {
            impl #translations_ident {
                #(#functions)*
            }
        }
    }

    fn generate_no_arg_entry_function(entry: &TranslationEntry) -> TokenStream {
        let function_name = entry.key_as_function_name();
        let entry_key = entry.key_as_token_string();
        quote! {
            fn #function_name() -> crate::localization::MessageBundle {
                crate::localization::MessageBundle::new(#entry_key, None)
            }
        }
    }

    fn generate_single_arg_entry_function(entry: &TranslationEntry) -> TokenStream {
        let function_name = entry.key_as_function_name();
        let entry_key = entry.key_as_token_string();
        let arg = entry
            .placeables
            .first()
            .expect("Already verified that there is one placeable");
        let arg_ident_as_string = format_ident!("{}", arg).to_string();
        quote! {
            fn #function_name(value: &str) -> String {
                let args = {
                    let mut map = std::collections::HashMap::new();
                    map.insert(#arg_ident_as_string.into(), value.into());
                    map
                };
                crate::localization::localized_message_using_default_locale_with_args(#entry_key, &args)
            }
        }
    }

    fn generate_multi_arg_entry_function(entry: &TranslationEntry) -> TokenStream {
        let function_name = entry.key_as_function_name();
        //let entry_key = entry.key_as_token_string();
        quote! {
            fn #function_name(value: &str) -> String { "placeholder".to_string() }
        }
    }
}

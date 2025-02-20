use convert_case::{Case, Casing};
use fluent_syntax::ast::{self, Entry, InlineExpression, PatternElement};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use serde::Deserialize;
use std::{env, fs};
use syn::{parse_macro_input, DeriveInput, Ident};

const CONFIG_FILE_NAME: &str = "config_wp_messages.toml";

pub fn wp_messages(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let original_input = input.clone();
    let messages_ident = parse_macro_input!(input as DeriveInput).ident;
    let config = Config::read_config();
    let entries = parse_messages_file(&config.path);

    proc_macro::TokenStream::from_iter([
        // Since attribute macros completely replace the input, we need to manually preserve it
        original_input,
        bindings::generate_bindings(messages_ident, entries).into(),
    ])
}

#[derive(Debug, Deserialize)]
struct Config {
    path: String,
}

impl Config {
    fn read_config() -> Config {
        let file_path = normalize_file_path(CONFIG_FILE_NAME);
        let contents = match fs::read_to_string(&file_path) {
            Ok(c) => toml::from_str(c.as_str()).unwrap_or_else(|e| {
                panic!(
                    "#[wp_messages] configuration file ('{file_path}'):\n{:#?}",
                    e
                )
            }),
            Err(_) => {
                panic!("#[wp_messages] configuration file is missing: '{file_path}'");
            }
        };
        contents
    }
}

fn parse_messages_file(file_path: &str) -> Vec<TranslationEntry> {
    let file_path = normalize_file_path(file_path);
    let contents =
        fs::read_to_string(file_path).expect("Couldn't read messages file in '{file_path}'");
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
        messages_ident: Ident,
        entries: Vec<TranslationEntry>,
    ) -> TokenStream {
        let functions = entries.iter().map(|e| {
            if e.placeables.is_empty() {
                generate_argless_function(e)
            } else {
                generate_function(e)
            }
        });
        quote! {
            impl #messages_ident {
                #(#functions)*
            }
        }
    }

    fn generate_argless_function(entry: &TranslationEntry) -> TokenStream {
        let function_name = entry.key_as_function_name();
        let entry_key = entry.key_as_token_string();
        quote! {
            fn #function_name() -> crate::localization::MessageBundle {
                crate::localization::MessageBundle::new(#entry_key, None)
            }
        }
    }

    fn generate_function(entry: &TranslationEntry) -> TokenStream {
        let function_name = entry.key_as_function_name();
        let entry_key = entry.key_as_token_string();
        let args = entry.placeables.iter().map(|placeable| {
            let placeable_ident = format_ident!("{}", placeable);
            quote! {
                #placeable_ident: impl Into<String>,
            }
        });
        let map_inserts = entry.placeables.iter().map(|placeable| {
            let placeable_ident = format_ident!("{}", placeable);
            quote! {
                map.insert(#placeable, #placeable_ident.into());
            }
        });
        quote! {
            fn #function_name(#(#args)*) -> crate::localization::MessageBundle {
                let map = {
                    let mut map = std::collections::HashMap::<&'static str, String>::new();
                    #(#map_inserts)*
                    map
                };
                crate::localization::MessageBundle::new(#entry_key, Some(map))
            }
        }
    }
}

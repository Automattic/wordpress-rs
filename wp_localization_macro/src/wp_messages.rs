use convert_case::{Case, Casing};
use fluent_syntax::ast::{self, Entry, Expression, InlineExpression, PatternElement};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::env;
use syn::{parse_macro_input, DeriveInput, Ident};

include!(concat!(
    env!("OUT_DIR"),
    "/generated_localization_contents.rs"
));

pub fn wp_messages(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let original_input = input.clone();
    let messages_ident = parse_macro_input!(input as DeriveInput).ident;
    let entries = parse_messages_file();

    proc_macro::TokenStream::from_iter([
        // Since attribute macros completely replace the input, we need to manually preserve it
        original_input,
        bindings::generate_bindings(messages_ident, entries).into(),
    ])
}

fn message_pattern_to_documentation(
    message: Option<&fluent_syntax::ast::Pattern<&'static str>>,
) -> Option<String> {
    message.map(|p| {
        p.elements
            .iter()
            .flat_map(|e| match e {
                PatternElement::TextElement { value } => Some(value.to_string()),
                PatternElement::Placeable { expression } => {
                    expression_to_variable_name(expression).map(|v| format!("{{${v}}}"))
                }
            })
            .collect::<Vec<String>>()
            .join("")
    })
}

fn expression_to_variable_name(expression: &Expression<&'static str>) -> Option<String> {
    match expression {
        ast::Expression::Select { .. } => {
            // Select expressions are not supported yet
            None
        }
        ast::Expression::Inline(inline) => {
            if let InlineExpression::VariableReference { id } = inline {
                Some(id.name.to_string())
            } else {
                // Only `fluent_syntax::ast::InlineExpression::VariableReference` supported
                None
            }
        }
    }
}

fn parse_messages_file() -> Vec<TranslationEntry> {
    let resource = fluent_syntax::parser::parse(LOCALIZATION_CONTENTS).unwrap();
    resource
        .body
        .into_iter()
        .flat_map(|e| {
            if let Entry::Message(message) = e {
                // TODO: We are iterating twice, once for documentation, once for placeables
                let documentation = message_pattern_to_documentation(message.value.as_ref());
                let key = message.id.name;
                let placeables = if let Some(pattern) = message.value {
                    pattern
                        .elements
                        .into_iter()
                        .filter_map(|pattern_element| {
                            if let PatternElement::Placeable { expression } = pattern_element {
                                expression_to_variable_name(&expression)
                            } else {
                                None
                            }
                        })
                        .collect()
                } else {
                    vec![]
                };
                Some(TranslationEntry {
                    documentation,
                    key,
                    placeables,
                })
            } else {
                None
            }
        })
        .collect()
}

#[derive(Debug)]
struct TranslationEntry {
    documentation: Option<String>,
    key: &'static str,
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
        let documentation = generate_documentation(entry);
        quote! {
            #documentation
            pub fn #function_name() -> crate::MessageBundle {
                crate::localization::MessageBundle::new(#entry_key, None)
            }
        }
    }

    fn generate_documentation(entry: &TranslationEntry) -> TokenStream {
        if let Some(doc) = &entry.documentation {
            quote! {
                #[doc = #doc]
            }
        } else {
            quote! {}
        }
    }

    fn generate_function(entry: &TranslationEntry) -> TokenStream {
        let function_name = entry.key_as_function_name();
        let entry_key = entry.key_as_token_string();
        let documentation = generate_documentation(entry);
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
            #documentation
            pub fn #function_name(#(#args)*) -> crate::MessageBundle {
                let map = {
                    let mut map = std::collections::HashMap::<&'static str, String>::new();
                    #(#map_inserts)*
                    map
                };
                crate::MessageBundle::new(#entry_key, Some(map))
            }
        }
    }
}

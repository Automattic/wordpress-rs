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
    let entries = parse_messages();

    proc_macro::TokenStream::from_iter([
        // Since attribute macros completely replace the input, we need to manually preserve it
        original_input,
        bindings::generate_bindings(messages_ident, entries).into(),
    ])
}

fn parse_messages() -> impl Iterator<Item = TranslationEntry> {
    let resource = fluent_syntax::parser::parse(LOCALIZATION_CONTENTS)
        .expect("Localization file should be parseable");
    resource.body.into_iter().flat_map(|e| {
        if let Entry::Message(message) = e {
            let mut documentation = String::new();
            let key = message.id.name;
            let placeables = if let Some(pattern) = message.value {
                pattern
                    .elements
                    .into_iter()
                    .filter_map(|pattern_element| match pattern_element {
                        PatternElement::TextElement { value } => {
                            documentation.push_str(value);
                            None
                        }
                        PatternElement::Placeable { expression } => {
                            if let Some(placeable_element) =
                                EntryPlaceable::placeable_from_expression(&expression)
                            {
                                documentation
                                    .push_str(format!("{{${}}}", placeable_element.0).as_str());
                                Some(placeable_element)
                            } else {
                                None
                            }
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
}

#[derive(Debug)]
struct EntryPlaceable(&'static str);

impl EntryPlaceable {
    fn placeable_from_expression(expression: &Expression<&'static str>) -> Option<Self> {
        match expression {
            ast::Expression::Select { .. } => {
                // Select expressions are not supported yet
                None
            }
            ast::Expression::Inline(inline) => {
                if let InlineExpression::VariableReference { id } = inline {
                    Some(Self(id.name))
                } else {
                    // Only `fluent_syntax::ast::InlineExpression::VariableReference` supported
                    None
                }
            }
        }
    }
}

#[derive(Debug)]
struct TranslationEntry {
    documentation: String,
    key: &'static str,
    placeables: Vec<EntryPlaceable>,
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
        entries: impl Iterator<Item = TranslationEntry>,
    ) -> TokenStream {
        let functions = entries.map(|e| {
            if e.placeables.is_empty() {
                generate_argless_function(&e)
            } else {
                generate_function(&e)
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
                crate::MessageBundle::new(#entry_key, None)
            }
        }
    }

    fn generate_documentation(entry: &TranslationEntry) -> TokenStream {
        let doc = &entry.documentation;
        quote! {
            #[doc = #doc]
        }
    }

    fn generate_function(entry: &TranslationEntry) -> TokenStream {
        let function_name = entry.key_as_function_name();
        let entry_key = entry.key_as_token_string();
        let documentation = generate_documentation(entry);
        let args = entry.placeables.iter().map(|placeable| {
            let placeable_ident = format_ident!("{}", placeable.0);
            quote! {
                #placeable_ident: impl Into<String>,
            }
        });
        let map_inserts = entry.placeables.iter().map(|placeable| {
            let placeable_string = placeable.0;
            let placeable_ident = format_ident!("{}", placeable_string);
            quote! {
                map.insert(#placeable_string, #placeable_ident.into());
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
